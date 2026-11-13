use super::{Finding, FindingKind};

use crate::content::MarkdownOptions;
use crate::content::{classify_content_reference, get_content, get_file_path, ContentKind};
use crate::link_check::page_links_in;
use std::collections::HashSet;
use std::path::Path;

/// Links in a page's own content to pages that don't exist, without needing a
/// build. Only addresses that can only be a page: anything naming a file could
/// be a static file or something `generate` writes, which `--output` checks.
pub(super) fn check_content_links(
    source: &str,
    path: &str,
    location: &str,
    reference: &str,
    link_targets: &HashSet<&str>,
    findings: &mut Vec<Finding>,
) {
    let readable = match classify_content_reference(reference) {
        ContentKind::Unrecognised => false,
        ContentKind::InlineHtml => true,
        ContentKind::MarkdownFile | ContentKind::HtmlFile => {
            Path::new(&get_file_path(source, path, reference)).exists()
        }
    };
    if !readable {
        return;
    }
    for target in page_links_in(&get_content(
        source,
        path,
        reference,
        MarkdownOptions::default(),
    )) {
        let page = target.trim_end_matches('/');
        let page = if page.is_empty() { "/" } else { page };
        if !link_targets.contains(page) {
            findings.push(Finding {
                kind: FindingKind::BrokenContentLink,
                location: String::from(location),
                detail: Some(target),
            });
        }
    }
}
