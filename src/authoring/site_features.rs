//! Authoring the site features that are plain settings: a page's event,
//! agenda and `noindex`, and main.json's search, spare tile and navbar. Each
//! checks what it can before writing anything, like the other content-authoring
//! commands; the settings themselves are in `vados.allium`.

use super::page::{page_dir, page_exists_at};
use crate::bulma::NavbarTheme;
use crate::config_files::{
    AgendaConfig, AgendaSetting, FeedConfig, FeedContent, MainConfig, OneOrMany, PageConfig,
    SearchConfig, SearchSetting, SpareTile, SpareTileLink,
};
use crate::event::{DeclaredEvent, EventAttendance, EventConfig, EventStatus};
use crate::json_files::{
    read_json_for_edit as read_json, read_json_or_for_edit as read_json_or, write_json_pretty,
};
use std::path::Path;

/// `--from` given any number of times, as page.json writes it.
fn sources(from: Vec<String>) -> Option<OneOrMany> {
    match from.len() {
        0 => None,
        1 => from.into_iter().next().map(OneOrMany::One),
        _ => Some(OneOrMany::Many(from)),
    }
}

/// Reads, changes and rewrites an existing page's page.json.
fn change_page(
    source: &str,
    path: &str,
    change: impl FnOnce(&mut PageConfig),
) -> Result<(), String> {
    if !page_exists_at(source, path) {
        return Err(format!("page {path} does not exist"));
    }
    let page_json = page_dir(source, path).join("page.json");
    let mut config =
        read_json_or(&page_json, || PageConfig::new(path)).map_err(|e| e.to_string())?;
    change(&mut config);
    write_json_pretty(page_json, &config).map_err(|e| e.to_string())
}

fn change_main(source: &str, change: impl FnOnce(&mut MainConfig)) -> Result<(), String> {
    let main_json = Path::new(source).join("main.json");
    let mut config: MainConfig = read_json(&main_json).map_err(|e| e.to_string())?;
    change(&mut config);
    write_json_pretty(main_json, &config).map_err(|e| e.to_string())
}

/// `page set-noindex`.
pub fn set_noindex(source: &str, path: &str, noindex: bool) -> Result<(), String> {
    change_page(source, path, |config| {
        config.noindex = noindex.then_some(true);
    })
}

/// `page enable-agenda`.
pub fn enable_agenda(
    source: &str,
    path: &str,
    from: Vec<String>,
    depth: Option<u32>,
    include_unlisted: bool,
) -> Result<(), String> {
    change_page(source, path, |config| {
        // Like `page enable-feed`, a setting not given stays as it is.
        let mut agenda = config.get_agenda().unwrap_or_default();
        if !from.is_empty() {
            agenda.from = sources(from);
        }
        if depth.is_some() {
            agenda.depth = depth;
        }
        if include_unlisted {
            agenda.include_unlisted = Some(true);
        }
        config.agenda = Some(if agenda == AgendaConfig::default() {
            AgendaSetting::Enabled(true)
        } else {
            AgendaSetting::Configured(agenda)
        });
    })
}

/// How someone attends an event, as `page set-event` takes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GivenAttendance {
    Online,
    InPerson,
    Mixed,
}

/// An event's status, as `page set-event` takes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GivenStatus {
    Scheduled,
    Cancelled,
    Postponed,
    Rescheduled,
}

/// Everything `page set-event` writes.
#[derive(Debug, Clone, Default)]
pub struct GivenEvent {
    pub start: String,
    pub end: Option<String>,
    pub time_zone: String,
    pub attendance: Option<GivenAttendance>,
    pub status: Option<GivenStatus>,
    pub organiser: Option<String>,
    pub venue: Option<String>,
    pub address: Option<String>,
    pub cost: Option<String>,
    pub registration_url: Option<String>,
    pub virtual_url: Option<String>,
    pub sequence: Option<u32>,
}

impl GivenEvent {
    fn to_config(&self, sequence: Option<u32>) -> EventConfig {
        EventConfig {
            start: self.start.clone(),
            end: self.end.clone(),
            time_zone: self.time_zone.clone(),
            attendance: self.attendance.map(|a| match a {
                GivenAttendance::Online => EventAttendance::Online,
                GivenAttendance::InPerson => EventAttendance::InPerson,
                GivenAttendance::Mixed => EventAttendance::Mixed,
            }),
            status: self.status.map(|s| match s {
                GivenStatus::Scheduled => EventStatus::Scheduled,
                GivenStatus::Cancelled => EventStatus::Cancelled,
                GivenStatus::Postponed => EventStatus::Postponed,
                GivenStatus::Rescheduled => EventStatus::Rescheduled,
            }),
            organiser: self.organiser.clone(),
            venue: self.venue.clone(),
            address: self.address.clone(),
            cost: self.cost.clone(),
            registration_url: self.registration_url.clone(),
            virtual_url: self.virtual_url.clone(),
            sequence,
        }
    }
}

/// `page set-event`: replaces the page's event as a whole, refusing an invalid
/// one with everything wrong with it. Unless `--sequence` says otherwise, a
/// change to an existing event raises its sequence, which is what makes
/// calendar subscribers take the change. Returns the sequence written.
pub fn set_event(source: &str, path: &str, given: &GivenEvent) -> Result<u32, String> {
    given
        .to_config(given.sequence)
        .resolve()
        .map_err(|problems| problems.join("; "))?;
    let mut written = 0;
    change_page(source, path, |config| {
        let existing = config.event.as_ref().and_then(DeclaredEvent::readable);
        let existing_sequence = existing.and_then(|e| e.sequence).unwrap_or(0);
        let unchanged = existing.is_some_and(|e| *e == given.to_config(e.sequence));
        written = given.sequence.unwrap_or(match existing {
            Some(_) if !unchanged => existing_sequence.saturating_add(1),
            _ => existing_sequence,
        });
        config.event = Some(DeclaredEvent::Readable(
            given.to_config((written > 0).then_some(written)),
        ));
    })?;
    Ok(written)
}

/// `page enable-feed`'s selection of entries, on top of its title and limit.
#[derive(Debug, Clone, Default)]
pub struct GivenFeedSelection {
    pub from: Vec<String>,
    pub depth: Option<u32>,
    pub include_unlisted: bool,
    pub summaries_only: bool,
}

impl GivenFeedSelection {
    pub(crate) fn apply_to(self, feed: &mut FeedConfig) {
        if !self.from.is_empty() {
            feed.from = sources(self.from);
        }
        if self.depth.is_some() {
            feed.depth = self.depth;
        }
        if self.include_unlisted {
            feed.include_unlisted = Some(true);
        }
        if self.summaries_only {
            feed.content = Some(FeedContent::Summary);
        }
    }
}

/// `search enable`.
pub fn enable_search(
    source: &str,
    from: Vec<String>,
    depth: Option<u32>,
    include_unlisted: bool,
    menu: bool,
) -> Result<(), String> {
    change_main(source, |config| {
        // Like `page enable-feed`, a setting not given stays as it is.
        let mut search = config.get_search().unwrap_or_default();
        if !from.is_empty() {
            search.from = sources(from);
        }
        if depth.is_some() {
            search.depth = depth;
        }
        if include_unlisted {
            search.include_unlisted = Some(true);
        }
        if !menu {
            search.menu = Some(false);
        }
        config.search = Some(if search == SearchConfig::default() {
            SearchSetting::Enabled(true)
        } else {
            SearchSetting::Configured(search)
        });
    })
}

/// `spare-tile set`: each link is `url=title`, split at the last `=`, because
/// an address is much more likely than a title to hold one.
pub fn set_spare_tile(source: &str, title: Option<String>, links: &[String]) -> Result<(), String> {
    let links = links
        .iter()
        .map(|link| match link.rsplit_once('=') {
            Some((url, title)) if !url.trim().is_empty() && !title.trim().is_empty() => {
                Ok(SpareTileLink {
                    url: url.trim().to_string(),
                    title: title.trim().to_string(),
                })
            }
            _ => Err(format!("`{link}` is not of the form /url=Title")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if links.is_empty() {
        return Err(String::from("a spare tile needs at least one --link"));
    }
    change_main(source, |config| {
        config.spare_tile = Some(SpareTile { title, links });
    })
}

/// `spare-tile remove`.
pub fn remove_spare_tile(source: &str) -> Result<(), String> {
    change_main(source, |config| config.spare_tile = None)
}

/// `navbar set`. A setting not given stays as it is.
pub fn set_navbar(
    source: &str,
    dark_theme: Option<bool>,
    dropdown_limit: Option<u32>,
) -> Result<(), String> {
    change_main(source, |config| {
        if let Some(dark) = dark_theme {
            config.navbar_theme = Some(if dark {
                NavbarTheme::Dark
            } else {
                NavbarTheme::Light
            });
        }
        if dropdown_limit.is_some() {
            config.menu_dropdown_limit = dropdown_limit;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    struct Site(PathBuf);

    impl Site {
        fn new() -> Site {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir().join(format!("vados_site_features_{id}"));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("events")).unwrap();
            fs::write(
                root.join("main.json"),
                r#"{"siteTitle":"T","jsFiles":[],"cssFiles":[],"footerContent":"<p>f</p>"}"#,
            )
            .unwrap();
            fs::write(
                root.join("events/page.json"),
                r#"{"title":"Events","content":"<p>x</p>"}"#,
            )
            .unwrap();
            Site(root)
        }
        fn source(&self) -> &str {
            self.0.to_str().unwrap()
        }
        fn page(&self) -> PageConfig {
            read_json(&self.0.join("events/page.json")).unwrap()
        }
        fn main(&self) -> MainConfig {
            read_json(&self.0.join("main.json")).unwrap()
        }
    }

    impl Drop for Site {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn event(venue: &str) -> GivenEvent {
        GivenEvent {
            start: String::from("2026-11-05T14:00:00+01:00"),
            time_zone: String::from("Europe/Amsterdam"),
            venue: Some(String::from(venue)),
            ..GivenEvent::default()
        }
    }

    #[test]
    fn noindex_turns_on_and_off_again() {
        let site = Site::new();
        set_noindex(site.source(), "/events", true).unwrap();
        assert_eq!(site.page().noindex, Some(true));
        set_noindex(site.source(), "/events", false).unwrap();
        assert_eq!(site.page().noindex, None);
    }

    #[test]
    fn a_missing_page_is_refused() {
        let site = Site::new();
        assert!(set_noindex(site.source(), "/nope", true).is_err());
        assert!(!site.0.join("nope").exists());
    }

    #[test]
    fn an_invalid_event_is_refused_and_nothing_is_written() {
        let site = Site::new();
        let mut bad = event("Hall");
        bad.start = String::from("2026-07-05T14:00:00+01:00");
        let error = set_event(site.source(), "/events", &bad).unwrap_err();
        assert!(error.contains("UTC offset"), "{error}");
        assert_eq!(site.page().event, None);
    }

    #[test]
    fn changing_an_event_raises_its_sequence_and_repeating_it_does_not() {
        let site = Site::new();
        assert_eq!(
            set_event(site.source(), "/events", &event("Hall")).unwrap(),
            0
        );
        assert_eq!(
            site.page().event.unwrap().readable().unwrap().sequence,
            None
        );
        assert_eq!(
            set_event(site.source(), "/events", &event("Hall")).unwrap(),
            0
        );
        assert_eq!(
            set_event(site.source(), "/events", &event("Barn")).unwrap(),
            1
        );
        assert_eq!(
            set_event(site.source(), "/events", &event("Barn")).unwrap(),
            1
        );
        assert_eq!(
            site.page().event.unwrap().readable().unwrap().sequence,
            Some(1)
        );
    }

    #[test]
    fn a_sequence_at_its_maximum_stays_there_when_the_event_changes() {
        let site = Site::new();
        let mut hall = event("Hall");
        hall.sequence = Some(u32::MAX);
        set_event(site.source(), "/events", &hall).unwrap();
        assert_eq!(
            set_event(site.source(), "/events", &event("Barn")).unwrap(),
            u32::MAX
        );
    }

    #[test]
    fn an_agenda_is_plain_true_unless_there_is_something_to_say() {
        let site = Site::new();
        enable_agenda(site.source(), "/events", vec![], None, false).unwrap();
        assert_eq!(site.page().agenda, Some(AgendaSetting::Enabled(true)));
        enable_agenda(site.source(), "/events", vec!["/a".into()], Some(2), true).unwrap();
        let agenda = site.page().get_agenda().unwrap();
        assert_eq!(agenda.from, Some(OneOrMany::One("/a".into())));
        assert_eq!(agenda.depth, Some(2));
    }

    #[test]
    fn search_settings_reach_main_json() {
        let site = Site::new();
        enable_search(site.source(), vec![], None, false, true).unwrap();
        assert_eq!(site.main().search, Some(SearchSetting::Enabled(true)));
        enable_search(
            site.source(),
            vec!["/a".into(), "/b".into()],
            None,
            true,
            false,
        )
        .unwrap();
        let search = site.main().get_search().unwrap();
        assert!(!search.shows_in_menu());
        assert_eq!(search.include_unlisted, Some(true));
    }

    #[test]
    fn rerunning_without_flags_keeps_what_is_set() {
        let site = Site::new();
        enable_agenda(site.source(), "/events", vec!["/a".into()], Some(2), true).unwrap();
        enable_agenda(site.source(), "/events", vec![], None, false).unwrap();
        let agenda = site.page().get_agenda().unwrap();
        assert_eq!(agenda.from, Some(OneOrMany::One("/a".into())));
        assert_eq!(agenda.depth, Some(2));
        assert_eq!(agenda.include_unlisted, Some(true));
        enable_search(site.source(), vec!["/a".into()], Some(3), true, false).unwrap();
        enable_search(site.source(), vec![], None, false, true).unwrap();
        let search = site.main().get_search().unwrap();
        assert_eq!(search.from, Some(OneOrMany::One("/a".into())));
        assert_eq!(search.depth, Some(3));
        assert!(!search.shows_in_menu());
    }

    #[test]
    fn a_spare_tile_link_splits_at_the_last_equals_sign() {
        let site = Site::new();
        set_spare_tile(site.source(), None, &["/search?q=rust=Search".into()]).unwrap();
        let tile = site.main().spare_tile.unwrap();
        assert_eq!(tile.links[0].url, "/search?q=rust");
        assert_eq!(tile.links[0].title, "Search");
    }

    #[test]
    fn spare_tile_links_are_url_equals_title() {
        let site = Site::new();
        assert!(set_spare_tile(site.source(), None, &[]).is_err());
        assert!(set_spare_tile(site.source(), None, &["no-title".into()]).is_err());
        set_spare_tile(
            site.source(),
            Some("More".into()),
            &["/topics=All topics".into(), "/feed.xml=Subscribe".into()],
        )
        .unwrap();
        let tile = site.main().spare_tile.unwrap();
        assert_eq!(tile.links[0].url, "/topics");
        assert_eq!(tile.links[1].title, "Subscribe");
        remove_spare_tile(site.source()).unwrap();
        assert_eq!(site.main().spare_tile, None);
    }

    #[test]
    fn navbar_settings_not_given_stay_as_they_are() {
        let site = Site::new();
        set_navbar(site.source(), Some(true), None).unwrap();
        set_navbar(site.source(), None, Some(8)).unwrap();
        let main = site.main();
        assert_eq!(main.navbar_theme, Some(NavbarTheme::Dark));
        assert_eq!(main.menu_dropdown_limit, Some(8));
    }
}
