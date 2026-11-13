use super::{check, CheckReport, FindingKind};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

pub(super) static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

pub(super) const VALID_MAIN_JSON: &str = r#"{
    "siteTitle": "Test site",
    "jsFiles": [],
    "includeDefaultJs": null,
    "cssFiles": [],
    "includeDefaultCss": null,
    "backgroundClass": null,
    "navbarColor": null,
    "footerContent": "<p>footer</p>"
}"#;

pub(super) const VALID_MENU_JSON: &str = r#"{
    "mainMenu": [],
    "socials": []
}"#;

pub(super) const VALID_PAGE_JSON: &str = r#"{
    "title": "Home",
    "subTitle": null,
    "image": null,
    "icon": null,
    "summary": null,
    "content": "<p>hi</p>",
    "order": null,
    "leftNotifications": null,
    "rightNotifications": null
}"#;

/// A disposable source/image tree for one test. Every test gets its own
/// directories under the OS temp dir, named after the test plus a counter
/// so parallel test runs never collide, and cleaned up on drop.
pub(super) struct TestSite {
    pub(super) root: PathBuf,
    pub(super) source: PathBuf,
    pub(super) img_source: PathBuf,
}

impl TestSite {
    pub(super) fn new(name: &str) -> Self {
        let id = NEXT_ID.fetch_add(1, AtomicOrdering::SeqCst);
        let root = std::env::temp_dir().join(format!("vados_check_test_{}_{}", name, id));
        let _ = fs::remove_dir_all(&root);
        let source = root.join("source");
        let img_source = root.join("images");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&img_source).unwrap();
        TestSite {
            root,
            source,
            img_source,
        }
    }

    pub(super) fn write_source(&self, rel_path: &str, contents: &str) {
        let path = self.source.join(rel_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    pub(super) fn write_image(&self, rel_path: &str, contents: &str) {
        let path = self.img_source.join(rel_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    pub(super) fn write_project(&self, rel_path: &str, contents: &str) {
        let path = self.root.join(rel_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    pub(super) fn check(&self) -> CheckReport {
        check(
            self.source.to_str().unwrap(),
            self.img_source.to_str().unwrap(),
        )
    }
}

impl Drop for TestSite {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(super) fn count_kind(report: &CheckReport, kind: FindingKind) -> usize {
    report.findings.iter().filter(|f| f.kind == kind).count()
}

pub(super) fn has_kind_at(report: &CheckReport, kind: FindingKind, location: &str) -> bool {
    report
        .findings
        .iter()
        .any(|f| f.kind == kind && f.location == location)
}

pub(super) fn date_ordered_blog(post_json: &str) -> TestSite {
    let site = TestSite::new("date_ordered_blog");
    site.write_source("main.json", VALID_MAIN_JSON);
    site.write_source("menu.json", VALID_MENU_JSON);
    site.write_source("page.json", VALID_PAGE_JSON);
    site.write_source(
        "blog/page.json",
        r#"{"title":"Blog","content":"<p>hi</p>","childOrder":"dateDesc"}"#,
    );
    site.write_source("blog/post/page.json", post_json);
    site
}

pub(super) fn main_json_with_site_url(site_url: &str) -> String {
    VALID_MAIN_JSON.replacen(
        r#""footerContent""#,
        &format!(r#""siteUrl": "{site_url}", "footerContent""#),
        1,
    )
}

pub(super) fn feed_site(name: &str, main_json: &str, post_json: &str) -> TestSite {
    let site = TestSite::new(name);
    site.write_source("main.json", main_json);
    site.write_source("menu.json", VALID_MENU_JSON);
    site.write_source("page.json", VALID_PAGE_JSON);
    site.write_source(
        "blog/page.json",
        r#"{"title":"Blog","content":"<p>hi</p>","childOrder":"dateDesc","feed":true}"#,
    );
    site.write_source("blog/post/page.json", post_json);
    site
}

pub(super) const DATED_POST: &str =
    r#"{"title":"Post","content":"<p>hi</p>","date":"2026-10-22","summary":"Hi"}"#;
