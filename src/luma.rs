//! Luma events: an event hosted on Luma, linked from a page's page.json and
//! shown in that page's main area. See `vados.allium`'s `LumaEvent`.
//!
//! Every address Luma serves lives here and nowhere else. The spec
//! deliberately leaves them out of the domain (they are Luma's to change,
//! and already moved once from lu.ma to luma.com), so when Luma moves them
//! again this is the one file to touch.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Luma's embeddable event page; `{id}` is the event ID.
const EMBED_URL: &str = "https://luma.com/embed/event/{id}/simple";

/// An event's own public page on Luma; `{id}` is the event ID. Used for the
/// plain link that is always shown, and as the register button's `href` so
/// the button still works without Luma's script.
const EVENT_URL: &str = "https://luma.com/event/{id}";

/// Luma's register-button script. It turns every
/// `a.luma-checkout--button` on the page into a button that opens Luma's
/// registration over the page.
pub(crate) const BUTTON_SCRIPT_URL: &str = "https://embed.lu.ma/checkout-button.js";

/// How a Luma event is shown. See `vados.allium`'s `LumaEventDisplay`.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Copy, Default)]
#[serde(rename_all = "camelCase")]
pub enum LumaEventDisplay {
    /// The default: the embedded event page already carries Luma's own
    /// registration, so a button is opted into rather than out of. See
    /// `config.default_luma_event_display`.
    #[default]
    EventPage,
    RegisterButton,
    Both,
}

impl LumaEventDisplay {
    pub(crate) fn shows_event_page(self) -> bool {
        matches!(self, LumaEventDisplay::EventPage | LumaEventDisplay::Both)
    }
    pub(crate) fn shows_register_button(self) -> bool {
        matches!(
            self,
            LumaEventDisplay::RegisterButton | LumaEventDisplay::Both
        )
    }
}

impl fmt::Display for LumaEventDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LumaEventDisplay::EventPage => "event page",
            LumaEventDisplay::RegisterButton => "register button",
            LumaEventDisplay::Both => "event page and register button",
        })
    }
}

/// The label a register button carries when page.json gives none. See
/// `config.default_luma_button_label`.
pub(crate) const DEFAULT_BUTTON_LABEL: &str = "Register";

/// One Luma event as page.json declares it. See `vados.allium`'s
/// `LumaEvent`; `title` is optional only so its absence can be reported.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LumaEvent {
    pub(crate) event_id: String,
    pub(crate) title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) display: Option<LumaEventDisplay>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) button_label: Option<String>,
}

impl LumaEvent {
    /// The title, if it has one worth showing: a blank title names nothing,
    /// so it counts as missing the same way blank alt text does.
    pub(crate) fn effective_title(&self) -> Option<&str> {
        self.title
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
    }
    pub(crate) fn display(&self) -> LumaEventDisplay {
        self.display.unwrap_or_default()
    }
    pub(crate) fn button_label(&self) -> &str {
        self.button_label
            .as_deref()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .unwrap_or(DEFAULT_BUTTON_LABEL)
    }
    pub(crate) fn embed_url(&self) -> String {
        EMBED_URL.replace("{id}", &self.event_id)
    }
    pub(crate) fn event_url(&self) -> String {
        EVENT_URL.replace("{id}", &self.event_id)
    }
}

/// Whether `id` is shaped like a Luma event ID: `evt-` followed by Luma's
/// identifier characters. Purely structural -- whether such an event exists
/// on Luma is never asked. See `vados.allium`'s `is_luma_event_id`.
pub fn is_luma_event_id(id: &str) -> bool {
    match id.strip_prefix("evt-") {
        Some(rest) => !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric()),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, title: Option<&str>) -> LumaEvent {
        LumaEvent {
            event_id: String::from(id),
            title: title.map(String::from),
            display: None,
            button_label: None,
        }
    }

    #[test]
    fn a_luma_event_id_is_evt_followed_by_alphanumerics() {
        assert!(is_luma_event_id("evt-AbC123xyz"));
        assert!(!is_luma_event_id("evt-"));
        assert!(!is_luma_event_id("AbC123"));
        assert!(!is_luma_event_id("cal-AbC123"));
        assert!(!is_luma_event_id("evt-AbC 123"));
        assert!(!is_luma_event_id("https://luma.com/event/evt-AbC123"));
        assert!(!is_luma_event_id("evt-\"><script>"));
    }

    #[test]
    fn display_defaults_to_the_event_page_only() {
        let e = event("evt-a1", Some("Jam"));
        assert_eq!(e.display(), LumaEventDisplay::EventPage);
        assert!(e.display().shows_event_page());
        assert!(!e.display().shows_register_button());
        assert!(LumaEventDisplay::Both.shows_event_page());
        assert!(LumaEventDisplay::Both.shows_register_button());
        assert!(!LumaEventDisplay::RegisterButton.shows_event_page());
    }

    #[test]
    fn a_blank_title_counts_as_missing() {
        assert_eq!(
            event("evt-a1", Some(" Jam ")).effective_title(),
            Some("Jam")
        );
        assert_eq!(event("evt-a1", Some("  ")).effective_title(), None);
        assert_eq!(event("evt-a1", None).effective_title(), None);
    }

    #[test]
    fn button_label_defaults_when_blank_or_missing() {
        let mut e = event("evt-a1", Some("Jam"));
        assert_eq!(e.button_label(), "Register");
        e.button_label = Some(String::from(" "));
        assert_eq!(e.button_label(), "Register");
        e.button_label = Some(String::from("Sign up"));
        assert_eq!(e.button_label(), "Sign up");
    }

    #[test]
    fn display_round_trips_through_page_json_in_camel_case() {
        let json = r#"{"eventId":"evt-a1","title":"Jam","display":"registerButton"}"#;
        let e: LumaEvent = serde_json::from_str(json).unwrap();
        assert_eq!(e.display, Some(LumaEventDisplay::RegisterButton));
        assert_eq!(serde_json::to_string(&e).unwrap(), json);
    }
}
