//! Tests that the README's list of Bulma modules stays complete: every class a
//! template or vados's own code can put on a page must belong to a module in
//! the list below, and every module below must be in the README. A new class
//! in a template fails here until it's accounted for, so a site compiling only
//! these modules can't silently lose its styling after an upgrade.

use crate::bulma::{spanning_rows, Color, ImageRatio, TilePlacement, MAX_ROW_SPAN};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Each module of `bulma/sass/` the templates need, with the classes they use
/// from it. `#` stands for a number filled in when rendering.
const MODULES: &[(&str, &[&str])] = &[
    ("base", &[]),
    ("themes", &["theme-light", "theme-dark"]),
    ("components/breadcrumb", &["breadcrumb"]),
    ("components/menu", &["menu", "menu-label", "menu-list"]),
    (
        "components/navbar",
        &[
            "navbar",
            "navbar-brand",
            "navbar-burger",
            "navbar-dropdown",
            "navbar-end",
            "navbar-item",
            "navbar-link",
            "navbar-menu",
            "navbar-start",
            "has-dropdown",
            "has-navbar-fixed-top",
            "is-fixed-top",
            "is-hoverable",
            "is-tab",
            "is-active",
            "is-right",
        ],
    ),
    ("elements/block", &["block"]),
    ("elements/box", &["box"]),
    ("elements/button", &["button"]),
    ("elements/content", &["content", "is-medium"]),
    ("elements/icon", &["icon", "is-large"]),
    (
        "elements/image",
        &[
            "image", "is-1by1", "is-5by4", "is-4by3", "is-3by2", "is-5by3", "is-16by9", "is-2by1",
            "is-3by1", "is-4by5", "is-3by4", "is-2by3", "is-3by5", "is-9by16", "is-1by2",
            "is-1by3",
        ],
    ),
    (
        "elements/notification",
        &[
            "notification",
            "is-white",
            "is-black",
            "is-light",
            "is-dark",
            "is-primary",
            "is-link",
            "is-info",
            "is-success",
            "is-warning",
            "is-danger",
        ],
    ),
    (
        "elements/title",
        &["title", "subtitle", "is-1", "is-3", "is-4", "is-5"],
    ),
    ("form/shared", &[]),
    ("form/input-textarea", &["input"]),
    (
        "form/tools",
        &["field", "label", "control", "has-addons", "is-expanded"],
    ),
    ("grid/columns", &["columns", "column", "is-9"]),
    (
        "grid/grid",
        &[
            "fixed-grid",
            "grid",
            "cell",
            "has-2-cols",
            "is-col-start-1-tablet",
            "is-col-start-2-tablet",
            "is-col-span-2-mobile",
            "is-row-span-#-tablet",
        ],
    ),
    ("helpers/color", &["has-background-light-soft"]),
    (
        "helpers/flexbox",
        &[
            "is-flex-direction-column",
            "is-flex-direction-row-reverse",
            "is-flex-grow-1",
        ],
    ),
    ("helpers/gap", &["is-gap-3"]),
    (
        "helpers/spacing",
        &[
            "mb-0", "mb-1", "mb-2", "mb-4", "ml-0", "mt-4", "m-0", "p-1", "p-2",
        ],
    ),
    ("helpers/typography", &["is-size-5", "is-size-7"]),
    (
        "helpers/visibility",
        &[
            "is-block",
            "is-flex",
            "is-hidden",
            "is-hidden-desktop",
            "is-hidden-mobile",
            "is-hidden-tablet",
            "is-hidden-touch",
        ],
    ),
    ("layout/container", &["container"]),
    ("layout/footer", &["footer"]),
    ("layout/section", &["section"]),
];

/// Classes that are vados's own hooks, styled by the page's inline style or
/// by nothing at all, or another party's.
const OWN_CLASSES: &[&str] = &[
    "skip-link",
    "stretch-last",
    "mgb-small",
    "mdi-icon",
    "nav-chevron",
    "nav-disclosure",
    "nav-more",
    "is-enhanced",
    "luma-checkout--button",
];

/// Every class in a template's `class="..."` attributes, whichever branch of
/// its conditionals it's in. What an expression fills in comes from the code
/// and is checked by `classes_from_code`.
fn classes_in_template(template: &str) -> BTreeSet<String> {
    let mut classes = BTreeSet::new();
    let mut rest = template;
    while let Some(start) = rest.find("class=\"") {
        rest = &rest[start + 7..];
        let mut value = String::new();
        loop {
            if let Some(after) = rest.strip_prefix("{%") {
                let end = after.find("%}").expect("an unclosed template tag");
                value.push(' ');
                rest = &after[end + 2..];
            } else if let Some(after) = rest.strip_prefix("{{") {
                let end = after.find("}}").expect("an unclosed template expression");
                value.push('#');
                rest = &after[end + 2..];
            } else if let Some(after) = rest.strip_prefix('"') {
                rest = after;
                break;
            } else {
                let c = rest.chars().next().expect("an unclosed class attribute");
                value.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
        classes.extend(
            value
                .split_whitespace()
                .filter(|c| !c.chars().all(|x| x == '#'))
                .map(String::from),
        );
    }
    classes
}

/// The classes the code itself writes.
fn classes_from_code() -> BTreeSet<String> {
    let colors = [
        Color::White,
        Color::Black,
        Color::Light,
        Color::Dark,
        Color::Primary,
        Color::Link,
        Color::Info,
        Color::Success,
        Color::Warning,
        Color::Danger,
    ];
    let ratios = [
        ImageRatio::Is1By1,
        ImageRatio::Is5By4,
        ImageRatio::Is4By3,
        ImageRatio::Is3By2,
        ImageRatio::Is5By3,
        ImageRatio::Is16By9,
        ImageRatio::Is2By1,
        ImageRatio::Is3By1,
        ImageRatio::Is4by5,
        ImageRatio::Is3By4,
        ImageRatio::Is2By3,
        ImageRatio::Is3By5,
        ImageRatio::Is9By16,
        ImageRatio::Is1By2,
        ImageRatio::Is1By3,
    ];
    let placements = [
        TilePlacement::Left,
        TilePlacement::Right,
        TilePlacement::Side,
    ];
    let mut written: Vec<String> = vec![
        String::from("has-background-light-soft"),
        // What templates pass to `icons::icon_span`.
        String::from("icon m-0"),
        // What `assets/js/*.js` adds.
        String::from("is-active block is-size-7"),
    ];
    written.extend(colors.iter().map(|c| c.to_css_class().to_string()));
    written.extend(ratios.iter().map(|r| r.to_css_class().to_string()));
    written.extend(placements.iter().map(|p| p.to_css_classes().to_string()));
    written.extend(placements.iter().map(|p| p.to_column_class().to_string()));
    written.push(spanning_rows("is-col-span-2-mobile", MAX_ROW_SPAN).replace("12", "#"));
    written
        .iter()
        .flat_map(|w| w.split_whitespace())
        .map(String::from)
        .collect()
}

fn every_class_used() -> BTreeSet<String> {
    let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let mut classes = classes_from_code();
    for entry in fs::read_dir(templates).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "html") {
            classes.extend(classes_in_template(&fs::read_to_string(path).unwrap()));
        }
    }
    classes
}

#[test]
fn every_class_vados_uses_belongs_to_a_listed_module() {
    let known: BTreeSet<&str> = MODULES
        .iter()
        .flat_map(|(_, classes)| classes.iter().copied())
        .chain(OWN_CLASSES.iter().copied())
        .collect();
    let unknown: Vec<String> = every_class_used()
        .into_iter()
        .filter(|c| !known.contains(c.as_str()))
        .collect();
    assert!(
        unknown.is_empty(),
        "classes in no listed module: {unknown:?}. Add each to MODULES under the Bulma \
         module providing it (and the README's list if that module is new), or to \
         OWN_CLASSES if it's vados's own."
    );
}

#[test]
fn no_module_is_listed_for_a_class_nothing_uses() {
    let used = every_class_used();
    let unused: Vec<String> = MODULES
        .iter()
        .flat_map(|(module, classes)| classes.iter().map(move |class| (module, class)))
        .filter(|(_, class)| !used.contains(**class))
        .map(|(module, class)| format!("{class} of {module}"))
        .collect();
    assert!(
        unused.is_empty(),
        "no longer used: {unused:?}; remove each, and its module if that was its last"
    );
}

#[test]
fn the_readme_lists_every_module() {
    let readme =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md")).unwrap();
    for (module, _) in MODULES {
        let line = format!("@use \"bulma/sass/{module}\"");
        assert!(readme.contains(&line), "the README doesn't list {line}");
    }
}

#[test]
fn classes_are_read_from_every_branch_and_expressions_are_skipped() {
    let classes = classes_in_template(
        r#"<a class="{% if path == "x" %}a b{% else %}c{% endif %} {{ color }} is-row-span-{{ n }}-tablet">"#,
    );
    let found: Vec<&str> = classes.iter().map(String::as_str).collect();
    assert_eq!(found, ["a", "b", "c", "is-row-span-#-tablet"]);
}
