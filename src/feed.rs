//! Atom feeds: a page that turns on `feed` in its page.json publishes one
//! next to itself, listing its dated children newest first. See
//! `vados.allium`'s `Feed`.

use crate::config_files::MainConfig;
use crate::content::get_content;
use crate::files::write_raw;
use crate::page_date::PageDate;
use crate::structure::{Item, Structure};
use askama::Template;
use std::sync::Arc;

pub use crate::config_files::is_site_url;

#[derive(Template)]
#[template(path = "feed.xml", escape = "html")]
struct FeedTemplate<'a> {
    language: &'a str,
    site_url: &'a str,
    title: String,
    page_url: String,
    feed_url: String,
    updated: String,
    author: String,
    entries: Vec<FeedEntry>,
}

struct FeedEntry {
    title: String,
    url: String,
    published: String,
    summary: Option<String>,
    content: String,
}

/// Where a page's feed is published, relative to the site root.
pub(crate) fn feed_href(path: &str) -> String {
    if path == "/" {
        String::from("/feed.xml")
    } else {
        format!("{path}/feed.xml")
    }
}

/// The feed's title: the one page.json gives, or else the site title, joined
/// by the section's own title for any feed but the root's.
pub(crate) fn feed_title(item: &Item, site_title: &str) -> String {
    let given = item
        .feed
        .as_ref()
        .and_then(|f| f.title.as_deref())
        .map(str::trim)
        .filter(|t| !t.is_empty());
    match given {
        Some(t) => String::from(t),
        None if item.path == "/" => String::from(site_title),
        None => format!("{} – {}", site_title, item.title),
    }
}

fn page_url(site_url: &str, path: &str) -> String {
    if path == "/" {
        format!("{site_url}/")
    } else {
        format!("{site_url}{path}")
    }
}

/// Drops the characters XML 1.0 can't hold at all, escaped or not: control
/// characters other than tab, newline and carriage return.
fn xml_safe(text: &str) -> String {
    text.chars()
        .filter(|c| matches!(c, '\t' | '\n' | '\r') || !c.is_control())
        .collect()
}

/// Writes every page's feed. Feeds can only give links in full, so without a
/// usable `siteUrl` none are written; `check` reports that as an error.
pub(crate) fn write_feeds(
    source: &str,
    destination: &str,
    main_config: &MainConfig,
    structure: &Structure,
) {
    let items = structure.get_feed_items();
    if items.is_empty() {
        return;
    }
    let Some(site_url) = main_config.get_site_url() else {
        println!(
            "main.json has no usable siteUrl (an absolute http(s) URL), so no feeds are published."
        );
        return;
    };
    for item in items {
        let xml = render_feed(source, site_url, main_config, structure, &item);
        write_raw(destination, &feed_href(&item.path), xml);
    }
}

fn render_feed(
    source: &str,
    site_url: &str,
    main_config: &MainConfig,
    structure: &Structure,
    item: &Item,
) -> String {
    let limit = item
        .feed
        .as_ref()
        .map(|f| f.get_limit())
        .unwrap_or_default();
    let mut dated: Vec<(PageDate, Arc<Item>)> = structure
        .get_children(&item.path)
        .into_iter()
        .filter_map(|c| c.date.map(|d| (d, c)))
        .collect();
    // Newest first whatever order the section lists its pages in; the sort
    // is stable, so pages with the same date keep their display order.
    dated.sort_by(|(a, _), (b, _)| b.cmp(a));
    dated.truncate(limit as usize);

    // A feed with no entries yet still needs an `updated`; today's date is
    // the honest answer.
    let updated = dated
        .first()
        .map(|(d, _)| *d)
        .unwrap_or_else(PageDate::today);
    let entries = dated
        .iter()
        .map(|(date, child)| FeedEntry {
            title: xml_safe(&child.title),
            url: page_url(site_url, &child.path),
            published: date.to_rfc3339(),
            summary: child
                .summary
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(xml_safe),
            content: xml_safe(&get_content(source, &child.path, &child.content)),
        })
        .collect();

    FeedTemplate {
        language: main_config.get_language(),
        site_url,
        title: xml_safe(&feed_title(item, &main_config.site_title)),
        page_url: page_url(site_url, &item.path),
        feed_url: format!("{site_url}{}", feed_href(&item.path)),
        updated: updated.to_rfc3339(),
        author: xml_safe(&main_config.site_title),
        entries,
    }
    .render()
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_urls_must_be_absolute_http_or_https() {
        assert!(is_site_url("https://example.com"));
        assert!(is_site_url("https://example.com/"));
        assert!(is_site_url("http://localhost:8080/sub/site"));
        assert!(!is_site_url("example.com"));
        assert!(!is_site_url("/blog"));
        assert!(!is_site_url("ftp://example.com"));
        assert!(!is_site_url("https://"));
        assert!(!is_site_url("https://example.com/?x=1"));
        assert!(!is_site_url("https://exa mple.com"));
    }

    #[test]
    fn feeds_sit_next_to_their_page() {
        assert_eq!(feed_href("/"), "/feed.xml");
        assert_eq!(feed_href("/blog"), "/blog/feed.xml");
    }

    #[test]
    fn xml_safe_drops_only_characters_xml_cannot_hold() {
        assert_eq!(xml_safe("a\u{0}b\u{8}c\td\ne é <"), "abc\td\ne é <");
    }
}
