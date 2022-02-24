use crate::bulma::Color;
use crate::config_files::{MainConfig, MenuConfig, Notification, PageConfig};
use crate::structure::{Item, SocialItem, Structure};
use crate::templates::{
    BreadcrumbsTemplate, ContentNotificationTemplate, ContentTemplate,
    ExternalNotificationTemplate, FooterTemplate, InternalNotificationTemplate, NavigationTemplate,
    PageTemplate, SideMenuTemplate,
};
use askama::Template;
use pulldown_cmark::{html, Parser};
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
}

impl GenericContent {
    pub(crate) fn new(source: &str, destination: &str, main_config: &MainConfig) -> GenericContent {
        let background_class = main_config.get_background_class();
        let css_links = main_config.get_css_links();
        let js_links = main_config.get_js_links(destination);
        let footer = get_footer(source, main_config);
        GenericContent {
            background_class,
            css_links,
            js_links,
            footer,
        }
    }
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
    pub(crate) fn get_main_content(&self, source: &str) -> String {
        get_main_content(source, self.path, &*self.item)
    }
    pub(crate) fn get_page(
        &self,
        navigation: &str,
        main_content: &str,
        generic_content: &GenericContent,
    ) -> String {
        let breadcrumbs = get_breadcrumbs(self.path, self.structure);
        let page_helper = PageHelper {
            navigation,
            breadcrumbs,
            side_menu: self.side_menu.as_ref(),
            main_content,
        };
        get_page(
            self.path,
            &*self.item,
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

pub(crate) fn to_content_items(source: &str, dir_path: String) -> ContentItems {
    let page_file_string = format!("{}/page.json", &dir_path);
    let page_file_path = Path::new(&page_file_string);
    let page_config = match File::open(page_file_path) {
        Ok(f) => serde_json::from_reader(f).expect("JSON was not well-formatted"),
        Err(_) => PageConfig::new(&dir_path),
    };
    let path = match dir_path {
        root if root == source => String::from("/"),
        mut d => d.split_off(source.len()),
    };
    let left_sub_notifications = match &page_config.left_notifications {
        None => None,
        Some(notifications) => {
            let mut result = vec![];
            for (i, notification) in notifications.iter().enumerate() {
                let id = format!("sub-l-{}", i);
                result.push(resolve_notification(
                    source,
                    &path,
                    notification.clone(),
                    id,
                ))
            }
            Some(result)
        }
    };
    let right_sub_notifications = match &page_config.right_notifications {
        None => None,
        Some(notifications) => {
            let mut result = vec![];
            for (i, notification) in notifications.iter().enumerate() {
                let id = format!("sub-r-{}", i);
                result.push(resolve_notification(
                    source,
                    &path,
                    notification.clone(),
                    id,
                ))
            }
            Some(result)
        }
    };
    let item = Item::new(path, page_config);
    ContentItems {
        item,
        left_sub_notifications,
        right_sub_notifications,
    }
}

fn resolve_notification(
    source: &str,
    path: &str,
    notification: Notification,
    id: String,
) -> String {
    let content = get_content(source, path, &*notification.content);
    match notification.url {
        None => {
            let color = notification.color.unwrap_or(Color::Info).to_css_class();
            ContentNotificationTemplate {
                title: &notification.title,
                color,
                content,
                id,
            }
            .render()
            .unwrap()
        }
        Some(internal) if internal.starts_with('/') => {
            let color = notification.color.unwrap_or(Color::Link).to_css_class();
            InternalNotificationTemplate {
                title: &notification.title,
                sub_title: &None,
                color,
                url: &*internal,
                content: Some(content),
                id,
            }
            .render()
            .unwrap()
        }
        Some(external) => {
            let color = notification.color.unwrap_or(Color::Link).to_css_class();
            ExternalNotificationTemplate {
                title: &notification.title,
                color,
                url: &*external,
                content,
                id,
            }
            .render()
            .unwrap()
        }
    }
}

pub(crate) fn items_to_side_notifications(items: Vec<Arc<Item>>) -> Vec<String> {
    let mut result = vec![];
    for (i, item) in items.iter().enumerate() {
        let id = format!("sub-s-{}", i);
        let color = Color::Info.to_css_class();
        let notification = InternalNotificationTemplate {
            title: &Some(item.title.clone()),
            sub_title: &item.sub_title,
            color,
            url: &*item.path,
            content: None,
            id,
        }
        .render()
        .unwrap();
        result.push(notification)
    }
    result
}

fn get_file_path(source: &str, path: &str, reference: &str) -> String {
    if path == "/" {
        format!("{}/{}", source, reference)
    } else {
        format!("{}{}/{}", source, path, reference)
    }
}

fn md_to_content(file_path: &str) -> String {
    match fs::read_to_string(file_path) {
        Ok(text) => {
            let mut html_output: String = String::with_capacity(text.len() * 3 / 2);
            let parser = Parser::new(&*text);
            html::push_html(&mut html_output, parser);
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

fn get_content(source: &str, path: &str, reference: &str) -> String {
    let file_path = get_file_path(source, path, reference);
    match reference {
        md if md.ends_with(".md") => md_to_content(&file_path),
        html if html.ends_with(".html") => html_to_content(&file_path),
        raw if raw.ends_with('>') => String::from(raw),
        _ => panic!(
            "Can't handle content reference that looks like: {}",
            reference
        ),
    }
}

fn get_footer(source: &str, main_config: &MainConfig) -> String {
    let footer_content = get_content(source, "/", &main_config.footer_content);
    FooterTemplate {
        content: &footer_content,
    }
    .render()
    .unwrap()
}

fn get_side_menu(path: &str, structure: &Structure) -> Option<String> {
    match structure.get_side_menu_items(path) {
        None => None,
        Some(menu_item) => {
            let s = SideMenuTemplate {
                path: &*path,
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
        site_title: &*main_config.site_title,
        color: main_config.get_navbar_color(),
        main_menu: &structure.get_main_menu_items(menu_config),
        socials: &menu_config.socials.iter().map(SocialItem::new).collect(),
        side_menu,
    }
    .render()
    .unwrap()
}

fn get_breadcrumbs(path: &str, structure: &Structure) -> Option<String> {
    structure.get_breadcrumbs(path).map(|crumbs| {
        BreadcrumbsTemplate {
            crumbs,
            last: structure.get_menu_item(path),
        }
        .render()
        .unwrap()
    })
}

fn get_main_content(source: &str, path: &str, item: &Item) -> String {
    ContentTemplate {
        title: &*item.title,
        sub_title: &item.sub_title,
        content: get_content(source, path, &item.content),
    }
    .render()
    .unwrap()
}

fn get_page(
    path: &str,
    item: &Item,
    page_helper: PageHelper,
    structure: &Structure,
    generic_content: &GenericContent,
) -> String {
    PageTemplate {
        title: &*item.title,
        summary: &item.summary,
        background_class: &generic_content.background_class,
        navigation: page_helper.navigation,
        breadcrumbs: page_helper.breadcrumbs,
        side_menu: page_helper.side_menu,
        main_content: page_helper.main_content,
        left_sub_notifications: &*structure.get_left_sub_notifications(path),
        right_sub_notifications: &*structure.get_right_sub_notifications(path),
        side_notifications: &structure.get_side_notifications(path),
        footer: &generic_content.footer,
        css_links: &generic_content.css_links,
        js_links: &generic_content.js_links,
    }
    .render()
    .unwrap()
}
