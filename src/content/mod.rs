//! Page assembly: what every page shares, and rendering one page from the
//! structure. Content references, navigation, child tiles and the page
//! template's parts each have their own module below.

use crate::authoring::social::{fediverse_handle_of, MASTODON_ICON};
use crate::bulma::ImageType;
use crate::config_files::{favicon_media_type, ColorScheme, MainConfig, MenuConfig, SpareTile};
use crate::files::write_raw;
use crate::highlight;
use crate::image::ProcessedImage;
use crate::structure::{Item, Structure};
use crate::templates::{FooterTemplate, InternalImageTemplate};
use crate::ui_text::{ui_text, UiText};
use askama::Template;
use std::fs;
use std::path::Path;
use std::sync::Arc;

mod navigation;
mod page;
mod reference;
mod tiles;

use navigation::{get_breadcrumbs, get_navigation, get_side_menu};
use page::{get_main_content, get_page};
pub(crate) use reference::{
    classify_content_reference, ensure_content_reference, escape_html, get_content, get_file_path,
    has_top_level_heading, ContentKind,
};
pub(crate) use tiles::{items_to_side_notifications, to_content_items};

/// Where the agenda script is published, for the pages showing an agenda.
pub(crate) const AGENDA_SCRIPT_HREF: &str = "/js/vados-agenda.js";

/// Options shared by every Markdown-to-HTML entry point. Feeds, search and
/// Markdown alternates deliberately disable browser-only diagrams.
#[derive(Clone, Copy, Default)]
pub(crate) struct MarkdownOptions {
    pub(crate) highlight: bool,
    pub(crate) mermaid: bool,
}

impl MarkdownOptions {
    pub(crate) fn for_site(config: &MainConfig) -> Self {
        Self {
            highlight: config.highlights_code(),
            mermaid: config.renders_mermaid(),
        }
    }
}

pub(crate) struct GenericContent {
    markdown: bool,
    background_class: String,
    /// Bulma's `data-theme` for `<html>`, when the site is pinned to one.
    data_theme: Option<&'static str>,
    /// Whether the last side notification grows to end with the page's
    /// content.
    stretch_side_notification: bool,
    css_links: Vec<String>,
    js_links: Vec<String>,
    footer: String,
    language: String,
    ui: &'static UiText,
    description: Option<String>,
    favicon: Option<Favicon>,
    site_title: String,
    title_separator: String,
    /// The usable `siteUrl`, without a trailing slash.
    site_url: Option<String>,
    default_image: Option<String>,
    author_name: String,
    author_url: Option<String>,
    /// The social links' URLs: the site owner's own profiles elsewhere.
    social_urls: Vec<String>,
    fediverse_creator: Option<String>,
    options: MarkdownOptions,
    mermaid_script: String,
    /// Whether feeds are published at all: only with a usable `siteUrl`.
    feeds_published: bool,
    spare_tile: Option<SpareTile>,
    /// Whether the site publishes a search.
    search: bool,
}

/// The site icon as pages link it: where it was published, and its media
/// type when the file extension names one browsers know.
struct Favicon {
    href: String,
    media_type: Option<&'static str>,
}

/// Publishes main.json's `favicon` at the site root under its own file name.
/// A missing file is skipped with a message, the same way a missing image is:
/// the site is still worth publishing without an icon, and `check` reports it.
fn publish_favicon(
    source: &str,
    destination: &crate::files::Output,
    favicon: &str,
) -> Result<Option<Favicon>, crate::generator::GenerateError> {
    let Some(file_name) = Path::new(favicon).file_name().and_then(|f| f.to_str()) else {
        return Ok(None);
    };
    let contents = match fs::read(format!("{}/{}", source, favicon)) {
        Ok(c) => c,
        Err(e) => {
            crate::parallel::message(format!(
                "favicon {} could not be read, skipping it: {}",
                favicon, e
            ));
            return Ok(None);
        }
    };
    let href = format!("/{}", file_name);
    write_raw(destination, &href, contents)?;
    let media_type = favicon_media_type(file_name);
    Ok(Some(Favicon { href, media_type }))
}

/// Writes the stylesheet for highlighted code when code is highlighted at
/// all, so pages with highlighted code can link it.
fn publish_highlight_stylesheet(
    destination: &crate::files::Output,
    main_config: &MainConfig,
) -> Result<bool, crate::generator::GenerateError> {
    let css = highlight::stylesheet();
    let highlight = main_config.highlights_code() && !css.is_empty();
    if highlight {
        write_raw(destination, highlight::STYLESHEET_PATH, css)?;
    }
    Ok(highlight)
}

impl GenericContent {
    pub(crate) fn new(
        source: &str,
        destination: &crate::files::Output,
        main_config: &MainConfig,
        menu_config: &MenuConfig,
    ) -> Result<GenericContent, crate::generator::GenerateError> {
        let background_class = main_config.get_background_class();
        let css_links = main_config.get_css_links();
        let js_links = main_config.get_js_links(destination)?;
        let footer = get_footer(source, main_config);
        let language = main_config.get_language().to_string();
        let ui = ui_text(&language);
        let description = main_config.get_description().map(String::from);
        let favicon = main_config
            .favicon
            .as_deref()
            .map(|f| publish_favicon(source, destination, f))
            .transpose()?
            .flatten();
        Ok(GenericContent {
            markdown: main_config.markdown == Some(true),
            background_class,
            data_theme: main_config.color_scheme.map(ColorScheme::to_data_theme),
            stretch_side_notification: main_config.stretch_side_notification.unwrap_or(false),
            css_links,
            js_links,
            footer,
            language,
            ui,
            description,
            favicon,
            site_title: main_config.site_title.clone(),
            title_separator: main_config.get_title_separator().to_string(),
            site_url: main_config.get_site_url().map(String::from),
            default_image: main_config.default_image.clone(),
            author_name: main_config.get_author_name().to_string(),
            author_url: main_config.author.as_ref().and_then(|a| a.url.clone()),
            social_urls: menu_config.socials.iter().map(|s| s.url.clone()).collect(),
            fediverse_creator: fediverse_creator(main_config, menu_config),
            options: MarkdownOptions {
                highlight: publish_highlight_stylesheet(destination, main_config)?,
                mermaid: main_config.renders_mermaid(),
            },
            mermaid_script: main_config.get_mermaid_script().to_string(),
            feeds_published: main_config.get_site_url().is_some(),
            spare_tile: main_config
                .spare_tile
                .clone()
                .filter(|s| !s.links.is_empty()),
            search: main_config.get_search().is_some(),
        })
    }
}

/// main.json's `fediverseCreator`, or else the handle of the first social
/// link with the Mastodon icon. See vados.allium's
/// `SiteConfig.effective_fediverse_creator`.
fn fediverse_creator(main_config: &MainConfig, menu_config: &MenuConfig) -> Option<String> {
    let given = main_config
        .fediverse_creator
        .as_deref()
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .map(|h| format!("@{}", h.trim_start_matches('@')));
    given.or_else(|| {
        menu_config
            .socials
            .iter()
            .filter(|s| s.icon.as_deref() == Some(MASTODON_ICON))
            .find_map(|s| fediverse_handle_of(&s.url))
    })
}

pub(crate) struct ContentHelper<'a> {
    path: &'a str,
    item: Arc<Item>,
    side_menu: Option<String>,
    structure: &'a Structure,
}

impl ContentHelper<'_> {
    pub(crate) fn new<'a>(path: &'a str, structure: &'a Structure) -> ContentHelper<'a> {
        let item = structure.get_item(path);
        let side_menu = get_side_menu(path, structure);
        ContentHelper {
            path,
            item,
            side_menu,
            structure,
        }
    }
    pub(crate) fn get_navigation(
        &self,
        main_config: &MainConfig,
        menu_config: &MenuConfig,
    ) -> Result<String, String> {
        get_navigation(
            self.path,
            main_config,
            menu_config,
            self.structure,
            self.side_menu.as_ref(),
        )
    }
    pub(crate) fn get_main_content(
        &self,
        source: &str,
        generic_content: &GenericContent,
    ) -> String {
        get_main_content(
            source,
            self.path,
            &self.item,
            self.structure,
            generic_content,
            generic_content.options,
        )
    }
    pub(crate) fn get_page(
        &self,
        navigation: &str,
        main_content: &str,
        generic_content: &GenericContent,
    ) -> String {
        let breadcrumbs = get_breadcrumbs(self.path, self.structure, generic_content.ui);
        let page_helper = PageHelper {
            navigation,
            breadcrumbs,
            side_menu: self.side_menu.as_ref(),
            main_content,
        };
        get_page(
            self.path,
            &self.item,
            page_helper,
            self.structure,
            generic_content,
        )
    }

    pub(crate) fn get_markdown(&self, source: &str, generic: &GenericContent) -> String {
        let mut main = get_main_content(
            source,
            self.path,
            &self.item,
            self.structure,
            generic,
            MarkdownOptions::default(),
        );
        // Keep the authored Markdown intact rather than round-tripping it
        // through HTML. The surrounding generated content uses the same
        // rendering and selection rules as the HTML page.
        let authored =
            if classify_content_reference(&self.item.content) == ContentKind::MarkdownFile {
                fs::read_to_string(get_file_path(source, self.path, &self.item.content)).ok()
            } else {
                None
            };
        let mut marker = String::from("VADOSMARKDOWNBODYPLACEHOLDER");
        while main.contains(&marker) {
            marker.push('X');
        }
        if authored.is_some() {
            let rendered = get_content(
                source,
                self.path,
                &self.item.content,
                MarkdownOptions::default(),
            );
            if !rendered.is_empty() {
                main = main.replacen(&rendered, &marker, 1);
            }
        }
        let page = self.get_page("", &main, generic);
        crate::markdown::render(
            &page,
            self.path,
            generic.site_url.as_deref(),
            authored.as_deref().map(|text| (marker.as_str(), text)),
            self.item.updated.map(|d| d.to_string()).as_deref(),
        )
    }
}

struct PageHelper<'a> {
    navigation: &'a str,
    breadcrumbs: Option<String>,
    side_menu: Option<&'a String>,
    main_content: &'a str,
}

fn get_footer(source: &str, main_config: &MainConfig) -> String {
    let footer_content = get_content(
        source,
        "/",
        &main_config.footer_content,
        MarkdownOptions::for_site(main_config),
    );
    FooterTemplate {
        content: &footer_content,
    }
    .render()
    .unwrap()
}

pub(crate) fn to_internal_image(
    processed_image: Arc<ProcessedImage>,
    image_type: ImageType,
) -> String {
    InternalImageTemplate {
        ratio: processed_image.ratio.to_css_class(),
        title: &processed_image.title,
        image_type,
        src: &processed_image.src,
        srcset: &processed_image.srcset,
        width: processed_image.src_width,
        height: processed_image.src_height,
        alt: &processed_image.alt,
    }
    .render()
    .unwrap()
}
