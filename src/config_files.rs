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
//
// In tests they also derive `JsonSchema`, which generates the published
// schemas in `schemas/` (see `crate::schemas`). Their `///` comments become
// the descriptions editors show while a site's author types, so they are
// written for that author; notes for maintainers use `//`.

/// The JSON Schema `vados init` and the authoring commands point new files
/// to, published from the repository at this version's tag.
pub(crate) fn schema_url(file: &str) -> String {
    let stem = file.trim_end_matches(".json");
    format!(
        "https://raw.githubusercontent.com/gklijs/vados/v{}/schemas/{stem}.schema.json",
        env!("CARGO_PKG_VERSION")
    )
}

/// Site-wide settings: `main.json` in the root of the source tree.
#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields, title = "vados main.json"))]
#[serde(rename_all = "camelCase")]
pub(crate) struct MainConfig {
    /// The JSON Schema this file follows, so editors can complete and check
    /// it. vados itself ignores it.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub(crate) schema: Option<String>,
    /// Publishes a Markdown version of every page next to its HTML, such as
    /// `/blog/post.md`. Off when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) markdown: Option<bool>,
    /// The site's name: the home page's title, and part of every other
    /// page's `<title>`.
    pub(crate) site_title: String,
    /// Extra scripts every page loads, as URLs or paths from the site root.
    #[serde(default)]
    pub(crate) js_files: Vec<String>,
    /// Whether pages also load vados's own script, which runs the navbar's
    /// dropdowns and burger menu. On when absent.
    pub(crate) include_default_js: Option<bool>,
    /// Extra stylesheets every page loads, as URLs or paths from the site
    /// root. They must use Bulma 1.0 or later.
    #[serde(default)]
    pub(crate) css_files: Vec<String>,
    /// Whether pages also load Bulma from jsDelivr. On when absent; turn it
    /// off when `cssFiles` holds a compiled Bulma of your own.
    pub(crate) include_default_css: Option<bool>,
    /// The CSS class(es) on the page's background; `has-background-light-soft`
    /// when absent.
    pub(crate) background_class: Option<String>,
    /// The navbar's Bulma colour; `Warning` when absent.
    pub(crate) navbar_color: Option<Color>,
    /// The site's language as a BCP 47 tag, such as `en`, `nl` or `nl-BE`;
    /// `en` when absent. It sets each page's `lang` and the language of the
    /// text vados adds itself.
    pub(crate) language: Option<String>,
    /// A short description of the site for search engines and link previews,
    /// used on every page without a `summary` of its own.
    //
    // See `vados.allium`'s `SiteConfig.description` and `PagesAreDescribed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    /// A file in the source root, such as `favicon.svg`, `favicon.png` or
    /// `favicon.ico`, published at the site root as every page's icon.
    //
    // See `vados.allium`'s `SiteConfig.favicon`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) favicon: Option<String>,
    /// Where the site is published, such as `https://example.com`. Feeds,
    /// the sitemap, calendar files and link-preview images need it.
    //
    // See `vados.allium`'s `SiteConfig.site_url`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) site_url: Option<String>,
    /// What goes between a page's title and the site title in `<title>`;
    /// ` · ` when absent.
    //
    // See `vados.allium`'s `SiteConfig.title_separator`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) title_separator: Option<String>,
    /// An image reference, like a page's `image` (`/team/alice`), shown in
    /// link previews of pages without a hero image of their own.
    //
    // See `vados.allium`'s `SiteConfig.default_image`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) default_image: Option<String>,
    /// Who writes the site, named in structured data and feeds. The site
    /// title when absent.
    //
    // See `vados.allium`'s `SiteConfig.author`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) author: Option<AuthorConfig>,
    /// A directory, relative to the source root, whose files are published
    /// as they are at the site root: `robots.txt`, `_headers`, downloads.
    //
    // See `vados.allium`'s `SiteConfig.static_dir`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) static_dir: Option<String>,
    /// Your fediverse handle, `@user@instance`, which Mastodon shows as the
    /// author of link previews. Taken from a Mastodon social link when absent.
    //
    // See `vados.allium`'s `SiteConfig.fediverse_creator`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) fediverse_creator: Option<String>,
    /// Whether fenced Markdown code blocks that name a language are
    /// highlighted. On when absent.
    //
    // See `vados.allium`'s `SiteConfig.code_highlight`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) code_highlight: Option<bool>,
    /// Whether fenced `mermaid` blocks become diagrams in the reader's
    /// browser. On when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) mermaid: Option<bool>,
    /// Where pages with a diagram load Mermaid from: an http(s) module URL,
    /// or a path from the site root to a file in `staticDir`. jsDelivr when
    /// absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) mermaid_script: Option<String>,
    /// Pins every page to Bulma's light or dark theme. Without it, pages
    /// follow the reader's colour-scheme preference.
    //
    // See `vados.allium`'s `SiteConfig.color_scheme` and
    // `FollowsTheReadersColourScheme`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) color_scheme: Option<ColorScheme>,
    /// Whether the last side notification grows to end where the page's
    /// content does. Off when absent.
    //
    // See `vados.allium`'s `SiteConfig.stretch_side_notification`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) stretch_side_notification: Option<bool>,
    /// Which Bulma theme sits on the navbar, for a stylesheet that recolours
    /// `navbarColor` to the opposite brightness. Without it, the colour's
    /// default brightness decides.
    //
    // See `vados.allium`'s `SiteConfig.navbar_theme`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) navbar_theme: Option<NavbarTheme>,
    /// How many children a main menu dropdown lists before ending in a link
    /// to the section; 12 when absent.
    //
    // See `vados.allium`'s `SiteConfig.menu_dropdown_limit`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) menu_dropdown_limit: Option<u32>,
    /// A block of links for the cells the shorter column of tiles leaves
    /// empty, such as an archive or a way to subscribe.
    //
    // See `vados.allium`'s `SiteConfig.spare_tile`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) spare_tile: Option<SpareTile>,
    /// Publishes a site search at `/search`: `true`, or which pages to
    /// search.
    //
    // See `vados.allium`'s `SiteSearch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) search: Option<SearchSetting>,
    /// The footer on every page: inline HTML, or the name of a `.md` or
    /// `.html` file in the source root.
    pub(crate) footer_content: String,
}

/// A block of links for the cells child tiles leave empty.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpareTile {
    /// The block's heading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    pub(crate) links: Vec<SpareTileLink>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpareTileLink {
    /// A page's path, such as `/topics`, or a full URL.
    pub(crate) url: String,
    /// The link's text.
    pub(crate) title: String,
}

/// `true` or `false`, or which pages to search.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(untagged)]
pub(crate) enum SearchSetting {
    Enabled(bool),
    Configured(SearchConfig),
}

/// Which pages the site search covers.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchConfig {
    /// Only pages below this path, or these paths. Every page when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) from: Option<OneOrMany>,
    /// How many levels below `from` to look; any depth when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) depth: Option<u32>,
    /// Also search pages with `"listed": false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) include_unlisted: Option<bool>,
    /// Whether the main menu gets an entry for the search. On when absent.
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

/// Bulma's `light` or `dark` theme.
// Bulma's `data-theme` values.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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

/// Who writes the site.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthorConfig {
    /// The author's name.
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
    pub(crate) fn renders_mermaid(&self) -> bool {
        self.mermaid.unwrap_or(true)
    }

    pub(crate) fn get_mermaid_script(&self) -> &str {
        self.mermaid_script
            .as_deref()
            .unwrap_or(crate::mermaid::DEFAULT_SCRIPT)
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
    pub(crate) fn get_js_links(
        &self,
        destination: &crate::files::Output,
    ) -> Result<Vec<String>, crate::generator::GenerateError> {
        Ok(match self.include_default_js {
            Some(b) if !b => self.js_files.clone(),
            _ => {
                write_raw(destination, default_js_links().first().unwrap(), vados_js())?;
                let mut result = self.js_files.clone();
                result.append(&mut default_js_links());
                result
            }
        })
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

/// The pattern of a page's `date` and `updated`, for the published schema.
#[cfg(test)]
pub(crate) const PAGE_DATE_PATTERN: &str = r"^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}(:\d{2})?)?$";

/// One page: the `page.json` in each directory of the source tree.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields, title = "vados page.json"))]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageConfig {
    /// The JSON Schema this file follows, so editors can complete and check
    /// it. vados itself ignores it.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub(crate) schema: Option<String>,
    /// The page's title: its heading, and its name in menus and tiles.
    pub(crate) title: String,
    /// A line shown under the title.
    pub(crate) sub_title: Option<String>,
    /// The page's hero image, as an image reference: the image's directory
    /// in the image tree and its file name without extension, such as
    /// `/team/alice`.
    pub(crate) image: Option<String>,
    /// A Material Design Icons name, such as `home` or `rss`, shown with the
    /// page in menus.
    pub(crate) icon: Option<String>,
    /// A short description of the page for search engines, link previews,
    /// feeds and the tiles advertising it.
    pub(crate) summary: Option<String>,
    /// The page's content: inline HTML, or the name of a `.md` or `.html`
    /// file in the page's directory.
    pub(crate) content: String,
    /// Where the page comes among its siblings when they are ordered by
    /// `order`: lower first, then by title.
    pub(crate) order: Option<u32>,
    /// Tiles in the left column below the page's content.
    pub(crate) left_notifications: Option<Vec<Notification>>,
    /// Tiles in the right column below the page's content.
    pub(crate) right_notifications: Option<Vec<Notification>>,
    /// Events hosted on Luma, shown after the page's content.
    // Left out of page.json entirely when there are none, so pages that never
    // use Luma don't grow a `"lumaEvents": null` line on every rewrite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) luma_events: Option<Vec<LumaEvent>>,
    /// When the page was published: `YYYY-MM-DD`, optionally with a time,
    /// such as `2026-10-22T09:30`.
    // Kept as the raw text so a malformed date is reported against this one
    // field instead of making the whole page.json unreadable. See
    // `vados.allium`'s `Page.date`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, schemars(pattern(PAGE_DATE_PATTERN)))]
    pub(crate) date: Option<String>,
    /// When the page last changed meaningfully, in the same form as `date`.
    // See `vados.allium`'s `Page.updated`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, schemars(pattern(PAGE_DATE_PATTERN)))]
    pub(crate) updated: Option<String>,
    /// `false` publishes the page but keeps it out of menus, side tiles,
    /// feeds, the site search and the sitemap.
    // See `vados.allium`'s `Page.listed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) listed: Option<bool>,
    /// How this page's children are ordered in menus and tiles; `order`
    /// when absent.
    // See `vados.allium`'s `Page.child_order`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) child_order: Option<ChildOrder>,
    /// Publishes an Atom feed of the page's dated children at the page's
    /// path plus `/feed.xml`: `true`, or the feed's settings. Needs
    /// main.json's `siteUrl`.
    // See `vados.allium`'s `Page.feed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) feed: Option<FeedSetting>,
    /// Shows the newest dated pages from elsewhere in the site as tiles.
    // See `vados.allium`'s `Page.latest_sources`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) latest: Option<LatestConfig>,
    /// Keeps the page out of search results, the site search and the
    /// sitemap, while it stays listed in menus.
    // See `vados.allium`'s `Page.noindex`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) noindex: Option<bool>,
    /// Makes the page an event.
    // See `vados.allium`'s `Event`. An event that doesn't match is still read,
    // so `check` can report it, but the schema only describes a valid one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, schemars(with = "Option<crate::event::EventConfig>"))]
    pub(crate) event: Option<DeclaredEvent>,
    /// Lists events from elsewhere on this page, soonest first: `true` for
    /// those below it, or which pages to take them from.
    // See `vados.allium`'s `Page.agenda`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) agenda: Option<AgendaSetting>,
}

/// `true` or `false`, or which pages to list events from.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(untagged)]
pub(crate) enum AgendaSetting {
    Enabled(bool),
    Configured(AgendaConfig),
}

/// Which pages an agenda lists events from.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgendaConfig {
    /// Events on pages below this path, or these paths. The pages below
    /// this one when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) from: Option<OneOrMany>,
    /// How many levels below `from` to look; any depth when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) depth: Option<u32>,
    /// Also list events on pages with `"listed": false`.
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

/// Which sections to take the newest dated pages from, how many, and how
/// deep below each to look.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct LatestConfig {
    /// A section's path, such as `/blog`, or a list of them.
    pub(crate) from: OneOrMany,
    /// How many pages to show; 4 when absent.
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

/// A page's path, such as `/blog`, or a list of them.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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

/// `true` or `false`, or the feed's settings.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(untagged)]
pub(crate) enum FeedSetting {
    Enabled(bool),
    Configured(FeedConfig),
}

/// A feed's title, size and entries.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeedConfig {
    /// The feed's title; the site title plus the page's when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    /// How many entries the feed holds; 20 when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) limit: Option<u32>,
    /// Takes entries from below this path, or these paths, instead of the
    /// page's own direct children.
    // See `vados.allium`'s `Page.feed_entries`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) from: Option<OneOrMany>,
    /// How many levels below `from` to look: any depth when `from` is
    /// given, only direct children when it isn't.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) depth: Option<u32>,
    /// Also takes pages with `"listed": false`, which feeds otherwise leave
    /// out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) include_unlisted: Option<bool>,
    /// What an entry carries; the page's full content when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<FeedContent>,
}

/// What a feed entry carries: `full` content, or only the `summary`.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum FeedContent {
    Full,
    // Only the `summary`, for a page that has one.
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

/// How a page orders its children in menus and tiles: `order` (by
/// `order`, then title), `dateDesc` (newest first) or `dateAsc` (oldest
/// first). Undated children come last.
// See `vados.allium`'s `ChildOrder`.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Copy, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ChildOrder {
    // By `order`, then title.
    #[default]
    Order,
    // Newest first; children without a date come last.
    DateDesc,
    // Oldest first; children without a date come last.
    DateAsc,
}

impl PageConfig {
    pub(crate) fn new(path: &str) -> PageConfig {
        let last = path.split('/').next_back().unwrap();
        PageConfig {
            schema: Some(schema_url("page.json")),
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

/// One entry of the main menu.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawMenuItem {
    /// A page's path, such as `/blog`, or a full URL.
    pub(crate) url: String,
    /// The entry's text. Required for a full URL; a page's own title when
    /// absent.
    pub(crate) title: Option<String>,
    /// A Material Design Icons name, such as `home`; the page's own icon
    /// when absent.
    pub(crate) icon: Option<String>,
}

/// The pattern of a social link's `color`, for the published schema.
#[cfg(test)]
pub(crate) const HEX_COLOR_PATTERN: &str =
    "^#?([0-9A-Fa-f]{3}|[0-9A-Fa-f]{4}|[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})$";

/// A link to your profile on another site, shown as an icon on every page.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawSocialItem {
    /// The profile's address. vados recognises GitHub, LinkedIn, Facebook,
    /// YouTube, Twitter/X and Mastodon links by it.
    pub(crate) url: String,
    /// A Material Design Icons name. Required for a site vados doesn't
    /// recognise.
    pub(crate) icon: Option<String>,
    /// The icon's colour, in hex with or without `#`, such as `6364FF`.
    /// Required for a site vados doesn't recognise; it also overrides a
    /// recognised site's own colour.
    #[cfg_attr(test, schemars(pattern(HEX_COLOR_PATTERN)))]
    pub(crate) color: Option<String>,
    /// What screen readers announce for the link, which has no visible
    /// text. Required for a site vados doesn't recognise.
    // See `vados.allium`'s `SocialLink.label`. Required for a provider
    // `is_recognized_social_provider` doesn't recognise, the same as
    // `icon`/`color`.
    pub(crate) label: Option<String>,
}

/// The main menu and social links: `menu.json` in the root of the source
/// tree.
#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields, title = "vados menu.json"))]
#[serde(rename_all = "camelCase")]
pub(crate) struct MenuConfig {
    /// The JSON Schema this file follows, so editors can complete and check
    /// it. vados itself ignores it.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub(crate) schema: Option<String>,
    /// The navbar's entries, in order.
    pub(crate) main_menu: Vec<RawMenuItem>,
    /// Links to your profiles on other sites, in order.
    pub(crate) socials: Vec<RawSocialItem>,
}

/// A tile below a page's content.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Notification {
    /// The tile's text: inline HTML, or the name of a `.md` or `.html` file
    /// in the page's directory.
    pub(crate) content: String,
    /// The tile's heading.
    pub(crate) title: Option<String>,
    /// An image reference, such as `/team/alice`, shown in the tile.
    pub(crate) image: Option<String>,
    /// Where the tile links to: a page's path or a full URL.
    pub(crate) url: Option<String>,
    /// The tile's Bulma colour; `Info` or `Link` when absent, depending on
    /// the tile.
    pub(crate) color: Option<Color>,
}

/// One image to publish.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageReference {
    /// The image's caption.
    pub(crate) title: Option<String>,
    /// The image file, in the same directory as this images.json.
    pub(crate) file_name: String,
    /// What the image shows, for readers who can't see it. Required.
    pub(crate) alt_text: String,
}

/// The images to publish from one directory: `images.json` in a directory
/// of the image tree.
#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields, title = "vados images.json"))]
pub(crate) struct ImageList {
    /// The JSON Schema this file follows, so editors can complete and check
    /// it. vados itself ignores it.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub(crate) schema: Option<String>,
    /// A title for the directory's images.
    pub(crate) title: Option<String>,
    /// The images, each published as responsive WebP variants.
    pub(crate) list: Vec<ImageReference>,
}

impl ImageList {
    /// An images.json with no images yet, as an authoring command creates it.
    pub(crate) fn new() -> ImageList {
        ImageList {
            schema: Some(schema_url("images.json")),
            title: None,
            list: vec![],
        }
    }
}
