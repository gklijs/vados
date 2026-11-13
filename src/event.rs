//! An event as a page: validated details in page.json's `event`, shown in the
//! page's main area and described to search engines and calendars. See
//! `vados.allium`'s `Event`.
//!
//! Nothing is ever invented. A detail page.json doesn't give is shown as "not
//! confirmed" to readers and left out of structured data and calendar files.

use crate::config_files::MainConfig;
use crate::feed::page_url;
use crate::files::write_raw;
use crate::page_date::PageDate;
use crate::structure::{Item, Structure};
use crate::ui_text::UiText;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// How people attend: `online`, `inPerson` or `mixed`.
// See `vados.allium`'s `EventAttendance`.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum EventAttendance {
    Online,
    InPerson,
    Mixed,
}

/// Whether the event goes ahead: `scheduled`, `cancelled`, `postponed` or
/// `rescheduled`.
// See `vados.allium`'s `EventStatus`.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum EventStatus {
    Scheduled,
    Cancelled,
    Postponed,
    Rescheduled,
}

/// The pattern of an event's `start` and `end`, for the published schema: a
/// date and time with a UTC offset, the forms `jiff::Timestamp` reads.
#[cfg(test)]
pub(crate) const MOMENT_PATTERN: &str = r"^\d{4}-\d{2}-\d{2}[Tt ]\d{2}:\d{2}(:\d{2}([.,]\d{1,9})?)?([Zz]|[+-]\d{2}(:?\d{2}(:?\d{2})?)?)$";

/// Makes the page an event. A detail left out is shown as "not confirmed",
/// never guessed.
// page.json's `event`, as written.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(crate) struct EventConfig {
    /// When it starts, with the UTC offset `timeZone` shows at that moment:
    /// `2026-11-05T14:00:00-05:00`.
    #[cfg_attr(test, schemars(pattern(MOMENT_PATTERN)))]
    pub(crate) start: String,
    /// When it ends, in the same form as `start`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, schemars(pattern(MOMENT_PATTERN)))]
    pub(crate) end: Option<String>,
    /// The IANA time zone it happens in, such as `America/New_York`.
    pub(crate) time_zone: String,
    /// How people attend.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) attendance: Option<EventAttendance>,
    /// Whether it goes ahead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<EventStatus>,
    /// Who organises it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) organiser: Option<String>,
    /// Where it happens: the venue's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) venue: Option<String>,
    /// The venue's address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) address: Option<String>,
    /// What it costs, as the organiser states it. Leaving it out means
    /// unknown, never free.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) cost: Option<String>,
    /// Where to register, as an http(s) URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) registration_url: Option<String>,
    /// Where to attend online, as an http(s) URL. Never taken from
    /// `registrationUrl`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) virtual_url: Option<String>,
    /// Raise it whenever the event changes, so calendar subscribers take the
    /// change. `vados page set-event` does so for you.
    // A static build keeps no state to work this out by itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) sequence: Option<u32>,
}

/// page.json's `event`, whether or not it could be read as one. Reading it apart
/// from the rest of page.json means a misspelt status costs the event, reported
/// by `check` and left off the page, rather than the whole build.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(untagged)]
pub(crate) enum DeclaredEvent {
    Readable(EventConfig),
    Unreadable(Value),
}

impl DeclaredEvent {
    pub(crate) fn resolve(&self) -> Result<Event, Vec<String>> {
        match self {
            DeclaredEvent::Readable(config) => config.resolve(),
            DeclaredEvent::Unreadable(value) => Err(vec![
                match serde_json::from_value::<EventConfig>(value.clone()) {
                    Ok(_) => String::from("it could not be read"),
                    Err(e) => e.to_string(),
                },
            ]),
        }
    }
    pub(crate) fn readable(&self) -> Option<&EventConfig> {
        match self {
            DeclaredEvent::Readable(config) => Some(config),
            DeclaredEvent::Unreadable(_) => None,
        }
    }
}

/// A validated event: only these reach a page.
#[derive(Debug, Clone)]
pub(crate) struct Event {
    pub(crate) config: EventConfig,
    pub(crate) start: Timestamp,
    pub(crate) end: Option<Timestamp>,
    time_zone: TimeZone,
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.config == other.config
    }
}

impl Eq for Event {}

/// A moment as page.json gives it, checked against the event's zone. The wall
/// time and offset must be what that zone really shows at that instant, which
/// rejects a wrong summer or winter offset and a time that a clock change
/// skips, while either offset of a repeated autumn hour stays valid.
fn read_moment(field: &str, text: &str, zone: Option<&TimeZone>) -> Result<Timestamp, String> {
    let text = text.trim();
    let moment: Timestamp = text.parse().map_err(|_| {
        format!("{field} `{text}` is not a date and time with a UTC offset, like 2026-11-05T14:00:00-05:00")
    })?;
    let Some(zone) = zone else {
        return Ok(moment);
    };
    let wall: DateTime = text
        .strip_suffix(['Z', 'z'])
        .unwrap_or(text)
        .parse()
        .map_err(|_| format!("{field} `{text}` could not be read as a date and time"))?;
    if moment.to_zoned(zone.clone()).datetime() != wall {
        return Err(format!(
            "{field} `{text}` is not a time that {} shows at that moment: check the UTC offset for that date",
            zone.iana_name().unwrap_or("the event's time zone")
        ));
    }
    Ok(moment)
}

/// An absolute http(s) address with a host, free of anything that could break
/// out of where it's written.
fn is_http_url(url: &str) -> bool {
    let url = url.trim();
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    rest.is_some_and(|rest| {
        !rest
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .is_empty()
    }) && !url.contains(|c: char| c.is_whitespace() || c.is_control())
}

impl EventConfig {
    /// Validates the whole event, naming everything wrong with it at once.
    pub(crate) fn resolve(&self) -> Result<Event, Vec<String>> {
        let mut problems = vec![];
        let zone = match TimeZone::get(self.time_zone.trim()) {
            Ok(zone) => Some(zone),
            Err(_) => {
                problems.push(format!(
                    "timeZone `{}` is not an IANA time zone, like Europe/Amsterdam",
                    self.time_zone
                ));
                None
            }
        };
        let start = read_moment("start", &self.start, zone.as_ref())
            .map_err(|e| problems.push(e))
            .ok();
        let end = self.end.as_deref().and_then(|end| {
            read_moment("end", end, zone.as_ref())
                .map_err(|e| problems.push(e))
                .ok()
        });
        if let (Some(start), Some(end)) = (start, end) {
            if end <= start {
                problems.push(String::from("end is not after start"));
            }
        }
        for (field, url) in [
            ("registrationUrl", &self.registration_url),
            ("virtualUrl", &self.virtual_url),
        ] {
            if let Some(url) = url.as_deref().filter(|u| !is_http_url(u)) {
                problems.push(format!("{field} `{url}` is not an absolute http(s) URL"));
            }
        }
        match (start, zone) {
            (Some(start), Some(time_zone)) if problems.is_empty() => Ok(Event {
                config: self.clone(),
                start,
                end,
                time_zone,
            }),
            _ => Err(problems),
        }
    }
}

/// A given detail, unless it's blank.
fn known(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty())
}

impl Event {
    fn zoned(&self, moment: Timestamp) -> Zoned {
        moment.to_zoned(self.time_zone.clone())
    }

    /// When the event is over, as far as anyone knows: its end, or its start.
    pub(crate) fn last_moment(&self) -> Timestamp {
        self.end.unwrap_or(self.start)
    }

    fn written_date(&self, moment: &Zoned, ui: &UiText) -> String {
        moment
            .strftime("%Y-%m-%d")
            .to_string()
            .parse::<PageDate>()
            .map(|d| ui.format_date(&d))
            .unwrap_or_default()
    }

    /// The event's time as a reader at the event would read it, in its own
    /// zone: `5 November 2026, 14:00–15:30 (America/New_York)`.
    pub(crate) fn written_time(&self, ui: &UiText) -> String {
        let start = self.zoned(self.start);
        let date = self.written_date(&start, ui);
        let start_time = start.strftime("%H:%M");
        let zone = self.config.time_zone.trim();
        match self.end.map(|e| self.zoned(e)) {
            None => format!("{date}, {start_time} ({zone})"),
            Some(end) if end.date() == start.date() => {
                format!("{date}, {start_time}–{} ({zone})", end.strftime("%H:%M"))
            }
            Some(end) => format!(
                "{date}, {start_time} – {}, {} ({zone})",
                self.written_date(&end, ui),
                end.strftime("%H:%M")
            ),
        }
    }

    pub(crate) fn status_text(&self, ui: &UiText) -> Option<&'static str> {
        match self.config.status? {
            EventStatus::Scheduled => None,
            EventStatus::Cancelled => Some(ui.event_cancelled),
            EventStatus::Postponed => Some(ui.event_postponed),
            EventStatus::Rescheduled => Some(ui.event_rescheduled),
        }
    }

    /// The label/value rows of the details, unknown values said to be so.
    pub(crate) fn detail_rows(&self, ui: &UiText) -> Vec<(&'static str, String)> {
        let or_unknown = |value: &Option<String>| {
            known(value)
                .map(String::from)
                .unwrap_or_else(|| String::from(ui.event_not_confirmed))
        };
        let attendance = match self.config.attendance {
            None => ui.event_not_confirmed,
            Some(EventAttendance::Online) => ui.event_online,
            Some(EventAttendance::InPerson) => ui.event_in_person,
            Some(EventAttendance::Mixed) => ui.event_mixed,
        };
        let mut rows = vec![(ui.event_attendance, String::from(attendance))];
        rows.push((ui.event_venue, or_unknown(&self.config.venue)));
        if let Some(address) = known(&self.config.address) {
            rows.push((ui.event_address, String::from(address)));
        }
        rows.push((ui.event_organiser, or_unknown(&self.config.organiser)));
        rows.push((ui.event_cost, or_unknown(&self.config.cost)));
        rows
    }

    pub(crate) fn registration_url(&self) -> Option<&str> {
        known(&self.config.registration_url)
    }

    pub(crate) fn virtual_url(&self) -> Option<&str> {
        known(&self.config.virtual_url)
    }

    /// schema.org's `Event`, with only what page.json actually says.
    pub(crate) fn to_structured_data(
        &self,
        item: &Item,
        url: Option<&str>,
        description: Option<&str>,
        image_url: Option<&str>,
    ) -> Value {
        let mut event = Map::new();
        event.insert("@context".into(), json!("https://schema.org"));
        event.insert("@type".into(), json!("Event"));
        if let Some(url) = url {
            // Stable through a reschedule: the page's address, not its date.
            event.insert("@id".into(), json!(format!("{url}#event")));
            event.insert("url".into(), json!(url));
        }
        event.insert("name".into(), json!(item.title));
        event.insert("startDate".into(), json!(self.config.start.trim()));
        if let Some(end) = known(&self.config.end) {
            event.insert("endDate".into(), json!(end));
        }
        if let Some(description) = description {
            event.insert("description".into(), json!(description));
        }
        if let Some(image) = image_url {
            event.insert("image".into(), json!([image]));
        }
        if let Some(status) = self.config.status {
            let status = match status {
                EventStatus::Scheduled => "EventScheduled",
                EventStatus::Cancelled => "EventCancelled",
                EventStatus::Postponed => "EventPostponed",
                EventStatus::Rescheduled => "EventRescheduled",
            };
            event.insert(
                "eventStatus".into(),
                json!(format!("https://schema.org/{status}")),
            );
        }
        if let Some(attendance) = self.config.attendance {
            let mode = match attendance {
                EventAttendance::Online => "OnlineEventAttendanceMode",
                EventAttendance::InPerson => "OfflineEventAttendanceMode",
                EventAttendance::Mixed => "MixedEventAttendanceMode",
            };
            event.insert(
                "eventAttendanceMode".into(),
                json!(format!("https://schema.org/{mode}")),
            );
        }
        let mut locations = vec![];
        let venue = known(&self.config.venue);
        let address = known(&self.config.address);
        if venue.is_some() || address.is_some() {
            let mut place = Map::new();
            place.insert("@type".into(), json!("Place"));
            if let Some(venue) = venue {
                place.insert("name".into(), json!(venue));
            }
            if let Some(address) = address {
                place.insert("address".into(), json!(address));
            }
            locations.push(Value::Object(place));
        }
        if let Some(url) = self.virtual_url() {
            locations.push(json!({ "@type": "VirtualLocation", "url": url }));
        }
        match locations.len() {
            0 => {}
            1 => {
                event.insert("location".into(), locations.remove(0));
            }
            _ => {
                event.insert("location".into(), Value::Array(locations));
            }
        }
        if let Some(organiser) = known(&self.config.organiser) {
            event.insert(
                "organizer".into(),
                json!({ "@type": "Organization", "name": organiser }),
            );
        }
        // No price: `cost` is free text, and a number would be a guess.
        if let Some(url) = self.registration_url() {
            let mut offer = Map::new();
            offer.insert("@type".into(), json!("Offer"));
            offer.insert("url".into(), json!(url));
            if let Some(cost) = known(&self.config.cost) {
                offer.insert("description".into(), json!(cost));
            }
            event.insert("offers".into(), Value::Object(offer));
        }
        Value::Object(event)
    }

    /// The event's `VEVENT` lines, unfolded.
    fn to_ics_lines(&self, entry: &CalendarEntry) -> Vec<String> {
        let stamp = entry
            .modified
            .map(|d| d.to_rfc3339().parse::<Timestamp>().unwrap_or(self.start))
            .unwrap_or(self.start);
        let mut lines = vec![
            String::from("BEGIN:VEVENT"),
            format!("UID:{}", ics_text(&entry.uid)),
            format!("DTSTAMP:{}", ics_time(stamp)),
            format!("SEQUENCE:{}", self.config.sequence.unwrap_or(0)),
            format!("DTSTART:{}", ics_time(self.start)),
        ];
        if let Some(end) = self.end {
            lines.push(format!("DTEND:{}", ics_time(end)));
        }
        lines.push(format!("SUMMARY:{}", ics_text(&entry.title)));
        if let Some(summary) = entry.summary.as_deref() {
            lines.push(format!("DESCRIPTION:{}", ics_text(summary)));
        }
        let location: Vec<&str> = [known(&self.config.venue), known(&self.config.address)]
            .into_iter()
            .flatten()
            .collect();
        if !location.is_empty() {
            lines.push(format!("LOCATION:{}", ics_text(&location.join(", "))));
        }
        lines.push(format!("URL:{}", entry.url));
        match self.config.status {
            Some(EventStatus::Cancelled) => lines.push(String::from("STATUS:CANCELLED")),
            Some(EventStatus::Postponed) => lines.push(String::from("STATUS:TENTATIVE")),
            Some(EventStatus::Scheduled | EventStatus::Rescheduled) => {
                lines.push(String::from("STATUS:CONFIRMED"))
            }
            None => {}
        }
        lines.push(String::from("END:VEVENT"));
        lines
    }
}

/// One event in a calendar file.
pub(crate) struct CalendarEntry<'a> {
    pub(crate) event: &'a Event,
    pub(crate) title: String,
    pub(crate) summary: Option<String>,
    pub(crate) url: String,
    /// Stable through a reschedule: an address, not a time.
    pub(crate) uid: String,
    pub(crate) modified: Option<PageDate>,
}

impl<'a> CalendarEntry<'a> {
    /// An event page's entry.
    pub(crate) fn of_page(event: &'a Event, item: &Item, site_url: &str) -> CalendarEntry<'a> {
        let url = page_url(site_url, &item.path);
        CalendarEntry {
            event,
            title: item.title.clone(),
            summary: known(&item.summary).map(String::from),
            uid: format!("{url}#event"),
            url,
            modified: item.modified(),
        }
    }
}

/// UTC needs no `VTIMEZONE`, and calendars show it in the reader's own zone.
fn ics_time(moment: Timestamp) -> String {
    moment.strftime("%Y%m%dT%H%M%SZ").to_string()
}

/// Escapes text for an iCalendar value.
fn ics_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace("\r\n", "\\n")
        .replace('\n', "\\n")
        .chars()
        .filter(|c| !c.is_control())
        .collect()
}

/// Folds a line to iCalendar's 75 octets, never inside a character.
fn folded(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + 8);
    let mut used = 0;
    for c in line.chars() {
        if used + c.len_utf8() > 75 {
            out.push_str("\r\n ");
            used = 1;
        }
        out.push(c);
        used += c.len_utf8();
    }
    out.push_str("\r\n");
    out
}

/// Where a single event's calendar file is published. The home page's takes the
/// name of its `index.html`, which no other page can have.
pub(crate) fn event_calendar_href(path: &str) -> String {
    if path == "/" {
        String::from("/index.ics")
    } else {
        format!("{path}.ics")
    }
}

/// Where an agenda's subscribable calendar is published.
pub(crate) fn agenda_calendar_href(path: &str) -> String {
    if path == "/" {
        String::from("/calendar.ics")
    } else {
        format!("{path}/calendar.ics")
    }
}

/// A complete calendar file holding `entries`.
pub(crate) fn calendar_file(name: &str, entries: &[CalendarEntry]) -> String {
    let mut lines = vec![
        String::from("BEGIN:VCALENDAR"),
        String::from("VERSION:2.0"),
        String::from("PRODID:-//vados//EN"),
        String::from("CALSCALE:GREGORIAN"),
        format!("X-WR-CALNAME:{}", ics_text(name)),
    ];
    for entry in entries {
        lines.extend(entry.event.to_ics_lines(entry));
    }
    lines.push(String::from("END:VCALENDAR"));
    lines.iter().map(|l| folded(l)).collect()
}

/// Writes every event's calendar file, and every agenda's. Calendar entries
/// need a full address as their identity, so without a usable `siteUrl` there
/// are none; `check` warns about that.
pub(crate) fn write_calendars(
    destination: &crate::files::Output,
    main_config: &MainConfig,
    structure: &Structure,
) -> Result<(), crate::generator::GenerateError> {
    let Some(site_url) = main_config.get_site_url() else {
        return Ok(());
    };
    let items = structure.get_all_items();
    for item in &items {
        if let Some(event) = &item.event {
            let entry = CalendarEntry::of_page(event, item, site_url);
            let file = calendar_file(&item.title, &[entry]);
            write_raw(destination, &event_calendar_href(&item.path), file)?;
        }
    }
    // After the events, so that an agenda at `/a` beats an event page at
    // `/a/calendar` the same way on every build; `check` warns about that.
    for item in &items {
        if let Some(agenda) = &item.agenda {
            let listed = structure.get_agenda_events(item, agenda);
            let entries: Vec<CalendarEntry> = listed
                .iter()
                .map(|l| l.to_calendar_entry(site_url))
                .collect();
            let name = format!("{} – {}", main_config.site_title, item.title);
            let file = calendar_file(&name, &entries);
            write_raw(destination, &agenda_calendar_href(&item.path), file)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_files::PageConfig;
    use crate::ui_text::ui_text;

    fn config(start: &str, end: Option<&str>, zone: &str) -> EventConfig {
        EventConfig {
            start: start.into(),
            end: end.map(String::from),
            time_zone: zone.into(),
            attendance: None,
            status: None,
            organiser: None,
            venue: None,
            address: None,
            cost: None,
            registration_url: None,
            virtual_url: None,
            sequence: None,
        }
    }

    fn problems(c: EventConfig) -> Vec<String> {
        c.resolve().err().unwrap_or_default()
    }

    #[test]
    fn the_right_offset_for_the_date_is_accepted() {
        let c = config("2026-11-05T14:00:00-05:00", None, "America/New_York");
        assert!(c.resolve().is_ok());
        let c = config("2026-07-05T14:00:00-04:00", None, "America/New_York");
        assert!(c.resolve().is_ok());
    }

    #[test]
    fn a_winter_offset_in_summer_is_rejected() {
        let c = config("2026-07-05T14:00:00-05:00", None, "America/New_York");
        assert!(problems(c)[0].contains("UTC offset"));
    }

    #[test]
    fn a_time_the_spring_clock_change_skips_is_rejected() {
        // Clocks in Amsterdam go from 02:00 to 03:00 on 29 March 2026.
        for offset in ["+01:00", "+02:00"] {
            let c = config(
                &format!("2026-03-29T02:30:00{offset}"),
                None,
                "Europe/Amsterdam",
            );
            assert!(!problems(c).is_empty(), "{offset}");
        }
    }

    #[test]
    fn both_occurrences_of_the_repeated_autumn_hour_are_valid_and_differ() {
        // Clocks in Amsterdam go from 03:00 back to 02:00 on 25 October 2026.
        let first = config("2026-10-25T02:30:00+02:00", None, "Europe/Amsterdam")
            .resolve()
            .unwrap();
        let second = config("2026-10-25T02:30:00+01:00", None, "Europe/Amsterdam")
            .resolve()
            .unwrap();
        assert!(first.start < second.start);
    }

    #[test]
    fn utc_written_with_z_is_checked_like_any_offset() {
        assert!(config("2026-11-05T14:00:00Z", None, "UTC")
            .resolve()
            .is_ok());
        assert!(!problems(config("2026-07-05T14:00:00Z", None, "Europe/London")).is_empty());
    }

    #[test]
    fn everything_wrong_is_named_at_once() {
        let mut c = config("tomorrow", Some("later"), "Mars/Olympus");
        c.registration_url = Some("/register".into());
        assert_eq!(problems(c).len(), 4);
    }

    #[test]
    fn a_missing_offset_is_rejected() {
        let c = config("2026-11-05T14:00:00", None, "Europe/Amsterdam");
        assert!(problems(c)[0].contains("UTC offset"));
    }

    #[test]
    fn an_end_before_the_start_is_rejected() {
        let c = config(
            "2026-11-05T14:00:00+01:00",
            Some("2026-11-05T13:00:00+01:00"),
            "Europe/Amsterdam",
        );
        assert_eq!(problems(c), ["end is not after start"]);
    }

    #[test]
    fn times_are_written_in_the_events_zone_and_language() {
        let ui = ui_text("nl");
        let e = config(
            "2026-11-05T14:00:00+01:00",
            Some("2026-11-05T15:30:00+01:00"),
            "Europe/Amsterdam",
        )
        .resolve()
        .unwrap();
        assert_eq!(
            e.written_time(ui),
            "5 november 2026, 14:00–15:30 (Europe/Amsterdam)"
        );
        let e = config(
            "2026-11-05T22:00:00+01:00",
            Some("2026-11-06T01:00:00+01:00"),
            "Europe/Amsterdam",
        )
        .resolve()
        .unwrap();
        assert_eq!(
            e.written_time(ui),
            "5 november 2026, 22:00 – 6 november 2026, 01:00 (Europe/Amsterdam)"
        );
    }

    fn event_item(c: EventConfig) -> Item {
        let mut page = PageConfig::new("/agenda/jam");
        page.title = String::from("Jam, with; friends");
        page.event = Some(DeclaredEvent::Readable(c));
        Item::new(String::from("/agenda/jam"), page)
    }

    #[test]
    fn unknown_details_are_said_to_be_so_and_never_inferred() {
        let ui = ui_text("en");
        let mut c = config("2026-11-05T14:00:00+01:00", None, "Europe/Amsterdam");
        c.registration_url = Some("https://example.com/r".into());
        let item = event_item(c);
        let event = item.event.as_ref().unwrap();
        let rows = event.detail_rows(ui);
        assert!(rows
            .iter()
            .any(|(l, v)| *l == ui.event_cost && v == ui.event_not_confirmed));
        let data = event.to_structured_data(&item, None, None, None);
        assert_eq!(data["offers"]["url"], "https://example.com/r");
        assert!(data["offers"].get("price").is_none());
        assert!(data.get("location").is_none());
        assert!(data.get("eventStatus").is_none());
        assert!(data.get("eventAttendanceMode").is_none());
    }

    #[test]
    fn structured_data_has_a_stable_identity_and_given_details() {
        let mut c = config("2026-11-05T14:00:00+01:00", None, "Europe/Amsterdam");
        c.attendance = Some(EventAttendance::Mixed);
        c.status = Some(EventStatus::Cancelled);
        c.venue = Some("Hall".into());
        c.virtual_url = Some("https://example.com/live".into());
        let item = event_item(c);
        let data = item.event.as_ref().unwrap().to_structured_data(
            &item,
            Some("https://example.com/agenda/jam"),
            Some("About"),
            None,
        );
        assert_eq!(data["@type"], "Event");
        assert_eq!(data["@id"], "https://example.com/agenda/jam#event");
        assert_eq!(data["startDate"], "2026-11-05T14:00:00+01:00");
        assert_eq!(data["eventStatus"], "https://schema.org/EventCancelled");
        assert_eq!(data["location"][0]["name"], "Hall");
        assert_eq!(data["location"][1]["@type"], "VirtualLocation");
    }

    #[test]
    fn a_calendar_file_is_crlf_folded_escaped_and_in_utc() {
        let mut c = config(
            "2026-11-05T14:00:00-05:00",
            Some("2026-11-05T15:30:00-05:00"),
            "America/New_York",
        );
        c.sequence = Some(3);
        c.status = Some(EventStatus::Cancelled);
        c.organiser = Some("x".repeat(200));
        c.venue = Some("é".repeat(100));
        let item = event_item(c);
        let entry =
            CalendarEntry::of_page(item.event.as_ref().unwrap(), &item, "https://example.com");
        let ics = calendar_file("Agenda", &[entry]);
        assert!(ics.starts_with("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n"));
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
        assert!(ics.contains("UID:https://example.com/agenda/jam#event\r\n"));
        assert!(ics.contains("SEQUENCE:3\r\n"));
        assert!(ics.contains("DTSTART:20261105T190000Z\r\n"));
        assert!(ics.contains("DTEND:20261105T203000Z\r\n"));
        assert!(ics.contains("SUMMARY:Jam\\, with\\; friends\r\n"));
        assert!(ics.contains("STATUS:CANCELLED\r\n"));
        assert!(ics.split("\r\n").all(|line| line.len() <= 75));
    }

    #[test]
    fn urls_need_a_host_and_nothing_that_could_break_out() {
        assert!(is_http_url("https://example.com/r?a=b#c"));
        assert!(!is_http_url("https://"));
        assert!(!is_http_url("https:///path"));
        assert!(!is_http_url("https://example.com/a\nb"));
        assert!(!is_http_url("https://example.com/a\u{7}b"));
        assert!(!is_http_url("ftp://example.com"));
    }

    #[test]
    fn calendar_text_keeps_no_control_characters() {
        assert_eq!(ics_text("a\rb\u{7}c\nd"), "abc\\nd");
    }

    #[test]
    fn an_event_that_cannot_be_read_says_why_and_costs_only_itself() {
        let page: PageConfig = serde_json::from_str(
            r#"{"title":"T","content":"<p>x</p>","event":{"start":"2026-11-05T14:00:00+01:00","timeZone":"UTC","status":"canceled"}}"#,
        )
        .unwrap();
        let problem = page.event.unwrap().resolve().unwrap_err().join(" ");
        assert!(problem.contains("canceled"), "{problem}");
        let page: PageConfig =
            serde_json::from_str(r#"{"title":"T","content":"<p>x</p>","event":{"venue":"Hall"}}"#)
                .unwrap();
        let problem = page.event.unwrap().resolve().unwrap_err().join(" ");
        assert!(problem.contains("start"), "{problem}");
    }

    #[test]
    fn calendar_files_sit_next_to_their_page() {
        assert_eq!(event_calendar_href("/agenda/jam"), "/agenda/jam.ics");
        assert_eq!(agenda_calendar_href("/agenda"), "/agenda/calendar.ics");
        assert_eq!(agenda_calendar_href("/"), "/calendar.ics");
        assert_eq!(event_calendar_href("/"), "/index.ics");
    }
}
