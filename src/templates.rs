use crate::structure::MenuType::Internal;
use crate::structure::{MenuItem, SocialItem};
use askama::Template;

#[derive(Template)]
#[template(path = "page.html")]
pub(crate) struct PageTemplate<'a> {
    pub(crate) title: &'a str,
    pub(crate) summary: &'a Option<String>,
    pub(crate) background_class: &'a str,
    pub(crate) navigation: &'a str,
    pub(crate) breadcrumbs: Option<String>,
    pub(crate) side_menu: Option<&'a String>,
    pub(crate) main_content: &'a str,
    pub(crate) left_sub_notifications: &'a Vec<String>,
    pub(crate) right_sub_notifications: &'a Vec<String>,
    pub(crate) side_notifications: &'a Vec<String>,
    pub(crate) footer: &'a str,
    pub(crate) css_links: &'a [String],
    pub(crate) js_links: &'a [String],
}

#[derive(Template)]
#[template(path = "navigation.html")]
pub(crate) struct NavigationTemplate<'a> {
    pub(crate) path: &'a str,
    pub(crate) site_title: &'a str,
    pub(crate) color: &'a str,
    pub(crate) main_menu: &'a Vec<MenuItem>,
    pub(crate) socials: &'a Vec<SocialItem>,
    pub(crate) side_menu: Option<&'a String>,
}

#[derive(Template)]
#[template(path = "breadcrumbs.html")]
pub(crate) struct BreadcrumbsTemplate {
    pub(crate) crumbs: Vec<MenuItem>,
    pub(crate) last: MenuItem,
}

#[derive(Template)]
#[template(path = "side_menu.html")]
pub(crate) struct SideMenuTemplate<'a> {
    pub(crate) path: &'a str,
    pub(crate) menu_item: &'a MenuItem,
}

#[derive(Template)]
#[template(path = "content.html")]
pub(crate) struct ContentTemplate<'a> {
    pub(crate) title: &'a str,
    pub(crate) sub_title: &'a Option<String>,
    pub(crate) content: String,
}

#[derive(Template)]
#[template(path = "internal_notification.html")]
pub(crate) struct InternalNotificationTemplate<'a> {
    pub(crate) title: &'a Option<String>,
    pub(crate) sub_title: &'a Option<String>,
    pub(crate) color: &'a str,
    pub(crate) url: &'a str,
    pub(crate) content: Option<String>,
    pub(crate) id: String,
}

#[derive(Template)]
#[template(path = "external_notification.html")]
pub(crate) struct ExternalNotificationTemplate<'a> {
    pub(crate) title: &'a Option<String>,
    pub(crate) color: &'a str,
    pub(crate) url: &'a str,
    pub(crate) content: String,
    pub(crate) id: String,
}

#[derive(Template)]
#[template(path = "content_notification.html")]
pub(crate) struct ContentNotificationTemplate<'a> {
    pub(crate) title: &'a Option<String>,
    pub(crate) color: &'a str,
    pub(crate) content: String,
    pub(crate) id: String,
}

#[derive(Template)]
#[template(path = "footer.html")]
pub(crate) struct FooterTemplate<'a> {
    pub(crate) content: &'a str,
}
