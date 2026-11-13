use super::{Finding, FindingKind};

use std::path::Path;

pub(super) fn read_json<T: serde::de::DeserializeOwned>(
    path: &str,
    unreadable_kind: FindingKind,
    findings: &mut Vec<Finding>,
) -> Option<T> {
    let result = std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|bytes| {
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            findings.extend(
                crate::config_keys::unknown_keys(Path::new(path), &value)
                    .into_iter()
                    .map(|detail| Finding {
                        kind: FindingKind::UnknownConfigKey,
                        location: path.to_owned(),
                        detail: Some(detail),
                    }),
            );
            // Report all missing top-level required fields together. Nested
            // values and types are still validated by the actual serde types.
            let required: &[&str] = match unreadable_kind {
                FindingKind::SiteConfigUnreadable => &["siteTitle", "footerContent"],
                FindingKind::MenuUnreadable => &["mainMenu", "socials"],
                _ => &[],
            };
            if let Some(object) = value.as_object() {
                let missing: Vec<_> = required
                    .iter()
                    .filter(|key| !object.contains_key(**key))
                    .copied()
                    .collect();
                if !missing.is_empty() {
                    return Err(format!("missing required fields: {}", missing.join(", ")));
                }
            }
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())
        });
    match result {
        Ok(value) => Some(value),
        Err(detail) => {
            findings.push(Finding {
                kind: unreadable_kind,
                location: String::from(path),
                detail: Some(detail),
            });
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::check::test_support::*;
    use crate::check::FindingKind;

    #[test]
    fn missing_main_and_menu_json_are_errors() {
        let site = TestSite::new("missing_main_menu");
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::SiteConfigUnreadable), 1);
        assert_eq!(count_kind(&report, FindingKind::MenuUnreadable), 1);
        assert!(!report.passed());
        assert_eq!(report.error_count(), 2);
    }
}
