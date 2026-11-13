use super::{Finding, FindingKind};

use crate::luma::{is_luma_event_id, LumaEvent};
use std::collections::HashMap;

pub(super) fn check_luma_events(
    page_path: &str,
    events: &[LumaEvent],
    findings: &mut Vec<Finding>,
) {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for event in events {
        *counts.entry(event.event_id.as_str()).or_insert(0) += 1;
    }
    for event in events {
        if let Some(Err(problems)) = event.schedule() {
            for problem in problems {
                findings.push(Finding {
                    kind: FindingKind::InvalidEvent,
                    location: String::from(page_path),
                    detail: Some(format!("{}: {problem}", event.event_id)),
                });
            }
        }
        let mut report = |kind| {
            findings.push(Finding {
                kind,
                location: String::from(page_path),
                detail: Some(event.event_id.clone()),
            })
        };
        if !is_luma_event_id(&event.event_id) {
            report(FindingKind::MalformedLumaEventId);
        }
        if event.effective_title().is_none() {
            report(FindingKind::LumaEventWithoutTitle);
        }
        if counts[event.event_id.as_str()] > 1 {
            report(FindingKind::DuplicateLumaEvent);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::check::test_support::*;
    use crate::check::FindingKind;

    fn page_with_luma_events(events: &str) -> String {
        format!(r#"{{"title":"Home","content":"<p>hi</p>","lumaEvents":{events}}}"#)
    }

    #[test]
    fn well_formed_titled_luma_events_have_no_findings() {
        let site = TestSite::new("luma_valid");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            &page_with_luma_events(
                r#"[{"eventId":"evt-a1","title":"Jam","display":"both"},
                    {"eventId":"evt-b2","title":"Gig","buttonLabel":"Sign up"}]"#,
            ),
        );

        let report = site.check();

        assert!(report.findings.is_empty(), "{:?}", report.findings);
    }

    #[test]
    fn malformed_luma_event_id_is_an_error() {
        let site = TestSite::new("luma_malformed");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            &page_with_luma_events(r#"[{"eventId":"https://luma.com/jam","title":"Jam"}]"#),
        );

        let report = site.check();

        assert!(has_kind_at(&report, FindingKind::MalformedLumaEventId, "/"));
        assert!(!report.passed());
    }

    #[test]
    fn luma_event_without_title_is_an_error() {
        let site = TestSite::new("luma_untitled");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            &page_with_luma_events(
                r#"[{"eventId":"evt-a1","title":null},{"eventId":"evt-b2","title":" "}]"#,
            ),
        );

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::LumaEventWithoutTitle), 2);
        assert!(!report.passed());
    }

    #[test]
    fn duplicate_luma_event_is_a_warning_for_each_occurrence() {
        let site = TestSite::new("luma_duplicate");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            &page_with_luma_events(
                r#"[{"eventId":"evt-a1","title":"Jam"},{"eventId":"evt-a1","title":"Jam again"}]"#,
            ),
        );

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::DuplicateLumaEvent), 2);
        assert!(report.passed());
    }
}
