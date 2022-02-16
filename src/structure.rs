use crate::config_files::{MenuConfig, PageConfig, RawMenuItem, RawSocialItem};
use crate::content::items_to_side_notifications;
use crate::structure::SocialItem::{Facebook, Github, LinkedIn, Other, YouTube};

use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use std::cmp::Ordering;
use std::sync::Arc;

fn parent_path(path: &str) -> Option<String> {
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
    pub(crate) fn new() -> Structure {
        Structure {
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
                let i = self.by_path.get(&*item.url).unwrap();
                let children = self.by_parent.get(&*item.url).map(|entry| {
                    entry
                        .value()
                        .iter()
                        .map(|i| i.to_side_menu_item(None))
                        .collect()
                });
                result.push(i.to_main_menu_item(item.title.clone(), item.icon.clone(), children))
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
        let items = self
            .by_parent
            .get(path)
            .unwrap_or_else(|| self.by_parent.get(&*parent_path(path).unwrap()).unwrap());
        let mut result = vec![];
        for item in items.value().iter().rev().take(4) {
            if result.len() < 3 && item.path != path {
                result.push(item.clone())
            }
        }
        result.reverse();
        items_to_side_notifications(result)
    }
    pub(crate) fn get_menu_item(&self, path: &str) -> MenuItem {
        self.by_path.get(path).unwrap().to_side_menu_item(None)
    }
    pub(crate) fn get_item(&self, path: &str) -> Arc<Item> {
        self.by_path.get(path).unwrap().clone()
    }
}

#[derive(Debug, PartialEq)]
pub enum SocialItem {
    Github(String),
    LinkedIn(String),
    Facebook(String),
    YouTube(String),
    Other(String, String, String),
}

impl SocialItem {
    pub(crate) fn new(raw: &RawSocialItem) -> SocialItem {
        match raw.url.clone() {
            url if url.starts_with("https://github.com/") => Github(url),
            url if url.starts_with("https://www.linkedin.com/") => LinkedIn(url),
            url if url.starts_with("https://www.facebook.com/") => Facebook(url),
            url if url.starts_with("https://www.youtube.com/") => YouTube(url),
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
            SocialItem::Other(url, _, _) => url,
        }
    }

    pub(crate) fn get_icon(&self) -> &str {
        match self {
            SocialItem::Github(_) => "github",
            SocialItem::LinkedIn(_) => "linkedin",
            SocialItem::Facebook(_) => "facebook",
            SocialItem::YouTube(_) => "youtube",
            SocialItem::Other(_, icon, _) => icon,
        }
    }

    pub(crate) fn get_color(&self) -> &str {
        match self {
            SocialItem::Github(_) => "171515",
            SocialItem::LinkedIn(_) => "0077b5",
            SocialItem::Facebook(_) => "4267B2",
            SocialItem::YouTube(_) => "c4302b",
            SocialItem::Other(_, _, color) => color,
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
