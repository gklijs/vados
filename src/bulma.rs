use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq, Clone, Hash)]
pub(crate) enum Color {
    White,
    Black,
    Light,
    Dark,
    Primary,
    Link,
    Info,
    Succes,
    Warning,
    Danger,
}

impl Color {
    pub(crate) fn to_css_class(&self) -> &'static str {
        match self {
            Color::White => "is-white",
            Color::Black => "is-black",
            Color::Light => "is-light",
            Color::Dark => "is-dark",
            Color::Primary => "is-primary",
            Color::Link => "is-link",
            Color::Info => "is-info",
            Color::Succes => "is-success",
            Color::Warning => "is-warning",
            Color::Danger => "is-danger",
        }
    }
}

pub(crate) fn default_css_links() -> Vec<String> {
    vec![
        String::from(
            "https://cdn.jsdelivr.net/npm/@mdi/font@6.5.95/css/materialdesignicons.min.css",
        ),
        String::from("https://cdn.jsdelivr.net/npm/bulma@0.9.3/css/bulma.min.css"),
    ]
}

pub(crate) fn default_js_links() -> Vec<String> {
    vec![String::from("/js/vados.js")]
}

pub(crate) fn vados_js() -> &'static str {
    r#"let burger_menu_active = false
let side_menu_active = false
let burger_menu = document.getElementById("burger-menu")
let main_menu = document.getElementById("main-menu")
let burger_side_menu = document.getElementById("burger-side-menu")
let side_menu = document.getElementById("side-menu-mobile")
const toggleBurgerMenu = function () {
    if (side_menu_active) {
        toggleSideMenu()
    }
    if (burger_menu_active) {
        burger_menu.classList.remove("is-active")
        main_menu.classList.remove("is-active")
    } else {
        burger_menu.classList.add("is-active")
        main_menu.classList.add("is-active")
    }
    burger_menu_active = !burger_menu_active
}
const toggleSideMenu = function () {
    if (burger_menu_active) {
        toggleBurgerMenu()
    }
    if (side_menu_active) {
        burger_side_menu.classList.remove("is-active")
        side_menu.classList.add("is-hidden")
    } else {
        burger_side_menu.classList.add("is-active")
        side_menu.classList.remove("is-hidden")
    }
    side_menu_active = !side_menu_active
}
burger_menu.onclick = function () {
    toggleBurgerMenu()
}
if (typeof (burger_side_menu) != 'undefined' && burger_side_menu != null) {
    burger_side_menu.onclick = function () {
        toggleSideMenu()
    }
}"#
}
