//! Integration tests against a vendored snapshot of the real `vados_bass`
//! content tree (https://github.com/gklijs/vados_bass), which is also the
//! source of the live site at https://bass.gklijs.tech/. Unlike the
//! hand-built fixtures in `check.rs`'s unit tests, this is real authored
//! content nobody wrote to exercise vados: nested log entries five levels
//! deep, images with real EXIF and alpha channels, a social link with no
//! recognised provider, notifications with and without images. It caught two
//! real bugs during development (see vados.allium's history and
//! src/structure.rs) that no hand-written fixture had reached.
//!
//! See tests/fixtures/vados_bass/README.md for how the snapshot was taken
//! and how to refresh it.
//!
//! The content facts asserted below (titles, subtitles, footer text, nav
//! structure) were cross-checked against the live site on 2026-08-07 by
//! fetching https://bass.gklijs.tech/, /gear and /log; they describe what is
//! actually deployed, not just what the fixture happens to contain.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use vados::check::check;
use vados::generator::generate;

fn fixture_root() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/vados_bass/root").to_string()
}

fn fixture_img_root() -> String {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/vados_bass/imgroot"
    )
    .to_string()
}

/// Runs `generate()` on the real fixture exactly once for the whole test
/// binary, however many `#[test]` functions ask for it. Resizing a dozen real
/// photos through Lanczos3 in a debug build takes well over a minute; a
/// second full run per assertion group would buy nothing, since every test
/// here only reads the output, never mutates it.
fn generated_site() -> &'static Path {
    static DEST: OnceLock<PathBuf> = OnceLock::new();
    DEST.get_or_init(|| {
        let dest = std::env::temp_dir().join("vados_bass_integration_dest");
        let _ = std::fs::remove_dir_all(&dest);
        generate(&fixture_root(), &fixture_img_root(), dest.to_str().unwrap());
        dest
    })
}

#[test]
fn check_reports_no_findings_on_the_real_site() {
    let report = check(&fixture_root(), &fixture_img_root());

    assert!(
        report.findings.is_empty(),
        "a real, currently-deployed site should check clean; got {:?}",
        report.findings
    );
    assert!(report.passed());
}

#[test]
fn generate_produces_every_expected_page_and_asset() {
    let dest = generated_site();

    // One representative page per nesting depth actually present in the real
    // tree: the root, a top-level section, a leaf under a single-segment
    // dated path, and a leaf five directories deep.
    let expected_pages = [
        "index.html",
        "gear.html",
        "gear/amber_stars.html",
        "gear/tc_electronic_combo.html",
        "gear/TRBX304.html",
        "log.html",
        "log/2022.html",
        "log/2022/2.html",
        "log/2022/2/10.html",
        "log/2022/4/7-15.html",
    ];
    for page in expected_pages {
        assert!(
            dest.join(page).is_file(),
            "expected generated page {} to exist",
            page
        );
    }

    // The default JS asset main.json opts into.
    assert!(dest.join("js/vados.js").is_file());

    // At least one responsive variant per declared image, across all three
    // image directories, actually landed on disk as a real webp file.
    for variant in [
        "img/gear/amber_stars_horizontal-w318.webp",
        "img/log/sbl_level2-w318.webp",
        "img/months/202202-w318.webp",
    ] {
        let path = dest.join(variant);
        assert!(path.is_file(), "expected image variant {} to exist", variant);
        assert!(
            std::fs::metadata(&path).unwrap().len() > 0,
            "{} exists but is empty",
            variant
        );
    }
}

/// Spot-checks generated content against facts fetched from the live site
/// (see module docs). Not a byte-for-byte diff -- the live site's exact vados
/// version, minifier and CSS build differ from this checkout -- but the
/// content itself, which lives in the vendored source tree, should not have
/// silently drifted from what visitors actually see.
#[test]
fn generated_pages_match_facts_observed_on_the_live_site() {
    let dest = generated_site();

    let index = std::fs::read_to_string(dest.join("index.html")).unwrap();
    // Site title, shown in the navbar brand.
    assert!(index.contains("Bass - gklijs"));
    // Home page's own title and subtitle (main.json is site-wide; this is
    // root/page.json).
    assert!(index.contains(">Home<"));
    assert!(index.contains("Introduction to Gerard&#x27;s bass playing adventures"));
    // Main menu, built from menu.json with titles inherited from the pages
    // it points at.
    assert!(index.contains(">Gear<"));
    assert!(index.contains(">Log<"));
    assert!(index.contains(">Amber Stars<"));
    assert!(index.contains(">TC Electronic combo<"));
    assert!(index.contains(">Yamaha TRBX304<"));
    assert!(index.contains(">2022<"));
    // Social links: two recognised providers plus one unrecognised provider
    // (Twitter) that supplied its own icon and color.
    assert!(index.contains("https://github.com/gklijs"));
    assert!(index.contains("https://twitter.com/GKlijs"));
    assert!(index.contains("mdi-twitter"));
    // Footer, shared site-wide from main.json's footerContent.
    assert!(index.contains("Creative Commons Attribution-NonCommercial-ShareAlike"));
    assert!(index.contains("vados_bass"));

    let gear = std::fs::read_to_string(dest.join("gear.html")).unwrap();
    assert!(gear.contains("Something about the equipment I use"));
}
