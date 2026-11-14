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
    Bluesky,
    Instagram,
    Threads,
    TikTok,
    Twitch,
    Discord,
    Reddit,
    GitLab,
    Codeberg,
    StackOverflow,
    Medium,
    Spotify,
    SoundCloud,
    Vimeo,
    Pinterest,
    Telegram,
    WhatsApp,
    Signal,
    Email,
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
    /// Exact hosts, ignoring a leading `www.`. Paths never identify providers.
    hosts: &'static [&'static str],
    /// The profile address a bare handle is appended to.
    profile_url_prefix: &'static str,
    /// Appended after the handle (LinkedIn keeps its trailing slash).
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
        hosts: &["github.com"],
        profile_url_prefix: "https://github.com/",
        profile_url_suffix: "",
        icon: "github",
        brand_color: "171515",
        label: "GitHub",
    },
    ProviderSpec {
        kind: SocialProviderKind::LinkedIn,
        hosts: &["linkedin.com"],
        profile_url_prefix: "https://www.linkedin.com/in/",
        profile_url_suffix: "/",
        icon: "linkedin",
        brand_color: "0077b5",
        label: "LinkedIn",
    },
    ProviderSpec {
        kind: SocialProviderKind::Facebook,
        hosts: &["facebook.com"],
        profile_url_prefix: "https://www.facebook.com/",
        profile_url_suffix: "",
        icon: "facebook",
        brand_color: "4267B2",
        label: "Facebook",
    },
    ProviderSpec {
        kind: SocialProviderKind::YouTube,
        hosts: &["youtube.com"],
        profile_url_prefix: "https://www.youtube.com/",
        profile_url_suffix: "",
        icon: "youtube",
        brand_color: "c4302b",
        label: "YouTube",
    },
    ProviderSpec {
        kind: SocialProviderKind::Twitter,
        hosts: &["x.com", "twitter.com"],
        profile_url_prefix: "https://x.com/",
        profile_url_suffix: "",
        // MDI 6.5.95 has no X logo; a boxed X in X's black is closer to
        // the current brand than the old bird in Twitter blue.
        icon: "alpha-x-box",
        brand_color: "000000",
        label: "X",
    },
    ProviderSpec {
        kind: SocialProviderKind::Bluesky,
        hosts: &["bsky.app"],
        profile_url_prefix: "https://bsky.app/profile/",
        profile_url_suffix: "",
        icon: "bluesky",
        brand_color: "1185FE",
        label: "Bluesky",
    },
    ProviderSpec {
        kind: SocialProviderKind::Instagram,
        hosts: &["instagram.com"],
        profile_url_prefix: "https://www.instagram.com/",
        profile_url_suffix: "",
        icon: "instagram",
        brand_color: "FF0069",
        label: "Instagram",
    },
    ProviderSpec {
        kind: SocialProviderKind::Threads,
        hosts: &["threads.com", "threads.net"],
        profile_url_prefix: "https://www.threads.com/@",
        profile_url_suffix: "",
        icon: "threads",
        brand_color: "000000",
        label: "Threads",
    },
    ProviderSpec {
        kind: SocialProviderKind::TikTok,
        hosts: &["tiktok.com"],
        profile_url_prefix: "https://www.tiktok.com/@",
        profile_url_suffix: "",
        icon: "tiktok",
        brand_color: "000000",
        label: "TikTok",
    },
    ProviderSpec {
        kind: SocialProviderKind::Twitch,
        hosts: &["twitch.tv"],
        profile_url_prefix: "https://www.twitch.tv/",
        profile_url_suffix: "",
        icon: "twitch",
        brand_color: "9146FF",
        label: "Twitch",
    },
    ProviderSpec {
        kind: SocialProviderKind::Discord,
        hosts: &["discord.com", "discord.gg"],
        profile_url_prefix: "https://discord.com/users/",
        profile_url_suffix: "",
        icon: "discord",
        brand_color: "5865F2",
        label: "Discord",
    },
    ProviderSpec {
        kind: SocialProviderKind::Reddit,
        hosts: &["reddit.com"],
        profile_url_prefix: "https://www.reddit.com/user/",
        profile_url_suffix: "",
        icon: "reddit",
        brand_color: "FF4500",
        label: "Reddit",
    },
    ProviderSpec {
        kind: SocialProviderKind::GitLab,
        hosts: &["gitlab.com"],
        profile_url_prefix: "https://gitlab.com/",
        profile_url_suffix: "",
        icon: "gitlab",
        brand_color: "FC6D26",
        label: "GitLab",
    },
    ProviderSpec {
        kind: SocialProviderKind::Codeberg,
        hosts: &["codeberg.org"],
        profile_url_prefix: "https://codeberg.org/",
        profile_url_suffix: "",
        icon: "codeberg",
        brand_color: "2185D0",
        label: "Codeberg",
    },
    ProviderSpec {
        kind: SocialProviderKind::StackOverflow,
        hosts: &["stackoverflow.com"],
        profile_url_prefix: "https://stackoverflow.com/users/",
        profile_url_suffix: "",
        icon: "stack-overflow",
        brand_color: "F58025",
        label: "Stack Overflow",
    },
    ProviderSpec {
        kind: SocialProviderKind::Medium,
        hosts: &["medium.com"],
        profile_url_prefix: "https://medium.com/@",
        profile_url_suffix: "",
        icon: "medium",
        brand_color: "000000",
        label: "Medium",
    },
    ProviderSpec {
        kind: SocialProviderKind::Spotify,
        hosts: &["open.spotify.com"],
        profile_url_prefix: "https://open.spotify.com/user/",
        profile_url_suffix: "",
        icon: "spotify",
        brand_color: "1ED760",
        label: "Spotify",
    },
    ProviderSpec {
        kind: SocialProviderKind::SoundCloud,
        hosts: &["soundcloud.com"],
        profile_url_prefix: "https://soundcloud.com/",
        profile_url_suffix: "",
        icon: "soundcloud",
        brand_color: "FF5500",
        label: "SoundCloud",
    },
    ProviderSpec {
        kind: SocialProviderKind::Vimeo,
        hosts: &["vimeo.com"],
        profile_url_prefix: "https://vimeo.com/",
        profile_url_suffix: "",
        icon: "vimeo",
        brand_color: "1AB7EA",
        label: "Vimeo",
    },
    ProviderSpec {
        kind: SocialProviderKind::Pinterest,
        hosts: &["pinterest.com"],
        profile_url_prefix: "https://www.pinterest.com/",
        profile_url_suffix: "",
        icon: "pinterest",
        brand_color: "BD081C",
        label: "Pinterest",
    },
    ProviderSpec {
        kind: SocialProviderKind::Telegram,
        hosts: &["t.me", "telegram.me"],
        profile_url_prefix: "https://t.me/",
        profile_url_suffix: "",
        icon: "telegram",
        brand_color: "26A5E4",
        label: "Telegram",
    },
    ProviderSpec {
        kind: SocialProviderKind::WhatsApp,
        hosts: &["wa.me", "api.whatsapp.com"],
        profile_url_prefix: "https://wa.me/",
        profile_url_suffix: "",
        icon: "whatsapp",
        brand_color: "25D366",
        label: "WhatsApp",
    },
    ProviderSpec {
        kind: SocialProviderKind::Signal,
        hosts: &["signal.me"],
        profile_url_prefix: "https://signal.me/#eu/",
        profile_url_suffix: "",
        icon: "signal-messenger",
        brand_color: "3B45FD",
        label: "Signal",
    },
    ProviderSpec {
        kind: SocialProviderKind::Email,
        hosts: &[],
        profile_url_prefix: "mailto:",
        profile_url_suffix: "",
        icon: "email",
        brand_color: "000000",
        label: "Email",
    },
];

fn provider_spec(kind: SocialProviderKind) -> Option<&'static ProviderSpec> {
    PROVIDERS.iter().find(|p| p.kind == kind)
}

/// Recognises exact hosts (with optional `www.`) and email's `mailto:` scheme.
pub fn classify_social_provider(address: &str) -> SocialProviderKind {
    let Ok(url) = url::Url::parse(address) else {
        return SocialProviderKind::Other;
    };
    if url.scheme() == "mailto" && !url.path().is_empty() {
        return SocialProviderKind::Email;
    }
    if !matches!(url.scheme(), "http" | "https") {
        return SocialProviderKind::Other;
    }
    let host = url.host_str().unwrap_or_default();
    let host = host.strip_prefix("www.").unwrap_or(host);
    PROVIDERS
        .iter()
        .find(|p| p.hosts.contains(&host))
        .map_or(SocialProviderKind::Other, |p| p.kind)
}

impl SocialProviderKind {
    /// Preserve the original five providers' precedence. Newly recognised
    /// providers used to be custom links, so their explicit metadata wins.
    pub(crate) fn prefers_canonical_metadata(self) -> bool {
        matches!(
            self,
            Self::Github | Self::LinkedIn | Self::Facebook | Self::YouTube | Self::Twitter
        )
    }
}

/// Whether a social link's URL matches one of the providers `PROVIDERS`
/// recognises automatically, so `check` can flag an unrecognised provider
/// missing its required icon and color before generation would panic on it.
pub(crate) fn is_recognized_social_provider(url: &str) -> bool {
    classify_social_provider(url) != SocialProviderKind::Other
}

/// The bundled icon name a recognised provider defaults to. `None`
/// for `Other`, which has no canonical icon -- the maintainer must supply
/// one.
pub fn canonical_icon(provider: SocialProviderKind) -> Option<&'static str> {
    provider_spec(provider).map(|p| p.icon)
}

/// The brand color a recognised provider defaults to. `None` for `Other`, the
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

/// The accessible name a recognised provider defaults to. `None` for
/// `Other`, the same way as `canonical_icon`. See `vados.allium`'s
/// `SocialLink.label`.
pub fn canonical_label(provider: SocialProviderKind) -> Option<&'static str> {
    provider_spec(provider).map(|p| p.label)
}

pub(crate) fn resolve_metadata(
    provider: SocialProviderKind,
    given: &Option<String>,
    canonical: Option<&str>,
) -> Option<String> {
    let canonical = canonical.map(String::from);
    if provider.prefers_canonical_metadata() {
        canonical.or_else(|| given.clone())
    } else {
        given.clone().or(canonical)
    }
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
        let icon = resolve_metadata(provider, &raw.icon, canonical_icon(provider))
            .ok_or_else(|| missing("icon"))?;
        // An explicit color wins over the provider's brand color.
        let brand_color = raw
            .color
            .as_deref()
            .map(normalize_brand_color)
            .or_else(|| canonical_brand_color(provider).map(String::from))
            .ok_or_else(|| missing("color"))?;
        let label = resolve_metadata(provider, &raw.label, canonical_label(provider))
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
    Bluesky,
    Instagram,
    Threads,
    TikTok,
    Twitch,
    Discord,
    Reddit,
    GitLab,
    Codeberg,
    StackOverflow,
    Medium,
    Spotify,
    SoundCloud,
    Vimeo,
    Pinterest,
    Telegram,
    WhatsApp,
    Signal,
    Email,
}

impl RecognizedSocialProvider {
    fn kind(self) -> SocialProviderKind {
        match self {
            RecognizedSocialProvider::Github => SocialProviderKind::Github,
            RecognizedSocialProvider::LinkedIn => SocialProviderKind::LinkedIn,
            RecognizedSocialProvider::Facebook => SocialProviderKind::Facebook,
            RecognizedSocialProvider::YouTube => SocialProviderKind::YouTube,
            RecognizedSocialProvider::Twitter => SocialProviderKind::Twitter,
            RecognizedSocialProvider::Bluesky => SocialProviderKind::Bluesky,
            RecognizedSocialProvider::Instagram => SocialProviderKind::Instagram,
            RecognizedSocialProvider::Threads => SocialProviderKind::Threads,
            RecognizedSocialProvider::TikTok => SocialProviderKind::TikTok,
            RecognizedSocialProvider::Twitch => SocialProviderKind::Twitch,
            RecognizedSocialProvider::Discord => SocialProviderKind::Discord,
            RecognizedSocialProvider::Reddit => SocialProviderKind::Reddit,
            RecognizedSocialProvider::GitLab => SocialProviderKind::GitLab,
            RecognizedSocialProvider::Codeberg => SocialProviderKind::Codeberg,
            RecognizedSocialProvider::StackOverflow => SocialProviderKind::StackOverflow,
            RecognizedSocialProvider::Medium => SocialProviderKind::Medium,
            RecognizedSocialProvider::Spotify => SocialProviderKind::Spotify,
            RecognizedSocialProvider::SoundCloud => SocialProviderKind::SoundCloud,
            RecognizedSocialProvider::Vimeo => SocialProviderKind::Vimeo,
            RecognizedSocialProvider::Pinterest => SocialProviderKind::Pinterest,
            RecognizedSocialProvider::Telegram => SocialProviderKind::Telegram,
            RecognizedSocialProvider::WhatsApp => SocialProviderKind::WhatsApp,
            RecognizedSocialProvider::Signal => SocialProviderKind::Signal,
            RecognizedSocialProvider::Email => SocialProviderKind::Email,
        }
    }

    fn spec(self) -> &'static ProviderSpec {
        provider_spec(self.kind())
            .expect("every RecognizedSocialProvider variant has a PROVIDERS entry")
    }

    pub fn all() -> [RecognizedSocialProvider; 24] {
        use RecognizedSocialProvider::*;
        [
            Github,
            LinkedIn,
            Facebook,
            YouTube,
            Twitter,
            Bluesky,
            Instagram,
            Threads,
            TikTok,
            Twitch,
            Discord,
            Reddit,
            GitLab,
            Codeberg,
            StackOverflow,
            Medium,
            Spotify,
            SoundCloud,
            Vimeo,
            Pinterest,
            Telegram,
            WhatsApp,
            Signal,
            Email,
        ]
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
            spec.profile_url_prefix, handle, spec.profile_url_suffix
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_provider_recognises_its_hosts_and_has_a_bundled_icon() {
        for spec in PROVIDERS {
            assert!(crate::icons::is_known_icon(spec.icon), "{}", spec.label);
            for host in spec.hosts {
                for prefix in ["https://", "https://www.", "http://"] {
                    // Deliberately use the same path for all providers.
                    assert_eq!(
                        classify_social_provider(&format!("{prefix}{host}/@someone")),
                        spec.kind
                    );
                }
                assert_eq!(
                    classify_social_provider(&format!("https://{host}.example/@someone")),
                    SocialProviderKind::Other
                );
                assert_eq!(
                    classify_social_provider(&format!("https://{host}@example.com/@someone")),
                    SocialProviderKind::Other
                );
            }
            let raw = RawSocialItem {
                url: format!(
                    "{}someone{}",
                    spec.profile_url_prefix, spec.profile_url_suffix
                ),
                icon: None,
                color: None,
                label: None,
            };
            let item = SocialItem::new(&raw).unwrap();
            assert_eq!(item.get_icon(), spec.icon);
            assert_eq!(item.get_color(), spec.brand_color);
            assert_eq!(item.get_label(), spec.label);
            let mut custom = raw.clone();
            custom.color = Some("#abcdef".into());
            assert_eq!(SocialItem::new(&custom).unwrap().get_color(), "abcdef");
            if !spec.kind.prefers_canonical_metadata() {
                custom.icon = Some("home".into());
                custom.label = Some("My profile".into());
                let item = SocialItem::new(&custom).unwrap();
                assert_eq!(item.get_icon(), "home");
                assert_eq!(item.get_label(), "My profile");
            }
        }
        assert_eq!(
            classify_social_provider("mailto:you@example.com"),
            SocialProviderKind::Email
        );
        for url in [
            "mailto:",
            "https://unknown.example/@you",
            "ftp://github.com/you",
            "not a url",
        ] {
            assert_eq!(classify_social_provider(url), SocialProviderKind::Other);
        }
    }

    #[test]
    fn handles_become_the_expected_profile_addresses() {
        use RecognizedSocialProvider::*;
        let cases = [
            (Github, "https://github.com/you"),
            (LinkedIn, "https://www.linkedin.com/in/you/"),
            (Facebook, "https://www.facebook.com/you"),
            (YouTube, "https://www.youtube.com/you"),
            (Twitter, "https://x.com/you"),
            (Bluesky, "https://bsky.app/profile/you"),
            (Instagram, "https://www.instagram.com/you"),
            (Threads, "https://www.threads.com/@you"),
            (TikTok, "https://www.tiktok.com/@you"),
            (Twitch, "https://www.twitch.tv/you"),
            (Discord, "https://discord.com/users/you"),
            (Reddit, "https://www.reddit.com/user/you"),
            (GitLab, "https://gitlab.com/you"),
            (Codeberg, "https://codeberg.org/you"),
            (StackOverflow, "https://stackoverflow.com/users/you"),
            (Medium, "https://medium.com/@you"),
            (Spotify, "https://open.spotify.com/user/you"),
            (SoundCloud, "https://soundcloud.com/you"),
            (Vimeo, "https://vimeo.com/you"),
            (Pinterest, "https://www.pinterest.com/you"),
            (Telegram, "https://t.me/you"),
            (WhatsApp, "https://wa.me/you"),
            (Signal, "https://signal.me/#eu/you"),
            (Email, "mailto:you"),
        ];
        assert_eq!(cases.len(), RecognizedSocialProvider::all().len());
        for (provider, address) in cases {
            assert_eq!(provider.profile_url("you"), address);
            assert_eq!(classify_social_provider(address), provider.kind());
        }
    }

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
