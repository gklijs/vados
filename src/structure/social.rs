//! Social links and the providers this crate recognises from their URLs.

use crate::config_files::RawSocialItem;

/// Which provider a social link's URL was recognised as. See
/// `vados.allium`'s `SocialProvider` enum and `social_provider_of`. Public so
/// the `social add`/`update`/`remove` commands can classify a URL the
/// maintainer gave before any icon/color has been supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocialProviderKind {
    Github,
    LinkedIn,
    Facebook,
    YouTube,
    Twitter,
    Other,
}

/// Everything this crate knows about one recognised provider, in one place.
/// Every accessor below -- classifying a url, a provider's canonical icon
/// and brand color, a `RecognizedSocialProvider`'s wizard label and the
/// profile url a bare handle resolves to -- derives from this table instead
/// of each hand-maintaining its own copy of the same facts. Adding a
/// provider is one row here (plus one variant each on `SocialProviderKind`
/// and, if it should be handle-driven, `RecognizedSocialProvider`) rather
/// than a matching arm in half a dozen functions that could silently drift
/// out of step with each other.
struct ProviderSpec {
    kind: SocialProviderKind,
    /// Every url prefix this provider is recognised from when classifying an
    /// already-written url. The first is also the one a profile url is built
    /// from. More than one only for Twitter/X: `x.com` and `twitter.com`
    /// both name the same provider, but only `x.com` is written going
    /// forward.
    url_prefixes: &'static [&'static str],
    /// Appended after the handle when building a profile url from one; empty
    /// for every provider but LinkedIn.
    profile_url_suffix: &'static str,
    icon: &'static str,
    brand_color: &'static str,
    /// Doubles as the `init` wizard's short prompt label and the accessible
    /// name (`SocialItem.label`/`vados.allium`'s `SocialLink.label`) a
    /// screen reader announces for the icon-only rendered link -- both are
    /// just "this provider's own name", so one string serves both.
    label: &'static str,
}

const PROVIDERS: &[ProviderSpec] = &[
    ProviderSpec {
        kind: SocialProviderKind::Github,
        url_prefixes: &["https://github.com/"],
        profile_url_suffix: "",
        icon: "github",
        brand_color: "171515",
        label: "GitHub",
    },
    ProviderSpec {
        kind: SocialProviderKind::LinkedIn,
        url_prefixes: &["https://www.linkedin.com/in/"],
        profile_url_suffix: "/",
        icon: "linkedin",
        brand_color: "0077b5",
        label: "LinkedIn",
    },
    ProviderSpec {
        kind: SocialProviderKind::Facebook,
        url_prefixes: &["https://www.facebook.com/"],
        profile_url_suffix: "",
        icon: "facebook",
        brand_color: "4267B2",
        label: "Facebook",
    },
    ProviderSpec {
        kind: SocialProviderKind::YouTube,
        url_prefixes: &["https://www.youtube.com/"],
        profile_url_suffix: "",
        icon: "youtube",
        brand_color: "c4302b",
        label: "YouTube",
    },
    ProviderSpec {
        kind: SocialProviderKind::Twitter,
        url_prefixes: &["https://x.com/", "https://twitter.com/"],
        profile_url_suffix: "",
        // MDI 6.5.95 has no X logo; a boxed X in X's black is closer to
        // the current brand than the old bird in Twitter blue.
        icon: "alpha-x-box",
        brand_color: "000000",
        label: "X",
    },
];

fn provider_spec(kind: SocialProviderKind) -> Option<&'static ProviderSpec> {
    PROVIDERS.iter().find(|p| p.kind == kind)
}

/// Classifies a social link's url by matching it against every recognised
/// provider's url prefixes in `PROVIDERS`.
pub fn classify_social_provider(url: &str) -> SocialProviderKind {
    PROVIDERS
        .iter()
        .find(|p| p.url_prefixes.iter().any(|prefix| url.starts_with(prefix)))
        .map_or(SocialProviderKind::Other, |p| p.kind)
}

/// Whether a social link's URL matches one of the providers `PROVIDERS`
/// recognises automatically, so `check` can flag an unrecognised provider
/// missing its required icon and color before generation would panic on it.
pub(crate) fn is_recognized_social_provider(url: &str) -> bool {
    classify_social_provider(url) != SocialProviderKind::Other
}

/// The Material Design Icons name a recognised provider always uses. `None`
/// for `Other`, which has no canonical icon -- the maintainer must supply
/// one.
pub fn canonical_icon(provider: SocialProviderKind) -> Option<&'static str> {
    provider_spec(provider).map(|p| p.icon)
}

/// The brand color a recognised provider always uses. `None` for `Other`, the
/// same way as `canonical_icon`.
pub fn canonical_brand_color(provider: SocialProviderKind) -> Option<&'static str> {
    provider_spec(provider).map(|p| p.brand_color)
}

/// A brand color as the template wants it: hex digits without a leading
/// `#`, since `navigation.html` writes its own. `"#ffffff"` and `"ffffff"`
/// both become `"ffffff"`.
pub fn normalize_brand_color(color: &str) -> String {
    color.trim().trim_start_matches('#').to_string()
}

/// The accessible name a recognised provider always uses. `None` for
/// `Other`, the same way as `canonical_icon`. See `vados.allium`'s
/// `SocialLink.label`.
pub fn canonical_label(provider: SocialProviderKind) -> Option<&'static str> {
    provider_spec(provider).map(|p| p.label)
}

/// A link to the maintainer's presence on another site, rendered site-wide.
/// Resolved once at construction from `PROVIDERS` rather than carrying its
/// own per-provider match, the same way
/// `canonical_icon`/`canonical_brand_color`/`canonical_label` do -- there is
/// nothing left for `get_icon`/`get_color`/`get_label` to branch on.
#[derive(Debug, PartialEq)]
pub struct SocialItem {
    url: String,
    icon: String,
    brand_color: String,
    /// The link's accessible name: what a screen reader announces for it,
    /// since it is rendered as an icon with no visible text. See
    /// `vados.allium`'s `SocialLink.label` and
    /// `IconOnlyControlsCarryAnAccessibleName`.
    label: String,
}

impl SocialItem {
    pub(crate) fn new(raw: &RawSocialItem) -> Result<SocialItem, String> {
        let provider = classify_social_provider(&raw.url);
        let missing = |field| format!("social link {} has no {}", raw.url, field);
        let icon = canonical_icon(provider)
            .map(String::from)
            .or_else(|| raw.icon.clone())
            .ok_or_else(|| missing("icon"))?;
        // An explicit color wins over the provider's brand color.
        let brand_color = raw
            .color
            .as_deref()
            .map(normalize_brand_color)
            .or_else(|| canonical_brand_color(provider).map(String::from))
            .ok_or_else(|| missing("color"))?;
        let label = canonical_label(provider)
            .map(String::from)
            .or_else(|| raw.label.clone())
            .ok_or_else(|| missing("label"))?;
        Ok(SocialItem {
            url: raw.url.clone(),
            icon,
            brand_color,
            label,
        })
    }
    pub(crate) fn get_url(&self) -> &str {
        &self.url
    }

    pub(crate) fn get_icon(&self) -> &str {
        &self.icon
    }

    pub(crate) fn get_color(&self) -> &str {
        &self.brand_color
    }

    /// The link's accessible name: what a screen reader announces for it.
    /// See `vados.allium`'s `SocialLink.label`.
    pub(crate) fn get_label(&self) -> &str {
        &self.label
    }
}

/// A provider `init`'s wizard can gather a bare handle for. See
/// `vados.allium`'s `config.recognized_social_providers`: every variant here
/// has an entry in `PROVIDERS`, so a handle alone is enough to produce a
/// complete social link -- unlike an unrecognised (`Other`) provider, which
/// needs an icon and color supplied explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecognizedSocialProvider {
    Github,
    LinkedIn,
    Facebook,
    YouTube,
    Twitter,
}

impl RecognizedSocialProvider {
    fn kind(self) -> SocialProviderKind {
        match self {
            RecognizedSocialProvider::Github => SocialProviderKind::Github,
            RecognizedSocialProvider::LinkedIn => SocialProviderKind::LinkedIn,
            RecognizedSocialProvider::Facebook => SocialProviderKind::Facebook,
            RecognizedSocialProvider::YouTube => SocialProviderKind::YouTube,
            RecognizedSocialProvider::Twitter => SocialProviderKind::Twitter,
        }
    }

    fn spec(self) -> &'static ProviderSpec {
        provider_spec(self.kind())
            .expect("every RecognizedSocialProvider variant has a PROVIDERS entry")
    }

    pub fn all() -> [RecognizedSocialProvider; 5] {
        use RecognizedSocialProvider::*;
        [Github, LinkedIn, Facebook, YouTube, Twitter]
    }

    /// A short label for the wizard prompt.
    pub fn label(self) -> &'static str {
        self.spec().label
    }

    /// The profile URL a bare handle resolves to for this provider. Built
    /// from the same `PROVIDERS` entry `classify_social_provider` matches
    /// against, so a handle the wizard gathers always produces a url this
    /// crate recognises as belonging to that provider.
    pub fn profile_url(self, handle: &str) -> String {
        let spec = self.spec();
        format!(
            "{}{}{}",
            spec.url_prefixes[0], handle, spec.profile_url_suffix
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_social(url: &str, icon: Option<&str>, color: Option<&str>) -> RawSocialItem {
        RawSocialItem {
            url: url.to_string(),
            icon: icon.map(String::from),
            color: color.map(String::from),
            label: Some("Label".to_string()),
        }
    }

    #[test]
    fn a_social_color_with_a_leading_hash_is_rendered_without_it() {
        let item = SocialItem::new(&raw_social(
            "https://hachyderm.io/@gklijs",
            Some("mastodon"),
            Some("#ffffff"),
        ))
        .unwrap();
        assert_eq!(item.get_color(), "ffffff");
    }

    #[test]
    fn an_explicit_social_color_wins_over_the_brand_color() {
        let item = SocialItem::new(&raw_social(
            "https://github.com/gklijs",
            None,
            Some("eeeeee"),
        ))
        .unwrap();
        assert_eq!(item.get_color(), "eeeeee");
        let item = SocialItem::new(&raw_social("https://github.com/gklijs", None, None)).unwrap();
        assert_eq!(item.get_color(), "171515");
    }

    #[test]
    fn x_links_get_the_x_icon_and_color() {
        let item = SocialItem::new(&raw_social("https://x.com/gklijs", None, None)).unwrap();
        assert_eq!(item.get_icon(), "alpha-x-box");
        assert_eq!(item.get_color(), "000000");
    }
}
