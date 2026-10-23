use crate::authoring::social::{fediverse_handle_of, MASTODON_ICON};
use crate::bulma::{Color, ImageType};
use crate::config_files::{favicon_media_type, MainConfig, MenuConfig, Notification, PageConfig};
use crate::feed::{feed_href, feed_title, page_url};
use crate::files::write_raw;
use crate::highlight::{self, highlight_code_blocks, HIGHLIGHTED_MARKER};
use crate::image::ProcessedImage;
use crate::luma::BUTTON_SCRIPT_URL;
use crate::structure::{Item, SocialItem, Structure};
use crate::structured_data::{structured_data, PageFacts, SiteFacts};
use crate::templates::{
    BreadcrumbsTemplate, ContentNotificationTemplate, ContentTemplate, DisplayDate,
    ExternalNotificationTemplate, FeedLink, FooterTemplate, InternalImageTemplate,
    InternalNotificationTemplate, LinkPreview, LinkPreviewImage, LumaEventView, LumaEventsTemplate,
    NavigationTemplate, PageTemplate, SideMenuTemplate,
};
use crate::ui_text::{ui_text, UiText};
use askama::Template;
use pulldown_cmark::{html, Event, HeadingLevel, Parser, Tag};
use std::fs;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

pub(crate) struct ContentItems {
    pub(crate) item: Item,
    pub(crate) left_sub_notifications: Option<Vec<String>>,
    pub(crate) right_sub_notifications: Option<Vec<String>>,
}

pub(crate) struct GenericContent {
    background_class: String,
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
    highlight: bool,
    /// Whether feeds are published at all: only with a usable `siteUrl`.
    feeds_published: bool,
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
fn publish_favicon(source: &str, destination: &str, favicon: &str) -> Option<Favicon> {
    let file_name = Path::new(favicon).file_name()?.to_str()?;
    let contents = match fs::read(format!("{}/{}", source, favicon)) {
        Ok(c) => c,
        Err(e) => {
            println!("favicon {} could not be read, skipping it: {}", favicon, e);
            return None;
        }
    };
    let href = format!("/{}", file_name);
    write_raw(destination, &href, contents);
    let media_type = favicon_media_type(file_name);
    Some(Favicon { href, media_type })
}

/// Writes the stylesheet for highlighted code when code is highlighted at
/// all, so pages with highlighted code can link it.
fn publish_highlight_stylesheet(destination: &str, main_config: &MainConfig) -> bool {
    let css = highlight::stylesheet();
    let highlight = main_config.highlights_code() && !css.is_empty();
    if highlight {
        write_raw(destination, highlight::STYLESHEET_PATH, css);
    }
    highlight
}

impl GenericContent {
    pub(crate) fn new(
        source: &str,
        destination: &str,
        main_config: &MainConfig,
        menu_config: &MenuConfig,
    ) -> GenericContent {
        let background_class = main_config.get_background_class();
        let css_links = main_config.get_css_links();
        let js_links = main_config.get_js_links(destination);
        let footer = get_footer(source, main_config);
        let language = main_config.get_language().to_string();
        let ui = ui_text(&language);
        let description = main_config.get_description().map(String::from);
        let favicon = main_config
            .favicon
            .as_deref()
            .and_then(|f| publish_favicon(source, destination, f));
        GenericContent {
            background_class,
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
            highlight: publish_highlight_stylesheet(destination, main_config),
            feeds_published: main_config.get_site_url().is_some(),
        }
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
    ) -> String {
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
    structure: &Structure,
    highlight: bool,
) -> ContentItems {
    let page_file_string = format!("{}/page.json", dir_path);
    let page_file_path = Path::new(&page_file_string);
    let page_config = match File::open(page_file_path) {
        Ok(f) => serde_json::from_reader(f).expect("JSON was not well-formatted"),
        Err(_) => PageConfig::new(&dir_path),
    };
    let path = match dir_path {
        root if root == source => String::from("/"),
        mut d => d.split_off(source.len()),
    };
    let left_sub_notifications = page_config.left_notifications.as_ref().map(|n| {
        resolve_notifications(
            source,
            &path,
            n,
            NotificationSide::Left,
            structure,
            highlight,
        )
    });
    let right_sub_notifications = page_config.right_notifications.as_ref().map(|n| {
        resolve_notifications(
            source,
            &path,
            n,
            NotificationSide::Right,
            structure,
            highlight,
        )
    });
    let item = Item::new(path, page_config);
    ContentItems {
        item,
        left_sub_notifications,
        right_sub_notifications,
    }
}

#[derive(Clone, Copy)]
enum NotificationSide {
    Left,
    Right,
}

fn resolve_notifications(
    source: &str,
    path: &str,
    notifications: &[Notification],
    side: NotificationSide,
    structure: &Structure,
    highlight: bool,
) -> Vec<String> {
    notifications
        .iter()
        .enumerate()
        .map(|(i, notification)| {
            let (id, order) = match side {
                NotificationSide::Left => (format!("sub-l-{}", i), 2 * i as isize),
                NotificationSide::Right => (format!("sub-r-{}", i), 2 * i as isize + 1),
            };
            resolve_notification(
                source,
                path,
                notification.clone(),
                id,
                order,
                structure,
                highlight,
            )
        })
        .collect()
}

fn resolve_notification(
    source: &str,
    path: &str,
    notification: Notification,
    id: String,
    order: isize,
    structure: &Structure,
    highlight: bool,
) -> String {
    let content = get_content(source, path, &notification.content, highlight);
    let image = notification.image.and_then(|i| {
        // The first row of child tiles is above the fold.
        structure.process_image(&i, ImageType::Sub { eager: order < 2 })
    });
    match notification.url {
        None => {
            let color = notification.color.unwrap_or(Color::Info).to_css_class();
            ContentNotificationTemplate {
                title: &notification.title,
                color,
                image,
                content,
                id,
                order: Some(order),
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
                order: Some(order),
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
                order: Some(order),
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
            order: None,
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

fn md_to_content(file_path: &str, highlight: bool) -> String {
    match fs::read_to_string(file_path) {
        Ok(text) => {
            let mut html_output: String = String::with_capacity(text.len() * 3 / 2);
            let parser = Parser::new(&text);
            if highlight {
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

/// The content a reference names, as HTML. `highlight` colours fenced
/// Markdown code blocks; see `highlight.rs`.
pub(crate) fn get_content(source: &str, path: &str, reference: &str, highlight: bool) -> String {
    let file_path = get_file_path(source, path, reference);
    match classify_content_reference(reference) {
        ContentKind::MarkdownFile => md_to_content(&file_path, highlight),
        ContentKind::HtmlFile => html_to_content(&file_path),
        ContentKind::InlineHtml => String::from(reference),
        ContentKind::Unrecognised => panic!(
            "Can't handle content reference that looks like: {}",
            reference
        ),
    }
}

fn get_footer(source: &str, main_config: &MainConfig) -> String {
    let footer_content = get_content(
        source,
        "/",
        &main_config.footer_content,
        main_config.highlights_code(),
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
) -> String {
    NavigationTemplate {
        path,
        ui: ui_text(main_config.get_language()),
        site_title: &main_config.site_title,
        color: main_config.get_navbar_color(),
        main_menu: &structure.get_main_menu_items(menu_config),
        socials: &menu_config.socials.iter().map(SocialItem::new).collect(),
        side_menu,
    }
    .render()
    .unwrap()
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
        content: get_content(source, path, &item.content, generic_content.highlight),
        feed: item
            .feed
            .as_ref()
            .filter(|_| generic_content.feeds_published)
            .map(|_| feed_link(item, generic_content)),
        ui,
    }
    .render()
    .unwrap()
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
    // Only a page that shows highlighted code loads its stylesheet.
    let mut css_links = generic_content.css_links.clone();
    let shows_highlighted_code = std::iter::once(page_helper.main_content)
        .chain(left.iter().chain(&right).map(String::as_str))
        .chain(std::iter::once(generic_content.footer.as_str()))
        .any(|html| html.contains(HIGHLIGHTED_MARKER));
    if shows_highlighted_code {
        css_links.push(String::from(highlight::STYLESHEET_PATH));
    }
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
        navigation: page_helper.navigation,
        breadcrumbs: page_helper.breadcrumbs,
        side_menu: page_helper.side_menu,
        main_content: page_helper.main_content,
        luma_events: get_luma_events(item, generic_content.ui),
        left_sub_notifications: &left,
        right_sub_notifications: &right,
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
    let count = pages.len();
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
            // Before the page's own tiles, which start at 0, once the
            // columns are stacked on a phone.
            order: Some(i as isize - count as isize),
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
