use crate::bulma::ImageType;
use crate::config_files::{MenuConfig, PageConfig, RawMenuItem, RawSocialItem};
use crate::content::{items_to_side_notifications, to_internal_image};
use crate::image::ProcessedImage;
use crate::structure::SocialItem::{Facebook, Github, LinkedIn, Other, Twitter, YouTube};
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

#[derive(Debug, PartialEq)]
pub enum SocialItem {
    Github(String),
    LinkedIn(String),
    Facebook(String),
    YouTube(String),
    Twitter(String),
    Other(String, String, String),
}

/// Which provider a social link's URL was recognised as, without needing a
/// `RawSocialItem` (an icon and color) the way `SocialItem::new` does. See
/// `vados.allium`'s `SocialProvider` enum and `social_provider_of`. Public so
/// the `social add`/`update`/`remove` commands can classify a URL the
/// maintainer gave before any icon/color has been supplied.
///
/// `twitter.com` and `x.com` are both recognised as the same provider; the
/// rebrand changed the URL, not the identity of the account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocialProviderKind {
    Github,
    LinkedIn,
    Facebook,
    YouTube,
    Twitter,
    Other,
}

/// Kept in sync with `SocialItem::new`'s matching below: the two must always
/// agree on which provider a URL belongs to.
pub fn classify_social_provider(url: &str) -> SocialProviderKind {
    use SocialProviderKind::*;
    match url {
        u if u.starts_with("https://github.com/") => Github,
        u if u.starts_with("https://www.linkedin.com/") => LinkedIn,
        u if u.starts_with("https://www.facebook.com/") => Facebook,
        u if u.starts_with("https://www.youtube.com/") => YouTube,
        u if u.starts_with("https://twitter.com/") || u.starts_with("https://x.com/") => Twitter,
        _ => Other,
    }
}

/// Whether a social link's URL matches one of the providers `SocialItem`
/// recognises automatically, so `check` can flag an unrecognised provider
/// missing its required icon and color before generation would panic on it.
pub(crate) fn is_recognized_social_provider(url: &str) -> bool {
    classify_social_provider(url) != SocialProviderKind::Other
}

/// The Material Design Icons name a recognised provider always uses. `None`
/// for `Other`, which has no canonical icon -- the maintainer must supply
/// one. Kept in sync with `SocialItem::get_icon` below.
pub fn canonical_icon(provider: SocialProviderKind) -> Option<&'static str> {
    use SocialProviderKind::*;
    match provider {
        Github => Some("github"),
        LinkedIn => Some("linkedin"),
        Facebook => Some("facebook"),
        YouTube => Some("youtube"),
        Twitter => Some("twitter"),
        Other => None,
    }
}

/// The brand color a recognised provider always uses. `None` for `Other`, the
/// same way as `canonical_icon`. Kept in sync with `SocialItem::get_color`
/// below.
pub fn canonical_brand_color(provider: SocialProviderKind) -> Option<&'static str> {
    use SocialProviderKind::*;
    match provider {
        Github => Some("171515"),
        LinkedIn => Some("0077b5"),
        Facebook => Some("4267B2"),
        YouTube => Some("c4302b"),
        Twitter => Some("1DA1F2"),
        Other => None,
    }
}

impl SocialItem {
    pub(crate) fn new(raw: &RawSocialItem) -> SocialItem {
        match raw.url.clone() {
            url if url.starts_with("https://github.com/") => Github(url),
            url if url.starts_with("https://www.linkedin.com/") => LinkedIn(url),
            url if url.starts_with("https://www.facebook.com/") => Facebook(url),
            url if url.starts_with("https://www.youtube.com/") => YouTube(url),
            url if url.starts_with("https://twitter.com/") || url.starts_with("https://x.com/") => {
                Twitter(url)
            }
            url => {
                let icon = raw
                    .icon
                    .as_ref()
                    .expect("Other social links should have icon.");
                let color = raw
                    .color
                    .as_ref()
                    .expect("Other social links should have color.");
                Other(url, icon.clone(), color.clone())
            }
        }
    }
    pub(crate) fn get_url(&self) -> &str {
        match self {
            SocialItem::Github(url) => url,
            SocialItem::LinkedIn(url) => url,
            SocialItem::Facebook(url) => url,
            SocialItem::YouTube(url) => url,
            SocialItem::Twitter(url) => url,
            SocialItem::Other(url, _, _) => url,
        }
    }

    pub(crate) fn get_icon(&self) -> &str {
        match self {
            SocialItem::Github(_) => "github",
            SocialItem::LinkedIn(_) => "linkedin",
            SocialItem::Facebook(_) => "facebook",
            SocialItem::YouTube(_) => "youtube",
            SocialItem::Twitter(_) => "twitter",
            SocialItem::Other(_, icon, _) => icon,
        }
    }

    pub(crate) fn get_color(&self) -> &str {
        match self {
            SocialItem::Github(_) => "171515",
            SocialItem::LinkedIn(_) => "0077b5",
            SocialItem::Facebook(_) => "4267B2",
            SocialItem::YouTube(_) => "c4302b",
            SocialItem::Twitter(_) => "1DA1F2",
            SocialItem::Other(_, _, color) => color,
        }
    }
}

/// A provider `init`'s wizard can gather a bare handle for. See
/// `vados.allium`'s `config.recognized_social_providers`: every variant here
/// has a canonical icon and brand color (kept in sync with `SocialItem`
/// above), so a handle alone is enough to produce a complete social link --
/// unlike `SocialItem::Other`, which needs both supplied explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecognizedSocialProvider {
    Github,
    LinkedIn,
    Facebook,
    YouTube,
    Twitter,
}

impl RecognizedSocialProvider {
    pub fn all() -> [RecognizedSocialProvider; 5] {
        use RecognizedSocialProvider::*;
        [Github, LinkedIn, Facebook, YouTube, Twitter]
    }

    /// A short label for the wizard prompt.
    pub fn label(self) -> &'static str {
        match self {
            RecognizedSocialProvider::Github => "GitHub",
            RecognizedSocialProvider::LinkedIn => "LinkedIn",
            RecognizedSocialProvider::Facebook => "Facebook",
            RecognizedSocialProvider::YouTube => "YouTube",
            RecognizedSocialProvider::Twitter => "X",
        }
    }

    /// The profile URL a bare handle resolves to for this provider. Kept in
    /// sync with `SocialItem::new`'s prefix matching above, so a handle the
    /// wizard gathers always produces a URL `is_recognized_social_provider`
    /// recognizes.
    pub fn profile_url(self, handle: &str) -> String {
        match self {
            RecognizedSocialProvider::Github => format!("https://github.com/{handle}"),
            RecognizedSocialProvider::LinkedIn => {
                format!("https://www.linkedin.com/in/{handle}/")
            }
            RecognizedSocialProvider::Facebook => format!("https://www.facebook.com/{handle}"),
            RecognizedSocialProvider::YouTube => format!("https://www.youtube.com/{handle}"),
            RecognizedSocialProvider::Twitter => format!("https://x.com/{handle}"),
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
}
