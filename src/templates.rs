use crate::bulma::ImageType;
use crate::page_date::PageDate;
use crate::structure::MenuType::Internal;
use crate::structure::{MenuItem, SocialItem};
use crate::ui_text::UiText;
use askama::Template;

#[derive(Template)]
#[template(path = "page.html")]
pub(crate) struct PageTemplate<'a> {
    /// The page's own title, as link previews show it next to the site name.
    pub(crate) title: &'a str,
    /// What browser tabs and search results show: the page title followed by
    /// the site title. See `document_title`.
    pub(crate) document_title: String,
    pub(crate) site_title: &'a str,
    pub(crate) preview: LinkPreview,
    /// The page's JSON-LD, already escaped for a `<script>` element.
    pub(crate) structured_data: Option<String>,
    pub(crate) description: Option<&'a str>,
    pub(crate) language: &'a str,
    pub(crate) ui: &'a UiText,
    pub(crate) favicon_href: Option<&'a str>,
    pub(crate) favicon_type: Option<&'a str>,
    pub(crate) feeds: Vec<FeedLink>,
    pub(crate) background_class: &'a str,
    pub(crate) navigation: &'a str,
    pub(crate) breadcrumbs: Option<String>,
    pub(crate) side_menu: Option<&'a String>,
    pub(crate) main_content: &'a str,
    pub(crate) luma_events: Option<String>,
    pub(crate) left_sub_notifications: &'a Vec<String>,
    pub(crate) right_sub_notifications: &'a Vec<String>,
    pub(crate) side_notifications: &'a Vec<String>,
    pub(crate) footer: &'a str,
    pub(crate) css_links: &'a [String],
    pub(crate) js_links: &'a [String],
    pub(crate) luma_button_script: Option<&'a str>,
}

/// What a page tells link previews (Open Graph, X cards) and search engines
/// about itself beyond its title and description. Everything given as a full
/// URL is only there when the site URL is known.
pub(crate) struct LinkPreview {
    /// `article` for a dated page, `website` otherwise.
    pub(crate) og_type: &'static str,
    /// The site language as Open Graph writes it: `en`, `nl_BE`.
    pub(crate) locale: String,
    pub(crate) published_time: Option<String>,
    /// The page's own address in full, as its canonical URL.
    pub(crate) url: Option<String>,
    pub(crate) image: Option<LinkPreviewImage>,
}

pub(crate) struct LinkPreviewImage {
    pub(crate) url: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) alt: String,
}

/// One Luma event as the page shows it, its Luma addresses already built.
pub(crate) struct LumaEventView<'a> {
    pub(crate) event_id: &'a str,
    pub(crate) title: &'a str,
    pub(crate) event_url: String,
    pub(crate) embed_url: String,
    pub(crate) shows_event_page: bool,
    pub(crate) shows_register_button: bool,
    pub(crate) button_label: &'a str,
}

#[derive(Template)]
#[template(path = "luma_events.html")]
pub(crate) struct LumaEventsTemplate<'a> {
    pub(crate) events: Vec<LumaEventView<'a>>,
}

#[derive(Template)]
#[template(path = "navigation.html")]
pub(crate) struct NavigationTemplate<'a> {
    pub(crate) path: &'a str,
    pub(crate) ui: &'a UiText,
    pub(crate) site_title: &'a str,
    pub(crate) color: &'a str,
    pub(crate) main_menu: &'a Vec<MenuItem>,
    pub(crate) socials: &'a Vec<SocialItem>,
    pub(crate) side_menu: Option<&'a String>,
}

#[derive(Template)]
#[template(path = "breadcrumbs.html")]
pub(crate) struct BreadcrumbsTemplate<'a> {
    pub(crate) ui: &'a UiText,
    pub(crate) crumbs: Vec<MenuItem>,
    pub(crate) last: MenuItem,
}

#[derive(Template)]
#[template(path = "side_menu.html")]
pub(crate) struct SideMenuTemplate<'a> {
    pub(crate) path: &'a str,
    pub(crate) menu_item: &'a MenuItem,
}

/// A page's date, both machine-readable (for `<time datetime>`) and as the
/// reader sees it.
pub(crate) struct DisplayDate {
    pub(crate) iso: String,
    pub(crate) text: String,
}

impl DisplayDate {
    pub(crate) fn new(date: &PageDate, ui: &UiText) -> DisplayDate {
        DisplayDate {
            iso: date.to_string(),
            text: ui.format_date(date),
        }
    }
}

/// A feed as a page links to it: in its `<head>` for feed readers to find,
/// and on the feed's own page for people to follow.
pub(crate) struct FeedLink {
    pub(crate) title: String,
    pub(crate) href: String,
}

#[derive(Template)]
#[template(path = "content.html")]
pub(crate) struct ContentTemplate<'a> {
    pub(crate) title: &'a str,
    pub(crate) sub_title: &'a Option<String>,
    pub(crate) date: Option<DisplayDate>,
    pub(crate) image: Option<String>,
    pub(crate) content: String,
    pub(crate) feed: Option<FeedLink>,
    pub(crate) ui: &'a UiText,
}

#[derive(Template)]
#[template(path = "internal_notification.html")]
pub(crate) struct InternalNotificationTemplate<'a> {
    pub(crate) title: &'a Option<String>,
    pub(crate) sub_title: &'a Option<String>,
    pub(crate) date: Option<DisplayDate>,
    pub(crate) color: &'a str,
    pub(crate) url: &'a str,
    pub(crate) image: Option<String>,
    pub(crate) content: Option<String>,
    pub(crate) id: String,
    /// The tile's place among a page's child tiles once both columns are
    /// stacked into one on mobile: left tile `i` is `2i`, right tile `i` is
    /// `2i + 1`, so they read in the order they pair up on wider screens.
    /// `None` for side tiles.
    pub(crate) order: Option<usize>,
}

#[derive(Template)]
#[template(path = "external_notification.html")]
pub(crate) struct ExternalNotificationTemplate<'a> {
    pub(crate) title: &'a Option<String>,
    pub(crate) color: &'a str,
    pub(crate) url: &'a str,
    pub(crate) image: Option<String>,
    pub(crate) content: String,
    pub(crate) id: String,
    /// The tile's place among a page's child tiles once both columns are
    /// stacked into one on mobile: left tile `i` is `2i`, right tile `i` is
    /// `2i + 1`, so they read in the order they pair up on wider screens.
    /// `None` for side tiles.
    pub(crate) order: Option<usize>,
}

#[derive(Template)]
#[template(path = "content_notification.html")]
pub(crate) struct ContentNotificationTemplate<'a> {
    pub(crate) title: &'a Option<String>,
    pub(crate) color: &'a str,
    pub(crate) image: Option<String>,
    pub(crate) content: String,
    pub(crate) id: String,
    /// The tile's place among a page's child tiles once both columns are
    /// stacked into one on mobile: left tile `i` is `2i`, right tile `i` is
    /// `2i + 1`, so they read in the order they pair up on wider screens.
    /// `None` for side tiles.
    pub(crate) order: Option<usize>,
}

#[derive(Template)]
#[template(path = "footer.html")]
pub(crate) struct FooterTemplate<'a> {
    pub(crate) content: &'a str,
}

#[derive(Template)]
#[template(path = "internal_image.html")]
pub(crate) struct InternalImageTemplate<'a> {
    pub(crate) ratio: &'a str,
    pub(crate) title: &'a str,
    pub(crate) image_type: ImageType,
    pub(crate) src: &'a str,
    pub(crate) srcset: &'a str,
    pub(crate) alt: &'a str,
}
