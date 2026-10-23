use crate::bulma::{default_css_links, default_js_links, vados_js, Color};
use crate::files::write_raw;
use crate::luma::LumaEvent;
use serde::{Deserialize, Serialize};
use std::path::Path;

// Every config struct below derives `Serialize` as well as `Deserialize`:
// the content-authoring commands (`page new`, `image add`, `page add-image`,
// `social add`/`update`/`remove`, `footer set`, `menu add-item`) read one of
// these back, change the one thing they were asked to, and write the whole
// struct out again -- so the shape read and the shape written must always be
// exactly the same one. `generate`/`check` only ever read these, so before
// those commands existed there was nothing to round-trip.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MainConfig {
    pub(crate) site_title: String,
    pub(crate) js_files: Vec<String>,
    pub(crate) include_default_js: Option<bool>,
    pub(crate) css_files: Vec<String>,
    pub(crate) include_default_css: Option<bool>,
    pub(crate) background_class: Option<String>,
    pub(crate) navbar_color: Option<Color>,
    pub(crate) language: Option<String>,
    /// The site's description for search engines, used by every page that
    /// has no `summary` of its own. See `vados.allium`'s
    /// `SiteConfig.description` and `PagesAreDescribed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    /// A file in the source root (`favicon.svg`, `favicon.png`, `favicon.ico`)
    /// published at the site root and linked as every page's icon. See
    /// `vados.allium`'s `SiteConfig.favicon`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) favicon: Option<String>,
    /// Where the site is published, e.g. `https://example.com`. Feeds need it
    /// to give every link in full. See `vados.allium`'s `SiteConfig.site_url`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) site_url: Option<String>,
    /// What goes between a page's title and the site title in `<title>`;
    /// ` · ` when absent. See `vados.allium`'s `SiteConfig.title_separator`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) title_separator: Option<String>,
    pub(crate) footer_content: String,
}

impl MainConfig {
    pub(crate) fn get_background_class(&self) -> String {
        match self.background_class.as_ref() {
            None => String::from("has-background-light"),
            Some(s) => s.clone(),
        }
    }
    pub(crate) fn get_navbar_color(&self) -> &'static str {
        match self.navbar_color.as_ref() {
            None => Color::Warning.to_css_class(),
            Some(s) => s.to_css_class(),
        }
    }
    /// The site's content language, as an IETF BCP 47 tag, driving every
    /// published page's `<html lang>`. See `vados.allium`'s
    /// `SiteConfig.effective_language` and `SiteLanguageIsDeclared`.
    pub(crate) fn get_language(&self) -> &str {
        match self.language.as_deref() {
            None => "en",
            Some(l) => l,
        }
    }
    /// The site description, if it has one worth publishing: a blank one
    /// describes nothing, so it counts as missing.
    pub(crate) fn get_description(&self) -> Option<&str> {
        self.description
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
    }
    /// The site URL without a trailing slash, if it's an absolute http(s)
    /// URL -- the only kind a feed can build its links from.
    pub(crate) fn get_site_url(&self) -> Option<&str> {
        self.site_url
            .as_deref()
            .map(str::trim)
            .filter(|u| is_site_url(u))
            .map(|u| u.trim_end_matches('/'))
    }
    pub(crate) fn get_title_separator(&self) -> &str {
        self.title_separator.as_deref().unwrap_or(" · ")
    }
    pub(crate) fn get_css_links(&self) -> Vec<String> {
        match self.include_default_css {
            Some(b) if !b => self.css_files.clone(),
            _ => {
                let mut result = self.css_files.clone();
                result.append(&mut default_css_links());
                result
            }
        }
    }
    pub(crate) fn get_js_links(&self, destination: &str) -> Vec<String> {
        match self.include_default_js {
            Some(b) if !b => self.js_files.clone(),
            _ => {
                write_raw(destination, default_js_links().first().unwrap(), vados_js());
                let mut result = self.js_files.clone();
                result.append(&mut default_js_links());
                result
            }
        }
    }
}

/// Whether `url` can serve as the site URL: absolute, http or https, with a
/// host and nothing after the path (no query or fragment). See
/// `vados.allium`'s `is_site_url`.
pub fn is_site_url(url: &str) -> bool {
    let rest = match url.trim().split_once("://") {
        Some(("http" | "https", rest)) => rest,
        _ => return false,
    };
    let host = rest.split('/').next().unwrap_or_default();
    !host.is_empty() && !rest.contains(['?', '#']) && !rest.contains(char::is_whitespace)
}

/// The media type of a favicon, by file extension, for the kinds browsers
/// reliably show as a tab icon (SVG, PNG, ICO); `None` for anything else.
pub(crate) fn favicon_media_type(file_name: &str) -> Option<&'static str> {
    let extension = Path::new(file_name).extension()?.to_str()?;
    match extension.to_ascii_lowercase().as_str() {
        "svg" => Some("image/svg+xml"),
        "png" => Some("image/png"),
        "ico" => Some("image/x-icon"),
        _ => None,
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageConfig {
    pub(crate) title: String,
    pub(crate) sub_title: Option<String>,
    pub(crate) image: Option<String>,
    pub(crate) icon: Option<String>,
    pub(crate) summary: Option<String>,
    pub(crate) content: String,
    pub(crate) order: Option<u32>,
    pub(crate) left_notifications: Option<Vec<Notification>>,
    pub(crate) right_notifications: Option<Vec<Notification>>,
    /// Luma events shown in the page's main area, after its content. Left
    /// out of page.json entirely when there are none, so pages that never
    /// use Luma don't grow a `"lumaEvents": null` line on every rewrite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) luma_events: Option<Vec<LumaEvent>>,
    /// When the page was published, as `YYYY-MM-DD` with an optional
    /// `THH:MM[:SS]`. Kept as the raw text so a malformed date is reported
    /// against this one field instead of making the whole page.json
    /// unreadable. See `vados.allium`'s `Page.date`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) date: Option<String>,
    /// How this page's children are ordered; `order` when absent. See
    /// `vados.allium`'s `Page.child_order`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) child_order: Option<ChildOrder>,
    /// Whether this page publishes an Atom feed of its dated children. See
    /// `vados.allium`'s `Page.feed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) feed: Option<FeedSetting>,
}

impl PageConfig {
    /// The page's feed settings, if it publishes a feed at all.
    pub(crate) fn get_feed(&self) -> Option<FeedConfig> {
        match &self.feed {
            None | Some(FeedSetting::Enabled(false)) => None,
            Some(FeedSetting::Enabled(true)) => Some(FeedConfig::default()),
            Some(FeedSetting::Configured(c)) => Some(c.clone()),
        }
    }
}

/// page.json's `feed`: `true`/`false`, or an object giving the feed's title
/// and how many entries it holds.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(untagged)]
pub(crate) enum FeedSetting {
    Enabled(bool),
    Configured(FeedConfig),
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeedConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) limit: Option<u32>,
}

impl FeedConfig {
    /// How many entries a feed holds when page.json doesn't say.
    pub(crate) const DEFAULT_LIMIT: u32 = 20;

    pub(crate) fn get_limit(&self) -> u32 {
        self.limit.unwrap_or(Self::DEFAULT_LIMIT)
    }
}

/// How a page orders its children -- in menus, side menus and side
/// notifications alike. See `vados.allium`'s `ChildOrder`.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Copy, Default)]
#[serde(rename_all = "camelCase")]
pub enum ChildOrder {
    /// By `order`, then title.
    #[default]
    Order,
    /// Newest first; children without a date come last.
    DateDesc,
    /// Oldest first; children without a date come last.
    DateAsc,
}

impl PageConfig {
    pub(crate) fn new(path: &str) -> PageConfig {
        let last = path.split('/').next_back().unwrap();
        PageConfig {
            title: String::from(last),
            sub_title: None,
            image: None,
            icon: None,
            summary: None,
            content: format!("<p>{}</p>", last),
            order: None,
            left_notifications: None,
            right_notifications: None,
            luma_events: None,
            date: None,
            child_order: None,
            feed: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawMenuItem {
    pub(crate) url: String,
    pub(crate) title: Option<String>,
    pub(crate) icon: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawSocialItem {
    pub(crate) url: String,
    pub(crate) icon: Option<String>,
    pub(crate) color: Option<String>,
    /// The link's accessible name (see `vados.allium`'s `SocialLink.label`).
    /// Required for a provider `is_recognized_social_provider` doesn't
    /// recognise, the same as `icon`/`color`.
    pub(crate) label: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MenuConfig {
    pub(crate) main_menu: Vec<RawMenuItem>,
    pub(crate) socials: Vec<RawSocialItem>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Notification {
    pub(crate) content: String,
    pub(crate) title: Option<String>,
    pub(crate) image: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) color: Option<Color>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageReference {
    pub(crate) title: Option<String>,
    pub(crate) file_name: String,
    pub(crate) alt_text: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ImageList {
    pub(crate) title: Option<String>,
    pub(crate) list: Vec<ImageReference>,
}
