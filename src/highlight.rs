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

    /// GitHub's light colours, and base16 Ocean's dark ones for readers
    /// who prefer a dark scheme, which Bulma follows. Each theme's own
    /// background is left out, so code sits on Bulma's `pre` background,
    /// and every colour too faint to read on it is darkened (light) or
    /// lightened (dark) until it is. The dark colours apply where Bulma's
    /// dark theme does: by the reader's preference unless the page pins
    /// light, or wherever a page pins dark.
    pub(super) fn stylesheet() -> String {
        let themes = ThemeSet::load_defaults();
        let colours = |name: &str, background: [u8; 3]| {
            let css = css_for_theme_with_class_style(&themes.themes[name], CLASS_STYLE)
                .unwrap_or_default();
            super::readable_rules(&css, background)
        };
        let light = colours("InspiredGitHub", super::LIGHT_CODE_BACKGROUND);
        let dark = colours("base16-ocean.dark", super::DARK_CODE_BACKGROUND);
        let mut css = String::from("/* Light: GitHub, after InspiredGitHub. */\n");
        for (selectors, declarations) in &light {
            css.push_str(&format!("{} {{{}}}\n", selectors.join(", "), declarations));
        }
        css.push_str("/* Dark: base16 Ocean. */\n@media (prefers-color-scheme: dark) {\n");
        for (selectors, declarations) in &dark {
            let scoped: Vec<String> = selectors
                .iter()
                .map(|s| format!(":root:not([data-theme=light]):not(.theme-light) {s}"))
                .collect();
            css.push_str(&format!("{} {{{}}}\n", scoped.join(", "), declarations));
        }
        css.push_str("}\n");
        for (selectors, declarations) in &dark {
            let scoped: Vec<String> = selectors
                .iter()
                .flat_map(|s| [format!("[data-theme=dark] {s}"), format!(".theme-dark {s}")])
                .collect();
            css.push_str(&format!("{} {{{}}}\n", scoped.join(", "), declarations));
        }
        css
    }
}

/// Bulma's `pre` background, which highlighted code sits on, in its light
/// theme: `--bulma-background`, hsl(221, 14%, 96%).
#[cfg(feature = "highlight")]
const LIGHT_CODE_BACKGROUND: [u8; 3] = [0xf3, 0xf4, 0xf6];

/// The same in Bulma's dark theme: hsl(221, 14%, 14%).
#[cfg(feature = "highlight")]
const DARK_CODE_BACKGROUND: [u8; 3] = [0x1f, 0x22, 0x29];

/// WCAG's minimum contrast for body text.
#[cfg(feature = "highlight")]
const MIN_CONTRAST: f64 = 4.5;

/// A syntect theme stylesheet as rules -- each a list of selectors and its
/// declarations -- without comments or backgrounds, and with every colour
/// made `readable` on `background`.
#[cfg(feature = "highlight")]
fn readable_rules(css: &str, background: [u8; 3]) -> Vec<(Vec<String>, String)> {
    let mut without_comments = String::new();
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        without_comments.push_str(&rest[..start]);
        rest = rest[start..]
            .find("*/")
            .map_or("", |end| &rest[start + end + 2..]);
    }
    without_comments.push_str(rest);
    without_comments
        .split('}')
        .filter_map(|rule| {
            let (selectors, body) = rule.split_once('{')?;
            let selectors: Vec<String> = selectors
                .split(',')
                .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|s| !s.is_empty())
                .collect();
            let declarations: Vec<String> = body
                .split(';')
                .map(str::trim)
                .filter(|d| !d.is_empty() && !d.starts_with("background"))
                .map(|d| match d.split_once('#') {
                    Some((before, hex)) if hex.len() >= 6 => {
                        match readable(&hex[..6], background) {
                            Some(colour) => format!("{before}#{colour}{}", &hex[6..]),
                            None => d.to_string(),
                        }
                    }
                    _ => d.to_string(),
                })
                .collect();
            (!selectors.is_empty() && !declarations.is_empty())
                .then(|| (selectors, format!(" {}; ", declarations.join("; "))))
        })
        .collect()
}

/// `hex` (`rrggbb`), moved away from `background` in steps -- darkened on a
/// light background, lightened on a dark one -- until it reaches
/// `MIN_CONTRAST` against it; `None` when it isn't a colour.
#[cfg(feature = "highlight")]
fn readable(hex: &str, background: [u8; 3]) -> Option<String> {
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    let mut rgb = [channel(0)?, channel(2)?, channel(4)?];
    let on_dark = luminance(background) < 0.5;
    while contrast(rgb, background) < MIN_CONTRAST {
        rgb = if on_dark {
            rgb.map(|c| c + ((255 - c) as f64 * 0.1).ceil() as u8)
        } else {
            rgb.map(|c| (c as f64 * 0.9) as u8)
        };
    }
    Some(format!("{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]))
}

#[cfg(feature = "highlight")]
fn luminance(rgb: [u8; 3]) -> f64 {
    let [r, g, b] = rgb.map(|c| {
        let c = c as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

#[cfg(feature = "highlight")]
fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
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
        assert!(!css.contains("background"), "{css}");
    }

    #[test]
    fn dark_colours_follow_bulmas_dark_theme() {
        let css = stylesheet();
        assert!(css.contains("@media (prefers-color-scheme: dark)"), "{css}");
        assert!(
            css.contains(":root:not([data-theme=light]):not(.theme-light) .hl-"),
            "{css}"
        );
        assert!(css.contains("[data-theme=dark] .hl-"), "{css}");
        assert!(css.contains(".theme-dark .hl-"), "{css}");
    }

    /// The colours of the rules inside and outside the dark scopes.
    fn colours(css: &str) -> (Vec<[u8; 3]>, Vec<[u8; 3]>) {
        let (mut light, mut dark) = (vec![], vec![]);
        let mut in_media = false;
        for line in css.lines() {
            if line.starts_with("@media") {
                in_media = true;
                continue;
            }
            if line == "}" {
                in_media = false;
                continue;
            }
            let is_dark = in_media || line.starts_with("[data-theme=dark]");
            for hex in line.split('#').skip(1).map(|r| &r[..6]) {
                let channel = |i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap();
                let rgb = [channel(0), channel(2), channel(4)];
                if is_dark {
                    dark.push(rgb)
                } else {
                    light.push(rgb)
                }
            }
        }
        (light, dark)
    }

    #[test]
    fn every_colour_is_readable_on_bulmas_code_background_in_its_theme() {
        let (light, dark) = colours(&stylesheet());
        assert!(!light.is_empty() && !dark.is_empty());
        for rgb in light {
            assert!(
                contrast(rgb, LIGHT_CODE_BACKGROUND) >= MIN_CONTRAST,
                "{rgb:?}"
            );
        }
        for rgb in dark {
            assert!(
                contrast(rgb, DARK_CODE_BACKGROUND) >= MIN_CONTRAST,
                "{rgb:?}"
            );
        }
    }

    #[test]
    fn readable_leaves_contrasting_colours_alone_and_moves_faint_ones() {
        assert_eq!(
            readable("323232", LIGHT_CODE_BACKGROUND).as_deref(),
            Some("323232")
        );
        let darkened = readable("b3b3b3", LIGHT_CODE_BACKGROUND).unwrap();
        assert!(darkened.as_str() < "b3b3b3", "{darkened}");
        assert_eq!(
            readable("c0c5ce", DARK_CODE_BACKGROUND).as_deref(),
            Some("c0c5ce")
        );
        let lightened = readable("65737e", DARK_CODE_BACKGROUND).unwrap();
        assert!(lightened.as_str() > "65737e", "{lightened}");
        assert_eq!(readable("zzzzzz", LIGHT_CODE_BACKGROUND), None);
    }
}
