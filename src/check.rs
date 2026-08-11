//! The `check` command: a pre-flight validation pass over the source and
//! image trees. It walks exactly the trees `generate` would and declares the
//! same things from them, per `vados.allium`'s `SourceTree` surface, but it
//! never writes anything and never decodes an image. Its whole point is to
//! find every problem in one pass instead of stopping at the first one, so
//! every check below runs regardless of what earlier checks found.

use crate::config_files::{ImageList, MainConfig, MenuConfig, Notification, PageConfig};
use crate::content::{classify_content_reference, get_file_path, ContentKind};
use crate::files::get_all_directory_paths;
use crate::image::reference_key_for;
use crate::structure::is_recognized_social_provider;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::path::Path;

/// Whether a finding fails the check. Fixed policy, mirroring
/// `config.error_finding_kinds` in the spec: a finding is an error when it
/// breaks a stated invariant or when a real `generate` run would abort on
/// it; everything `generate` tolerates by skipping or degrading is a
/// warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        })
    }
}

/// What kind of problem was found. Each kind names one authoring mistake the
/// maintainer can act on; see `vados.allium`'s `FindingKind` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    SiteConfigUnreadable,
    MenuUnreadable,
    PageConfigUnreadable,
    ImagesManifestUnreadable,
    UnrecognisedContentReference,
    MissingContentFile,
    ExternalMenuLinkWithoutTitle,
    UnresolvedMenuLink,
    UnrecognisedSocialLinkIncomplete,
    DuplicateImageReference,
    ImageWithoutAlternativeText,
    UnknownPageImageReference,
    UnknownNotificationImageReference,
    MissingImageSourceFile,
    DeadInternalNotificationLink,
}

impl FindingKind {
    pub fn severity(self) -> Severity {
        use FindingKind::*;
        match self {
            SiteConfigUnreadable
            | MenuUnreadable
            | PageConfigUnreadable
            | UnrecognisedContentReference
            | ExternalMenuLinkWithoutTitle
            | UnrecognisedSocialLinkIncomplete
            | DuplicateImageReference
            | ImageWithoutAlternativeText => Severity::Error,
            ImagesManifestUnreadable
            | MissingContentFile
            | UnresolvedMenuLink
            | UnknownPageImageReference
            | UnknownNotificationImageReference
            | MissingImageSourceFile
            | DeadInternalNotificationLink => Severity::Warning,
        }
    }
}

impl fmt::Display for FindingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use FindingKind::*;
        f.write_str(match self {
            SiteConfigUnreadable => "main.json could not be read",
            MenuUnreadable => "menu.json could not be read",
            PageConfigUnreadable => "page.json could not be read",
            ImagesManifestUnreadable => "images.json could not be read",
            UnrecognisedContentReference => {
                "content reference is not a Markdown file, an HTML file, or inline HTML"
            }
            MissingContentFile => "content file does not exist",
            ExternalMenuLinkWithoutTitle => "external menu link has no title",
            UnresolvedMenuLink => "menu link matches no page",
            UnrecognisedSocialLinkIncomplete => {
                "unrecognised social provider is missing an icon, a color, or an accessible-name label"
            }
            DuplicateImageReference => "two images declare the same reference key",
            ImageWithoutAlternativeText => "image has no alternative text",
            UnknownPageImageReference => "page image reference matches no declared image",
            UnknownNotificationImageReference => {
                "notification image reference matches no declared image"
            }
            MissingImageSourceFile => "image source file does not exist",
            DeadInternalNotificationLink => "notification link matches no page",
        })
    }
}

/// One problem `check` found: a record, not an exception. Findings are
/// collected and carried to the end of the run, so finding one thing wrong
/// never prevents finding the next.
#[derive(Debug, Clone)]
pub struct Finding {
    pub kind: FindingKind,
    /// Where the maintainer should look: a file path, a page path or a url.
    pub location: String,
    /// The offending value, where naming it helps.
    pub detail: Option<String>,
}

impl Finding {
    pub fn severity(&self) -> Severity {
        self.kind.severity()
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:<7} {} ({}", self.severity(), self.kind, self.location)?;
        if let Some(detail) = &self.detail {
            write!(f, ", {}", detail)?;
        }
        write!(f, ")")
    }
}

/// Everything a check run found: the whole list, and the counts and verdict
/// derived from it.
pub struct CheckReport {
    pub findings: Vec<Finding>,
}

impl CheckReport {
    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity() == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity() == Severity::Warning)
            .count()
    }

    /// The check passes exactly when nothing error-severity was found.
    /// Warnings are reported and counted but never change this.
    pub fn passed(&self) -> bool {
        self.error_count() == 0
    }
}

struct CheckedImage {
    reference_key: String,
    source_file: String,
    alt_text: String,
}

/// Runs the check pass: reads `source` and `img_source` exactly as `generate`
/// would, and reports every problem found instead of writing a site.
pub fn check(source: &str, img_source: &str) -> CheckReport {
    let mut findings = Vec::new();

    // Unreadable site configuration is the one problem that could strand the
    // rest of the check: everything else is still walked and still reported,
    // and only the footer's content reference (which lives in main.json)
    // goes unchecked without it.
    let main_config = read_json::<MainConfig>(
        &format!("{}/main.json", source),
        FindingKind::SiteConfigUnreadable,
        &mut findings,
    );
    let menu_config = read_json::<MenuConfig>(
        &format!("{}/menu.json", source),
        FindingKind::MenuUnreadable,
        &mut findings,
    );

    let images = collect_images(img_source, &mut findings);
    findings.extend(duplicate_image_findings(&images));
    for image in &images {
        if image.alt_text.trim().is_empty() {
            findings.push(Finding {
                kind: FindingKind::ImageWithoutAlternativeText,
                location: image.source_file.clone(),
                detail: Some(image.reference_key.clone()),
            });
        }
        if !Path::new(&image.source_file).exists() {
            findings.push(Finding {
                kind: FindingKind::MissingImageSourceFile,
                location: image.source_file.clone(),
                detail: Some(image.reference_key.clone()),
            });
        }
    }
    let image_keys: HashSet<&str> = images.iter().map(|i| i.reference_key.as_str()).collect();

    let pages = collect_pages(source, &mut findings);
    let page_paths: HashSet<&str> = pages.keys().map(|p| p.as_str()).collect();

    if let Some(menu_config) = &menu_config {
        for item in &menu_config.main_menu {
            if item.url.starts_with("https://") {
                if item.title.is_none() {
                    findings.push(Finding {
                        kind: FindingKind::ExternalMenuLinkWithoutTitle,
                        location: item.url.clone(),
                        detail: None,
                    });
                }
            } else if !page_paths.contains(item.url.as_str()) {
                findings.push(Finding {
                    kind: FindingKind::UnresolvedMenuLink,
                    location: item.url.clone(),
                    detail: None,
                });
            }
        }
        for social in &menu_config.socials {
            if !is_recognized_social_provider(&social.url)
                && (social.icon.is_none() || social.color.is_none() || social.label.is_none())
            {
                findings.push(Finding {
                    kind: FindingKind::UnrecognisedSocialLinkIncomplete,
                    location: social.url.clone(),
                    detail: None,
                });
            }
        }
    }

    for (path, page_config) in &pages {
        check_content_reference(source, path, &page_config.content, &mut findings);
        if let Some(image_ref) = &page_config.image {
            if !image_keys.contains(image_ref.as_str()) {
                findings.push(Finding {
                    kind: FindingKind::UnknownPageImageReference,
                    location: path.clone(),
                    detail: Some(image_ref.clone()),
                });
            }
        }
        for notification in page_config
            .left_notifications
            .iter()
            .flatten()
            .chain(page_config.right_notifications.iter().flatten())
        {
            check_notification(
                source,
                path,
                notification,
                &page_paths,
                &image_keys,
                &mut findings,
            );
        }
    }

    if let Some(main_config) = &main_config {
        check_content_reference(source, "/", &main_config.footer_content, &mut findings);
    }

    CheckReport { findings }
}

fn read_json<T: serde::de::DeserializeOwned>(
    path: &str,
    unreadable_kind: FindingKind,
    findings: &mut Vec<Finding>,
) -> Option<T> {
    let result = File::open(path).map_err(|e| e.to_string()).and_then(|f| {
        serde_json::from_reader(f).map_err(|e| e.to_string())
    });
    match result {
        Ok(value) => Some(value),
        Err(detail) => {
            findings.push(Finding {
                kind: unreadable_kind,
                location: String::from(path),
                detail: Some(detail),
            });
            None
        }
    }
}

fn collect_images(img_source: &str, findings: &mut Vec<Finding>) -> Vec<CheckedImage> {
    let mut images = Vec::new();
    for dir in get_all_directory_paths(img_source) {
        let path = format!("{}/images.json", dir);
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => continue, // images.json is optional per directory
        };
        let list: ImageList = match serde_json::from_reader(file) {
            Ok(l) => l,
            Err(e) => {
                findings.push(Finding {
                    kind: FindingKind::ImagesManifestUnreadable,
                    location: path,
                    detail: Some(e.to_string()),
                });
                continue;
            }
        };
        let path_start = &dir[img_source.len()..];
        for reference in list.list {
            let source_file = format!("{}/{}", dir, reference.file_name);
            match reference_key_for(path_start, &reference.file_name) {
                Some(reference_key) => images.push(CheckedImage {
                    reference_key,
                    source_file,
                    alt_text: reference.alt_text,
                }),
                None => findings.push(Finding {
                    kind: FindingKind::ImagesManifestUnreadable,
                    location: source_file,
                    detail: Some(String::from("file name has no extension")),
                }),
            }
        }
    }
    images
}

fn duplicate_image_findings(images: &[CheckedImage]) -> Vec<Finding> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for image in images {
        *counts.entry(image.reference_key.as_str()).or_insert(0) += 1;
    }
    images
        .iter()
        .filter(|i| counts[i.reference_key.as_str()] > 1)
        .map(|i| Finding {
            kind: FindingKind::DuplicateImageReference,
            location: i.source_file.clone(),
            detail: Some(i.reference_key.clone()),
        })
        .collect()
}

fn collect_pages(source: &str, findings: &mut Vec<Finding>) -> HashMap<String, PageConfig> {
    let mut pages = HashMap::new();
    for dir in get_all_directory_paths(source) {
        let page_path = if dir == source {
            String::from("/")
        } else {
            dir[source.len()..].to_string()
        };
        let page_json = format!("{}/page.json", dir);
        let config = match File::open(&page_json) {
            Ok(f) => match serde_json::from_reader(f) {
                Ok(config) => config,
                Err(e) => {
                    findings.push(Finding {
                        kind: FindingKind::PageConfigUnreadable,
                        location: page_json,
                        detail: Some(e.to_string()),
                    });
                    PageConfig::new(&dir)
                }
            },
            Err(_) => PageConfig::new(&dir), // page.json is optional per directory
        };
        pages.insert(page_path, config);
    }
    pages
}

fn check_content_reference(
    source: &str,
    location: &str,
    reference: &str,
    findings: &mut Vec<Finding>,
) {
    match classify_content_reference(reference) {
        ContentKind::Unrecognised => findings.push(Finding {
            kind: FindingKind::UnrecognisedContentReference,
            location: String::from(location),
            detail: Some(String::from(reference)),
        }),
        ContentKind::InlineHtml => {}
        ContentKind::MarkdownFile | ContentKind::HtmlFile => {
            let file_path = get_file_path(source, location, reference);
            if !Path::new(&file_path).exists() {
                findings.push(Finding {
                    kind: FindingKind::MissingContentFile,
                    location: String::from(location),
                    detail: Some(String::from(reference)),
                });
            }
        }
    }
}

fn check_notification(
    source: &str,
    page_path: &str,
    notification: &Notification,
    page_paths: &HashSet<&str>,
    image_keys: &HashSet<&str>,
    findings: &mut Vec<Finding>,
) {
    check_content_reference(source, page_path, &notification.content, findings);
    if let Some(image_ref) = &notification.image {
        if !image_keys.contains(image_ref.as_str()) {
            findings.push(Finding {
                kind: FindingKind::UnknownNotificationImageReference,
                location: String::from(page_path),
                detail: Some(image_ref.clone()),
            });
        }
    }
    if let Some(url) = &notification.url {
        if url.starts_with('/') && !page_paths.contains(url.as_str()) {
            findings.push(Finding {
                kind: FindingKind::DeadInternalNotificationLink,
                location: String::from(page_path),
                detail: Some(url.clone()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    const VALID_MAIN_JSON: &str = r#"{
        "siteTitle": "Test site",
        "jsFiles": [],
        "includeDefaultJs": null,
        "cssFiles": [],
        "includeDefaultCss": null,
        "backgroundClass": null,
        "navbarColor": null,
        "footerContent": "<p>footer</p>"
    }"#;

    const VALID_MENU_JSON: &str = r#"{
        "mainMenu": [],
        "socials": []
    }"#;

    const VALID_PAGE_JSON: &str = r#"{
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
    struct TestSite {
        root: PathBuf,
        source: PathBuf,
        img_source: PathBuf,
    }

    impl TestSite {
        fn new(name: &str) -> Self {
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

        fn write_source(&self, rel_path: &str, contents: &str) {
            let path = self.source.join(rel_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        fn write_image(&self, rel_path: &str, contents: &str) {
            let path = self.img_source.join(rel_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        fn check(&self) -> CheckReport {
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

    fn count_kind(report: &CheckReport, kind: FindingKind) -> usize {
        report.findings.iter().filter(|f| f.kind == kind).count()
    }

    fn has_kind_at(report: &CheckReport, kind: FindingKind, location: &str) -> bool {
        report
            .findings
            .iter()
            .any(|f| f.kind == kind && f.location == location)
    }

    #[test]
    fn valid_minimal_site_has_no_findings() {
        let site = TestSite::new("valid_minimal");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(
            report.findings.is_empty(),
            "expected no findings, got {:?}",
            report.findings
        );
        assert!(report.passed());
        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 0);
    }

    #[test]
    fn missing_main_and_menu_json_are_errors() {
        let site = TestSite::new("missing_main_menu");
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::SiteConfigUnreadable), 1);
        assert_eq!(count_kind(&report, FindingKind::MenuUnreadable), 1);
        assert!(!report.passed());
        assert_eq!(report.error_count(), 2);
    }

    #[test]
    fn page_config_unreadable_is_an_error_but_check_continues() {
        let site = TestSite::new("page_unreadable");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", "{ not valid json");

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::PageConfigUnreadable), 1);
        assert!(!report.passed());
        // The unreadable page.json still yields a page (falling back to the
        // directory-derived default), so the footer's content reference is
        // still checked rather than the whole run being stranded.
        assert_eq!(
            count_kind(&report, FindingKind::UnrecognisedContentReference),
            0
        );
    }

    #[test]
    fn unresolved_internal_menu_link_is_a_warning() {
        let site = TestSite::new("unresolved_menu_link");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[{"url":"/missing","title":null,"icon":null}],"socials":[]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnresolvedMenuLink,
            "/missing"
        ));
        assert_eq!(
            FindingKind::UnresolvedMenuLink.severity(),
            Severity::Warning
        );
        assert!(report.passed());
    }

    #[test]
    fn external_menu_link_without_title_is_an_error() {
        let site = TestSite::new("external_link_no_title");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[{"url":"https://example.com","title":null,"icon":null}],"socials":[]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::ExternalMenuLinkWithoutTitle,
            "https://example.com"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn unrecognised_social_provider_missing_icon_and_color_is_an_error() {
        let site = TestSite::new("social_unrecognised");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://mastodon.social/@someone","icon":null,"color":null}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnrecognisedSocialLinkIncomplete,
            "https://mastodon.social/@someone"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn unrecognised_social_provider_missing_only_the_label_is_an_error() {
        // icon and color are given; the accessible-name label is not. It is
        // just as required as the other two -- see
        // vados.allium's UnrecognisedSocialLinkIsFullyDescribed.
        let site = TestSite::new("social_missing_label");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://mastodon.social/@someone","icon":"mastodon","color":"6364FF"}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnrecognisedSocialLinkIncomplete,
            "https://mastodon.social/@someone"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn unrecognised_social_provider_with_icon_color_and_label_is_not_flagged() {
        let site = TestSite::new("social_fully_described");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://mastodon.social/@someone","icon":"mastodon","color":"6364FF","label":"Mastodon"}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::UnrecognisedSocialLinkIncomplete),
            0
        );
        assert!(report.passed());
    }

    #[test]
    fn recognized_social_provider_without_icon_is_not_flagged() {
        let site = TestSite::new("social_recognized");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://github.com/someone","icon":null,"color":null}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::UnrecognisedSocialLinkIncomplete),
            0
        );
        assert!(report.passed());
    }

    #[test]
    fn unrecognised_content_reference_is_an_error() {
        let site = TestSite::new("content_unrecognised");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"not-a-recognised-reference","order":null,
               "leftNotifications":null,"rightNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnrecognisedContentReference,
            "/"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn missing_content_file_is_a_warning() {
        let site = TestSite::new("content_missing_file");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"missing.md","order":null,
               "leftNotifications":null,"rightNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::MissingContentFile,
            "/"
        ));
        assert!(report.passed());
    }

    #[test]
    fn duplicate_image_reference_is_an_error_for_each_offender() {
        let site = TestSite::new("image_duplicate");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_image(
            "images.json",
            r#"{"title":null,"list":[
                {"title":null,"fileName":"photo.jpg","altText":"a photo"},
                {"title":null,"fileName":"photo.png","altText":"a photo again"}
            ]}"#,
        );
        site.write_image("photo.jpg", "not really a jpg");
        site.write_image("photo.png", "not really a png");

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::DuplicateImageReference), 2);
        assert!(!report.passed());
    }

    #[test]
    fn image_without_alternative_text_is_an_error() {
        let site = TestSite::new("image_no_alt");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_image(
            "images.json",
            r#"{"title":null,"list":[{"title":null,"fileName":"photo.jpg","altText":""}]}"#,
        );
        site.write_image("photo.jpg", "not really a jpg");

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::ImageWithoutAlternativeText),
            1
        );
        assert!(!report.passed());
    }

    #[test]
    fn missing_image_source_file_is_a_warning() {
        let site = TestSite::new("image_missing_source");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_image(
            "images.json",
            r#"{"title":null,"list":[{"title":null,"fileName":"ghost.jpg","altText":"a ghost"}]}"#,
        );
        // Deliberately not writing images/ghost.jpg.

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::MissingImageSourceFile),
            1
        );
        assert!(report.passed());
    }

    #[test]
    fn unknown_page_image_reference_is_a_warning() {
        let site = TestSite::new("page_image_unknown");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":"/nope","icon":null,"summary":null,
               "content":"<p>hi</p>","order":null,
               "leftNotifications":null,"rightNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnknownPageImageReference,
            "/"
        ));
        assert!(report.passed());
    }

    #[test]
    fn unknown_notification_image_reference_is_a_warning() {
        let site = TestSite::new("notification_image_unknown");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"<p>hi</p>","order":null,
               "leftNotifications":[{"content":"<p>n</p>","title":null,"image":"/nope","url":null,"color":null}],
               "rightNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnknownNotificationImageReference,
            "/"
        ));
        assert!(report.passed());
    }

    #[test]
    fn dead_internal_notification_link_is_a_warning() {
        let site = TestSite::new("notification_dead_link");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"<p>hi</p>","order":null,
               "rightNotifications":[{"content":"<p>n</p>","title":null,"image":null,"url":"/missing","color":null}],
               "leftNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::DeadInternalNotificationLink,
            "/"
        ));
        assert!(report.passed());
    }

    #[test]
    fn images_manifest_unreadable_is_a_warning() {
        let site = TestSite::new("images_manifest_unreadable");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_image("images.json", "{ not valid json");

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::ImagesManifestUnreadable),
            1
        );
        assert!(report.passed());
    }
}
