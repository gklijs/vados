use crate::bulma::ImageRatio::{
    Is16By9, Is1By1, Is1By2, Is1By3, Is2By1, Is2By3, Is3By1, Is3By2, Is3By4, Is3By5, Is4By3,
    Is4by5, Is5By3, Is5By4, Is9By16,
};
use serde::{Deserialize, Serialize};

/// One of Bulma's named colours.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Hash)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub(crate) enum Color {
    White,
    Black,
    Light,
    Dark,
    Primary,
    Link,
    Info,
    Success,
    Warning,
    Danger,
}

impl Color {
    /// The colour's Bulma class, and the Bulma theme matching the colour's
    /// brightness, so what sits on it -- headings, code, a navbar's menus --
    /// reads as on a light or dark page whatever the reader's colour scheme.
    /// Which theme follows from the text colour Bulma picks for each colour
    /// (its `-invert`): dark text means a light colour. See vados.allium's
    /// `FollowsTheReadersColourScheme`.
    pub(crate) fn to_css_class(&self) -> &'static str {
        match self {
            Color::White => "is-white theme-light",
            Color::Black => "is-black theme-dark",
            Color::Light => "is-light theme-light",
            Color::Dark => "is-dark theme-dark",
            Color::Primary => "is-primary theme-light",
            Color::Link => "is-link theme-dark",
            Color::Info => "is-info theme-light",
            Color::Success => "is-success theme-light",
            Color::Warning => "is-warning theme-light",
            Color::Danger => "is-danger theme-light",
        }
    }

    /// The classes of a navbar in this colour. `theme` replaces the theme the
    /// colour's default brightness implies, for a site whose stylesheet
    /// recolours it to the opposite brightness, which vados can't see.
    pub(crate) fn to_navbar_class(&self, theme: Option<NavbarTheme>) -> String {
        let class = self.to_css_class();
        match theme {
            None => String::from(class),
            Some(theme) => {
                let name = class.split(' ').next().unwrap_or_default();
                format!("{name} {}", theme.to_css_class())
            }
        }
    }
}

/// The Bulma theme on the navbar: `light` or `dark`.
// What sits on the navbar, when the colour's default brightness isn't what
// the site compiled.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub(crate) enum NavbarTheme {
    Light,
    Dark,
}

impl NavbarTheme {
    fn to_css_class(self) -> &'static str {
        match self {
            NavbarTheme::Light => "theme-light",
            NavbarTheme::Dark => "theme-dark",
        }
    }
}

pub(crate) fn default_css_links() -> Vec<String> {
    // No icon font: icons are inlined as SVG (see `icons.rs`).
    vec![String::from(
        "https://cdn.jsdelivr.net/npm/bulma@1.0.4/css/bulma.min.css",
    )]
}

/// Where a notification sits on its page. Child tiles are cells of a
/// two-column Bulma grid, read row by row: on a phone every cell spans both
/// columns, so they stack in the order they pair up on wider screens. See
/// `vados.allium`'s `ChildTilesPairUp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TilePlacement {
    /// The left column of the child tiles.
    Left,
    /// The right column of the child tiles.
    Right,
    /// The side column, next to the page's own content.
    Side,
}

impl TilePlacement {
    /// The classes to add after the notification's own, with a leading
    /// space.
    pub(crate) fn to_css_classes(self) -> &'static str {
        match self {
            TilePlacement::Left => " cell mb-0 is-col-start-1-tablet is-col-span-2-mobile",
            TilePlacement::Right => " cell mb-0 is-col-start-2-tablet is-col-span-2-mobile",
            // Side tiles are links in a flex column, not grid cells; make
            // them blocks.
            TilePlacement::Side => " is-block",
        }
    }
}

impl TilePlacement {
    /// The class putting a grid cell in this column on a tablet and wider.
    pub(crate) fn to_column_class(self) -> &'static str {
        match self {
            TilePlacement::Left => "is-col-start-1-tablet",
            TilePlacement::Right => "is-col-start-2-tablet",
            TilePlacement::Side => "",
        }
    }
}

/// The most rows Bulma's grid can make a cell span.
pub(crate) const MAX_ROW_SPAN: usize = 12;

/// A rendered child tile made to span `rows` rows of the grid from where it
/// starts, on a tablet and wider; on a phone the tiles stack anyway.
pub(crate) fn spanning_rows(tile: &str, rows: usize) -> String {
    let rows = rows.clamp(1, MAX_ROW_SPAN);
    const ANCHOR: &str = "is-col-span-2-mobile";
    let Some(anchor) = tile.find(ANCHOR) else {
        return tile.to_string();
    };
    let insert_at = anchor + ANCHOR.len();
    format!(
        "{} is-row-span-{rows}-tablet{}",
        &tile[..insert_at],
        &tile[insert_at..]
    )
}

pub(crate) fn default_js_links() -> Vec<String> {
    vec![String::from("/js/vados.js")]
}

pub(crate) fn vados_js() -> &'static str {
    include_str!("../assets/js/vados.js")
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
    /// A child tile's image; `eager` for the first row, which is above the
    /// fold and often the largest thing a listing page paints.
    Sub {
        eager: bool,
    },
    Side,
}

impl ImageType {
    pub(crate) fn sizes(&self) -> &'static str {
        match self {
            ImageType::Main => "(min-width: 1408px) 986px, (min-width: 769px) calc(75vw - 94px), calc(100vw - 64px)",
            ImageType::Sub { .. } => "(min-width: 1408px) 425px, (min-width: 769px) calc(37.5vw - 106px), calc(100vw - 112px)",
            ImageType::Side => "(min-width: 1408px) 318px, (min-width: 769px) calc(25vw - 94px), calc(100vw - 112px)",
        }
    }
    pub(crate) fn decoding(&self) -> &'static str {
        match self {
            ImageType::Main => "sync",
            ImageType::Sub { .. } => "async",
            ImageType::Side => "async",
        }
    }
    pub(crate) fn loading(&self) -> &'static str {
        match self {
            ImageType::Main | ImageType::Sub { eager: true } => "eager",
            ImageType::Sub { eager: false } | ImageType::Side => "lazy",
        }
    }
    /// Only the hero image jumps the queue: it's what the page is about, and
    /// usually the largest thing it paints.
    pub(crate) fn fetch_priority(&self) -> Option<&'static str> {
        match self {
            ImageType::Main => Some("high"),
            ImageType::Sub { .. } | ImageType::Side => None,
        }
    }
}
