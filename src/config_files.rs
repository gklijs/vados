use crate::bulma::{default_css_links, default_js_links, vados_js, Color};
use crate::files::write_raw;
use serde::{Deserialize, Serialize};

// Every config struct below derives `Serialize` as well as `Deserialize`:
// the content-authoring commands (`page new`, `image add`, `page add-image`,
// `social add`/`update`/`remove`, `footer set`, `menu add-item`) read one of
// these back, change the one thing they were asked to, and write the whole
// struct out again -- so the shape read and the shape written must always be
// exactly the same one. `generate`/`check` only ever read these, so before
// those commands existed there was nothing to round-trip.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MainConfig {
    pub(crate) site_title: String,
    pub(crate) js_files: Vec<String>,
    pub(crate) include_default_js: Option<bool>,
    pub(crate) css_files: Vec<String>,
    pub(crate) include_default_css: Option<bool>,
    pub(crate) background_class: Option<String>,
    pub(crate) navbar_color: Option<Color>,
    pub(crate) language: Option<String>,
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
    /// The site's content language, as an IETF BCP 47 tag, driving every
    /// published page's `<html lang>`. See `vados.allium`'s
    /// `SiteConfig.effective_language` and `SiteLanguageIsDeclared`.
    pub(crate) fn get_language(&self) -> &str {
        match self.language.as_deref() {
            None => "en",
            Some(l) => l,
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

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageConfig {
    pub(crate) title: String,
    pub(crate) sub_title: Option<String>,
    pub(crate) image: Option<String>,
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
            image: None,
            icon: None,
            summary: None,
            content: format!("<h1>{}</h1>", last),
            order: None,
            left_notifications: None,
            right_notifications: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawMenuItem {
    pub(crate) url: String,
    pub(crate) title: Option<String>,
    pub(crate) icon: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawSocialItem {
    pub(crate) url: String,
    pub(crate) icon: Option<String>,
    pub(crate) color: Option<String>,
    /// The link's accessible name (see `vados.allium`'s `SocialLink.label`).
    /// Required for a provider `is_recognized_social_provider` doesn't
    /// recognise, the same as `icon`/`color`.
    pub(crate) label: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MenuConfig {
    pub(crate) main_menu: Vec<RawMenuItem>,
    pub(crate) socials: Vec<RawSocialItem>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Notification {
    pub(crate) content: String,
    pub(crate) title: Option<String>,
    pub(crate) image: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) color: Option<Color>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageReference {
    pub(crate) title: Option<String>,
    pub(crate) file_name: String,
    pub(crate) alt_text: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ImageList {
    pub(crate) title: Option<String>,
    pub(crate) list: Vec<ImageReference>,
}
