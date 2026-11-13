//! The navbar, side menu and breadcrumbs.

use crate::config_files::{MainConfig, MenuConfig};
use crate::structure::{SocialItem, Structure};
use crate::templates::{BreadcrumbsTemplate, NavigationTemplate, SideMenuTemplate};
use crate::ui_text::{ui_text, UiText};
use askama::Template;

pub(super) fn get_side_menu(path: &str, structure: &Structure) -> Option<String> {
    match structure.get_side_menu_items(path) {
        None => None,
        Some(menu_item) => {
            let s = SideMenuTemplate {
                path,
                menu_item: &menu_item,
            };
            Some(s.render().unwrap())
        }
    }
}

pub(super) fn get_navigation(
    path: &str,
    main_config: &MainConfig,
    menu_config: &MenuConfig,
    structure: &Structure,
    side_menu: Option<&String>,
) -> Result<String, String> {
    Ok(NavigationTemplate {
        path,
        ui: ui_text(main_config.get_language()),
        site_title: &main_config.site_title,
        color: main_config.get_navbar_color(),
        main_menu: &structure.get_main_menu_items(
            menu_config,
            main_config.get_menu_dropdown_limit(),
            main_config.get_search().is_some_and(|s| s.shows_in_menu()),
        )?,
        socials: &menu_config
            .socials
            .iter()
            .map(SocialItem::new)
            .collect::<Result<Vec<_>, _>>()?,
        side_menu,
    }
    .render()
    .unwrap())
}

pub(super) fn get_breadcrumbs(path: &str, structure: &Structure, ui: &UiText) -> Option<String> {
    structure.get_breadcrumbs(path).map(|crumbs| {
        BreadcrumbsTemplate {
            ui,
            crumbs,
            last: structure.get_menu_item(path),
        }
        .render()
        .unwrap()
    })
}
