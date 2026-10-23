//! Material Design Icons, inlined as SVG wherever vados shows an icon, so a
//! page loads no icon font. See `vados.allium`'s `IconsAreInlined`.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Every icon's SVG path from `@mdi/svg`, as `name path` lines, then the
/// aliases of renamed icons as `alias =name`. See `assets/mdi/README.md`.
const PATHS: &str = include_str!("../assets/mdi/paths.txt");

fn paths() -> &'static HashMap<&'static str, &'static str> {
    static PATHS_BY_NAME: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    PATHS_BY_NAME.get_or_init(|| {
        let mut by_name = HashMap::new();
        let mut aliases = vec![];
        for line in PATHS.lines() {
            if let Some((name, rest)) = line.split_once(' ') {
                match rest.strip_prefix('=') {
                    Some(target) => aliases.push((name, target)),
                    None => {
                        by_name.insert(name, rest);
                    }
                }
            }
        }
        for (alias, target) in aliases {
            if let Some(path) = by_name.get(target).copied() {
                by_name.insert(alias, path);
            }
        }
        by_name
    })
}

/// Whether `name` is a Material Design Icons name vados can show. Accepts
/// the name with or without the `mdi-` prefix the icon font used.
pub(crate) fn is_known_icon(name: &str) -> bool {
    paths().contains_key(bare_name(name))
}

fn bare_name(name: &str) -> &str {
    let name = name.trim();
    name.strip_prefix("mdi-").unwrap_or(name)
}

/// The icon as an inline SVG of `size` pixels, in the surrounding text
/// colour. Decorative: whatever it sits in carries the accessible name. An
/// unknown icon renders as nothing; `check` reports it.
pub(crate) fn icon_svg(name: &str, size: u32) -> String {
    match paths().get(bare_name(name)) {
        Some(path) => format!(
            r#"<svg class="mdi-icon" width="{size}" height="{size}" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" focusable="false"><path d="{path}"/></svg>"#
        ),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_icons_render_as_inline_svg() {
        let svg = icon_svg("rss", 24);
        assert!(
            svg.starts_with(r#"<svg class="mdi-icon" width="24" height="24" viewBox="0 0 24 24""#)
        );
        assert!(svg.contains(r#"<path d="M6.18,15.64"#));
        assert_eq!(icon_svg("mdi-rss", 24), svg);
    }

    #[test]
    fn every_icon_vados_uses_itself_is_known() {
        for name in [
            "rss",
            "github",
            "linkedin",
            "facebook",
            "youtube",
            "alpha-x-box",
            "mastodon",
        ] {
            assert!(is_known_icon(name), "{name}");
        }
    }

    #[test]
    fn renamed_icons_are_known_by_their_old_name_too() {
        let (alias, target) = PATHS
            .lines()
            .find_map(|l| l.split_once(" ="))
            .expect("the icon set has aliases");
        assert!(is_known_icon(alias));
        assert_eq!(icon_svg(alias, 18), icon_svg(target, 18));
    }

    #[test]
    fn unknown_icons_render_as_nothing() {
        assert!(!is_known_icon("no-such-icon"));
        assert_eq!(icon_svg("no-such-icon", 24), "");
    }
}
