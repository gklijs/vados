//! Browser-rendered Mermaid diagrams, with readable source as the fallback.
use crate::content::escape_html;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag, TagEnd};

pub(crate) const DEFAULT_SCRIPT: &str =
    "https://cdn.jsdelivr.net/npm/mermaid@12.3.0/dist/mermaid.esm.min.mjs";
pub(crate) const LOADER_PATH: &str = "/js/vados-mermaid.js";
pub(crate) const MARKER: &str = "<div class=\"vados-mermaid\">";

pub(crate) fn is_mermaid(info: &str) -> bool {
    info.split([' ', ',', '{', '\t']).next() == Some("mermaid")
}

pub(crate) fn diagram_blocks<'a>(
    events: impl Iterator<Item = Event<'a>>,
    enabled: bool,
) -> impl Iterator<Item = Event<'a>> {
    let mut source: Option<String> = None;
    events.filter_map(move |event| match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info)))
            if enabled && is_mermaid(info) =>
        {
            source = Some(String::new());
            None
        }
        Event::Text(text) if source.is_some() => {
            source.as_mut().unwrap().push_str(&text);
            None
        }
        Event::End(TagEnd::CodeBlock) if source.is_some() => {
            Some(Event::Html(CowStr::from(format!(
                "{MARKER}<pre class=\"mermaid\">{}</pre></div>\n",
                escape_html(&source.take().unwrap()).replace('"', "&quot;")
            ))))
        }
        other => Some(other),
    })
}

/// Reject protocol-relative URLs, traversal, queries and fragments in static
/// paths. Require the URL spelling to match the static file name.
pub(crate) fn script_location(script: &str) -> Option<Option<String>> {
    if script.trim() != script || script.chars().any(char::is_control) {
        return None;
    }
    if script.starts_with('/') {
        if script.starts_with("//") || script.contains(['\\', '?', '#']) {
            return None;
        }
        let url = url::Url::parse(&format!("https://vados.invalid{script}")).ok()?;
        let path = url.path().trim_start_matches('/');
        // Static file names must match their URL spelling. Encoded paths are
        // deliberately unsupported rather than silently checking another file.
        return (!path.is_empty() && !path.contains('%') && url.path() == script)
            .then(|| Some(path.to_string()));
    }
    let url = url::Url::parse(script).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then_some(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{html, Parser};

    fn render(text: &str, enabled: bool) -> String {
        let mut out = String::new();
        html::push_html(&mut out, diagram_blocks(Parser::new(text), enabled));
        out
    }

    #[test]
    fn diagrams_escape_source_and_accept_info_suffixes_and_unclosed_fences() {
        for info in ["mermaid", "mermaid title", "mermaid,foo", "mermaid{a}"] {
            let out = render(
                &format!("```{info}\nflowchart LR\n A --> B[\"<&\"]\n"),
                true,
            );
            assert!(out.contains(MARKER), "{out}");
            assert!(
                out.contains(" A --&gt; B[&quot;&lt;&amp;&quot;]\n"),
                "{out}"
            );
            assert!(out.ends_with("</pre></div>\n"));
        }
    }

    #[test]
    fn multiple_diagrams_and_plain_code_are_independent() {
        let text = "```mermaid\na\n```\n\n```rust\nfn main() {}\n```\n\n```mermaid\nb\n```\n\n    indented\n\n`inline`\n";
        let out = render(text, true);
        assert_eq!(out.matches(MARKER).count(), 2);
        assert!(out.contains("language-rust"));
        assert!(out.contains("<pre><code>indented"));
        assert!(out.contains("<code>inline</code>"));
        assert!(!render(text, false).contains(MARKER));
        assert!(!render("```mermaidx\nx\n```", true).contains(MARKER));
    }

    #[test]
    fn script_locations_are_restricted_to_http_or_local_files() {
        for script in [
            "//evil/x",
            "javascript:alert(1)",
            "file:///a",
            "x.js",
            "/",
            "/a?b",
            "/a#b",
            "/a\\b",
            "/%2fetc",
            "/a/../b",
            "/a b",
            " https://example.com/a",
        ] {
            assert!(script_location(script).is_none(), "{script}");
        }
        assert_eq!(
            script_location("/js/mermaid.mjs"),
            Some(Some("js/mermaid.mjs".into()))
        );
        assert_eq!(script_location(DEFAULT_SCRIPT), Some(None));
    }
}
