//! One page's main content and the page template around it.

use super::tiles::{in_reading_order, latest_tiles};
use super::{get_content, GenericContent, MarkdownOptions, PageHelper, AGENDA_SCRIPT_HREF};
use crate::bulma::ImageType;
use crate::event::{agenda_calendar_href, event_calendar_href};
use crate::feed::{feed_href, feed_title, page_url};
use crate::highlight::{self, HIGHLIGHTED_MARKER};
use crate::luma::BUTTON_SCRIPT_URL;
use crate::search::{SEARCH_INDEX_HREF, SEARCH_PATH, SEARCH_SCRIPT_HREF};
use crate::structure::{Item, Structure};
use crate::structured_data::{structured_data, PageFacts, SiteFacts};
use crate::templates::{
    AgendaEntry, AgendaTemplate, ContentTemplate, DisplayDate, EventDetailsTemplate, FeedLink,
    LinkPreview, LinkPreviewImage, LumaEventView, LumaEventsTemplate, PageTemplate, SearchTemplate,
};
use crate::ui_text::UiText;
use askama::Template;

pub(super) fn get_main_content(
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

pub(super) fn get_page(
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
}
