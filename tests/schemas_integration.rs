//! The published JSON Schemas accept what vados accepts and reject the
//! mistakes they are meant to catch while typing: the `vados_bass` fixture,
//! the README's examples, a freshly scaffolded project, and misspelt keys,
//! enum values and forms.

mod schema_support;

use schema_support::{schema_errors, tree_errors};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use vados::init::{self, ProjectBasics};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vados_bass")
}

#[test]
fn every_file_in_the_vados_bass_fixture_validates() {
    let errors = tree_errors(&fixture());
    assert!(errors.is_empty(), "{errors:#?}");
}

/// The smallest valid file of each kind, for fragments to be added to.
fn base(file: &str) -> Value {
    match file {
        "main.json" => json!({ "siteTitle": "Site", "footerContent": "<p>Footer</p>" }),
        "page.json" => json!({ "title": "Page", "content": "<p>Content</p>" }),
        _ => unreachable!(),
    }
}

/// A README example: a whole object, or keys from one written without braces.
fn example_object(text: &str) -> Map<String, Value> {
    let text = text.trim();
    let text = if text.starts_with('{') {
        text.to_string()
    } else {
        format!("{{{text}}}")
    };
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("unreadable example {text}: {e}"))
}

/// Errors for `example` when added to the smallest valid `file`.
fn example_errors(file: &str, example: &Map<String, Value>) -> Vec<String> {
    let mut value = base(file);
    value.as_object_mut().unwrap().extend(example.clone());
    schema_errors(file, &value)
}

#[test]
fn every_json_example_in_the_readme_validates() {
    let readme =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md")).unwrap();
    let mut examples = 0;
    for block in readme.split("```json").skip(1) {
        let block = block.strip_prefix('c').unwrap_or(block);
        let body = block.split("```").next().unwrap();
        // A `jsonc` block names each example's file in a comment above it.
        let mut segments: Vec<(Option<String>, String)> = vec![(None, String::new())];
        for line in body.lines() {
            match line.trim().strip_prefix("//") {
                Some(name) => segments.push((Some(name.trim().to_string()), String::new())),
                None => segments
                    .last_mut()
                    .unwrap()
                    .1
                    .push_str(&format!("{line}\n")),
            }
        }
        for (name, text) in segments.into_iter().filter(|(_, t)| !t.trim().is_empty()) {
            examples += 1;
            let example = example_object(&text);
            let candidates = match name.as_deref() {
                Some(name) if name.ends_with("main.json") => vec!["main.json"],
                Some(name) if name.ends_with("page.json") => vec!["page.json"],
                Some(name) => panic!("README example for unknown file {name}"),
                // Unlabelled fragments are keys of main.json or of a page.json.
                None => vec!["main.json", "page.json"],
            };
            let errors: Vec<_> = candidates
                .iter()
                .map(|file| example_errors(file, &example))
                .collect();
            assert!(
                errors.iter().any(Vec::is_empty),
                "README example doesn't validate:\n{text}\n{errors:#?}"
            );
        }
    }
    assert!(examples >= 13, "found only {examples} README examples");
}

#[test]
fn a_freshly_scaffolded_project_points_to_its_schemas_and_validates() {
    let dir = std::env::temp_dir().join(format!("vados_schemas_scaffold_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    init::scaffold(
        &dir,
        ProjectBasics {
            site_title: "Site".to_string(),
            home_intro: None,
            primary_color: None,
            footer_text: None,
            language: None,
            favicon: None,
            site_url: Some("https://example.com".to_string()),
            socials: vec![],
        },
    )
    .unwrap();

    let source = dir.join(init::SOURCE_DIR);
    for file in ["main.json", "menu.json", "page.json"] {
        let text = fs::read_to_string(source.join(file)).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        let expected = format!(
            "https://raw.githubusercontent.com/gklijs/vados/v{}/schemas/{}",
            env!("CARGO_PKG_VERSION"),
            file.replace(".json", ".schema.json")
        );
        assert_eq!(value["$schema"], json!(expected), "{file}");
        assert!(
            text.trim_start().starts_with("{\n  \"$schema\""),
            "{file} should start with $schema:\n{text}"
        );
    }
    let errors = tree_errors(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(errors.is_empty(), "{errors:#?}");
}

/// Asserts that `value` breaks the schema for `file` at `path`, or at a
/// parent of it: an optional object is "this or null", which the validator
/// reports where the object is. Every case changes one value in a valid
/// document, so the error can only come from that value.
fn assert_rejected_at(file: &str, value: Value, path: &str) {
    let errors = schema_errors(file, &value);
    let at_or_above = |error: &String| {
        let location = error.split(": ").next().unwrap();
        path == location || path.starts_with(&format!("{location}/"))
    };
    assert!(
        errors.iter().any(at_or_above),
        "expected an error at {path} for {value}, got {errors:#?}"
    );
}

#[test]
fn misspelt_keys_are_rejected_at_every_level() {
    let mut main = base("main.json");
    main["siteTtle"] = json!("x");
    assert_rejected_at("main.json", main, "");

    let mut page = base("page.json");
    page["feed"] = json!({ "limt": 3 });
    assert_rejected_at("page.json", page, "/feed");

    let mut page = base("page.json");
    page["leftNotifications"] = json!([{ "conten": "x" }]);
    assert_rejected_at("page.json", page, "/leftNotifications/0");

    assert_rejected_at(
        "menu.json",
        json!({ "mainMenu": [{ "url": "/", "titel": "Home" }], "socials": [] }),
        "/mainMenu/0",
    );
    assert_rejected_at(
        "images.json",
        json!({ "title": null, "list": [{ "fileName": "a.jpg", "altText": "A", "alt": "A" }] }),
        "/list/0",
    );
}

#[test]
fn misspelt_enum_values_are_rejected() {
    let cases = [
        ("page.json", "childOrder", json!("datedesc"), "/childOrder"),
        ("main.json", "colorScheme", json!("Dark"), "/colorScheme"),
        ("main.json", "navbarColor", json!("warning"), "/navbarColor"),
        ("main.json", "navbarTheme", json!("black"), "/navbarTheme"),
        (
            "page.json",
            "event",
            json!({ "start": "2026-11-05T14:00:00-05:00", "timeZone": "America/New_York", "status": "canceled" }),
            "/event/status",
        ),
        (
            "page.json",
            "event",
            json!({ "start": "2026-11-05T14:00:00-05:00", "timeZone": "America/New_York", "attendance": "in-person" }),
            "/event/attendance",
        ),
        (
            "page.json",
            "lumaEvents",
            json!([{ "eventId": "evt-a1", "title": "Jam", "display": "button" }]),
            "/lumaEvents/0/display",
        ),
        (
            "page.json",
            "feed",
            json!({ "content": "summaries" }),
            "/feed/content",
        ),
    ];
    for (file, key, value, path) in cases {
        let mut document = base(file);
        document[key] = value;
        assert_rejected_at(file, document, path);
    }
}

#[test]
fn values_vados_checks_the_form_of_are_rejected_when_misshapen() {
    let cases = [
        ("date", json!("2026-1-5"), "/date"),
        ("updated", json!("22-10-2026"), "/updated"),
        (
            "event",
            json!({ "start": "2026-11-05T14:00:00", "timeZone": "America/New_York" }),
            "/event/start",
        ),
        (
            "lumaEvents",
            json!([{ "eventId": "cal-a1", "title": "Jam" }]),
            "/lumaEvents/0/eventId",
        ),
        (
            "lumaEvents",
            json!([{ "eventId": "evt-a1" }]),
            "/lumaEvents/0",
        ),
    ];
    for (key, value, path) in cases {
        let mut page = base("page.json");
        page[key] = value;
        assert_rejected_at("page.json", page, path);
    }
    assert_rejected_at(
        "menu.json",
        json!({ "mainMenu": [], "socials": [{ "url": "https://x.example", "icon": "x", "color": "blue", "label": "X" }] }),
        "/socials/0/color",
    );
}

#[test]
fn forms_vados_reads_are_accepted() {
    for file in ["main.json", "page.json"] {
        assert_eq!(schema_errors(file, &base(file)), Vec::<String>::new());
    }
    let mut page = base("page.json");
    page["date"] = json!("2026-10-22T09:30");
    page["updated"] = json!("2026-10-23T09:30:15");
    page["event"] = json!({ "start": "2026-11-05T19:00:00Z", "end": "2026-11-05T21:30+01:00", "timeZone": "Europe/Amsterdam" });
    page["lumaEvents"] =
        json!([{ "eventId": "evt-AbC123", "title": "Jam", "start": "2026-11-05T19:00:00+01:00" }]);
    page["feed"] = json!(true);
    page["agenda"] = json!({ "from": ["/a", "/b"] });
    page["latest"] = json!({ "from": "/blog" });
    assert_eq!(schema_errors("page.json", &page), Vec::<String>::new());

    let menu = json!({ "$schema": "https://example.com/menu.schema.json", "mainMenu": [], "socials": [
        { "url": "https://a.example", "icon": "a", "color": "#6364FF", "label": "A" },
        { "url": "https://github.com/x", "icon": null, "color": null }
    ] });
    assert_eq!(schema_errors("menu.json", &menu), Vec::<String>::new());
}
