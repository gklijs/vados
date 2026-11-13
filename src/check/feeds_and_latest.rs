use super::{Finding, FindingKind};

use crate::config_files::{PageConfig, PageSelection};
use crate::event::agenda_calendar_href;
use crate::page_date::PageDate;
use std::collections::{HashMap, HashSet};

/// What a page's feed and agenda draw from.
pub(super) fn check_page_selections(
    page_path: &str,
    page_config: &PageConfig,
    pages: &HashMap<String, PageConfig>,
    page_paths: &HashSet<&str>,
    findings: &mut Vec<Finding>,
) {
    let feed = page_config.get_feed();
    let agenda = page_config.get_agenda();
    let sources = feed
        .iter()
        .flat_map(|f| f.from.iter())
        .chain(agenda.iter().flat_map(|a| a.from.iter()))
        .flat_map(|f| f.paths());
    for from in sources {
        if !page_paths.contains(from) {
            findings.push(Finding {
                kind: FindingKind::UnresolvedSelectionSource,
                location: String::from(page_path),
                detail: Some(from.to_string()),
            });
        }
    }
    if feed
        .iter()
        .map(|f| f.depth)
        .chain(agenda.iter().map(|a| a.depth))
        .any(|d| d == Some(0))
    {
        findings.push(Finding {
            kind: FindingKind::SelectionDepthOfZero,
            location: String::from(page_path),
            detail: None,
        });
    }
    if agenda.is_some() {
        let calendar = agenda_calendar_href(page_path);
        let event_page = calendar.strip_suffix(".ics").unwrap_or_default();
        if pages.get(event_page).is_some_and(|p| p.event.is_some()) {
            findings.push(Finding {
                kind: FindingKind::CalendarFileCollision,
                location: String::from(event_page),
                detail: Some(calendar),
            });
        }
    }
    if let Some(feed) = &feed {
        let selection: PageSelection = feed.selection(page_path);
        let has_entry = pages.iter().any(|(path, page)| {
            path != page_path
                && selection.selects(path, page.listed.unwrap_or(true))
                && page
                    .date
                    .as_deref()
                    .is_some_and(|d| d.parse::<PageDate>().is_ok())
        });
        if !has_entry {
            findings.push(Finding {
                kind: FindingKind::EmptyFeed,
                location: String::from(page_path),
                detail: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::check::test_support::*;

    #[test]
    fn feed_with_a_usable_site_url_and_dated_posts_has_no_findings() {
        let site = feed_site(
            "feed_ok",
            &main_json_with_site_url("https://example.com"),
            DATED_POST,
        );

        let report = site.check();

        assert!(report.findings.is_empty(), "{:?}", report.findings);
    }
}
