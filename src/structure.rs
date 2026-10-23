use crate::bulma::ImageType;
use crate::config_files::{
    ChildOrder, FeedConfig, MenuConfig, PageConfig, RawMenuItem, RawSocialItem,
};
use crate::content::{items_to_side_notifications, to_internal_image};
use crate::image::ProcessedImage;
use crate::luma::{is_luma_event_id, LumaEvent};
use crate::page_date::PageDate;
use crate::ui_text::UiText;
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use std::cmp::Ordering;
use std::sync::Arc;

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
    pub(crate) child_order: ChildOrder,
    /// Set when this page publishes a feed of its dated children.
    pub(crate) feed: Option<FeedConfig>,
}

impl Item {
    pub(crate) fn new(path: String, page_config: PageConfig) -> Item {
        let feed = page_config.get_feed();
        let luma_events = published_luma_events(&path, page_config.luma_events);
        let date = published_date(&path, page_config.date.as_deref());
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
            child_order: page_config.child_order.unwrap_or_default(),
            feed,
        }
    }
}

/// The page's date, if it gives one that can be read. A malformed date is
/// reported and the page treated as undated, the same way a malformed Luma
/// event is reported and dropped; `check` reports it as an error.
fn published_date(path: &str, declared: Option<&str>) -> Option<PageDate> {
    match declared?.parse() {
        Ok(date) => Some(date),
        Err(e) => {
            println!(
                "Page {} has an unreadable date: {}; treating it as undated.",
                path, e
            );
            None
        }
    }
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

/// Drops the Luma events that can't be shown -- a malformed ID or no title
/// -- reporting each as it goes, the same way an unresolvable main menu
/// entry is reported and dropped. `check` reports the same events as
/// errors; see `vados.allium`'s `AttachLumaEvent`.
fn published_luma_events(path: &str, declared: Option<Vec<LumaEvent>>) -> Vec<LumaEvent> {
    declared
        .unwrap_or_default()
        .into_iter()
        .filter(|e| {
            if !is_luma_event_id(&e.event_id) {
                println!(
                    "Luma event {} on page {} is not a Luma event ID; leaving it off the page.",
                    e.event_id, path
                );
                false
            } else if e.effective_title().is_none() {
                println!(
                    "Luma event {} on page {} has no title; leaving it off the page.",
                    e.event_id, path
                );
                false
            } else {
                true
            }
        })
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
        }
    }
    fn to_side_menu_item(&self, children: Option<Vec<MenuItem>>) -> MenuItem {
        MenuItem {
            menu_type: MenuType::Internal,
            url: self.path.clone(),
            title: self.title.clone(),
            icon: self.icon.clone(),
            children,
        }
    }
}

impl RawMenuItem {
    fn to_external_item(&self) -> MenuItem {
        MenuItem {
            menu_type: MenuType::External,
            url: self.url.clone(),
            title: self
                .title
                .as_ref()
                .expect("External link should have a title")
                .clone(),
            icon: self.icon.clone(),
            children: None,
        }
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
    pub(crate) fn get_main_menu_items(&self, menu_config: &MenuConfig) -> Vec<MenuItem> {
        let mut result = vec![];
        for item in &menu_config.main_menu {
            if item.url.starts_with("https://") {
                result.push(item.to_external_item())
            } else {
                match self.by_path.get(&*item.url) {
                    None => println!(
                        "Main menu entry with url {} does not match any page; dropping it from the menu.",
                        item.url
                    ),
                    Some(i) => {
                        let children = self.by_parent.get(&*item.url).map(|entry| {
                            entry
                                .value()
                                .iter()
                                .map(|i| i.to_side_menu_item(None))
                                .collect()
                        });
                        result.push(i.to_main_menu_item(
                            item.title.clone(),
                            item.icon.clone(),
                            children,
                        ))
                    }
                }
            }
        }
        result
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
    /// Every page that publishes a feed.
    pub(crate) fn get_feed_items(&self) -> Vec<Arc<Item>> {
        self.by_path
            .iter()
            .filter(|i| i.feed.is_some())
            .map(|i| i.value().clone())
            .collect()
    }
    /// A page's direct children, in display order.
    pub(crate) fn get_children(&self, path: &str) -> Vec<Arc<Item>> {
        self.by_parent
            .get(path)
            .map(|c| c.value().clone())
            .unwrap_or_default()
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
    /// Every url prefix this provider is recognised from when classifying an
    /// already-written url. The first is also the one a profile url is built
    /// from. More than one only for Twitter/X: `x.com` and `twitter.com`
    /// both name the same provider, but only `x.com` is written going
    /// forward.
    url_prefixes: &'static [&'static str],
    /// Appended after the handle when building a profile url from one; empty
    /// for every provider but LinkedIn.
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
        url_prefixes: &["https://github.com/"],
        profile_url_suffix: "",
        icon: "github",
        brand_color: "171515",
        label: "GitHub",
    },
    ProviderSpec {
        kind: SocialProviderKind::LinkedIn,
        url_prefixes: &["https://www.linkedin.com/in/"],
        profile_url_suffix: "/",
        icon: "linkedin",
        brand_color: "0077b5",
        label: "LinkedIn",
    },
    ProviderSpec {
        kind: SocialProviderKind::Facebook,
        url_prefixes: &["https://www.facebook.com/"],
        profile_url_suffix: "",
        icon: "facebook",
        brand_color: "4267B2",
        label: "Facebook",
    },
    ProviderSpec {
        kind: SocialProviderKind::YouTube,
        url_prefixes: &["https://www.youtube.com/"],
        profile_url_suffix: "",
        icon: "youtube",
        brand_color: "c4302b",
        label: "YouTube",
    },
    ProviderSpec {
        kind: SocialProviderKind::Twitter,
        url_prefixes: &["https://x.com/", "https://twitter.com/"],
        profile_url_suffix: "",
        // MDI 6.5.95 has no X logo; a boxed X in X's black is closer to
        // the current brand than the old bird in Twitter blue.
        icon: "alpha-x-box",
        brand_color: "000000",
        label: "X",
    },
];

fn provider_spec(kind: SocialProviderKind) -> Option<&'static ProviderSpec> {
    PROVIDERS.iter().find(|p| p.kind == kind)
}

/// Classifies a social link's url by matching it against every recognised
/// provider's url prefixes in `PROVIDERS`.
pub fn classify_social_provider(url: &str) -> SocialProviderKind {
    PROVIDERS
        .iter()
        .find(|p| p.url_prefixes.iter().any(|prefix| url.starts_with(prefix)))
        .map_or(SocialProviderKind::Other, |p| p.kind)
}

/// Whether a social link's URL matches one of the providers `PROVIDERS`
/// recognises automatically, so `check` can flag an unrecognised provider
/// missing its required icon and color before generation would panic on it.
pub(crate) fn is_recognized_social_provider(url: &str) -> bool {
    classify_social_provider(url) != SocialProviderKind::Other
}

/// The Material Design Icons name a recognised provider always uses. `None`
/// for `Other`, which has no canonical icon -- the maintainer must supply
/// one.
pub fn canonical_icon(provider: SocialProviderKind) -> Option<&'static str> {
    provider_spec(provider).map(|p| p.icon)
}

/// The brand color a recognised provider always uses. `None` for `Other`, the
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

/// The accessible name a recognised provider always uses. `None` for
/// `Other`, the same way as `canonical_icon`. See `vados.allium`'s
/// `SocialLink.label`.
pub fn canonical_label(provider: SocialProviderKind) -> Option<&'static str> {
    provider_spec(provider).map(|p| p.label)
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
    pub(crate) fn new(raw: &RawSocialItem) -> SocialItem {
        let provider = classify_social_provider(&raw.url);
        let icon = canonical_icon(provider)
            .map(String::from)
            .unwrap_or_else(|| {
                raw.icon
                    .clone()
                    .expect("Other social links should have icon.")
            });
        // An explicit color wins over the provider's brand color, so a site
        // can render every icon in one color (for example on a dark navbar).
        let brand_color = raw
            .color
            .as_deref()
            .map(normalize_brand_color)
            .or_else(|| canonical_brand_color(provider).map(String::from))
            .expect("Other social links should have color.");
        let label = canonical_label(provider)
            .map(String::from)
            .unwrap_or_else(|| {
                raw.label
                    .clone()
                    .expect("Other social links should have label.")
            });
        SocialItem {
            url: raw.url.clone(),
            icon,
            brand_color,
            label,
        }
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
}

impl RecognizedSocialProvider {
    fn kind(self) -> SocialProviderKind {
        match self {
            RecognizedSocialProvider::Github => SocialProviderKind::Github,
            RecognizedSocialProvider::LinkedIn => SocialProviderKind::LinkedIn,
            RecognizedSocialProvider::Facebook => SocialProviderKind::Facebook,
            RecognizedSocialProvider::YouTube => SocialProviderKind::YouTube,
            RecognizedSocialProvider::Twitter => SocialProviderKind::Twitter,
        }
    }

    fn spec(self) -> &'static ProviderSpec {
        provider_spec(self.kind())
            .expect("every RecognizedSocialProvider variant has a PROVIDERS entry")
    }

    pub fn all() -> [RecognizedSocialProvider; 5] {
        use RecognizedSocialProvider::*;
        [Github, LinkedIn, Facebook, YouTube, Twitter]
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
            spec.url_prefixes[0], handle, spec.profile_url_suffix
        )
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_text::ui_text;

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
        ));
        assert_eq!(item.get_color(), "ffffff");
    }

    #[test]
    fn an_explicit_social_color_wins_over_the_brand_color() {
        let item = SocialItem::new(&raw_social(
            "https://github.com/gklijs",
            None,
            Some("eeeeee"),
        ));
        assert_eq!(item.get_color(), "eeeeee");
        let item = SocialItem::new(&raw_social("https://github.com/gklijs", None, None));
        assert_eq!(item.get_color(), "171515");
    }

    #[test]
    fn x_links_get_the_x_icon_and_color() {
        let item = SocialItem::new(&raw_social("https://x.com/gklijs", None, None));
        assert_eq!(item.get_icon(), "alpha-x-box");
        assert_eq!(item.get_color(), "000000");
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
