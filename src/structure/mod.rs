use crate::bulma::ImageType;
use crate::config_files::{
    is_below, AgendaConfig, ChildOrder, FeedConfig, LatestConfig, MenuConfig, PageConfig,
    PageSelection, RawMenuItem, SearchConfig,
};
use crate::content::{items_to_side_notifications, to_internal_image};
use crate::event::{CalendarEntry, DeclaredEvent, Event};
use crate::image::ProcessedImage;
use crate::luma::{is_luma_event_id, LumaEvent};
use crate::page_date::PageDate;
use crate::search::SEARCH_PATH;
use crate::ui_text::UiText;
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::Arc;

mod social;

pub(crate) use social::is_recognized_social_provider;
pub use social::{
    canonical_brand_color, canonical_icon, canonical_label, classify_social_provider,
    normalize_brand_color, RecognizedSocialProvider, SocialItem, SocialProviderKind,
};

/// The path one level up from `path`, or `None` for the root itself. Public
/// (rather than crate-private) because the content-authoring commands need
/// it too: a page-creation request's parent must already be a page (see
/// `vados.allium`'s `DetectPageCreationBlockers`), the same relationship this
/// module already needed for menus, breadcrumbs and side notifications.
pub fn parent_path(path: &str) -> Option<String> {
    if path.len() <= 1 {
        return None;
    }
    match String::from(path).rsplit_once('/') {
        None => Some(String::from("/")),
        Some(("", _)) => Some(String::from("/")),
        Some((first, _)) => Some(String::from(first)),
    }
}

/// Whether the page at `path` is `section` or below it, by whole path segments:
/// `/blogging` isn't in `/blog`. Everything is below the home page, so the home
/// page only counts as itself; otherwise its menu entry would always be active.
pub(crate) fn is_within(path: &str, section: &str) -> bool {
    path == section
        || (section != "/"
            && path
                .strip_prefix(section)
                .is_some_and(|rest| rest.starts_with('/')))
}

/// See `vados.allium`'s `config.max_side_notifications`.
const MAX_SIDE_NOTIFICATIONS: usize = 3;

#[derive(Debug, Eq)]
pub(crate) struct Item {
    pub(crate) path: String,
    pub(crate) title: String,
    pub(crate) sub_title: Option<String>,
    pub(crate) image: Option<String>,
    pub(crate) icon: Option<String>,
    pub(crate) summary: Option<String>,
    pub(crate) content: String,
    pub(crate) order: u32,
    /// Only the events that can actually be shown; see `vados.allium`'s
    /// `Page.published_luma_events`.
    pub(crate) luma_events: Vec<LumaEvent>,
    /// `None` both when page.json gives no date and when the one it gives
    /// can't be read; see `vados.allium`'s `Page.published_date`.
    pub(crate) date: Option<PageDate>,
    /// Read the same way as `date`.
    pub(crate) updated: Option<PageDate>,
    /// Whether the page shows up wherever pages are listed.
    pub(crate) listed: bool,
    pub(crate) child_order: ChildOrder,
    /// Set when this page publishes a feed of its dated children.
    pub(crate) feed: Option<FeedConfig>,
    pub(crate) latest: Option<LatestConfig>,
    /// Whether the page stays out of search engines and the sitemap.
    pub(crate) noindex: bool,
    /// `None` when page.json gives no event, or one that isn't valid.
    pub(crate) event: Option<Event>,
    pub(crate) agenda: Option<AgendaConfig>,
}

impl Item {
    pub(crate) fn new(path: String, page_config: PageConfig) -> Item {
        let feed = page_config.get_feed();
        let agenda = page_config.get_agenda();
        let event = published_event(&path, page_config.event.as_ref());
        let luma_events = published_luma_events(&path, page_config.luma_events);
        let date = published_date(&path, page_config.date.as_deref());
        let updated = published_date(&path, page_config.updated.as_deref());
        Item {
            path,
            title: page_config.title,
            sub_title: page_config.sub_title,
            image: page_config.image,
            icon: page_config.icon,
            summary: page_config.summary,
            content: page_config.content,
            order: page_config.order.unwrap_or(u32::MAX),
            luma_events,
            date,
            updated,
            listed: page_config.listed.unwrap_or(true),
            child_order: page_config.child_order.unwrap_or_default(),
            feed,
            latest: page_config.latest,
            noindex: page_config.noindex.unwrap_or(false),
            event,
            agenda,
        }
    }
    /// When the page last changed: its `updated` date, or else its `date`.
    pub(crate) fn modified(&self) -> Option<PageDate> {
        self.updated.or(self.date)
    }
}

/// Malformed dates are treated as undated; preflight collects their warnings.
fn published_date(_path: &str, declared: Option<&str>) -> Option<PageDate> {
    declared?.parse().ok()
}

/// Invalid events are omitted. The generation preflight collects their warnings.
fn published_event(_path: &str, declared: Option<&DeclaredEvent>) -> Option<Event> {
    declared?.resolve().ok()
}

/// Where `a` goes relative to `b` among the children of a page that orders
/// them by `child_order`. Ordering by date puts dated children first, newest
/// or oldest first as asked, and undated ones after them; whatever the date
/// leaves tied falls back to the `order`-then-title ordering. See
/// `vados.allium`'s `Page.precedes`.
fn display_order(child_order: ChildOrder, a: &Item, b: &Item) -> Ordering {
    let by_date = match (child_order, &a.date, &b.date) {
        (ChildOrder::Order, _, _) => Ordering::Equal,
        (_, Some(_), None) => Ordering::Less,
        (_, None, Some(_)) => Ordering::Greater,
        (_, None, None) => Ordering::Equal,
        (ChildOrder::DateAsc, Some(x), Some(y)) => x.cmp(y),
        (ChildOrder::DateDesc, Some(x), Some(y)) => y.cmp(x),
    };
    by_date.then_with(|| a.cmp(b))
}

/// Drops Luma events with a malformed ID or no title. Preflight collects the
/// warnings; see `vados.allium`'s `AttachLumaEvent`.
fn published_luma_events(_path: &str, declared: Option<Vec<LumaEvent>>) -> Vec<LumaEvent> {
    declared
        .unwrap_or_default()
        .into_iter()
        .filter(|e| is_luma_event_id(&e.event_id) && e.effective_title().is_some())
        .collect()
}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.order != other.order {
            self.order.cmp(&other.order)
        } else {
            self.title.cmp(&other.title)
        }
    }
}

impl PartialOrd for Item {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

/// One line of an agenda: a page's event, or a Luma event a page links.
pub(crate) struct ListedEvent {
    pub(crate) event: Event,
    pub(crate) title: String,
    /// Set for a Luma event, whose identity and address are Luma's.
    pub(crate) luma_url: Option<String>,
    pub(crate) page: Arc<Item>,
}

impl ListedEvent {
    pub(crate) fn summary(&self) -> Option<String> {
        if self.luma_url.is_some() {
            return None;
        }
        self.page
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    }
    pub(crate) fn to_calendar_entry(&self, site_url: &str) -> CalendarEntry<'_> {
        match &self.luma_url {
            None => CalendarEntry::of_page(&self.event, &self.page, site_url),
            Some(url) => CalendarEntry {
                event: &self.event,
                title: self.title.clone(),
                summary: None,
                url: url.clone(),
                uid: url.clone(),
                modified: self.page.modified(),
            },
        }
    }
}

#[derive(Debug)]
pub(crate) struct Structure {
    image_meta_cache: DashMap<String, Arc<ProcessedImage>>,
    by_path: DashMap<String, Arc<Item>>,
    by_parent: DashMap<String, Vec<Arc<Item>>>,
    left_sub_notifications_cache: DashMap<String, Arc<Vec<String>>>,
    right_sub_notifications_cache: DashMap<String, Arc<Vec<String>>>,
}

impl Item {
    fn to_main_menu_item(
        &self,
        title: Option<String>,
        icon: Option<String>,
        children: Option<Vec<MenuItem>>,
    ) -> MenuItem {
        let icon = match icon {
            None => self.icon.clone(),
            Some(i) => Some(i),
        };
        MenuItem {
            menu_type: MenuType::Internal,
            url: self.path.clone(),
            title: title.unwrap_or_else(|| self.title.clone()),
            icon,
            children,
            more: false,
        }
    }
    fn to_side_menu_item(&self, children: Option<Vec<MenuItem>>) -> MenuItem {
        MenuItem {
            menu_type: MenuType::Internal,
            url: self.path.clone(),
            title: self.title.clone(),
            icon: self.icon.clone(),
            children,
            more: false,
        }
    }
}

impl RawMenuItem {
    fn to_external_item(&self) -> Result<MenuItem, String> {
        Ok(MenuItem {
            menu_type: MenuType::External,
            url: self.url.clone(),
            title: self
                .title
                .as_ref()
                .ok_or_else(|| format!("external menu link {} has no title", self.url))?
                .clone(),
            icon: self.icon.clone(),
            children: None,
            more: false,
        })
    }
}

impl Structure {
    pub(crate) fn new(image_meta_cache: DashMap<String, Arc<ProcessedImage>>) -> Structure {
        Structure {
            image_meta_cache,
            by_path: DashMap::new(),
            by_parent: DashMap::new(),
            left_sub_notifications_cache: DashMap::new(),
            right_sub_notifications_cache: DashMap::new(),
        }
    }
    pub(crate) fn add_item(&self, item: Item) {
        let i = Arc::new(item);
        self.by_path.insert(i.path.clone(), i.clone());
        // An unlisted page is no one's child as far as listing goes, so it
        // stays out of menus, side tiles and feeds without each of them
        // having to know. See vados.allium's `Page.listed`.
        if !i.listed {
            return;
        }
        match parent_path(&i.path) {
            None => {}
            Some(p) => {
                match self.by_parent.entry(p) {
                    Entry::Occupied(mut e) => e.get_mut().push(i),
                    Entry::Vacant(e) => {
                        e.insert(vec![i]);
                    }
                };
            }
        }
    }
    pub(crate) fn add_left_sub_notifications(&self, path: &str, notifications: Vec<String>) {
        self.left_sub_notifications_cache
            .insert(String::from(path), Arc::new(notifications));
    }
    pub(crate) fn add_right_sub_notifications(&self, path: &str, notifications: Vec<String>) {
        self.right_sub_notifications_cache
            .insert(String::from(path), Arc::new(notifications));
    }
    pub(crate) fn sort(&self) {
        self.by_parent.iter_mut().for_each(|mut r| {
            let child_order = self.child_order_of(r.key());
            r.value_mut()
                .sort_by(|a, b| display_order(child_order, a, b))
        })
    }
    fn child_order_of(&self, path: &str) -> ChildOrder {
        self.by_path
            .get(path)
            .map(|p| p.child_order)
            .unwrap_or_default()
    }
    pub(crate) fn get_main_menu_items(
        &self,
        menu_config: &MenuConfig,
        dropdown_limit: usize,
        search_entry: bool,
    ) -> Result<Vec<MenuItem>, String> {
        let mut result = vec![];
        for item in &menu_config.main_menu {
            if item.url.starts_with("https://") {
                result.push(item.to_external_item()?)
            } else {
                match self.by_path.get(&*item.url) {
                    None => println!(
                        "Main menu entry with url {} does not match any page; dropping it from the menu.",
                        item.url
                    ),
                    Some(i) => {
                        let children = self.by_parent.get(&*item.url);
                        let more = children
                            .as_ref()
                            .is_some_and(|c| c.value().len() > dropdown_limit);
                        let children = children.map(|entry| {
                            entry
                                .value()
                                .iter()
                                .take(dropdown_limit)
                                .map(|i| i.to_side_menu_item(None))
                                .collect()
                        });
                        let mut menu_item = i.to_main_menu_item(
                            item.title.clone(),
                            item.icon.clone(),
                            children,
                        );
                        menu_item.more = more;
                        result.push(menu_item)
                    }
                }
            }
        }
        // Unless the menu already names the search, in its own place.
        if search_entry && !menu_config.main_menu.iter().any(|i| i.url == SEARCH_PATH) {
            if let Some(search) = self.by_path.get(SEARCH_PATH) {
                let icon = search.icon.clone().or(Some(String::from("magnify")));
                result.push(search.to_main_menu_item(None, icon, None));
            }
        }
        Ok(result)
    }
    pub(crate) fn get_side_menu_items(&self, path: &str) -> Option<MenuItem> {
        let depth = path.matches('/').count();
        match depth {
            d if d <= 2 => None,
            3 => self
                .by_parent
                .get(path)
                .map(|entry| {
                    entry
                        .value()
                        .iter()
                        .map(|i| i.to_side_menu_item(None))
                        .collect()
                })
                .map(|c| self.by_path.get(path).unwrap().to_side_menu_item(Some(c))),
            _ => {
                match self.by_parent.get(path).map(|entry| {
                    entry
                        .value()
                        .iter()
                        .map(|i| i.to_side_menu_item(None))
                        .collect()
                }) {
                    None => {
                        let parent_path = parent_path(path).unwrap();
                        let children = self.by_parent.get(&parent_path).map(|entry| {
                            entry
                                .value()
                                .iter()
                                .map(|i| i.to_side_menu_item(None))
                                .collect()
                        });
                        Some(
                            self.by_path
                                .get(&parent_path)
                                .unwrap()
                                .to_side_menu_item(children),
                        )
                    }
                    Some(c) => Some(self.by_path.get(path).unwrap().to_side_menu_item(Some(c))),
                }
            }
        }
    }
    pub(crate) fn get_breadcrumbs(&self, path: &str) -> Option<Vec<MenuItem>> {
        let depth = path.matches('/').count();
        match depth {
            d if d <= 1 => None,
            _ => {
                let mut result = vec![];
                let mut parent = parent_path(path);
                loop {
                    match parent {
                        None => {
                            result.reverse();
                            return Some(result);
                        }
                        Some(p) => {
                            result.push(self.by_path.get(&*p).unwrap().to_side_menu_item(None));
                            parent = parent_path(&p)
                        }
                    }
                }
            }
        }
    }
    pub(crate) fn get_left_sub_notifications(&self, path: &str) -> Arc<Vec<String>> {
        let mut notifications = self.left_sub_notifications_cache.get(path);
        let mut parent = parent_path(path);
        loop {
            match notifications {
                Some(entry) => return entry.value().clone(),
                None => match parent {
                    None => return Arc::new(vec![]),
                    Some(p) => {
                        notifications = self.left_sub_notifications_cache.get(&*p);
                        parent = parent_path(&p)
                    }
                },
            }
        }
    }
    pub(crate) fn get_right_sub_notifications(&self, path: &str) -> Arc<Vec<String>> {
        let mut notifications = self.right_sub_notifications_cache.get(path);
        let mut parent = parent_path(path);
        loop {
            match notifications {
                Some(entry) => return entry.value().clone(),
                None => match parent {
                    None => return Arc::new(vec![]),
                    Some(p) => {
                        notifications = self.right_sub_notifications_cache.get(&*p);
                        parent = parent_path(&p)
                    }
                },
            }
        }
    }
    pub(crate) fn get_side_notifications(&self, path: &str, ui: &UiText) -> Vec<String> {
        // Children first, falling back to siblings; see vados.allium's
        // `side_notification_source`. A childless root page has no parent to
        // take siblings from either -- per the spec's `siblings: Page with
        // parent = this.parent`, a page with no parent has no siblings, so
        // there is nothing to show rather than something to panic over.
        let source = if self.by_parent.contains_key(path) {
            Some(String::from(path))
        } else {
            parent_path(path)
        };
        let Some(items) = source.as_deref().and_then(|s| self.by_parent.get(s)) else {
            return vec![];
        };
        let others: Vec<Arc<Item>> = items
            .value()
            .iter()
            .filter(|item| item.path != path)
            .cloned()
            .collect();
        // Taken from the newest end of display order: the start when the
        // source is ordered newest first, the end otherwise. See
        // vados.allium's `side_notification_pages`.
        let result = if self.child_order_of(source.as_deref().unwrap()) == ChildOrder::DateDesc {
            others.into_iter().take(MAX_SIDE_NOTIFICATIONS).collect()
        } else {
            others[others.len().saturating_sub(MAX_SIDE_NOTIFICATIONS)..].to_vec()
        };
        items_to_side_notifications(result, self, ui)
    }
    pub(crate) fn get_menu_item(&self, path: &str) -> MenuItem {
        self.by_path.get(path).unwrap().to_side_menu_item(None)
    }
    pub(crate) fn get_item(&self, path: &str) -> Arc<Item> {
        self.by_path.get(path).unwrap().clone()
    }
    /// The newest listed, dated pages below any of `sources`, at most
    /// `depth` levels down when given, newest first, leaving out `path`
    /// itself. See vados.allium's `Page.latest_pages`.
    pub(crate) fn get_latest(
        &self,
        path: &str,
        sources: &[&str],
        depth: Option<u32>,
        limit: u32,
    ) -> Vec<Arc<Item>> {
        let mut latest: Vec<Arc<Item>> = self
            .by_path
            .iter()
            .filter(|i| i.listed && i.date.is_some() && i.path != path)
            .filter(|i| sources.iter().any(|s| is_below(&i.path, s, depth)))
            .map(|i| i.value().clone())
            .collect();
        // Newest first; the path settles ties, so the result is the same on
        // every run.
        latest.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.path.cmp(&b.path)));
        latest.truncate(limit as usize);
        latest
    }
    /// Every page that is listed; see vados.allium's `Page.listed`.
    pub(crate) fn get_listed_items(&self) -> Vec<Arc<Item>> {
        self.by_path
            .iter()
            .filter(|i| i.listed && !i.noindex)
            .map(|i| i.value().clone())
            .collect()
    }
    /// The page at `path`, if there is one.
    pub(crate) fn find_item(&self, path: &str) -> Option<Arc<Item>> {
        self.by_path.get(path).map(|i| i.value().clone())
    }
    /// Every page, in no particular order.
    pub(crate) fn get_all_items(&self) -> Vec<Arc<Item>> {
        self.by_path.iter().map(|i| i.value().clone()).collect()
    }
    /// Whether any page shows an agenda.
    pub(crate) fn has_agenda(&self) -> bool {
        self.by_path.iter().any(|i| i.agenda.is_some())
    }
    fn select(&self, selection: &PageSelection, own_path: Option<&str>) -> Vec<Arc<Item>> {
        self.by_path
            .iter()
            .filter(|i| Some(i.path.as_str()) != own_path)
            .filter(|i| selection.selects(&i.path, i.listed))
            .map(|i| i.value().clone())
            .collect()
    }
    /// The pages a feed lists: selected, dated, newest first, at most its
    /// limit. Pages with the same date go in display order. See
    /// vados.allium's `Page.feed_entries`.
    pub(crate) fn get_feed_entries(&self, item: &Item, feed: &FeedConfig) -> Vec<Arc<Item>> {
        let mut entries = self.select(&feed.selection(&item.path), Some(&item.path));
        entries.retain(|i| i.date.is_some());
        entries.sort_by(|a, b| {
            b.date
                .cmp(&a.date)
                .then_with(|| a.as_ref().cmp(b.as_ref()))
                .then_with(|| a.path.cmp(&b.path))
        });
        entries.truncate(feed.get_limit() as usize);
        entries
    }
    /// The events an agenda lists, soonest first: the selected pages' events,
    /// and the Luma events on them, including the agenda's own, that say when
    /// they are. See vados.allium's `Page.agenda_events`.
    pub(crate) fn get_agenda_events(&self, item: &Item, agenda: &AgendaConfig) -> Vec<ListedEvent> {
        let mut pages = self.select(&agenda.selection(&item.path), Some(&item.path));
        if !item.luma_events.is_empty() {
            pages.extend(self.find_item(&item.path));
        }
        let mut listed = vec![];
        for page in pages {
            if page.path != item.path {
                if let Some(event) = &page.event {
                    listed.push(ListedEvent {
                        event: event.clone(),
                        title: page.title.clone(),
                        luma_url: None,
                        page: page.clone(),
                    });
                }
            }
            for luma in &page.luma_events {
                if let Some(Ok(event)) = luma.schedule() {
                    listed.push(ListedEvent {
                        event,
                        title: luma.effective_title().unwrap_or_default().to_string(),
                        luma_url: Some(luma.event_url()),
                        page: page.clone(),
                    });
                }
            }
        }
        listed.sort_by(|a, b| {
            a.event
                .start
                .cmp(&b.event.start)
                .then_with(|| a.page.path.cmp(&b.page.path))
                .then_with(|| a.title.cmp(&b.title))
        });
        // The same Luma event linked from two pages is still one event, and a
        // calendar can't hold one identity twice.
        let mut seen = HashSet::new();
        listed.retain(|l| l.luma_url.as_ref().is_none_or(|u| seen.insert(u.clone())));
        listed
    }
    /// The pages a search covers. See vados.allium's `SiteSearch`.
    pub(crate) fn get_searchable_items(&self, config: &SearchConfig) -> Vec<Arc<Item>> {
        let selection = PageSelection {
            sources: config.from.as_ref().map(|f| f.paths()),
            depth: config.depth,
            include_unlisted: config.include_unlisted.unwrap_or(false),
        };
        let mut items = self.select(&selection, Some(SEARCH_PATH));
        items.retain(|i| !i.noindex);
        items
    }
    /// Every page that publishes a feed.
    pub(crate) fn get_feed_items(&self) -> Vec<Arc<Item>> {
        self.by_path
            .iter()
            .filter(|i| i.feed.is_some())
            .map(|i| i.value().clone())
            .collect()
    }
    /// The page at `path` and its ancestors, nearest first -- the pages
    /// whose feeds `path` sits inside.
    pub(crate) fn get_self_and_ancestors(&self, path: &str) -> Vec<Arc<Item>> {
        let mut result = vec![];
        let mut current = Some(String::from(path));
        while let Some(p) = current {
            if let Some(item) = self.by_path.get(&p) {
                result.push(item.value().clone());
            }
            current = parent_path(&p);
        }
        result
    }
    pub(crate) fn get_image(&self, image_reference: &str) -> Option<Arc<ProcessedImage>> {
        self.image_meta_cache
            .get(image_reference)
            .map(|i| i.value().clone())
    }
    pub(crate) fn process_image(
        &self,
        image_reference: &str,
        image_type: ImageType,
    ) -> Option<String> {
        if let Some(p) = self.image_meta_cache.get(image_reference) {
            Some(to_internal_image(p.clone(), image_type))
        } else {
            println!("No image was found with reference {}.", image_reference);
            None
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MenuType {
    Internal,
    External,
}

#[derive(Debug)]
pub(crate) struct MenuItem {
    pub(crate) menu_type: MenuType,
    pub(crate) url: String,
    pub(crate) title: String,
    pub(crate) icon: Option<String>,
    pub(crate) children: Option<Vec<MenuItem>>,
    /// Whether `children` was cut short, so the list ends in a link to this
    /// item's own page. See `vados.allium`'s `DropdownsStayShort`.
    pub(crate) more: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_text::ui_text;

    #[test]
    fn a_page_is_within_a_section_by_whole_segments() {
        assert!(is_within("/blog", "/blog"));
        assert!(is_within("/blog/post", "/blog"));
        assert!(!is_within("/blogging", "/blog"));
        assert!(!is_within("/about", "/a"));
        assert!(is_within("/", "/"));
        assert!(!is_within("/blog", "/"));
        assert!(!is_within("/blog", "https://example.com"));
    }

    fn page(path: &str, date: Option<&str>, order: Option<u32>) -> Item {
        let mut config = PageConfig::new(path);
        config.date = date.map(String::from);
        config.order = order;
        Item::new(String::from(path), config)
    }

    fn section(child_order: ChildOrder, children: Vec<Item>) -> Structure {
        let structure = Structure::new(DashMap::new());
        let mut root = PageConfig::new("/blog");
        root.child_order = Some(child_order);
        structure.add_item(Item::new(String::from("/blog"), root));
        children.into_iter().for_each(|c| structure.add_item(c));
        structure.sort();
        structure
    }

    fn child_paths(structure: &Structure) -> Vec<String> {
        structure
            .by_parent
            .get("/blog")
            .unwrap()
            .iter()
            .map(|i| i.path.clone())
            .collect()
    }

    fn posts() -> Vec<Item> {
        vec![
            page("/blog/b-undated", None, None),
            page("/blog/old", Some("2025-01-05"), None),
            page("/blog/new", Some("2026-10-22"), None),
            page("/blog/a-undated", None, None),
            page("/blog/same-day-later", Some("2026-03-01T18:00"), None),
            page("/blog/same-day", Some("2026-03-01"), None),
        ]
    }

    #[test]
    fn date_desc_puts_the_newest_first_and_undated_last() {
        let structure = section(ChildOrder::DateDesc, posts());
        assert_eq!(
            child_paths(&structure),
            [
                "/blog/new",
                "/blog/same-day-later",
                "/blog/same-day",
                "/blog/old",
                "/blog/a-undated",
                "/blog/b-undated"
            ]
        );
    }

    #[test]
    fn date_asc_puts_the_oldest_first_and_undated_last() {
        let structure = section(ChildOrder::DateAsc, posts());
        assert_eq!(
            child_paths(&structure),
            [
                "/blog/old",
                "/blog/same-day",
                "/blog/same-day-later",
                "/blog/new",
                "/blog/a-undated",
                "/blog/b-undated"
            ]
        );
    }

    #[test]
    fn order_ignores_dates_entirely() {
        let structure = section(
            ChildOrder::Order,
            vec![
                page("/blog/new", Some("2026-10-22"), None),
                page("/blog/old", Some("2025-01-05"), Some(1)),
            ],
        );
        assert_eq!(child_paths(&structure), ["/blog/old", "/blog/new"]);
    }

    #[test]
    fn same_dates_fall_back_to_order_then_title() {
        let structure = section(
            ChildOrder::DateDesc,
            vec![
                page("/blog/b", Some("2026-10-22"), None),
                page("/blog/a", Some("2026-10-22"), None),
                page("/blog/c", Some("2026-10-22"), Some(1)),
            ],
        );
        assert_eq!(child_paths(&structure), ["/blog/c", "/blog/a", "/blog/b"]);
    }

    #[test]
    fn an_unreadable_date_counts_as_undated() {
        assert_eq!(page("/blog/x", Some("yesterday"), None).date, None);
    }

    fn side_notification_paths(structure: &Structure, path: &str) -> Vec<String> {
        structure
            .get_side_notifications(path, ui_text("en"))
            .iter()
            .map(|html| {
                let start = html.find("href=\"").unwrap() + 6;
                let end = start + html[start..].find('"').unwrap();
                html[start..end].to_string()
            })
            .collect()
    }

    #[test]
    fn side_notifications_show_the_newest_pages_when_ordered_newest_first() {
        let structure = section(ChildOrder::DateDesc, posts());
        assert_eq!(
            side_notification_paths(&structure, "/blog"),
            ["/blog/new", "/blog/same-day-later", "/blog/same-day"]
        );
        assert_eq!(
            side_notification_paths(&structure, "/blog/new"),
            ["/blog/same-day-later", "/blog/same-day", "/blog/old"]
        );
    }

    #[test]
    fn side_notifications_otherwise_come_from_the_end() {
        let structure = section(ChildOrder::DateAsc, posts());
        assert_eq!(
            side_notification_paths(&structure, "/blog"),
            ["/blog/new", "/blog/a-undated", "/blog/b-undated"]
        );
        assert_eq!(
            side_notification_paths(&structure, "/blog/b-undated"),
            ["/blog/same-day-later", "/blog/new", "/blog/a-undated"]
        );
    }

    #[test]
    fn side_notifications_show_each_pages_date() {
        let structure = section(ChildOrder::DateDesc, posts());
        let html = structure
            .get_side_notifications("/blog", ui_text("en"))
            .join("");
        assert!(html.contains(r#"<time datetime="2026-10-22">22 October 2026</time>"#));
    }
}
