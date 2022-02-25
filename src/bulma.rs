use crate::bulma::ImageRatio::{
    Is16By9, Is1By1, Is1By2, Is1By3, Is2By1, Is2By3, Is3By1, Is3By2, Is3By4, Is3By5, Is4By3,
    Is4by5, Is5By3, Is5By4, Is9By16,
};
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

#[derive(Debug)]
pub(crate) enum ImageRatio {
    Is1By1,
    Is5By4,
    Is4By3,
    Is3By2,
    Is5By3,
    Is16By9,
    Is2By1,
    Is3By1,
    Is4by5,
    Is3By4,
    Is2By3,
    Is3By5,
    Is9By16,
    Is1By2,
    Is1By3,
}

impl ImageRatio {
    pub(crate) fn best_fitting(width: &u32, height: &u32) -> ImageRatio {
        match *width as f32 / *height as f32 {
            r if r > 2.50 => Is3By1,
            r if r > 1.88 => Is2By1,
            r if r > 1.72 => Is16By9,
            r if r > 1.58 => Is5By3,
            r if r > 1.42 => Is3By2,
            r if r > 1.29 => Is4By3,
            r if r > 1.13 => Is5By4,
            r if r > 0.90 => Is1By1,
            r if r > 0.78 => Is4by5,
            r if r > 0.71 => Is3By4,
            r if r > 0.63 => Is2By3,
            r if r > 0.58 => Is3By5,
            r if r > 0.53 => Is9By16,
            r if r > 0.42 => Is1By2,
            _ => Is1By3,
        }
    }
    pub(crate) fn get_height(&self, width: &u32) -> u32 {
        match self {
            Is1By1 => *width,
            Is5By4 => width * 4 / 5,
            Is4By3 => width * 3 / 4,
            Is3By2 => width * 2 / 3,
            Is5By3 => width * 3 / 5,
            Is16By9 => width * 9 / 16,
            Is2By1 => width / 2,
            Is3By1 => width / 3,
            Is4by5 => width * 5 / 4,
            Is3By4 => width * 4 / 3,
            Is2By3 => width * 3 / 2,
            Is3By5 => width * 5 / 3,
            Is9By16 => width * 16 / 9,
            Is1By2 => width * 2,
            Is1By3 => width * 3,
        }
    }
    pub(crate) fn get_width(&self, height: &u32) -> u32 {
        match self {
            Is1By1 => *height,
            Is5By4 => height * 5 / 4,
            Is4By3 => height * 4 / 3,
            Is3By2 => height * 3 / 2,
            Is5By3 => height * 5 / 3,
            Is16By9 => height * 16 / 9,
            Is2By1 => height * 2,
            Is3By1 => height * 3,
            Is4by5 => height * 4 / 5,
            Is3By4 => height * 3 / 4,
            Is2By3 => height * 2 / 3,
            Is3By5 => height * 3 / 5,
            Is9By16 => height * 9 / 16,
            Is1By2 => height / 2,
            Is1By3 => height / 3,
        }
    }
    pub(crate) fn to_css_class(&self) -> &'static str {
        match self {
            Is1By1 => "is-1by1",
            Is5By4 => "is-5by4",
            Is4By3 => "is-4by3",
            Is3By2 => "is-3by2",
            Is5By3 => "is-5by3",
            Is16By9 => "is-16by9",
            Is2By1 => "is-2by1",
            Is3By1 => "is-3by1",
            Is4by5 => "is-4by5",
            Is3By4 => "is-3by4",
            Is2By3 => "is-2by3",
            Is3By5 => "is-3by5",
            Is9By16 => "is-9by16",
            Is1By2 => "is-1by2",
            Is1By3 => "is-1by3",
        }
    }
}

pub(crate) enum ImageType {
    Main,
    Sub,
    Side,
}

impl ImageType {
    pub(crate) fn sizes(&self) -> &'static str {
        match self {
            ImageType::Main => "(min-width: 1408px) 986px, (min-width: 769px) calc(75vw - 94px), calc(100vw - 64px)",
            ImageType::Sub => "(min-width: 1408px) 425px, (min-width: 769px) calc(37.5vw - 106px), calc(100vw - 112px)",
            ImageType::Side => "(min-width: 1408px) 318px, (min-width: 769px) calc(25vw - 94px), calc(100vw - 112px)",
        }
    }
    pub(crate) fn decoding(&self) -> &'static str {
        match self {
            ImageType::Main => "sync",
            ImageType::Sub => "async",
            ImageType::Side => "async",
        }
    }
    pub(crate) fn loading(&self) -> &'static str {
        match self {
            ImageType::Main => "eager",
            ImageType::Sub => "lazy",
            ImageType::Side => "lazy",
        }
    }
}
