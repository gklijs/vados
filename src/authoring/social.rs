//! `social add`, `social update` and `social remove`: adjusts menu.json's
//! list of social links. See `vados.allium`'s `SocialLinkChangeRun`.

use crate::config_files::{MenuConfig, RawSocialItem};
use crate::json_files::{read_json, write_json_pretty};
use crate::structure::{canonical_brand_color, canonical_icon};
pub use crate::structure::{
    classify_social_provider, RecognizedSocialProvider, SocialProviderKind,
};
use std::fmt;
use std::io;
use std::path::Path;

/// Why a `social add`, `social update` or `social remove` request was
/// rejected. Unlike the page/image block reasons, a social-link change
/// carries at most one -- matching an existing link and judging completeness
/// are both cheap enough that there is never more than one thing wrong to
/// report. See `vados.allium`'s `SocialLinkChangeRejectionReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocialLinkChangeRejectionReason {
    UnrecognisedProviderIncomplete,
    UrlAlreadyRegistered,
    NoMatchingSocialLink,
    AmbiguousSocialLinkMatch,
    MatchIndexOutOfRange,
}

impl fmt::Display for SocialLinkChangeRejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SocialLinkChangeRejectionReason::UnrecognisedProviderIncomplete => {
                "an unrecognised provider needs both an icon and a brand color"
            }
            SocialLinkChangeRejectionReason::UrlAlreadyRegistered => {
                "a social link with this url is already registered"
            }
            SocialLinkChangeRejectionReason::NoMatchingSocialLink => "no social link matches",
            SocialLinkChangeRejectionReason::AmbiguousSocialLinkMatch => {
                "more than one social link matches; use --index to pick one"
            }
            SocialLinkChangeRejectionReason::MatchIndexOutOfRange => {
                "--index does not name one of the matching social links"
            }
        })
    }
}

/// How the maintainer named the link they're adding or replacing an existing
/// one with: a handle for a recognised provider, or a url they supply
/// directly (with its own icon/brand color, for an unrecognised provider).
/// Exactly one of `handle`/`url` is expected to be set; see `vados.allium`'s
/// comment on `SocialLinkChangeRun.given_handle`/`given_url`.
#[derive(Debug, Clone)]
pub struct GivenSocialLink {
    pub handle: Option<(RecognizedSocialProvider, String)>,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub brand_color: Option<String>,
}

impl GivenSocialLink {
    fn resulting_url(&self) -> String {
        // Trimmed here rather than left to callers: `init`'s own wizard
        // trims a handle before building the same kind of URL
        // (`s.handle.trim()` in `write_menu_config`), and the interactive
        // prompts this crate's CLI falls back to trim too (`prompt_required`/
        // `prompt_optional`). Only a handle/url given directly as a `--handle`
        // or `--url` flag bypassed that -- trimming once here, on the one
        // path that builds a URL from either, keeps all three ways of naming
        // a link agreeing with each other instead of two of them trimming
        // and the third silently keeping stray whitespace.
        match &self.handle {
            Some((provider, handle)) => provider.profile_url(handle.trim()),
            None => self
                .url
                .as_deref()
                .map(str::trim)
                .expect("callers guarantee a handle or a url is given")
                .to_string(),
        }
    }
}

fn provider_is_complete(
    provider: SocialProviderKind,
    icon: &Option<String>,
    brand_color: &Option<String>,
) -> bool {
    provider != SocialProviderKind::Other || (icon.is_some() && brand_color.is_some())
}

/// What a social-link change did, once applied. See `vados.allium`'s
/// `SocialLinkChangedOutcome`.
#[derive(Debug)]
pub struct SocialLinkChangedOutcome {
    pub url: String,
    pub provider: SocialProviderKind,
    pub icon: Option<String>,
    pub brand_color: Option<String>,
    /// The link's previous url, for `social update` only.
    pub previous_url: Option<String>,
}

fn resolved_icon(provider: SocialProviderKind, given: &Option<String>) -> Option<String> {
    canonical_icon(provider)
        .map(String::from)
        .or_else(|| given.clone())
}

fn resolved_brand_color(provider: SocialProviderKind, given: &Option<String>) -> Option<String> {
    canonical_brand_color(provider)
        .map(String::from)
        .or_else(|| given.clone())
}

fn menu_json_path(source: &str) -> std::path::PathBuf {
    Path::new(source).join("menu.json")
}

/// `social add`. See `vados.allium`'s `ApplySocialLinkAddition`.
pub fn add_social_link(
    source: &str,
    given: GivenSocialLink,
) -> io::Result<Result<SocialLinkChangedOutcome, SocialLinkChangeRejectionReason>> {
    let path = menu_json_path(source);
    let mut menu_config: MenuConfig = read_json(&path)?;

    let url = given.resulting_url();
    let provider = classify_social_provider(&url);
    if !provider_is_complete(provider, &given.icon, &given.brand_color) {
        return Ok(Err(
            SocialLinkChangeRejectionReason::UnrecognisedProviderIncomplete,
        ));
    }
    if menu_config.socials.iter().any(|s| s.url == url) {
        return Ok(Err(SocialLinkChangeRejectionReason::UrlAlreadyRegistered));
    }

    menu_config.socials.push(RawSocialItem {
        url: url.clone(),
        icon: given.icon.clone(),
        color: given.brand_color.clone(),
    });
    write_json_pretty(path, &menu_config)?;

    Ok(Ok(SocialLinkChangedOutcome {
        icon: resolved_icon(provider, &given.icon),
        brand_color: resolved_brand_color(provider, &given.brand_color),
        url,
        provider,
        previous_url: None,
    }))
}

/// The indices, into `menu_config.socials`, of every entry whose url
/// contains `given_match` as a substring. Backs `matching_social_link_count`.
fn matching_indices(menu_config: &MenuConfig, given_match: &str) -> Vec<usize> {
    menu_config
        .socials
        .iter()
        .enumerate()
        .filter(|(_, s)| s.url.contains(given_match))
        .map(|(i, _)| i)
        .collect()
}

/// Resolves `given_match`/`given_index` to a single social link, or the one
/// reason it couldn't be. `given_index` ranks among the *matches*, not the
/// full list -- see `vados.allium`'s bound on `SocialLinkChangeRun.given_index`
/// against `match_count`, not the full social link count.
fn select_match(
    menu_config: &MenuConfig,
    given_match: &str,
    given_index: Option<i64>,
) -> Result<usize, SocialLinkChangeRejectionReason> {
    let matches = matching_indices(menu_config, given_match);
    if matches.is_empty() {
        return Err(SocialLinkChangeRejectionReason::NoMatchingSocialLink);
    }
    match given_index {
        Some(i) if i < 0 || i as usize >= matches.len() => {
            Err(SocialLinkChangeRejectionReason::MatchIndexOutOfRange)
        }
        Some(i) => Ok(matches[i as usize]),
        None if matches.len() > 1 => Err(SocialLinkChangeRejectionReason::AmbiguousSocialLinkMatch),
        None => Ok(matches[0]),
    }
}

/// `social update`. See `vados.allium`'s `ApplySocialLinkUpdate`. Replaces
/// the matched link wholesale rather than merging individual fields into
/// it -- the maintainer gives a complete replacement, the same shape `add`
/// accepts for a new one.
pub fn update_social_link(
    source: &str,
    given_match: &str,
    given_index: Option<i64>,
    given: GivenSocialLink,
) -> io::Result<Result<SocialLinkChangedOutcome, SocialLinkChangeRejectionReason>> {
    let path = menu_json_path(source);
    let mut menu_config: MenuConfig = read_json(&path)?;

    let index = match select_match(&menu_config, given_match, given_index) {
        Ok(i) => i,
        Err(reason) => return Ok(Err(reason)),
    };

    let url = given.resulting_url();
    let provider = classify_social_provider(&url);
    if !provider_is_complete(provider, &given.icon, &given.brand_color) {
        return Ok(Err(
            SocialLinkChangeRejectionReason::UnrecognisedProviderIncomplete,
        ));
    }

    let previous_url = menu_config.socials[index].url.clone();
    menu_config.socials[index] = RawSocialItem {
        url: url.clone(),
        icon: given.icon.clone(),
        color: given.brand_color.clone(),
    };
    write_json_pretty(path, &menu_config)?;

    Ok(Ok(SocialLinkChangedOutcome {
        icon: resolved_icon(provider, &given.icon),
        brand_color: resolved_brand_color(provider, &given.brand_color),
        url,
        provider,
        previous_url: Some(previous_url),
    }))
}

/// `social remove`. See `vados.allium`'s `ApplySocialLinkRemoval`.
pub fn remove_social_link(
    source: &str,
    given_match: &str,
    given_index: Option<i64>,
) -> io::Result<Result<String, SocialLinkChangeRejectionReason>> {
    let path = menu_json_path(source);
    let mut menu_config: MenuConfig = read_json(&path)?;

    let index = match select_match(&menu_config, given_match, given_index) {
        Ok(i) => i,
        Err(reason) => return Ok(Err(reason)),
    };

    let removed = menu_config.socials.remove(index);
    write_json_pretty(path, &menu_config)?;

    Ok(Ok(removed.url))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    struct TestSource {
        path: std::path::PathBuf,
    }

    impl TestSource {
        fn new(name: &str, menu_json: &str) -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!("vados_social_test_{name}_{id}"));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("menu.json"), menu_json).unwrap();
            TestSource { path }
        }

        fn root(&self) -> &str {
            self.path.to_str().unwrap()
        }

        fn menu_config(&self) -> MenuConfig {
            read_json(&self.path.join("menu.json")).unwrap()
        }
    }

    impl Drop for TestSource {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    const EMPTY_MENU: &str = r#"{"mainMenu":[],"socials":[]}"#;

    fn url_link(url: &str) -> GivenSocialLink {
        GivenSocialLink {
            handle: None,
            url: Some(url.to_string()),
            icon: None,
            brand_color: None,
        }
    }

    #[test]
    fn adding_a_recognised_provider_by_handle_needs_no_icon_or_color() {
        let source = TestSource::new("add_handle", EMPTY_MENU);

        let result = add_social_link(
            source.root(),
            GivenSocialLink {
                handle: Some((RecognizedSocialProvider::Github, "gklijs".to_string())),
                url: None,
                icon: None,
                brand_color: None,
            },
        )
        .unwrap();

        let outcome = result.unwrap();
        assert_eq!(outcome.url, "https://github.com/gklijs");
        assert_eq!(outcome.provider, SocialProviderKind::Github);
        assert_eq!(outcome.icon, Some("github".to_string()));
        assert_eq!(source.menu_config().socials.len(), 1);
    }

    #[test]
    fn a_handle_with_stray_whitespace_is_trimmed_before_building_the_url() {
        let source = TestSource::new("add_handle_untrimmed", EMPTY_MENU);

        let result = add_social_link(
            source.root(),
            GivenSocialLink {
                handle: Some((RecognizedSocialProvider::Github, "  gklijs  ".to_string())),
                url: None,
                icon: None,
                brand_color: None,
            },
        )
        .unwrap();

        assert_eq!(result.unwrap().url, "https://github.com/gklijs");
    }

    #[test]
    fn a_url_with_stray_whitespace_is_trimmed() {
        let source = TestSource::new("add_url_untrimmed", EMPTY_MENU);

        let result =
            add_social_link(source.root(), url_link("  https://github.com/gklijs  ")).unwrap();

        assert_eq!(result.unwrap().url, "https://github.com/gklijs");
    }

    #[test]
    fn adding_an_unrecognised_provider_without_icon_or_color_is_rejected() {
        let source = TestSource::new("add_incomplete", EMPTY_MENU);

        let result =
            add_social_link(source.root(), url_link("https://mastodon.social/@x")).unwrap();

        assert_eq!(
            result.unwrap_err(),
            SocialLinkChangeRejectionReason::UnrecognisedProviderIncomplete
        );
        assert!(source.menu_config().socials.is_empty());
    }

    #[test]
    fn adding_an_unrecognised_provider_with_icon_and_color_succeeds() {
        let source = TestSource::new("add_complete", EMPTY_MENU);

        let result = add_social_link(
            source.root(),
            GivenSocialLink {
                handle: None,
                url: Some("https://mastodon.social/@x".to_string()),
                icon: Some("mastodon".to_string()),
                brand_color: Some("6364FF".to_string()),
            },
        )
        .unwrap();

        let outcome = result.unwrap();
        assert_eq!(outcome.provider, SocialProviderKind::Other);
        assert_eq!(outcome.icon, Some("mastodon".to_string()));
    }

    #[test]
    fn adding_a_url_already_registered_is_rejected() {
        let source = TestSource::new(
            "add_duplicate",
            r#"{"mainMenu":[],"socials":[{"url":"https://github.com/gklijs","icon":null,"color":null}]}"#,
        );

        let result = add_social_link(source.root(), url_link("https://github.com/gklijs")).unwrap();

        assert_eq!(
            result.unwrap_err(),
            SocialLinkChangeRejectionReason::UrlAlreadyRegistered
        );
    }

    #[test]
    fn updating_with_no_match_is_rejected() {
        let source = TestSource::new("update_no_match", EMPTY_MENU);

        let result =
            update_social_link(source.root(), "github", None, url_link("https://x.com/a")).unwrap();

        assert_eq!(
            result.unwrap_err(),
            SocialLinkChangeRejectionReason::NoMatchingSocialLink
        );
    }

    #[test]
    fn updating_an_ambiguous_match_without_an_index_is_rejected() {
        let source = TestSource::new(
            "update_ambiguous",
            r#"{"mainMenu":[],"socials":[
                {"url":"https://github.com/alice","icon":null,"color":null},
                {"url":"https://github.com/bob","icon":null,"color":null}
            ]}"#,
        );

        let result =
            update_social_link(source.root(), "github", None, url_link("https://x.com/a")).unwrap();

        assert_eq!(
            result.unwrap_err(),
            SocialLinkChangeRejectionReason::AmbiguousSocialLinkMatch
        );
    }

    #[test]
    fn updating_an_ambiguous_match_with_an_out_of_range_index_is_rejected() {
        let source = TestSource::new(
            "update_out_of_range",
            r#"{"mainMenu":[],"socials":[
                {"url":"https://github.com/alice","icon":null,"color":null},
                {"url":"https://github.com/bob","icon":null,"color":null}
            ]}"#,
        );

        let result = update_social_link(
            source.root(),
            "github",
            Some(5),
            url_link("https://x.com/a"),
        )
        .unwrap();

        assert_eq!(
            result.unwrap_err(),
            SocialLinkChangeRejectionReason::MatchIndexOutOfRange
        );
    }

    #[test]
    fn updating_with_an_index_disambiguates_and_replaces_wholesale() {
        let source = TestSource::new(
            "update_disambiguated",
            r#"{"mainMenu":[],"socials":[
                {"url":"https://github.com/alice","icon":null,"color":null},
                {"url":"https://github.com/bob","icon":null,"color":null}
            ]}"#,
        );

        let result = update_social_link(
            source.root(),
            "github",
            Some(1),
            url_link("https://github.com/bobby"),
        )
        .unwrap();

        let outcome = result.unwrap();
        assert_eq!(
            outcome.previous_url,
            Some("https://github.com/bob".to_string())
        );
        assert_eq!(outcome.url, "https://github.com/bobby");
        let socials = source.menu_config().socials;
        assert_eq!(socials[0].url, "https://github.com/alice");
        assert_eq!(socials[1].url, "https://github.com/bobby");
    }

    #[test]
    fn removing_the_single_match_drops_it() {
        let source = TestSource::new(
            "remove_single",
            r#"{"mainMenu":[],"socials":[{"url":"https://github.com/gklijs","icon":null,"color":null}]}"#,
        );

        let result = remove_social_link(source.root(), "github", None).unwrap();

        assert_eq!(result.unwrap(), "https://github.com/gklijs");
        assert!(source.menu_config().socials.is_empty());
    }

    #[test]
    fn removing_an_ambiguous_match_without_an_index_is_rejected() {
        let source = TestSource::new(
            "remove_ambiguous",
            r#"{"mainMenu":[],"socials":[
                {"url":"https://github.com/alice","icon":null,"color":null},
                {"url":"https://github.com/bob","icon":null,"color":null}
            ]}"#,
        );

        let result = remove_social_link(source.root(), "github", None).unwrap();

        assert_eq!(
            result.unwrap_err(),
            SocialLinkChangeRejectionReason::AmbiguousSocialLinkMatch
        );
    }
}
