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
    /// Names a dropdown's button, followed by the parent's title.
    pub(crate) subpages_of: &'static str,
    /// Ends a capped dropdown, followed by the parent's title.
    pub(crate) menu_more: &'static str,
    pub(crate) search_title: &'static str,
    pub(crate) search_intro: &'static str,
    pub(crate) search_label: &'static str,
    pub(crate) search_button: &'static str,
    pub(crate) search_needs_script: &'static str,
    pub(crate) search_searching: &'static str,
    pub(crate) search_none: &'static str,
    pub(crate) search_one: &'static str,
    /// `{n}` is replaced by the number of results.
    pub(crate) search_many: &'static str,
    pub(crate) search_failed: &'static str,
    pub(crate) event_when: &'static str,
    pub(crate) event_attendance: &'static str,
    pub(crate) event_venue: &'static str,
    pub(crate) event_address: &'static str,
    pub(crate) event_organiser: &'static str,
    pub(crate) event_cost: &'static str,
    pub(crate) event_not_confirmed: &'static str,
    pub(crate) event_online: &'static str,
    pub(crate) event_in_person: &'static str,
    pub(crate) event_mixed: &'static str,
    pub(crate) event_cancelled: &'static str,
    pub(crate) event_postponed: &'static str,
    pub(crate) event_rescheduled: &'static str,
    pub(crate) event_register: &'static str,
    pub(crate) event_join_online: &'static str,
    pub(crate) event_add_to_calendar: &'static str,
    pub(crate) agenda_show_past: &'static str,
    pub(crate) agenda_hide_past: &'static str,
    pub(crate) agenda_none_upcoming: &'static str,
    pub(crate) agenda_subscribe: &'static str,
    /// What a Luma register button says when page.json gives no label.
    pub(crate) luma_register: &'static str,
    /// The link to a section's Atom feed, on the section's own page.
    pub(crate) subscribe_feed: &'static str,
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
    subpages_of: "Subpages of",
    menu_more: "More in",
    search_title: "Search",
    search_intro: "Search all pages of this site.",
    search_label: "Search terms",
    search_button: "Search",
    search_needs_script:
        "Searching this site needs JavaScript, which is turned off or blocked in your browser.",
    search_searching: "Searching…",
    search_none: "Nothing found.",
    search_one: "1 result",
    search_many: "{n} results",
    search_failed: "The search could not be loaded. Please try again.",
    event_when: "When",
    event_attendance: "Attendance",
    event_venue: "Where",
    event_address: "Address",
    event_organiser: "Organiser",
    event_cost: "Cost",
    event_not_confirmed: "Not confirmed",
    event_online: "Online",
    event_in_person: "In person",
    event_mixed: "In person and online",
    event_cancelled: "Cancelled",
    event_postponed: "Postponed",
    event_rescheduled: "Rescheduled",
    event_register: "Register",
    event_join_online: "Join online",
    event_add_to_calendar: "Add to calendar",
    agenda_show_past: "Show past events",
    agenda_hide_past: "Hide past events",
    agenda_none_upcoming: "No upcoming events.",
    agenda_subscribe: "Subscribe to this calendar",
    luma_register: "Register",
    subscribe_feed: "Subscribe (Atom feed)",
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
    subpages_of: "Subpagina's van",
    menu_more: "Meer in",
    search_title: "Zoeken",
    search_intro: "Doorzoek alle pagina's van deze site.",
    search_label: "Zoektermen",
    search_button: "Zoeken",
    search_needs_script:
        "Zoeken op deze site werkt met JavaScript, dat in je browser uit staat of geblokkeerd is.",
    search_searching: "Bezig met zoeken…",
    search_none: "Niets gevonden.",
    search_one: "1 resultaat",
    search_many: "{n} resultaten",
    search_failed: "De zoekfunctie kon niet worden geladen. Probeer het opnieuw.",
    event_when: "Wanneer",
    event_attendance: "Deelname",
    event_venue: "Waar",
    event_address: "Adres",
    event_organiser: "Organisator",
    event_cost: "Kosten",
    event_not_confirmed: "Niet bevestigd",
    event_online: "Online",
    event_in_person: "Op locatie",
    event_mixed: "Op locatie en online",
    event_cancelled: "Afgelast",
    event_postponed: "Uitgesteld",
    event_rescheduled: "Verplaatst",
    event_register: "Aanmelden",
    event_join_online: "Online deelnemen",
    event_add_to_calendar: "Toevoegen aan agenda",
    agenda_show_past: "Eerdere activiteiten tonen",
    agenda_hide_past: "Eerdere activiteiten verbergen",
    agenda_none_upcoming: "Geen komende activiteiten.",
    agenda_subscribe: "Abonneren op deze agenda",
    luma_register: "Aanmelden",
    subscribe_feed: "Abonneren (Atom-feed)",
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
    subpages_of: "Unterseiten von",
    menu_more: "Mehr in",
    search_title: "Suche",
    search_intro: "Alle Seiten dieser Website durchsuchen.",
    search_label: "Suchbegriffe",
    search_button: "Suchen",
    search_needs_script: "Die Suche auf dieser Website benötigt JavaScript, das in Ihrem Browser deaktiviert oder blockiert ist.",
    search_searching: "Suche läuft…",
    search_none: "Nichts gefunden.",
    search_one: "1 Ergebnis",
    search_many: "{n} Ergebnisse",
    search_failed: "Die Suche konnte nicht geladen werden. Bitte versuchen Sie es erneut.",
    event_when: "Wann",
    event_attendance: "Teilnahme",
    event_venue: "Wo",
    event_address: "Adresse",
    event_organiser: "Veranstalter",
    event_cost: "Kosten",
    event_not_confirmed: "Nicht bestätigt",
    event_online: "Online",
    event_in_person: "Vor Ort",
    event_mixed: "Vor Ort und online",
    event_cancelled: "Abgesagt",
    event_postponed: "Verschoben",
    event_rescheduled: "Neu angesetzt",
    event_register: "Anmelden",
    event_join_online: "Online teilnehmen",
    event_add_to_calendar: "Zum Kalender hinzufügen",
    agenda_show_past: "Vergangene Termine anzeigen",
    agenda_hide_past: "Vergangene Termine ausblenden",
    agenda_none_upcoming: "Keine anstehenden Termine.",
    agenda_subscribe: "Diesen Kalender abonnieren",
    luma_register: "Anmelden",
    subscribe_feed: "Abonnieren (Atom-Feed)",
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
    subpages_of: "Sous-pages de",
    menu_more: "Plus dans",
    search_title: "Recherche",
    search_intro: "Rechercher dans toutes les pages de ce site.",
    search_label: "Termes de recherche",
    search_button: "Rechercher",
    search_needs_script:
        "La recherche sur ce site nécessite JavaScript, désactivé ou bloqué dans votre navigateur.",
    search_searching: "Recherche en cours…",
    search_none: "Aucun résultat.",
    search_one: "1 résultat",
    search_many: "{n} résultats",
    search_failed: "La recherche n'a pas pu être chargée. Veuillez réessayer.",
    event_when: "Quand",
    event_attendance: "Participation",
    event_venue: "Où",
    event_address: "Adresse",
    event_organiser: "Organisateur",
    event_cost: "Coût",
    event_not_confirmed: "Non confirmé",
    event_online: "En ligne",
    event_in_person: "Sur place",
    event_mixed: "Sur place et en ligne",
    event_cancelled: "Annulé",
    event_postponed: "Reporté",
    event_rescheduled: "Reprogrammé",
    event_register: "S'inscrire",
    event_join_online: "Participer en ligne",
    event_add_to_calendar: "Ajouter au calendrier",
    agenda_show_past: "Afficher les événements passés",
    agenda_hide_past: "Masquer les événements passés",
    agenda_none_upcoming: "Aucun événement à venir.",
    agenda_subscribe: "S'abonner à ce calendrier",
    luma_register: "S'inscrire",
    subscribe_feed: "S'abonner (flux Atom)",
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
    subpages_of: "Subpáginas de",
    menu_more: "Más en",
    search_title: "Buscar",
    search_intro: "Buscar en todas las páginas de este sitio.",
    search_label: "Términos de búsqueda",
    search_button: "Buscar",
    search_needs_script:
        "La búsqueda en este sitio necesita JavaScript, desactivado o bloqueado en tu navegador.",
    search_searching: "Buscando…",
    search_none: "No se ha encontrado nada.",
    search_one: "1 resultado",
    search_many: "{n} resultados",
    search_failed: "No se ha podido cargar la búsqueda. Inténtalo de nuevo.",
    event_when: "Cuándo",
    event_attendance: "Asistencia",
    event_venue: "Dónde",
    event_address: "Dirección",
    event_organiser: "Organización",
    event_cost: "Coste",
    event_not_confirmed: "Sin confirmar",
    event_online: "En línea",
    event_in_person: "Presencial",
    event_mixed: "Presencial y en línea",
    event_cancelled: "Cancelado",
    event_postponed: "Aplazado",
    event_rescheduled: "Reprogramado",
    event_register: "Inscribirse",
    event_join_online: "Participar en línea",
    event_add_to_calendar: "Añadir al calendario",
    agenda_show_past: "Mostrar eventos pasados",
    agenda_hide_past: "Ocultar eventos pasados",
    agenda_none_upcoming: "No hay próximos eventos.",
    agenda_subscribe: "Suscribirse a este calendario",
    luma_register: "Inscribirse",
    subscribe_feed: "Suscribirse (canal Atom)",
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
