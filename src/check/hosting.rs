use super::{Finding, FindingKind};

use crate::config_files::MainConfig;
use crate::files::{get_static_files, static_dir_path};
use crate::link_check::headers_file_findings;
use std::collections::HashSet;

pub(super) fn check_static_dir(
    source: &str,
    main_config: &MainConfig,
    page_paths: &HashSet<&str>,
    findings: &mut Vec<Finding>,
) {
    if let Some(static_dir) = &main_config.static_dir {
        if !static_dir_path(source, static_dir).is_dir() {
            findings.push(Finding {
                kind: FindingKind::MissingStaticDir,
                location: format!("{}/main.json", source),
                detail: Some(static_dir.clone()),
            });
        }
        let static_headers = static_dir_path(source, static_dir).join("_headers");
        findings.extend(headers_file_findings(&static_headers));
        let mut shadowed: Vec<(String, String)> = get_static_files(source, static_dir)
            .into_iter()
            .filter_map(|(published, from)| {
                let page = match published.as_str() {
                    "/index.html" => "/",
                    "/index.md" if main_config.markdown == Some(true) => "/",
                    p => p.strip_suffix(".html").or_else(|| {
                        (main_config.markdown == Some(true))
                            .then(|| p.strip_suffix(".md"))
                            .flatten()
                    })?,
                };
                page_paths
                    .contains(page)
                    .then(|| (from.display().to_string(), String::from(page)))
            })
            .collect();
        shadowed.sort();
        for (file, page) in shadowed {
            findings.push(Finding {
                kind: FindingKind::StaticFileShadowedByPage,
                location: file,
                detail: Some(page),
            });
        }
    }
}
