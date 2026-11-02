use crate::bulma::{default_css_links, default_js_links, vados_js, Color, NavbarTheme};
use crate::event::DeclaredEvent;
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
    /// An image reference, like a page's `image`, shown in link previews of
    /// every page without a hero image of its own. See `vados.allium`'s
    /// `SiteConfig.default_image`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) default_image: Option<String>,
    /// Who writes the site, named in structured data and feeds; the site
    /// title when absent. See `vados.allium`'s `SiteConfig.author`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) author: Option<AuthorConfig>,
    /// A directory, relative to the source root, whose files are published
    /// as they are at the site root. See `vados.allium`'s
    /// `SiteConfig.static_dir`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) static_dir: Option<String>,
    /// The site owner's fediverse handle, `@user@instance`, which Mastodon
    /// shows as the author of link previews. See `vados.allium`'s
    /// `SiteConfig.fediverse_creator`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) fediverse_creator: Option<String>,
    /// Whether fenced Markdown code blocks are highlighted; on when absent.
    /// See `vados.allium`'s `SiteConfig.code_highlight`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) code_highlight: Option<bool>,
    /// Pins the site to Bulma's light or dark theme instead of following the
    /// reader's colour-scheme preference. See `vados.allium`'s
    /// `SiteConfig.color_scheme` and `FollowsTheReadersColourScheme`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) color_scheme: Option<ColorScheme>,
    /// Whether the last side notification grows to end where the page's
    /// content does; off when absent. See `vados.allium`'s
    /// `SiteConfig.stretch_side_notification`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) stretch_side_notification: Option<bool>,
    /// Which theme sits on the navbar, for a colour a site's stylesheet
    /// recolours to the opposite brightness. See `vados.allium`'s
    /// `SiteConfig.navbar_theme`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) navbar_theme: Option<NavbarTheme>,
    /// How many children a main menu dropdown lists before ending in a link
    /// to the section. See `vados.allium`'s `SiteConfig.menu_dropdown_limit`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) menu_dropdown_limit: Option<u32>,
    /// What fills the cells the shorter column of child tiles leaves over.
    /// See `vados.allium`'s `SiteConfig.spare_tile`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) spare_tile: Option<SpareTile>,
    /// Publishes a site search. See `vados.allium`'s `SiteSearch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) search: Option<SearchSetting>,
    pub(crate) footer_content: String,
}

/// main.json's `spareTile`: a block of links for the cells child tiles leave.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpareTile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    pub(crate) links: Vec<SpareTileLink>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpareTileLink {
    pub(crate) url: String,
    pub(crate) title: String,
}

/// main.json's `search`: `true`/`false`, or which pages to search.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(untagged)]
pub(crate) enum SearchSetting {
    Enabled(bool),
    Configured(SearchConfig),
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchConfig {
    /// Every page when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) from: Option<OneOrMany>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) include_unlisted: Option<bool>,
    /// Whether the main menu gets an entry for it; on when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) menu: Option<bool>,
}

impl SearchConfig {
    pub(crate) fn shows_in_menu(&self) -> bool {
        self.menu.unwrap_or(true)
    }
}

/// Which pages a feed, agenda or search draws from; the same idea as
/// `latest`. See `vados.allium`'s `PageSelection`.
pub(crate) struct PageSelection<'a> {
    /// `None` selects every page.
    pub(crate) sources: Option<Vec<&'a str>>,
    pub(crate) depth: Option<u32>,
    pub(crate) include_unlisted: bool,
}

impl PageSelection<'_> {
    /// Whether the selection takes the page at `path`.
    pub(crate) fn selects(&self, path: &str, listed: bool) -> bool {
        (listed || self.include_unlisted)
            && match &self.sources {
                Some(sources) => sources
                    .iter()
                    .any(|source| is_below(path, source, self.depth)),
                // Every page, or those within `depth` levels of the home page.
                None => self
                    .depth
                    .is_none_or(|d| path.split('/').filter(|s| !s.is_empty()).count() as u32 <= d),
            }
    }
}

/// Whether `path` is below `source`, at most `depth` levels down when given.
pub(crate) fn is_below(path: &str, source: &str, depth: Option<u32>) -> bool {
    let rest = if source == "/" {
        path.strip_prefix('/')
    } else {
        path.strip_prefix(source).and_then(|r| r.strip_prefix('/'))
    };
    rest.filter(|r| !r.is_empty())
        .is_some_and(|r| depth.is_none_or(|d| r.split('/').count() as u32 <= d))
}

/// A colour scheme a site can be pinned to; Bulma's `data-theme` values.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ColorScheme {
    Light,
    Dark,
}

impl ColorScheme {
    pub(crate) fn to_data_theme(self) -> &'static str {
        match self {
            ColorScheme::Light => "light",
            ColorScheme::Dark => "dark",
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthorConfig {
    pub(crate) name: String,
    /// The author's own page, when it isn't the site's home page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
}

impl MainConfig {
    /// See `vados.allium`'s `config.default_menu_dropdown_limit`.
    pub(crate) const DEFAULT_MENU_DROPDOWN_LIMIT: u32 = 12;

    /// The search settings, when the site publishes a search.
    pub(crate) fn get_search(&self) -> Option<SearchConfig> {
        match &self.search {
            None | Some(SearchSetting::Enabled(false)) => None,
            Some(SearchSetting::Enabled(true)) => Some(SearchConfig::default()),
            Some(SearchSetting::Configured(c)) => Some(c.clone()),
        }
    }
    pub(crate) fn get_background_class(&self) -> String {
        match self.background_class.as_ref() {
            None => String::from("has-background-light-soft"),
            Some(s) => s.clone(),
        }
    }
    pub(crate) fn get_navbar_color(&self) -> String {
        self.navbar_color
            .as_ref()
            .unwrap_or(&Color::Warning)
            .to_navbar_class(self.navbar_theme)
    }
    /// How many children a main menu dropdown lists; at least one, since a
    /// dropdown of nothing but its "more" link would say nothing.
    pub(crate) fn get_menu_dropdown_limit(&self) -> usize {
        self.menu_dropdown_limit
            .unwrap_or(Self::DEFAULT_MENU_DROPDOWN_LIMIT)
            .max(1) as usize
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
    /// Who the site names as its author: main.json's `author`, or else the
    /// site title.
    pub(crate) fn get_author_name(&self) -> &str {
        self.author
            .as_ref()
            .map(|a| a.name.trim())
            .filter(|n| !n.is_empty())
            .unwrap_or(&self.site_title)
    }
    pub(crate) fn highlights_code(&self) -> bool {
        self.code_highlight.unwrap_or(true)
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
    /// When the page last changed meaningfully, in the same form as `date`.
    /// See `vados.allium`'s `Page.updated`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) updated: Option<String>,
    /// `false` keeps the page out of menus, side tiles, feeds and the
    /// sitemap while still publishing it. See `vados.allium`'s `Page.listed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) listed: Option<bool>,
    /// How this page's children are ordered; `order` when absent. See
    /// `vados.allium`'s `Page.child_order`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) child_order: Option<ChildOrder>,
    /// Whether this page publishes an Atom feed of its dated children. See
    /// `vados.allium`'s `Page.feed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) feed: Option<FeedSetting>,
    /// Shows the newest dated pages from elsewhere in the site as child
    /// tiles. See `vados.allium`'s `Page.latest_sources`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) latest: Option<LatestConfig>,
    /// Keeps the page out of search results and the sitemap, which `listed`
    /// doesn't. See `vados.allium`'s `Page.noindex`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) noindex: Option<bool>,
    /// Makes the page an event. See `vados.allium`'s `Event`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) event: Option<DeclaredEvent>,
    /// Lists events from elsewhere on this page, soonest first. See
    /// `vados.allium`'s `Page.agenda`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) agenda: Option<AgendaSetting>,
}

/// page.json's `agenda`: `true`/`false`, or which pages to list.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(untagged)]
pub(crate) enum AgendaSetting {
    Enabled(bool),
    Configured(AgendaConfig),
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgendaConfig {
    /// The page's own descendants when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) from: Option<OneOrMany>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) include_unlisted: Option<bool>,
}

impl AgendaConfig {
    pub(crate) fn selection<'a>(&'a self, page_path: &'a str) -> PageSelection<'a> {
        PageSelection {
            sources: Some(self.from.as_ref().map_or(vec![page_path], OneOrMany::paths)),
            depth: self.depth,
            include_unlisted: self.include_unlisted.unwrap_or(false),
        }
    }
}

/// page.json's `latest`: which sections to take the newest dated pages
/// from, how many, and how deep below each to look.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LatestConfig {
    pub(crate) from: OneOrMany,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) limit: Option<u32>,
    /// How many levels below a section to look; any depth when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) depth: Option<u32>,
}

impl LatestConfig {
    /// How many pages are shown when page.json doesn't say.
    pub(crate) const DEFAULT_LIMIT: u32 = 4;

    pub(crate) fn get_limit(&self) -> u32 {
        self.limit.unwrap_or(Self::DEFAULT_LIMIT)
    }
    pub(crate) fn sources(&self) -> Vec<&str> {
        self.from.paths()
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(untagged)]
pub(crate) enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    pub(crate) fn paths(&self) -> Vec<&str> {
        match self {
            OneOrMany::One(s) => vec![s.as_str()],
            OneOrMany::Many(v) => v.iter().map(String::as_str).collect(),
        }
    }
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
    /// The page's agenda settings, if it shows an agenda at all.
    pub(crate) fn get_agenda(&self) -> Option<AgendaConfig> {
        match &self.agenda {
            None | Some(AgendaSetting::Enabled(false)) => None,
            Some(AgendaSetting::Enabled(true)) => Some(AgendaConfig::default()),
            Some(AgendaSetting::Configured(c)) => Some(c.clone()),
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
    /// Sections to take entries from; the page's own direct children when
    /// absent. See `vados.allium`'s `Page.feed_entries`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) from: Option<OneOrMany>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) depth: Option<u32>,
    /// Takes unlisted pages too: "listed nowhere" otherwise includes feeds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) include_unlisted: Option<bool>,
    /// What an entry carries; the page's full content when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<FeedContent>,
}

/// What a feed entry carries as its content.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub(crate) enum FeedContent {
    Full,
    /// Only the `summary`, for a page that has one.
    Summary,
}

impl FeedConfig {
    /// How many entries a feed holds when page.json doesn't say.
    pub(crate) const DEFAULT_LIMIT: u32 = 20;

    pub(crate) fn get_limit(&self) -> u32 {
        self.limit.unwrap_or(Self::DEFAULT_LIMIT)
    }
    /// Which pages this feed may list, before leaving out undated ones.
    pub(crate) fn selection<'a>(&'a self, page_path: &'a str) -> PageSelection<'a> {
        PageSelection {
            sources: Some(self.from.as_ref().map_or(vec![page_path], OneOrMany::paths)),
            // Without `from` it stays the direct children it always was.
            depth: self.depth.or(self.from.is_none().then_some(1)),
            include_unlisted: self.include_unlisted.unwrap_or(false),
        }
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
            updated: None,
            listed: None,
            child_order: None,
            feed: None,
            latest: None,
            noindex: None,
            event: None,
            agenda: None,
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
