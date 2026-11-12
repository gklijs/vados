use crate::authoring::social::{fediverse_handle_of, MASTODON_ICON};
use crate::bulma::{spanning_rows, Color, ImageType, TilePlacement, MAX_ROW_SPAN};
use crate::config_files::{
    favicon_media_type, ColorScheme, MainConfig, MenuConfig, Notification, PageConfig, SpareTile,
};
use crate::event::{agenda_calendar_href, event_calendar_href};
use crate::feed::{feed_href, feed_title, page_url};
use crate::files::write_raw;
use crate::highlight::{self, highlight_code_blocks, HIGHLIGHTED_MARKER};
use crate::image::ProcessedImage;
use crate::luma::BUTTON_SCRIPT_URL;
use crate::search::{SEARCH_INDEX_HREF, SEARCH_PATH, SEARCH_SCRIPT_HREF};
use crate::structure::{Item, SocialItem, Structure};
use crate::structured_data::{structured_data, PageFacts, SiteFacts};
use crate::templates::{
    AgendaEntry, AgendaTemplate, BreadcrumbsTemplate, ContentNotificationTemplate, ContentTemplate,
    DisplayDate, EventDetailsTemplate, ExternalNotificationTemplate, FeedLink, FooterTemplate,
    InternalImageTemplate, InternalNotificationTemplate, LinkPreview, LinkPreviewImage,
    LumaEventView, LumaEventsTemplate, NavigationTemplate, PageTemplate, SearchTemplate,
    SideMenuTemplate, SpareTileTemplate,
};
use crate::ui_text::{ui_text, UiText};
use askama::Template;
use pulldown_cmark::{html, Event, HeadingLevel, Parser, Tag};
use std::fs;
use std::path::Path;
use std::sync::Arc;

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

pub(crate) struct ContentItems {
    pub(crate) item: Item,
    pub(crate) left_sub_notifications: Option<Vec<String>>,
    pub(crate) right_sub_notifications: Option<Vec<String>>,
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
            println!("favicon {} could not be read, skipping it: {}", favicon, e);
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

pub(crate) fn to_content_items(
    source: &str,
    dir_path: String,
    page_config: PageConfig,
    structure: &Structure,
    options: MarkdownOptions,
) -> ContentItems {
    let path = match dir_path {
        root if root == source => String::from("/"),
        mut d => d.split_off(source.len()),
    };
    let left_sub_notifications = page_config
        .left_notifications
        .as_ref()
        .map(|n| resolve_notifications(source, &path, n, TilePlacement::Left, structure, options));
    let right_sub_notifications = page_config
        .right_notifications
        .as_ref()
        .map(|n| resolve_notifications(source, &path, n, TilePlacement::Right, structure, options));
    let item = Item::new(path, page_config);
    ContentItems {
        item,
        left_sub_notifications,
        right_sub_notifications,
    }
}

fn resolve_notifications(
    source: &str,
    path: &str,
    notifications: &[Notification],
    placement: TilePlacement,
    structure: &Structure,
    options: MarkdownOptions,
) -> Vec<String> {
    notifications
        .iter()
        .enumerate()
        .map(|(i, notification)| {
            resolve_notification(
                source,
                path,
                notification.clone(),
                (placement, i),
                structure,
                options,
            )
        })
        .collect()
}

/// Renders the `index`th notification in the left or right column.
fn resolve_notification(
    source: &str,
    path: &str,
    notification: Notification,
    (placement, index): (TilePlacement, usize),
    structure: &Structure,
    options: MarkdownOptions,
) -> String {
    let id = match placement {
        TilePlacement::Left => format!("sub-l-{}", index),
        TilePlacement::Right => format!("sub-r-{}", index),
        TilePlacement::Side => {
            unreachable!("side notifications are rendered by `items_to_side_notifications`")
        }
    };
    // The first row of child tiles is above the fold.
    let eager = index == 0;
    let content = get_content(source, path, &notification.content, options);
    let image = notification
        .image
        .and_then(|i| structure.process_image(&i, ImageType::Sub { eager }));
    match notification.url {
        None => {
            let color = notification.color.unwrap_or(Color::Info).to_css_class();
            ContentNotificationTemplate {
                title: &notification.title,
                color,
                image,
                content,
                id,
                placement,
            }
            .render()
            .unwrap()
        }
        Some(internal) if internal.starts_with('/') => {
            let color = notification.color.unwrap_or(Color::Link).to_css_class();
            InternalNotificationTemplate {
                title: &notification.title,
                sub_title: &None,
                date: None,
                color,
                url: &internal,
                image,
                content: Some(content),
                id,
                placement,
            }
            .render()
            .unwrap()
        }
        Some(external) => {
            let color = notification.color.unwrap_or(Color::Link).to_css_class();
            ExternalNotificationTemplate {
                title: &notification.title,
                color,
                url: &external,
                image,
                content,
                id,
                placement,
            }
            .render()
            .unwrap()
        }
    }
}

pub(crate) fn items_to_side_notifications(
    items: Vec<Arc<Item>>,
    structure: &Structure,
    ui: &UiText,
) -> Vec<String> {
    let mut result = vec![];
    for (i, item) in items.iter().enumerate() {
        let id = format!("sub-s-{}", i);
        let color = Color::Info.to_css_class();
        let image = item
            .image
            .as_ref()
            .and_then(|i| structure.process_image(i, ImageType::Side));
        let notification = InternalNotificationTemplate {
            title: &Some(item.title.clone()),
            sub_title: &item.sub_title,
            date: item.date.map(|d| DisplayDate::new(&d, ui)),
            color,
            url: &item.path,
            image,
            content: None,
            id,
            placement: TilePlacement::Side,
        }
        .render()
        .unwrap();
        result.push(notification)
    }
    result
}

/// How a content reference (a page's `content`, a notification's `content`,
/// or the site's `footerContent`) resolves. Shared between rendering, which
/// panics on `Unrecognised`, and `check`, which reports it as a finding
/// instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContentKind {
    MarkdownFile,
    HtmlFile,
    InlineHtml,
    Unrecognised,
}

pub(crate) fn classify_content_reference(reference: &str) -> ContentKind {
    if reference.ends_with(".md") {
        ContentKind::MarkdownFile
    } else if reference.ends_with(".html") {
        ContentKind::HtmlFile
    } else if reference.ends_with('>') {
        ContentKind::InlineHtml
    } else {
        ContentKind::Unrecognised
    }
}

/// Escapes the characters that would otherwise be read as markup when
/// dropped straight into an HTML attribute or element.
pub(crate) fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Coerces free-form text into something that will actually classify as a
/// `ContentReference` rather than `Unrecognised`. Used wherever a
/// content-authoring command writes plain descriptive text -- a notification
/// caption, or an image's own alt text standing in for one -- into a field
/// that `check`/`generate` will read back as a content reference. Text that
/// already looks like one (a `.md`/`.html` file, or inline HTML ending in a
/// closing tag) passes through unchanged, so a maintainer who deliberately
/// names a file still gets that file, not a paragraph containing its name.
pub(crate) fn ensure_content_reference(text: String) -> String {
    match classify_content_reference(&text) {
        ContentKind::Unrecognised => format!("<p>{}</p>", escape_html(&text)),
        ContentKind::MarkdownFile | ContentKind::HtmlFile | ContentKind::InlineHtml => text,
    }
}

pub(crate) fn get_file_path(source: &str, path: &str, reference: &str) -> String {
    if path == "/" {
        format!("{}/{}", source, reference)
    } else {
        format!("{}{}/{}", source, path, reference)
    }
}

fn md_to_content(file_path: &str, options: MarkdownOptions) -> String {
    match fs::read_to_string(file_path) {
        Ok(text) => {
            let mut html_output: String = String::with_capacity(text.len() * 3 / 2);
            let parser = crate::mermaid::diagram_blocks(Parser::new(&text), options.mermaid);
            if options.highlight {
                html::push_html(&mut html_output, highlight_code_blocks(parser));
            } else {
                html::push_html(&mut html_output, parser);
            }
            html_output
        }
        Err(e) => {
            println!(
                "There was an error reading Markdown content from path {}.\n{}",
                file_path, e
            );
            String::from("")
        }
    }
}

/// Whether a page's content has a top-level heading of its own: a `#`
/// heading in Markdown, an `<h1>` in HTML. The page title is already the
/// page's `<h1>`, so this would make a second. A file that can't be read has
/// none; `check` reports it missing separately.
pub(crate) fn has_top_level_heading(source: &str, path: &str, reference: &str) -> bool {
    let file_path = get_file_path(source, path, reference);
    match classify_content_reference(reference) {
        ContentKind::MarkdownFile => fs::read_to_string(file_path).is_ok_and(|text| {
            Parser::new(&text).any(|e| {
                matches!(
                    e,
                    Event::Start(Tag::Heading {
                        level: HeadingLevel::H1,
                        ..
                    })
                )
            })
        }),
        ContentKind::HtmlFile => fs::read_to_string(file_path).is_ok_and(|text| html_has_h1(&text)),
        ContentKind::InlineHtml => html_has_h1(reference),
        ContentKind::Unrecognised => false,
    }
}

fn html_has_h1(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("<h1>") || lower.contains("<h1 ")
}

fn html_to_content(file_path: &str) -> String {
    match fs::read_to_string(file_path) {
        Ok(text) => text,
        Err(e) => {
            println!(
                "There was an error reading Html content from path {}.\n{}",
                file_path, e
            );
            String::from("")
        }
    }
}

/// The content a reference names, as HTML, with the requested Markdown transforms.
pub(crate) fn get_content(
    source: &str,
    path: &str,
    reference: &str,
    options: MarkdownOptions,
) -> String {
    let file_path = get_file_path(source, path, reference);
    match classify_content_reference(reference) {
        ContentKind::MarkdownFile => md_to_content(&file_path, options),
        ContentKind::HtmlFile => html_to_content(&file_path),
        ContentKind::InlineHtml => String::from(reference),
        // Generation rejects this during preflight. Keep internal rendering
        // callers safe as well, displaying arbitrary text without markup.
        ContentKind::Unrecognised => format!("<p>{}</p>", escape_html(reference)),
    }
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

fn get_side_menu(path: &str, structure: &Structure) -> Option<String> {
    match structure.get_side_menu_items(path) {
        None => None,
        Some(menu_item) => {
            let s = SideMenuTemplate {
                path,
                menu_item: &menu_item,
            };
            Some(s.render().unwrap())
        }
    }
}

fn get_navigation(
    path: &str,
    main_config: &MainConfig,
    menu_config: &MenuConfig,
    structure: &Structure,
    side_menu: Option<&String>,
) -> Result<String, String> {
    Ok(NavigationTemplate {
        path,
        ui: ui_text(main_config.get_language()),
        site_title: &main_config.site_title,
        color: main_config.get_navbar_color(),
        main_menu: &structure.get_main_menu_items(
            menu_config,
            main_config.get_menu_dropdown_limit(),
            main_config.get_search().is_some_and(|s| s.shows_in_menu()),
        )?,
        socials: &menu_config
            .socials
            .iter()
            .map(SocialItem::new)
            .collect::<Result<Vec<_>, _>>()?,
        side_menu,
    }
    .render()
    .unwrap())
}

fn get_breadcrumbs(path: &str, structure: &Structure, ui: &UiText) -> Option<String> {
    structure.get_breadcrumbs(path).map(|crumbs| {
        BreadcrumbsTemplate {
            ui,
            crumbs,
            last: structure.get_menu_item(path),
        }
        .render()
        .unwrap()
    })
}

fn get_main_content(
    source: &str,
    path: &str,
    item: &Item,
    structure: &Structure,
    generic_content: &GenericContent,
    options: MarkdownOptions,
) -> String {
    let ui = generic_content.ui;
    let image = match &item.image {
        None => None,
        Some(i) => structure.process_image(i, ImageType::Main),
    };
    ContentTemplate {
        title: &item.title,
        sub_title: &item.sub_title,
        date: item.date.map(|d| DisplayDate::new(&d, ui)),
        image,
        content: get_content(source, path, &item.content, options),
        feed: item
            .feed
            .as_ref()
            .filter(|_| generic_content.feeds_published)
            .map(|_| feed_link(item, generic_content)),
        event_details: get_event_details(item, generic_content),
        ui,
    }
    .render()
    .unwrap()
}

fn get_event_details(item: &Item, generic_content: &GenericContent) -> Option<String> {
    let event = item.event.as_ref()?;
    let ui = generic_content.ui;
    let details = EventDetailsTemplate {
        ui,
        status: event.status_text(ui),
        start: event.config.start.trim(),
        when: event.written_time(ui),
        rows: event.detail_rows(ui),
        registration_url: event.registration_url(),
        virtual_url: event.virtual_url(),
        calendar_href: generic_content
            .site_url
            .as_ref()
            .map(|_| event_calendar_href(&item.path)),
    };
    Some(details.render().unwrap())
}

/// What vados adds to a page's main area itself: its agenda, and the search
/// at the search page. The scripts that go with them are for this page only.
fn get_page_extra(
    path: &str,
    item: &Item,
    structure: &Structure,
    generic_content: &GenericContent,
) -> (Option<String>, Vec<&'static str>) {
    let ui = generic_content.ui;
    let mut extra = String::new();
    let mut scripts = vec![];
    if let Some(agenda) = &item.agenda {
        let events = structure
            .get_agenda_events(item, agenda)
            .iter()
            .map(|listed| AgendaEntry {
                url: listed
                    .luma_url
                    .clone()
                    .unwrap_or_else(|| listed.page.path.clone()),
                title: listed.title.clone(),
                start: listed.event.config.start.trim().to_string(),
                ends: listed.event.last_moment().to_string(),
                when: listed.event.written_time(ui),
                status: listed.event.status_text(ui),
                summary: listed.summary(),
            })
            .collect();
        let template = AgendaTemplate {
            ui,
            events,
            calendar_href: generic_content
                .site_url
                .as_ref()
                .map(|_| agenda_calendar_href(path)),
        };
        extra.push_str(&template.render().unwrap());
        scripts.push(AGENDA_SCRIPT_HREF);
    }
    if generic_content.search && path == SEARCH_PATH {
        let template = SearchTemplate {
            ui,
            path,
            index_href: SEARCH_INDEX_HREF,
        };
        extra.push_str(&template.render().unwrap());
        scripts.push(SEARCH_SCRIPT_HREF);
    }
    (Some(extra).filter(|e| !e.is_empty()), scripts)
}

fn feed_link(item: &Item, generic_content: &GenericContent) -> FeedLink {
    FeedLink {
        title: feed_title(item, &generic_content.site_title),
        href: feed_href(&item.path),
    }
}

/// The feeds a page sits inside: its own, and every ancestor's, nearest
/// first. See vados.allium's `FeedsAreDiscoverable`.
fn get_feed_links(
    path: &str,
    structure: &Structure,
    generic_content: &GenericContent,
) -> Vec<FeedLink> {
    if !generic_content.feeds_published {
        return vec![];
    }
    structure
        .get_self_and_ancestors(path)
        .iter()
        .filter(|i| i.feed.is_some())
        .map(|i| feed_link(i, generic_content))
        .collect()
}

fn get_page(
    path: &str,
    item: &Item,
    page_helper: PageHelper,
    structure: &Structure,
    generic_content: &GenericContent,
) -> String {
    // A page's own summary describes it best; the site description stands in
    // for every page without one. See vados.allium's `PagesAreDescribed`.
    let description = item
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or(generic_content.description.as_deref());
    let preview = link_preview(path, item, structure, generic_content);
    // The newest pages from elsewhere come first, alternating between the
    // columns, then the page's own notifications.
    let (mut left, mut right) = latest_tiles(path, item, structure, generic_content.ui);
    left.extend(structure.get_left_sub_notifications(path).iter().cloned());
    right.extend(structure.get_right_sub_notifications(path).iter().cloned());
    let child_tiles = in_reading_order(left, right, generic_content.spare_tile.as_ref());
    let (page_extra, page_scripts) = get_page_extra(path, item, structure, generic_content);
    // Only a page that shows highlighted code loads its stylesheet.
    let mut css_links = generic_content.css_links.clone();
    let shows_highlighted_code = std::iter::once(page_helper.main_content)
        .chain(child_tiles.iter().map(String::as_str))
        .chain(std::iter::once(generic_content.footer.as_str()))
        .any(|html| html.contains(HIGHLIGHTED_MARKER));
    if shows_highlighted_code {
        css_links.push(String::from(highlight::STYLESHEET_PATH));
    }
    let shows_mermaid = generic_content.options.mermaid
        && std::iter::once(page_helper.main_content)
            .chain(child_tiles.iter().map(String::as_str))
            .chain(std::iter::once(generic_content.footer.as_str()))
            .any(|html| html.contains(crate::mermaid::MARKER));
    let structured_data = structured_data(
        item,
        &PageFacts {
            url: preview.url.as_deref(),
            description: item
                .summary
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
            image_url: preview.image.as_ref().map(|i| i.url.as_str()),
        },
        &SiteFacts {
            site_title: &generic_content.site_title,
            description: generic_content.description.as_deref(),
            site_url: generic_content.site_url.as_deref(),
            author_name: &generic_content.author_name,
            author_url: generic_content.author_url.as_deref(),
            same_as: generic_content
                .social_urls
                .iter()
                .map(String::as_str)
                .collect(),
        },
    );
    PageTemplate {
        mermaid_script: shows_mermaid.then_some(generic_content.mermaid_script.as_str()),
        markdown_href: generic_content
            .markdown
            .then(|| crate::markdown::href(path)),
        title: &item.title,
        document_title: document_title(
            path,
            &item.title,
            &generic_content.site_title,
            &generic_content.title_separator,
        ),
        site_title: &generic_content.site_title,
        fediverse_creator: generic_content.fediverse_creator.as_deref(),
        preview,
        structured_data,
        description,
        language: &generic_content.language,
        ui: generic_content.ui,
        favicon_href: generic_content.favicon.as_ref().map(|f| f.href.as_str()),
        favicon_type: generic_content.favicon.as_ref().and_then(|f| f.media_type),
        feeds: get_feed_links(path, structure, generic_content),
        background_class: &generic_content.background_class,
        data_theme: generic_content.data_theme,
        stretch_side_notification: generic_content.stretch_side_notification,
        navigation: page_helper.navigation,
        breadcrumbs: page_helper.breadcrumbs,
        side_menu: page_helper.side_menu,
        main_content: page_helper.main_content,
        luma_events: get_luma_events(item, generic_content.ui),
        child_tiles: &child_tiles,
        side_notifications: &structure.get_side_notifications(path, generic_content.ui),
        footer: &generic_content.footer,
        css_links: &css_links,
        js_links: &generic_content.js_links,
        // Luma's script is loaded once, and only by a page that shows a
        // register button; see vados.allium's `ThirdPartyCodeOnlyWhereUsed`.
        luma_button_script: item
            .luma_events
            .iter()
            .any(|e| e.display().shows_register_button())
            .then_some(BUTTON_SCRIPT_URL),
        robots: if item.noindex {
            "noindex, follow"
        } else {
            "max-image-preview:large"
        },
        page_extra,
        page_scripts,
    }
    .render()
    .unwrap()
}

/// The child tiles row by row, the left one of each pair first: the order
/// the grid places them in and a phone stacks them in. Each tile carries the
/// column it belongs in, so once one column runs out the other's tiles stay
/// put. The rows the shorter column is missing hold the site's spare tile when
/// it has one. Without one its last tile stretches down alongside the others,
/// unless it has an image: then the stretch would be a large empty block, so
/// the cells stay empty. See vados.allium's `ChildTilesPairUp`.
fn in_reading_order(
    mut left: Vec<String>,
    mut right: Vec<String>,
    spare_tile: Option<&SpareTile>,
) -> Vec<String> {
    let rows = left.len().max(right.len());
    let (shorter, placement) = if left.len() < right.len() {
        (&mut left, TilePlacement::Left)
    } else {
        (&mut right, TilePlacement::Right)
    };
    let missing = rows - shorter.len();
    if missing > 0 {
        match spare_tile {
            Some(spare_tile) => shorter.push(render_spare_tile(spare_tile, placement, missing)),
            None => {
                if let Some(last) = shorter.last_mut().filter(|t| !t.contains("<img")) {
                    *last = spanning_rows(last, missing + 1);
                }
            }
        }
    }
    let mut tiles = Vec::with_capacity(left.len() + right.len());
    let (mut left, mut right) = (left.into_iter(), right.into_iter());
    loop {
        match (left.next(), right.next()) {
            (None, None) => return tiles,
            (l, r) => tiles.extend(l.into_iter().chain(r)),
        }
    }
}

/// The spare tile, spanning the `rows` cells it fills. Only on a tablet and
/// wider: on a phone tiles stack, so no cell is ever left over.
fn render_spare_tile(spare_tile: &SpareTile, placement: TilePlacement, rows: usize) -> String {
    SpareTileTemplate {
        title: spare_tile.title.as_deref(),
        links: &spare_tile.links,
        start_class: placement.to_column_class(),
        rows: rows.min(MAX_ROW_SPAN),
    }
    .render()
    .unwrap()
}

/// The page's `latest` pages as child tiles, split over the left and right
/// columns the way they pair up: the newest left, the next right, and so on.
/// See vados.allium's `LatestPagesComeFirst`.
fn latest_tiles(
    path: &str,
    item: &Item,
    structure: &Structure,
    ui: &UiText,
) -> (Vec<String>, Vec<String>) {
    let Some(latest) = &item.latest else {
        return (vec![], vec![]);
    };
    let pages = structure.get_latest(path, &latest.sources(), latest.depth, latest.get_limit());
    let (mut left, mut right) = (vec![], vec![]);
    for (i, page) in pages.iter().enumerate() {
        let image = page
            .image
            .as_ref()
            .and_then(|r| structure.process_image(r, ImageType::Sub { eager: i < 2 }));
        let content = page
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("<p>{}</p>", escape_html(s)));
        let tile = InternalNotificationTemplate {
            title: &Some(page.title.clone()),
            sub_title: &page.sub_title,
            date: page.date.map(|d| DisplayDate::new(&d, ui)),
            color: Color::Link.to_css_class(),
            url: &page.path,
            image,
            content,
            id: format!("latest-{}", i),
            placement: if i % 2 == 0 {
                TilePlacement::Left
            } else {
                TilePlacement::Right
            },
        }
        .render()
        .unwrap();
        if i % 2 == 0 {
            left.push(tile)
        } else {
            right.push(tile)
        }
    }
    (left, right)
}

/// See vados.allium's `PagesPreviewWell`.
fn link_preview(
    path: &str,
    item: &Item,
    structure: &Structure,
    generic_content: &GenericContent,
) -> LinkPreview {
    let site_url = generic_content.site_url.as_deref();
    let image = site_url.and_then(|site_url| {
        let processed = item
            .image
            .iter()
            .chain(&generic_content.default_image)
            .filter_map(|r| structure.get_image(r))
            .find(|i| i.preview.is_some())?;
        let preview = processed.preview.as_ref()?;
        Some(LinkPreviewImage {
            url: format!("{site_url}{}", preview.path),
            width: preview.width,
            height: preview.height,
            alt: processed.alt.clone(),
        })
    });
    LinkPreview {
        og_type: if item.date.is_some() {
            "article"
        } else {
            "website"
        },
        locale: generic_content.language.replace('-', "_"),
        published_time: item.date.map(|d| d.to_rfc3339()),
        url: site_url.map(|u| page_url(u, path)),
        image,
    }
}

/// A page's `<title>`: its own title followed by the site's, so browser tabs
/// and search results say whose site it is. The home page, and any page
/// already titled after the site, just carry the site title. See
/// vados.allium's `Page.document_title`.
fn document_title(path: &str, title: &str, site_title: &str, separator: &str) -> String {
    if path == "/" || title.trim() == site_title.trim() {
        String::from(site_title)
    } else {
        format!("{title}{separator}{site_title}")
    }
}

fn get_luma_events(item: &Item, ui: &UiText) -> Option<String> {
    if item.luma_events.is_empty() {
        return None;
    }
    let events = item
        .luma_events
        .iter()
        .map(|e| LumaEventView {
            event_id: &e.event_id,
            // Only publishable events reach an `Item`, and those have a title.
            title: e.effective_title().unwrap_or_default(),
            event_url: e.event_url(),
            embed_url: e.embed_url(),
            shows_event_page: e.display().shows_event_page(),
            shows_register_button: e.display().shows_register_button(),
            button_label: e.button_label(ui.luma_register),
        })
        .collect();
    Some(LumaEventsTemplate { events }.render().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(name: &str, placement: TilePlacement) -> String {
        format!(
            r#"<a class="notification is-link{}" id="{name}">"#,
            placement.to_css_classes()
        )
    }

    #[test]
    fn child_tiles_read_row_by_row_left_first() {
        let left = vec![
            tile("l0", TilePlacement::Left),
            tile("l1", TilePlacement::Left),
        ];
        let right = vec![
            tile("r0", TilePlacement::Right),
            tile("r1", TilePlacement::Right),
        ];
        let ids: Vec<String> = in_reading_order(left, right, None)
            .iter()
            .map(|t| t.rsplit("id=").next().unwrap().to_string())
            .collect();
        assert_eq!(ids, [r#""l0">"#, r#""r0">"#, r#""l1">"#, r#""r1">"#]);
    }

    #[test]
    fn the_shorter_columns_last_tile_spans_the_rows_left_over() {
        let left = (0..4)
            .map(|i| tile(&format!("l{i}"), TilePlacement::Left))
            .collect();
        let right = vec![
            tile("r0", TilePlacement::Right),
            tile("r1", TilePlacement::Right),
        ];
        let tiles = in_reading_order(left, right, None);
        let r1 = tiles.iter().find(|t| t.contains(r#"id="r1""#)).unwrap();
        assert!(
            r1.contains("is-col-span-2-mobile is-row-span-3-tablet\""),
            "{r1}"
        );
        let spanning = tiles.iter().filter(|t| t.contains("is-row-span")).count();
        assert_eq!(spanning, 1);
    }

    #[test]
    fn even_or_one_sided_columns_span_nothing() {
        let even = in_reading_order(
            vec![tile("l0", TilePlacement::Left)],
            vec![tile("r0", TilePlacement::Right)],
            None,
        );
        let one_sided = in_reading_order(
            vec![
                tile("l0", TilePlacement::Left),
                tile("l1", TilePlacement::Left),
            ],
            vec![],
            None,
        );
        assert!(even
            .iter()
            .chain(&one_sided)
            .all(|t| !t.contains("is-row-span")));
    }

    #[test]
    fn a_span_never_exceeds_what_bulma_offers() {
        let left = (0..20)
            .map(|i| tile(&format!("l{i}"), TilePlacement::Left))
            .collect();
        let tiles = in_reading_order(left, vec![tile("r0", TilePlacement::Right)], None);
        assert!(tiles.iter().any(|t| t.contains("is-row-span-12-tablet")));
    }

    fn with_image(name: &str, placement: TilePlacement) -> String {
        format!(r#"{}<img src="/x.webp">"#, tile(name, placement))
    }

    #[test]
    fn a_tile_with_an_image_is_not_stretched_into_an_empty_block() {
        let left = vec![
            tile("l0", TilePlacement::Left),
            tile("l1", TilePlacement::Left),
        ];
        let right = vec![with_image("r0", TilePlacement::Right)];
        let tiles = in_reading_order(left, right, None);
        assert!(tiles.iter().all(|t| !t.contains("is-row-span")));
    }

    fn spare() -> SpareTile {
        serde_json::from_str(
            r#"{"title":"More","links":[{"url":"/archive","title":"Archive & <more>"}]}"#,
        )
        .unwrap()
    }

    #[test]
    fn a_spare_tile_fills_the_missing_cells_and_nothing_stretches() {
        let left = (0..4)
            .map(|i| with_image(&format!("l{i}"), TilePlacement::Left))
            .collect();
        let right = vec![
            tile("r0", TilePlacement::Right),
            tile("r1", TilePlacement::Right),
        ];
        let tiles = in_reading_order(left, right, Some(&spare()));
        assert_eq!(tiles.len(), 7);
        assert!(!tiles[..5].iter().any(|t| t.contains("is-row-span")));
        // Placed right after the shorter column's last tile, on its row.
        let filler = &tiles[5];
        assert!(filler.contains(r#"id="spare-tile""#), "{filler}");
        assert!(filler.contains("is-col-start-2-tablet is-row-span-2-tablet is-hidden-mobile"));
        assert!(
            filler.contains(r#"<a href="/archive">Archive &#38; &#60;more&#62;</a>"#),
            "{filler}"
        );
    }

    #[test]
    fn a_spare_tile_is_left_out_when_the_columns_are_even() {
        let tiles = in_reading_order(
            vec![tile("l0", TilePlacement::Left)],
            vec![tile("r0", TilePlacement::Right)],
            Some(&spare()),
        );
        assert_eq!(tiles.len(), 2);
    }

    #[test]
    fn a_spare_tile_goes_in_the_left_column_when_that_is_shorter() {
        let tiles = in_reading_order(
            vec![],
            vec![tile("r0", TilePlacement::Right)],
            Some(&spare()),
        );
        assert!(tiles[0].contains("is-col-start-1-tablet"), "{}", tiles[0]);
    }

    #[test]
    fn a_right_notification_reads_after_the_latest_tile_on_its_row() {
        // Three latest pages leave the right column one row short: the page's
        // own right notification pairs with the third latest page, so it
        // reads before the left column's notification on a phone. The tile
        // keeps its column; see vados.allium's Notification and
        // ChildTilesPairUp.
        let left = vec![
            tile("latest-0", TilePlacement::Left),
            tile("latest-2", TilePlacement::Left),
            tile("sub-l-0", TilePlacement::Left),
        ];
        let right = vec![
            tile("latest-1", TilePlacement::Right),
            tile("sub-r-0", TilePlacement::Right),
        ];
        let ids: Vec<String> = in_reading_order(left, right, None)
            .iter()
            .map(|t| t.rsplit("id=").next().unwrap().to_string())
            .collect();
        assert_eq!(
            ids,
            [
                r#""latest-0">"#,
                r#""latest-1">"#,
                r#""latest-2">"#,
                r#""sub-r-0">"#,
                r#""sub-l-0">"#
            ]
        );
    }

    #[test]
    fn a_span_is_inserted_before_any_later_classes() {
        let tile = r#"<a class="notification is-link is-col-span-2-mobile extra" id="x">"#;
        let spanned = spanning_rows(tile, 3);
        assert!(
            spanned.contains("is-col-span-2-mobile is-row-span-3-tablet extra"),
            "{spanned}"
        );
    }

    #[test]
    fn document_title_follows_the_page_title_with_the_site_title() {
        assert_eq!(
            document_title("/blog/post", "A post", "My site", " · "),
            "A post · My site"
        );
        assert_eq!(
            document_title("/blog/post", "A post", "My site", " | "),
            "A post | My site"
        );
    }

    #[test]
    fn document_title_is_just_the_site_title_on_the_home_page_or_when_they_match() {
        assert_eq!(document_title("/", "Home", "My site", " · "), "My site");
        assert_eq!(
            document_title("/about", "My site", "My site", " · "),
            "My site"
        );
    }

    #[test]
    fn ensure_content_reference_leaves_a_markdown_reference_untouched() {
        assert_eq!(ensure_content_reference("notes.md".to_string()), "notes.md");
    }

    #[test]
    fn ensure_content_reference_leaves_an_html_reference_untouched() {
        assert_eq!(
            ensure_content_reference("notes.html".to_string()),
            "notes.html"
        );
    }

    #[test]
    fn ensure_content_reference_leaves_inline_html_untouched() {
        assert_eq!(
            ensure_content_reference("<p>already html</p>".to_string()),
            "<p>already html</p>"
        );
    }

    #[test]
    fn ensure_content_reference_wraps_plain_text_as_inline_html() {
        assert_eq!(
            ensure_content_reference("Alice smiling".to_string()),
            "<p>Alice smiling</p>"
        );
    }

    #[test]
    fn ensure_content_reference_escapes_markup_characters_in_plain_text() {
        // Deliberately doesn't end in '>' -- text that does is already
        // (however loosely) classified as inline HTML and left untouched.
        assert_eq!(
            ensure_content_reference("Rock & roll".to_string()),
            "<p>Rock &amp; roll</p>"
        );
    }
}
