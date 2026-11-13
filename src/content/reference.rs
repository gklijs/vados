//! Content references: how a page's, notification's or footer's `content`
//! resolves, and reading it as HTML.

use super::MarkdownOptions;
use crate::highlight::highlight_code_blocks;
use pulldown_cmark::{html, Event, HeadingLevel, Parser, Tag};
use std::fs;

/// How a content reference (a page's `content`, a notification's `content`,
/// or the site's `footerContent`) resolves. Shared between rendering, which
/// panics on `Unrecognised`, and `check`, which reports it as a finding
/// instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContentKind {
    MarkdownFile,
    HtmlFile,
    InlineHtml,
    Unrecognised,
}

pub(crate) fn classify_content_reference(reference: &str) -> ContentKind {
    if reference.ends_with(".md") {
        ContentKind::MarkdownFile
    } else if reference.ends_with(".html") {
        ContentKind::HtmlFile
    } else if reference.ends_with('>') {
        ContentKind::InlineHtml
    } else {
        ContentKind::Unrecognised
    }
}

/// Escapes the characters that would otherwise be read as markup when
/// dropped straight into an HTML attribute or element.
pub(crate) fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Coerces free-form text into something that will actually classify as a
/// `ContentReference` rather than `Unrecognised`. Used wherever a
/// content-authoring command writes plain descriptive text -- a notification
/// caption, or an image's own alt text standing in for one -- into a field
/// that `check`/`generate` will read back as a content reference. Text that
/// already looks like one (a `.md`/`.html` file, or inline HTML ending in a
/// closing tag) passes through unchanged, so a maintainer who deliberately
/// names a file still gets that file, not a paragraph containing its name.
pub(crate) fn ensure_content_reference(text: String) -> String {
    match classify_content_reference(&text) {
        ContentKind::Unrecognised => format!("<p>{}</p>", escape_html(&text)),
        ContentKind::MarkdownFile | ContentKind::HtmlFile | ContentKind::InlineHtml => text,
    }
}

pub(crate) fn get_file_path(source: &str, path: &str, reference: &str) -> String {
    if path == "/" {
        format!("{}/{}", source, reference)
    } else {
        format!("{}{}/{}", source, path, reference)
    }
}

fn md_to_content(file_path: &str, options: MarkdownOptions) -> String {
    match fs::read_to_string(file_path) {
        Ok(text) => {
            let mut html_output: String = String::with_capacity(text.len() * 3 / 2);
            let parser = crate::mermaid::diagram_blocks(Parser::new(&text), options.mermaid);
            if options.highlight {
                html::push_html(&mut html_output, highlight_code_blocks(parser));
            } else {
                html::push_html(&mut html_output, parser);
            }
            html_output
        }
        Err(e) => {
            println!(
                "There was an error reading Markdown content from path {}.\n{}",
                file_path, e
            );
            String::from("")
        }
    }
}

/// Whether a page's content has a top-level heading of its own: a `#`
/// heading in Markdown, an `<h1>` in HTML. The page title is already the
/// page's `<h1>`, so this would make a second. A file that can't be read has
/// none; `check` reports it missing separately.
pub(crate) fn has_top_level_heading(source: &str, path: &str, reference: &str) -> bool {
    let file_path = get_file_path(source, path, reference);
    match classify_content_reference(reference) {
        ContentKind::MarkdownFile => fs::read_to_string(file_path).is_ok_and(|text| {
            Parser::new(&text).any(|e| {
                matches!(
                    e,
                    Event::Start(Tag::Heading {
                        level: HeadingLevel::H1,
                        ..
                    })
                )
            })
        }),
        ContentKind::HtmlFile => fs::read_to_string(file_path).is_ok_and(|text| html_has_h1(&text)),
        ContentKind::InlineHtml => html_has_h1(reference),
        ContentKind::Unrecognised => false,
    }
}

fn html_has_h1(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("<h1>") || lower.contains("<h1 ")
}

fn html_to_content(file_path: &str) -> String {
    match fs::read_to_string(file_path) {
        Ok(text) => text,
        Err(e) => {
            println!(
                "There was an error reading Html content from path {}.\n{}",
                file_path, e
            );
            String::from("")
        }
    }
}

/// The content a reference names, as HTML, with the requested Markdown transforms.
pub(crate) fn get_content(
    source: &str,
    path: &str,
    reference: &str,
    options: MarkdownOptions,
) -> String {
    let file_path = get_file_path(source, path, reference);
    match classify_content_reference(reference) {
        ContentKind::MarkdownFile => md_to_content(&file_path, options),
        ContentKind::HtmlFile => html_to_content(&file_path),
        ContentKind::InlineHtml => String::from(reference),
        // Generation rejects this during preflight. Keep internal rendering
        // callers safe as well, displaying arbitrary text without markup.
        ContentKind::Unrecognised => format!("<p>{}</p>", escape_html(reference)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_content_reference_leaves_a_markdown_reference_untouched() {
        assert_eq!(ensure_content_reference("notes.md".to_string()), "notes.md");
    }

    #[test]
    fn ensure_content_reference_leaves_an_html_reference_untouched() {
        assert_eq!(
            ensure_content_reference("notes.html".to_string()),
            "notes.html"
        );
    }

    #[test]
    fn ensure_content_reference_leaves_inline_html_untouched() {
        assert_eq!(
            ensure_content_reference("<p>already html</p>".to_string()),
            "<p>already html</p>"
        );
    }

    #[test]
    fn ensure_content_reference_wraps_plain_text_as_inline_html() {
        assert_eq!(
            ensure_content_reference("Alice smiling".to_string()),
            "<p>Alice smiling</p>"
        );
    }

    #[test]
    fn ensure_content_reference_escapes_markup_characters_in_plain_text() {
        // Deliberately doesn't end in '>' -- text that does is already
        // (however loosely) classified as inline HTML and left untouched.
        assert_eq!(
            ensure_content_reference("Rock & roll".to_string()),
            "<p>Rock &amp; roll</p>"
        );
    }
}
