use super::{unknown_icon_detail, Finding, FindingKind};

use crate::config_files::MenuConfig;
use crate::icons::is_known_icon;
use crate::structure::is_recognized_social_provider;
use std::collections::HashSet;

pub(super) fn check_menu(
    menu_config: &MenuConfig,
    page_paths: &HashSet<&str>,
    findings: &mut Vec<Finding>,
) {
    for item in &menu_config.main_menu {
        if item.url.starts_with("https://") {
            if item.title.is_none() {
                findings.push(Finding {
                    kind: FindingKind::ExternalMenuLinkWithoutTitle,
                    location: item.url.clone(),
                    detail: None,
                });
            }
        } else if !page_paths.contains(item.url.as_str()) {
            findings.push(Finding {
                kind: FindingKind::UnresolvedMenuLink,
                location: item.url.clone(),
                detail: None,
            });
        }
    }
    let menu_icons = menu_config
        .main_menu
        .iter()
        .map(|i| (&i.url, &i.icon))
        .chain(menu_config.socials.iter().map(|s| (&s.url, &s.icon)));
    for (url, icon) in menu_icons {
        // A recognised provider's icon is its own, whatever menu.json says.
        let shown = !menu_config.socials.iter().any(|s| &s.url == url)
            || !is_recognized_social_provider(url);
        if let Some(icon) = icon.as_ref().filter(|i| shown && !is_known_icon(i)) {
            findings.push(Finding {
                kind: FindingKind::UnknownIcon,
                location: url.clone(),
                detail: Some(unknown_icon_detail(icon)),
            });
        }
    }
    for social in &menu_config.socials {
        if !is_recognized_social_provider(&social.url)
            && (social.icon.is_none() || social.color.is_none() || social.label.is_none())
        {
            findings.push(Finding {
                kind: FindingKind::UnrecognisedSocialLinkIncomplete,
                location: social.url.clone(),
                detail: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::check::test_support::*;
    use crate::check::{FindingKind, Severity};

    #[test]
    fn unresolved_internal_menu_link_is_a_warning() {
        let site = TestSite::new("unresolved_menu_link");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[{"url":"/missing","title":null,"icon":null}],"socials":[]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnresolvedMenuLink,
            "/missing"
        ));
        assert_eq!(
            FindingKind::UnresolvedMenuLink.severity(),
            Severity::Warning
        );
        assert!(report.passed());
    }

    #[test]
    fn external_menu_link_without_title_is_an_error() {
        let site = TestSite::new("external_link_no_title");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[{"url":"https://example.com","title":null,"icon":null}],"socials":[]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::ExternalMenuLinkWithoutTitle,
            "https://example.com"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn unrecognised_social_provider_missing_icon_and_color_is_an_error() {
        let site = TestSite::new("social_unrecognised");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://mastodon.social/@someone","icon":null,"color":null}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnrecognisedSocialLinkIncomplete,
            "https://mastodon.social/@someone"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn unrecognised_social_provider_missing_only_the_label_is_an_error() {
        // icon and color are given; the accessible-name label is not. It is
        // just as required as the other two -- see
        // vados.allium's UnrecognisedSocialLinkIsFullyDescribed.
        let site = TestSite::new("social_missing_label");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://mastodon.social/@someone","icon":"mastodon","color":"6364FF"}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnrecognisedSocialLinkIncomplete,
            "https://mastodon.social/@someone"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn unrecognised_social_provider_with_icon_color_and_label_is_not_flagged() {
        let site = TestSite::new("social_fully_described");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://mastodon.social/@someone","icon":"mastodon","color":"6364FF","label":"Mastodon"}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::UnrecognisedSocialLinkIncomplete),
            0
        );
        assert!(report.passed());
    }

    #[test]
    fn recognized_social_provider_without_icon_is_not_flagged() {
        let site = TestSite::new("social_recognized");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://github.com/someone","icon":null,"color":null}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::UnrecognisedSocialLinkIncomplete),
            0
        );
        assert!(report.passed());
    }
}
