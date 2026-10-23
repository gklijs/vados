//! Syntax highlighting for fenced Markdown code blocks, at build time: the
//! code is wrapped in spans carrying CSS classes, and one small stylesheet
//! colours them, so pages need no script. See `vados.allium`'s
//! `CodeIsHighlighted`.

use crate::content::escape_html;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag, TagEnd};

/// Where the stylesheet colouring highlighted code is published.
pub(crate) const STYLESHEET_PATH: &str = "/css/vados-highlight.css";

/// What marks a page as having highlighted code, so only such pages link
/// the stylesheet.
pub(crate) const HIGHLIGHTED_MARKER: &str = r#"<pre class="highlight">"#;

/// Replaces every fenced code block in a language the highlighter knows by
/// its highlighted HTML. Other code blocks, inline code and everything else
/// pass through untouched.
pub(crate) fn highlight_code_blocks<'a>(
    events: impl Iterator<Item = Event<'a>>,
) -> impl Iterator<Item = Event<'a>> {
    let mut language: Option<String> = None;
    let mut code = String::new();
    events.filter_map(move |event| match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) => {
            // The info string may carry more than the language: ```rust,ignore
            let token = info
                .split([' ', ',', '{'])
                .next()
                .unwrap_or_default()
                .to_string();
            if engine::knows(&token) {
                language = Some(token);
                code.clear();
                None
            } else {
                Some(event)
            }
        }
        Event::Text(text) if language.is_some() => {
            code.push_str(&text);
            None
        }
        Event::End(TagEnd::CodeBlock) if language.is_some() => {
            let language = language.take().unwrap();
            let spans = engine::highlight(&language, &code);
            Some(Event::Html(CowStr::from(format!(
                r#"{HIGHLIGHTED_MARKER}<code class="language-{}">{}</code></pre>"#,
                escape_html(&language).replace('"', "&quot;"),
                spans
            ))))
        }
        other => Some(other),
    })
}

/// The stylesheet for highlighted code.
pub(crate) fn stylesheet() -> String {
    engine::stylesheet()
}

#[cfg(feature = "highlight")]
mod engine {
    use std::sync::OnceLock;
    use syntect::highlighting::ThemeSet;
    use syntect::html::{css_for_theme_with_class_style, ClassStyle, ClassedHTMLGenerator};
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    const CLASS_STYLE: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "hl-" };

    fn syntaxes() -> &'static SyntaxSet {
        static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
        SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines)
    }

    pub(super) fn knows(language: &str) -> bool {
        !language.is_empty() && syntaxes().find_syntax_by_token(language).is_some()
    }

    pub(super) fn highlight(language: &str, code: &str) -> String {
        let syntaxes = syntaxes();
        let syntax = syntaxes
            .find_syntax_by_token(language)
            .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
        let mut generator =
            ClassedHTMLGenerator::new_with_class_style(syntax, syntaxes, CLASS_STYLE);
        for line in LinesWithEndings::from(code) {
            if generator
                .parse_html_for_line_which_includes_newline(line)
                .is_err()
            {
                // A grammar that trips over this code: show it plain rather
                // than half coloured.
                return crate::content::escape_html(code);
            }
        }
        generator.finalize()
    }

    /// GitHub's light colours: Bulma 0.9 has no dark mode, so code sits on
    /// its light `pre` background whatever the reader's preference. The
    /// theme's own background is left out for the same reason, and every
    /// colour too faint to read on Bulma's background is darkened until it
    /// is.
    pub(super) fn stylesheet() -> String {
        let themes = ThemeSet::load_defaults();
        let css = css_for_theme_with_class_style(&themes.themes["InspiredGitHub"], CLASS_STYLE)
            .unwrap_or_default();
        css.lines()
            .filter(|l| !l.trim_start().starts_with("background-color"))
            .map(|l| match l.split_once('#') {
                Some((before, rest)) if rest.len() >= 6 => {
                    let (hex, after) = rest.split_at(6);
                    match super::readable(hex) {
                        Some(color) => format!("{before}#{color}{after}"),
                        None => l.to_string(),
                    }
                }
                _ => l.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Bulma's `pre` background, which highlighted code sits on.
#[cfg(feature = "highlight")]
const CODE_BACKGROUND: [u8; 3] = [0xf5, 0xf5, 0xf5];

/// WCAG's minimum contrast for body text.
#[cfg(feature = "highlight")]
const MIN_CONTRAST: f64 = 4.5;

/// `hex` (`rrggbb`), darkened in steps until it reaches `MIN_CONTRAST`
/// against `CODE_BACKGROUND`; `None` when it isn't a colour.
#[cfg(feature = "highlight")]
fn readable(hex: &str) -> Option<String> {
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    let mut rgb = [channel(0)?, channel(2)?, channel(4)?];
    while contrast(rgb, CODE_BACKGROUND) < MIN_CONTRAST {
        rgb = rgb.map(|c| (c as f64 * 0.9) as u8);
    }
    Some(format!("{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]))
}

#[cfg(feature = "highlight")]
fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let luminance = |rgb: [u8; 3]| {
        let [r, g, b] = rgb.map(|c| {
            let c = c as f64 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * r + 0.7152 * g + 0.0722 * b
    };
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(not(feature = "highlight"))]
mod engine {
    pub(super) fn knows(_language: &str) -> bool {
        false
    }

    pub(super) fn highlight(_language: &str, code: &str) -> String {
        crate::content::escape_html(code)
    }

    pub(super) fn stylesheet() -> String {
        String::new()
    }
}

#[cfg(all(test, feature = "highlight"))]
mod tests {
    use super::*;
    use pulldown_cmark::{html, Parser};

    fn render(markdown: &str) -> String {
        let mut out = String::new();
        html::push_html(&mut out, highlight_code_blocks(Parser::new(markdown)));
        out
    }

    #[test]
    fn a_fenced_block_in_a_known_language_is_highlighted() {
        let out = render("```rust\nfn main() { let x = 1 < 2; }\n```\n");
        assert!(
            out.starts_with(r#"<pre class="highlight"><code class="language-rust">"#),
            "{out}"
        );
        assert!(out.contains(r#"<span class="hl-"#), "{out}");
        assert!(out.contains("&lt;"), "{out}");
        assert!(!out.contains("1 < 2"), "{out}");
    }

    #[test]
    fn extra_words_in_the_info_string_are_ignored() {
        assert!(render("```rust,ignore\nlet x = 1;\n```\n").contains("language-rust"));
    }

    #[test]
    fn unknown_or_missing_languages_and_indented_blocks_stay_plain() {
        for markdown in [
            "```klingon\nqapla'\n```\n",
            "```\nplain\n```\n",
            "    indented\n",
        ] {
            let out = render(markdown);
            assert!(!out.contains("hl-"), "{out}");
            assert!(out.starts_with("<pre><code"), "{out}");
        }
    }

    #[test]
    fn inline_code_is_untouched() {
        assert_eq!(render("Use `let x`.\n"), "<p>Use <code>let x</code>.</p>\n");
    }

    #[test]
    fn the_stylesheet_colours_the_classes_without_a_background() {
        let css = stylesheet();
        assert!(css.contains(".hl-"), "{css}");
        assert!(!css.contains("background-color"), "{css}");
    }

    #[test]
    fn every_colour_in_the_stylesheet_is_readable_on_bulmas_code_background() {
        let css = stylesheet();
        let colours: Vec<&str> = css.split('#').skip(1).map(|r| &r[..6]).collect();
        assert!(!colours.is_empty());
        for hex in colours {
            let channel = |i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap();
            let rgb = [channel(0), channel(2), channel(4)];
            assert!(contrast(rgb, CODE_BACKGROUND) >= MIN_CONTRAST, "#{hex}");
        }
    }

    #[test]
    fn readable_leaves_dark_colours_alone_and_darkens_faint_ones() {
        assert_eq!(readable("323232").as_deref(), Some("323232"));
        let darkened = readable("b3b3b3").unwrap();
        assert_ne!(darkened, "b3b3b3");
        assert_eq!(readable("zzzzzz"), None);
    }
}
