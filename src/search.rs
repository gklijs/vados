//! The optional site search: an index of pages and a small script, both
//! written by `generate` when `main.json` turns `search` on. See
//! `vados.allium`'s `SiteSearch`.

use crate::config_files::{PageConfig, SearchConfig};
use crate::content::get_content;
use crate::content::MarkdownOptions;
use crate::files::write_raw;
use crate::structure::{Item, Structure};
use crate::ui_text::UiText;
use serde::Serialize;

/// Where the search lives, unless the site brings a page of its own there.
pub const SEARCH_PATH: &str = "/search";
pub(crate) const SEARCH_INDEX_HREF: &str = "/search-index.json";
pub(crate) const SEARCH_SCRIPT_HREF: &str = "/js/vados-search.js";
/// How much of a page without a `summary` goes in the index.
const FALLBACK_TEXT_LENGTH: usize = 300;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchRecord {
    title: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    /// Sorts newest first; `dateText` is what a reader sees.
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date_text: Option<String>,
    /// The title of the top-level section the page is in, as a label.
    #[serde(skip_serializing_if = "Option::is_none")]
    section: Option<String>,
}

/// The page to publish at `/search` when the site has none there: unlisted
/// and kept out of search engines, like any page of search results.
pub(crate) fn generated_search_page(ui: &UiText) -> Item {
    let mut config = PageConfig::new(SEARCH_PATH);
    config.title = String::from(ui.search_title);
    config.content = format!("<p>{}</p>", ui.search_intro);
    config.listed = Some(false);
    config.noindex = Some(true);
    Item::new(String::from(SEARCH_PATH), config)
}

/// Text with its markup taken out and its whitespace collapsed.
pub(crate) fn plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        text.push_str(&rest[..start]);
        text.push(' ');
        let tag = &rest[start..];
        // What's inside these isn't text anyone reads.
        let skipped = ["script", "style"].iter().find_map(|name| {
            let lower = tag.get(1..=name.len())?.to_ascii_lowercase();
            (lower == *name).then(|| {
                let close = format!("</{name}");
                tag.to_ascii_lowercase()
                    .find(&close)
                    .map(|end| end + close.len())
            })?
        });
        let from = skipped.unwrap_or(0);
        match tag[from..].find('>') {
            Some(end) => rest = &tag[from + end + 1..],
            None => {
                rest = "";
                break;
            }
        }
    }
    text.push_str(rest);
    let text = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn shortened(text: String) -> String {
    match text.char_indices().nth(FALLBACK_TEXT_LENGTH) {
        Some((end, _)) => format!("{}…", text[..end].trim_end()),
        None => text,
    }
}

fn record(source: &str, item: &Item, structure: &Structure, ui: &UiText) -> SearchRecord {
    let summary = item
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let text = summary.unwrap_or_else(|| {
        shortened(plain_text(&get_content(
            source,
            &item.path,
            &item.content,
            MarkdownOptions::default(),
        )))
    });
    let section = item
        .path
        .split('/')
        .nth(1)
        .filter(|s| !s.is_empty())
        .map(|s| format!("/{s}"))
        .filter(|top| *top != item.path)
        .and_then(|top| structure.find_item(&top))
        .map(|top| top.title.clone());
    SearchRecord {
        title: item.title.clone(),
        url: item.path.clone(),
        text: Some(text).filter(|t| !t.is_empty()),
        date: item.date.map(|d| d.to_rfc3339()),
        date_text: item.date.map(|d| ui.format_date(&d)),
        section,
    }
}

/// The search index as published: every selected page, in path order so that
/// an unchanged site gives an unchanged file.
pub(crate) fn search_index(
    source: &str,
    config: &SearchConfig,
    structure: &Structure,
    ui: &UiText,
) -> String {
    let mut items = structure.get_searchable_items(config);
    items.sort_by(|a, b| a.path.cmp(&b.path));
    let records: Vec<SearchRecord> = items
        .iter()
        .map(|item| record(source, item, structure, ui))
        .collect();
    serde_json::to_string(&records).expect("search records always serialise")
}

/// Writes the index and the script.
pub(crate) fn write_search(
    source: &str,
    destination: &crate::files::Output,
    config: &SearchConfig,
    structure: &Structure,
    ui: &UiText,
) {
    write_raw(
        destination,
        SEARCH_INDEX_HREF,
        search_index(source, config, structure, ui),
    );
    write_raw(
        destination,
        SEARCH_SCRIPT_HREF,
        include_str!("../assets/js/search.js"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_drops_markup_and_what_scripts_and_styles_hold() {
        assert_eq!(
            plain_text("<h2 class=\"x\">Café &amp; bar</h2>\n<script>let a = '<p>no</p>'</script><p>Open <em>late</em></p><style>p{}</style>end"),
            "Café & bar Open late end"
        );
    }

    #[test]
    fn plain_text_copes_with_an_unclosed_tag() {
        assert_eq!(plain_text("before <p"), "before");
    }

    #[test]
    fn long_text_is_shortened_on_a_character_boundary() {
        let long = "é".repeat(FALLBACK_TEXT_LENGTH + 5);
        let short = shortened(long);
        assert_eq!(short.chars().count(), FALLBACK_TEXT_LENGTH + 1);
        assert!(short.ends_with('…'));
    }
}
