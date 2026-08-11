use crate::bulma::ImageType;
use crate::config_files::{MenuConfig, PageConfig, RawMenuItem, RawSocialItem};
use crate::content::{items_to_side_notifications, to_internal_image};
use crate::image::ProcessedImage;
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
        Some((first, _)) if first.is_empty() => Some(String::from("/")),
        Some((first, _)) => Some(String::from(first)),
    }
}

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
}

impl Item {
    pub(crate) fn new(path: String, page_config: PageConfig) -> Item {
        Item {
            path,
            title: page_config.title,
            sub_title: page_config.sub_title,
            image: page_config.image,
            icon: page_config.icon,
            summary: page_config.summary,
            content: page_config.content,
            order: page_config.order.unwrap_or(u32::MAX),
        }
    }
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
        self.by_parent.iter_mut().for_each(|mut r| r.sort())
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
            d if d == 3 => self
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
                            parent = parent_path(&*p)
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
    pub(crate) fn get_side_notifications(&self, path: &str) -> Vec<String> {
        // Children first, falling back to siblings; see vados.allium's
        // `side_notification_source`. A childless root page has no parent to
        // take siblings from either -- per the spec's `siblings: Page with
        // parent = this.parent`, a page with no parent has no siblings, so
        // there is nothing to show rather than something to panic over.
        let own_children = self.by_parent.get(path);
        let items = match own_children
            .or_else(|| parent_path(path).and_then(|parent| self.by_parent.get(&*parent)))
        {
            Some(items) => items,
            None => return vec![],
        };
        let mut result = vec![];
        for item in items.value().iter().rev().take(4) {
            if result.len() < 3 && item.path != path {
                result.push(item.clone())
            }
        }
        result.reverse();
        items_to_side_notifications(result, self)
    }
    pub(crate) fn get_menu_item(&self, path: &str) -> MenuItem {
        self.by_path.get(path).unwrap().to_side_menu_item(None)
    }
    pub(crate) fn get_item(&self, path: &str) -> Arc<Item> {
        self.by_path.get(path).unwrap().clone()
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
        icon: "twitter",
        brand_color: "1DA1F2",
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
        let brand_color = canonical_brand_color(provider)
            .map(String::from)
            .unwrap_or_else(|| {
                raw.color
                    .clone()
                    .expect("Other social links should have color.")
            });
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
