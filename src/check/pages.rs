use super::config::read_json;
use super::{checked_directories, Finding, FindingKind};

use crate::config_files::{ChildOrder, MainConfig, MenuConfig, Notification, PageConfig};
use crate::content::{classify_content_reference, get_file_path, ContentKind};
use crate::files::get_page_directory_paths;
use crate::page_date::PageDate;
use crate::structure::parent_path;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(super) fn collect_pages(
    source: &str,
    static_dir: Option<&str>,
    findings: &mut Vec<Finding>,
) -> HashMap<String, PageConfig> {
    let mut pages = HashMap::new();
    for dir in checked_directories(get_page_directory_paths(source, static_dir), findings) {
        let page_path = if dir == source {
            String::from("/")
        } else {
            dir[source.len()..].to_string()
        };
        let page_json = format!("{}/page.json", dir);
        let config = if !Path::new(&page_json).try_exists().unwrap_or(true) {
            PageConfig::new(&dir)
        } else {
            read_json(&page_json, FindingKind::PageConfigUnreadable, findings)
                .unwrap_or_else(|| PageConfig::new(&dir))
        };
        pages.insert(page_path, config);
    }
    pages
}

/// Inspect the same fenced blocks as the renderer, without loading Mermaid.
pub(super) fn check_mermaid_reference(
    source: &str,
    path: &str,
    reference: &str,
    config: Option<&MainConfig>,
    findings: &mut Vec<Finding>,
) {
    use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
    if classify_content_reference(reference) != ContentKind::MarkdownFile {
        return;
    }
    let file = get_file_path(source, path, reference);
    let Ok(text) = std::fs::read_to_string(&file) else {
        return;
    };
    let mut body: Option<String> = None;
    let mut index = 0;
    for event in Parser::new(&text) {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))
                if crate::mermaid::is_mermaid(&info) =>
            {
                body = Some(String::new())
            }
            Event::Text(text) if body.is_some() => body.as_mut().unwrap().push_str(&text),
            Event::End(TagEnd::CodeBlock) if body.is_some() => {
                index += 1;
                let body = body.take().unwrap();
                let mut push = |kind| {
                    findings.push(Finding {
                        kind,
                        location: file.clone(),
                        detail: Some(format!("diagram {index}")),
                    })
                };
                if body.trim().is_empty() {
                    push(FindingKind::EmptyMermaidDiagram);
                }
                let has_alternative = body.lines().map(str::trim).any(|line| {
                    ["accTitle:", "accDescr:"].iter().any(|key| {
                        line.strip_prefix(key)
                            .is_some_and(|text| !text.trim().is_empty())
                    }) || line.strip_prefix("accDescr").is_some_and(|rest| {
                        rest.trim_start().starts_with('{') && !rest.trim_start().starts_with("{}")
                    })
                });
                if !has_alternative {
                    push(FindingKind::MermaidWithoutAlternativeText);
                }
                if config.is_some_and(|c| !c.renders_mermaid()) {
                    push(FindingKind::MermaidDisabled);
                }
            }
            _ => {}
        }
    }
}

pub(super) fn check_content_reference(
    source: &str,
    location: &str,
    reference: &str,
    findings: &mut Vec<Finding>,
) {
    match classify_content_reference(reference) {
        ContentKind::Unrecognised => findings.push(Finding {
            kind: FindingKind::UnrecognisedContentReference,
            location: String::from(location),
            detail: Some(String::from(reference)),
        }),
        ContentKind::InlineHtml => {}
        ContentKind::MarkdownFile | ContentKind::HtmlFile => {
            let file_path = get_file_path(source, location, reference);
            if !Path::new(&file_path).exists() {
                findings.push(Finding {
                    kind: FindingKind::MissingContentFile,
                    location: String::from(location),
                    detail: Some(String::from(reference)),
                });
            }
        }
    }
}

pub(super) fn check_notification(
    source: &str,
    page_path: &str,
    notification: &Notification,
    page_paths: &HashSet<&str>,
    image_keys: &HashSet<&str>,
    findings: &mut Vec<Finding>,
) {
    check_content_reference(source, page_path, &notification.content, findings);
    if let Some(image_ref) = &notification.image {
        if !image_keys.contains(image_ref.as_str()) {
            findings.push(Finding {
                kind: FindingKind::UnknownNotificationImageReference,
                location: String::from(page_path),
                detail: Some(image_ref.clone()),
            });
        }
    }
    if let Some(url) = &notification.url {
        if url.starts_with('/') && !page_paths.contains(url.as_str()) {
            findings.push(Finding {
                kind: FindingKind::DeadInternalNotificationLink,
                location: String::from(page_path),
                detail: Some(url.clone()),
            });
        }
    }
}

pub(super) fn check_page_date(
    page_path: &str,
    page_config: &PageConfig,
    pages: &HashMap<String, PageConfig>,
    findings: &mut Vec<Finding>,
) {
    if let Some(updated) = &page_config.updated {
        if updated.parse::<PageDate>().is_err() {
            findings.push(Finding {
                kind: FindingKind::UnreadablePageDate,
                location: String::from(page_path),
                detail: Some(format!("updated: {updated}")),
            });
        }
    }
    match &page_config.date {
        Some(date) => {
            if date.parse::<PageDate>().is_err() {
                findings.push(Finding {
                    kind: FindingKind::UnreadablePageDate,
                    location: String::from(page_path),
                    detail: Some(date.clone()),
                });
            }
            let has_summary = page_config
                .summary
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty());
            if !has_summary {
                findings.push(Finding {
                    kind: FindingKind::DatedPageWithoutSummary,
                    location: String::from(page_path),
                    detail: None,
                });
            }
        }
        None => {
            let parent = parent_path(page_path).and_then(|p| pages.get(&p));
            let parent_orders_by_date = parent
                .and_then(|p| p.child_order)
                .is_some_and(|o| o != ChildOrder::Order);
            // A feed that names its sources picks its pages, and undated
            // ones under it were never meant for it.
            let parent_publishes_feed = parent
                .and_then(PageConfig::get_feed)
                .is_some_and(|f| f.from.is_none());
            // One warning per undated page: being left out of a feed is the
            // more consequential of the two.
            let kind = if parent_publishes_feed {
                Some(FindingKind::UndatedPageLeftOutOfFeed)
            } else if parent_orders_by_date {
                Some(FindingKind::UndatedPageInDateOrderedSection)
            } else {
                None
            };
            if let Some(kind) = kind {
                findings.push(Finding {
                    kind,
                    location: String::from(page_path),
                    detail: None,
                });
            }
        }
    }
}

/// A page kept out of search engines but still advertised on the site is
/// probably a mistake, though sometimes meant, so it's only a warning.
pub(super) fn check_noindex(
    page_path: &str,
    page_config: &PageConfig,
    menu_config: Option<&MenuConfig>,
    findings: &mut Vec<Finding>,
) {
    if page_config.noindex != Some(true) {
        return;
    }
    let in_main_menu = menu_config.is_some_and(|m| m.main_menu.iter().any(|i| i.url == page_path));
    let listed = page_config.listed.unwrap_or(true) && page_path != "/";
    if in_main_menu || listed {
        findings.push(Finding {
            kind: FindingKind::NoindexedPageIsListed,
            location: String::from(page_path),
            detail: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::check::test_support::*;
    use crate::check::FindingKind;

    #[test]
    fn page_config_unreadable_is_an_error_but_check_continues() {
        let site = TestSite::new("page_unreadable");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", "{ not valid json");

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::PageConfigUnreadable), 1);
        assert!(!report.passed());
        // The unreadable page.json still yields a page (falling back to the
        // directory-derived default), so the footer's content reference is
        // still checked rather than the whole run being stranded.
        assert_eq!(
            count_kind(&report, FindingKind::UnrecognisedContentReference),
            0
        );
    }

    #[test]
    fn unrecognised_content_reference_is_an_error() {
        let site = TestSite::new("content_unrecognised");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"not-a-recognised-reference","order":null,
               "leftNotifications":null,"rightNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnrecognisedContentReference,
            "/"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn missing_content_file_is_a_warning() {
        let site = TestSite::new("content_missing_file");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"missing.md","order":null,
               "leftNotifications":null,"rightNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(&report, FindingKind::MissingContentFile, "/"));
        assert!(report.passed());
    }

    #[test]
    fn unknown_notification_image_reference_is_a_warning() {
        let site = TestSite::new("notification_image_unknown");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"<p>hi</p>","order":null,
               "leftNotifications":[{"content":"<p>n</p>","title":null,"image":"/nope","url":null,"color":null}],
               "rightNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnknownNotificationImageReference,
            "/"
        ));
        assert!(report.passed());
    }

    #[test]
    fn dead_internal_notification_link_is_a_warning() {
        let site = TestSite::new("notification_dead_link");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"<p>hi</p>","order":null,
               "rightNotifications":[{"content":"<p>n</p>","title":null,"image":null,"url":"/missing","color":null}],
               "leftNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::DeadInternalNotificationLink,
            "/"
        ));
        assert!(report.passed());
    }

    #[test]
    fn dated_page_without_summary_is_a_warning_and_unreadable_updated_an_error() {
        let site = date_ordered_blog(
            r#"{"title":"Post","content":"<p>hi</p>","date":"2026-10-22","updated":"soon"}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::DatedPageWithoutSummary,
            "/blog/post"
        ));
        assert!(has_kind_at(
            &report,
            FindingKind::UnreadablePageDate,
            "/blog/post"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn dated_page_in_a_date_ordered_section_has_no_findings() {
        let site = date_ordered_blog(
            r#"{"title":"Post","content":"<p>hi</p>","date":"2026-10-22T09:30","summary":"Hi"}"#,
        );

        let report = site.check();

        assert!(report.findings.is_empty(), "{:?}", report.findings);
    }

    #[test]
    fn unreadable_page_date_is_an_error() {
        let site =
            date_ordered_blog(r#"{"title":"Post","content":"<p>hi</p>","date":"22-10-2026"}"#);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnreadablePageDate,
            "/blog/post"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn undated_page_in_a_date_ordered_section_is_a_warning() {
        let site = date_ordered_blog(r#"{"title":"Post","content":"<p>hi</p>"}"#);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UndatedPageInDateOrderedSection,
            "/blog/post"
        ));
        assert!(report.passed());
    }

    #[test]
    fn undated_page_under_an_order_ordered_parent_has_no_findings() {
        let site = TestSite::new("undated_order_parent");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_source(
            "about/page.json",
            r#"{"title":"About","content":"<p>hi</p>"}"#,
        );

        assert!(site.check().findings.is_empty());
    }

    #[test]
    fn undated_page_under_a_feed_gets_one_warning_about_the_feed() {
        let site = feed_site(
            "feed_undated",
            &main_json_with_site_url("https://example.com"),
            r#"{"title":"Post","content":"<p>hi</p>"}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UndatedPageLeftOutOfFeed,
            "/blog/post"
        ));
        assert_eq!(
            count_kind(&report, FindingKind::UndatedPageInDateOrderedSection),
            0
        );
        assert!(report.passed());
    }
}
