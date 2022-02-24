use crate::bulma::{default_css_links, default_js_links, vados_js, Color};
use crate::files::write_raw;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MainConfig {
    pub(crate) site_title: String,
    pub(crate) js_files: Vec<String>,
    pub(crate) include_default_js: Option<bool>,
    pub(crate) css_files: Vec<String>,
    pub(crate) include_default_css: Option<bool>,
    pub(crate) background_class: Option<String>,
    pub(crate) navbar_color: Option<Color>,
    pub(crate) footer_content: String,
}

impl MainConfig {
    pub(crate) fn get_background_class(&self) -> String {
        match self.background_class.as_ref() {
            None => String::from("has-background-light"),
            Some(s) => s.clone(),
        }
    }
    pub(crate) fn get_navbar_color(&self) -> &'static str {
        match self.navbar_color.as_ref() {
            None => Color::Warning.to_css_class(),
            Some(s) => s.to_css_class(),
        }
    }
    pub(crate) fn get_css_links(&self) -> Vec<String> {
        match self.include_default_css {
            Some(b) if !b => self.css_files.clone(),
            _ => {
                let mut result = self.css_files.clone();
                result.append(&mut default_css_links());
                result
            }
        }
    }
    pub(crate) fn get_js_links(&self, destination: &str) -> Vec<String> {
        match self.include_default_js {
            Some(b) if !b => self.js_files.clone(),
            _ => {
                write_raw(destination, default_js_links().get(0).unwrap(), vados_js());
                let mut result = self.js_files.clone();
                result.append(&mut default_js_links());
                result
            }
        }
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageConfig {
    pub(crate) title: String,
    pub(crate) sub_title: Option<String>,
    pub(crate) icon: Option<String>,
    pub(crate) summary: Option<String>,
    pub(crate) content: String,
    pub(crate) order: Option<u32>,
    pub(crate) left_notifications: Option<Vec<Notification>>,
    pub(crate) right_notifications: Option<Vec<Notification>>,
}

impl PageConfig {
    pub(crate) fn new(path: &str) -> PageConfig {
        let last = path.split('/').last().unwrap();
        PageConfig {
            title: String::from(last),
            sub_title: None,
            icon: None,
            summary: None,
            content: format!("<h1>{}</h1>", last),
            order: None,
            left_notifications: None,
            right_notifications: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawMenuItem {
    pub(crate) url: String,
    pub(crate) title: Option<String>,
    pub(crate) icon: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawSocialItem {
    pub(crate) url: String,
    pub(crate) icon: Option<String>,
    pub(crate) color: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MenuConfig {
    pub(crate) main_menu: Vec<RawMenuItem>,
    pub(crate) socials: Vec<RawSocialItem>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Notification {
    pub(crate) content: String,
    pub(crate) title: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) color: Option<Color>,
}
