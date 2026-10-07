//! `page add-luma-event`: links an event hosted on Luma to a page that's
//! already there. See `vados.allium`'s `LumaEventAttachmentRun`.

use super::page::{page_dir, page_exists_at};
use crate::config_files::PageConfig;
use crate::json_files::{read_json_or, write_json_pretty};
pub use crate::luma::LumaEventDisplay;
use crate::luma::{is_luma_event_id, LumaEvent};
use std::fmt;
use std::io;

/// Why a `page add-luma-event` request couldn't proceed. See
/// `vados.allium`'s `LumaEventAttachmentBlockReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LumaEventAttachmentBlockReason {
    TargetPageMissing,
    MalformedEventId,
    EventAlreadyOnPage,
}

impl fmt::Display for LumaEventAttachmentBlockReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LumaEventAttachmentBlockReason::TargetPageMissing => "the target page does not exist",
            LumaEventAttachmentBlockReason::MalformedEventId => {
                "not a Luma event ID (expected evt-...)"
            }
            LumaEventAttachmentBlockReason::EventAlreadyOnPage => {
                "the page already links this Luma event"
            }
        })
    }
}

#[derive(Debug)]
pub struct LumaEventAttachmentBlock {
    pub reason: LumaEventAttachmentBlockReason,
    pub detail: String,
}

fn read_page_config(source: &str, path: &str) -> io::Result<PageConfig> {
    read_json_or(&page_dir(source, path).join("page.json"), || {
        PageConfig::new(path)
    })
}

/// Whether the page at `path` already links `event_id`. A page.json that
/// can't be read links nothing as far as this is concerned; the write that
/// follows will surface the problem instead. Backs `page_links_luma_event`.
pub fn page_links_luma_event(source: &str, path: &str, event_id: &str) -> bool {
    read_page_config(source, path)
        .ok()
        .and_then(|c| c.luma_events)
        .is_some_and(|events| events.iter().any(|e| e.event_id == event_id))
}

/// Checks every way a `page add-luma-event` request could fail in one pass,
/// before the maintainer is asked for a title. See `vados.allium`'s
/// `DetectLumaEventAttachmentBlockers`.
pub fn detect_luma_event_blockers(
    source: &str,
    path: &str,
    event_id: &str,
) -> Vec<LumaEventAttachmentBlock> {
    let mut blocks = Vec::new();
    let page_exists = page_exists_at(source, path);
    if !page_exists {
        blocks.push(LumaEventAttachmentBlock {
            reason: LumaEventAttachmentBlockReason::TargetPageMissing,
            detail: String::from(path),
        });
    }
    if !is_luma_event_id(event_id) {
        blocks.push(LumaEventAttachmentBlock {
            reason: LumaEventAttachmentBlockReason::MalformedEventId,
            detail: String::from(event_id),
        });
    }
    if page_exists && page_links_luma_event(source, path, event_id) {
        blocks.push(LumaEventAttachmentBlock {
            reason: LumaEventAttachmentBlockReason::EventAlreadyOnPage,
            detail: String::from(event_id),
        });
    }
    blocks
}

/// Gathered once the blocker check has passed. See `vados.allium`'s
/// `LumaEventAttachmentDetailsProvided`; the title is required so the
/// command never writes an event `check` would report as untitled.
#[derive(Debug, Clone)]
pub struct LumaEventAttachmentDetails {
    pub title: String,
    pub display: Option<LumaEventDisplay>,
    pub button_label: Option<String>,
}

/// What `page add-luma-event` linked, once it has. See `vados.allium`'s
/// `LumaEventAttachedOutcome`.
#[derive(Debug)]
pub struct LumaEventAttachedOutcome {
    pub path: String,
    pub event_id: String,
    pub title: String,
    pub display: LumaEventDisplay,
}

/// Appends a Luma event to a page's page.json, after any it already links.
/// Callers are expected to have already run `detect_luma_event_blockers`
/// and found it empty.
pub fn attach_luma_event(
    source: &str,
    path: &str,
    event_id: &str,
    details: LumaEventAttachmentDetails,
) -> io::Result<LumaEventAttachedOutcome> {
    let title = details.title.trim().to_string();
    let button_label = details
        .button_label
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty());
    let mut config = read_page_config(source, path)?;
    config
        .luma_events
        .get_or_insert_with(Vec::new)
        .push(LumaEvent {
            event_id: event_id.trim().to_string(),
            title: Some(title.clone()),
            display: details.display,
            button_label,
        });
    write_json_pretty(page_dir(source, path).join("page.json"), &config)?;
    Ok(LumaEventAttachedOutcome {
        path: String::from(path),
        event_id: String::from(event_id),
        title,
        display: details.display.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    struct Tree {
        root: PathBuf,
    }

    impl Tree {
        fn new(name: &str) -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir().join(format!("vados_luma_test_{name}_{id}"));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Tree { root }
        }
        fn source(&self) -> &str {
            self.root.to_str().unwrap()
        }
        fn mkdir(&self, rel_path: &str) {
            fs::create_dir_all(self.root.join(rel_path)).unwrap();
        }
        fn page_json(&self, rel_path: &str) -> serde_json::Value {
            let text = fs::read_to_string(self.root.join(rel_path).join("page.json")).unwrap();
            serde_json::from_str(&text).unwrap()
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn details(title: &str) -> LumaEventAttachmentDetails {
        LumaEventAttachmentDetails {
            title: String::from(title),
            display: None,
            button_label: None,
        }
    }

    fn reasons(blocks: &[LumaEventAttachmentBlock]) -> Vec<LumaEventAttachmentBlockReason> {
        blocks.iter().map(|b| b.reason).collect()
    }

    #[test]
    fn an_existing_page_and_a_well_formed_id_have_no_blockers() {
        let tree = Tree::new("no_blockers");
        tree.mkdir("events");
        assert!(detect_luma_event_blockers(tree.source(), "/events", "evt-a1").is_empty());
    }

    #[test]
    fn every_blocker_is_found_in_one_pass() {
        let tree = Tree::new("all_blockers");
        let blocks = detect_luma_event_blockers(tree.source(), "/missing", "jam");
        assert_eq!(
            reasons(&blocks),
            vec![
                LumaEventAttachmentBlockReason::TargetPageMissing,
                LumaEventAttachmentBlockReason::MalformedEventId,
            ]
        );
    }

    #[test]
    fn linking_the_same_event_twice_is_blocked() {
        let tree = Tree::new("already_linked");
        tree.mkdir("events");
        attach_luma_event(tree.source(), "/events", "evt-a1", details("Jam")).unwrap();
        let blocks = detect_luma_event_blockers(tree.source(), "/events", "evt-a1");
        assert_eq!(
            reasons(&blocks),
            vec![LumaEventAttachmentBlockReason::EventAlreadyOnPage]
        );
    }

    #[test]
    fn attaching_to_a_page_without_page_json_creates_one() {
        let tree = Tree::new("no_page_json");
        tree.mkdir("events");
        let outcome =
            attach_luma_event(tree.source(), "/events", "evt-a1", details(" Jam ")).unwrap();
        assert_eq!(outcome.title, "Jam");
        assert_eq!(outcome.display, LumaEventDisplay::EventPage);
        let json = tree.page_json("events");
        assert_eq!(json["title"], "events");
        assert_eq!(
            json["lumaEvents"],
            serde_json::json!([{"eventId": "evt-a1", "title": "Jam"}])
        );
    }

    #[test]
    fn a_second_event_is_appended_after_the_first() {
        let tree = Tree::new("append");
        tree.mkdir("events");
        attach_luma_event(tree.source(), "/events", "evt-a1", details("Jam")).unwrap();
        attach_luma_event(
            tree.source(),
            "/events",
            "evt-b2",
            LumaEventAttachmentDetails {
                title: String::from("Gig"),
                display: Some(LumaEventDisplay::Both),
                button_label: Some(String::from("Sign up")),
            },
        )
        .unwrap();
        assert_eq!(
            tree.page_json("events")["lumaEvents"],
            serde_json::json!([
                {"eventId": "evt-a1", "title": "Jam"},
                {"eventId": "evt-b2", "title": "Gig", "display": "both", "buttonLabel": "Sign up"}
            ])
        );
    }

    #[test]
    fn the_root_page_can_link_an_event() {
        let tree = Tree::new("root");
        attach_luma_event(tree.source(), "/", "evt-a1", details("Jam")).unwrap();
        assert_eq!(tree.page_json("")["lumaEvents"][0]["eventId"], "evt-a1");
    }
}
