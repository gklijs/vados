use crate::page_date::PageDate;

/// The text vados itself puts on every page -- as opposed to the text the
/// maintainer authors -- in the site's language. See `vados.allium`'s
/// `InterfaceTextFollowsSiteLanguage`.
pub(crate) struct UiText {
    pub(crate) skip_link: &'static str,
    pub(crate) main_navigation: &'static str,
    pub(crate) breadcrumbs: &'static str,
    pub(crate) toggle_main_menu: &'static str,
    pub(crate) toggle_section_menu: &'static str,
    /// What a Luma register button says when page.json gives no label.
    pub(crate) luma_register: &'static str,
    /// Month names, January first, as they appear inside a written date.
    months: [&'static str; 12],
    /// Writes a date from its day, month name and year.
    date_pattern: fn(u8, &str, u16) -> String,
}

impl UiText {
    /// A page's date as a reader of the site's language would write it. The
    /// time of day, when there is one, only breaks ties in ordering and is
    /// not shown. See `vados.allium`'s `DatesAreReadable`.
    pub(crate) fn format_date(&self, date: &PageDate) -> String {
        let month = self.months[usize::from(date.month() - 1)];
        (self.date_pattern)(date.day(), month, date.year())
    }
}

const EN: UiText = UiText {
    skip_link: "Skip to main content",
    main_navigation: "Main navigation",
    breadcrumbs: "Breadcrumbs",
    toggle_main_menu: "Toggle main menu",
    toggle_section_menu: "Toggle section menu",
    luma_register: "Register",
    months: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
    date_pattern: |d, m, y| format!("{d} {m} {y}"),
};

const NL: UiText = UiText {
    skip_link: "Naar de hoofdinhoud",
    main_navigation: "Hoofdnavigatie",
    breadcrumbs: "Kruimelpad",
    toggle_main_menu: "Hoofdmenu openen of sluiten",
    toggle_section_menu: "Sectiemenu openen of sluiten",
    luma_register: "Aanmelden",
    months: [
        "januari",
        "februari",
        "maart",
        "april",
        "mei",
        "juni",
        "juli",
        "augustus",
        "september",
        "oktober",
        "november",
        "december",
    ],
    date_pattern: |d, m, y| format!("{d} {m} {y}"),
};

const DE: UiText = UiText {
    skip_link: "Zum Hauptinhalt springen",
    main_navigation: "Hauptnavigation",
    breadcrumbs: "Brotkrümelnavigation",
    toggle_main_menu: "Hauptmenü ein- oder ausblenden",
    toggle_section_menu: "Bereichsmenü ein- oder ausblenden",
    luma_register: "Anmelden",
    months: [
        "Januar",
        "Februar",
        "März",
        "April",
        "Mai",
        "Juni",
        "Juli",
        "August",
        "September",
        "Oktober",
        "November",
        "Dezember",
    ],
    date_pattern: |d, m, y| format!("{d}. {m} {y}"),
};

const FR: UiText = UiText {
    skip_link: "Aller au contenu principal",
    main_navigation: "Navigation principale",
    breadcrumbs: "Fil d'Ariane",
    toggle_main_menu: "Afficher ou masquer le menu principal",
    toggle_section_menu: "Afficher ou masquer le menu de la section",
    luma_register: "S'inscrire",
    months: [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ],
    date_pattern: |d, m, y| match d {
        1 => format!("1er {m} {y}"),
        _ => format!("{d} {m} {y}"),
    },
};

const ES: UiText = UiText {
    skip_link: "Saltar al contenido principal",
    main_navigation: "Navegación principal",
    breadcrumbs: "Ruta de navegación",
    toggle_main_menu: "Mostrar u ocultar el menú principal",
    toggle_section_menu: "Mostrar u ocultar el menú de la sección",
    luma_register: "Inscribirse",
    months: [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ],
    date_pattern: |d, m, y| format!("{d} de {m} de {y}"),
};

/// The interface text for a BCP 47 tag, chosen by its primary language
/// subtag alone (`nl-BE` gets the Dutch text), or `None` when vados has no
/// translation for that language.
fn translation(language: &str) -> Option<&'static UiText> {
    let primary = language.split('-').next().unwrap_or_default();
    match primary.to_ascii_lowercase().as_str() {
        "en" => Some(&EN),
        "nl" => Some(&NL),
        "de" => Some(&DE),
        "fr" => Some(&FR),
        "es" => Some(&ES),
        _ => None,
    }
}

/// Whether vados can write its interface text in `language`. `check` warns
/// when it cannot, since the site then falls back to English.
pub(crate) fn is_translated(language: &str) -> bool {
    translation(language).is_some()
}

/// The interface text for `language`, falling back to English.
pub(crate) fn ui_text(language: &str) -> &'static UiText {
    translation(language).unwrap_or(&EN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_subtags_and_case_are_ignored() {
        assert_eq!(ui_text("nl").skip_link, NL.skip_link);
        assert_eq!(ui_text("nl-BE").skip_link, NL.skip_link);
        assert_eq!(ui_text("NL").skip_link, NL.skip_link);
        assert_eq!(ui_text("fr-CA").skip_link, FR.skip_link);
    }

    #[test]
    fn dates_are_written_the_way_each_language_writes_them() {
        let date: PageDate = "2026-10-22T09:30".parse().unwrap();
        assert_eq!(ui_text("en").format_date(&date), "22 October 2026");
        assert_eq!(ui_text("nl").format_date(&date), "22 oktober 2026");
        assert_eq!(ui_text("de").format_date(&date), "22. Oktober 2026");
        assert_eq!(ui_text("fr").format_date(&date), "22 octobre 2026");
        assert_eq!(ui_text("es").format_date(&date), "22 de octubre de 2026");
        let first: PageDate = "2026-03-01".parse().unwrap();
        assert_eq!(ui_text("fr").format_date(&first), "1er mars 2026");
    }

    #[test]
    fn untranslated_language_falls_back_to_english() {
        assert!(!is_translated("pl"));
        assert_eq!(ui_text("pl").skip_link, EN.skip_link);
        assert!(is_translated("en-GB"));
    }
}
