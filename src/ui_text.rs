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
}

const EN: UiText = UiText {
    skip_link: "Skip to main content",
    main_navigation: "Main navigation",
    breadcrumbs: "Breadcrumbs",
    toggle_main_menu: "Toggle main menu",
    toggle_section_menu: "Toggle section menu",
    luma_register: "Register",
};

const NL: UiText = UiText {
    skip_link: "Naar de hoofdinhoud",
    main_navigation: "Hoofdnavigatie",
    breadcrumbs: "Kruimelpad",
    toggle_main_menu: "Hoofdmenu openen of sluiten",
    toggle_section_menu: "Sectiemenu openen of sluiten",
    luma_register: "Aanmelden",
};

const DE: UiText = UiText {
    skip_link: "Zum Hauptinhalt springen",
    main_navigation: "Hauptnavigation",
    breadcrumbs: "Brotkrümelnavigation",
    toggle_main_menu: "Hauptmenü ein- oder ausblenden",
    toggle_section_menu: "Bereichsmenü ein- oder ausblenden",
    luma_register: "Anmelden",
};

const FR: UiText = UiText {
    skip_link: "Aller au contenu principal",
    main_navigation: "Navigation principale",
    breadcrumbs: "Fil d'Ariane",
    toggle_main_menu: "Afficher ou masquer le menu principal",
    toggle_section_menu: "Afficher ou masquer le menu de la section",
    luma_register: "S'inscrire",
};

const ES: UiText = UiText {
    skip_link: "Saltar al contenido principal",
    main_navigation: "Navegación principal",
    breadcrumbs: "Ruta de navegación",
    toggle_main_menu: "Mostrar u ocultar el menú principal",
    toggle_section_menu: "Mostrar u ocultar el menú de la sección",
    luma_register: "Inscribirse",
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
    fn untranslated_language_falls_back_to_english() {
        assert!(!is_translated("pl"));
        assert_eq!(ui_text("pl").skip_link, EN.skip_link);
        assert!(is_translated("en-GB"));
    }
}
