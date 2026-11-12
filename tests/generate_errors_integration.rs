//! Generation failures are actionable CLI diagnostics, never Rust panics.
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use vados::generator::generate;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct Site(PathBuf);
impl Site {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "vados_generate_errors_{}_{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir_all(root.join("images")).unwrap();
        let site = Self(root);
        site.write(
            "source/main.json",
            r#"{"siteTitle":"Test","jsFiles":[],"cssFiles":[],"footerContent":"<p>Footer</p>"}"#,
        );
        site.write("source/menu.json", r#"{"mainMenu":[],"socials":[]}"#);
        site.write(
            "source/page.json",
            r#"{"title":"Home","content":"<p>Home</p>"}"#,
        );
        site
    }
    fn write(&self, path: &str, contents: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    fn run(&self) -> (bool, String) {
        let output = Command::new(env!("CARGO_BIN_EXE_vados"))
            .args(["generate", "--source"])
            .arg(self.0.join("source"))
            .arg("--img-source")
            .arg(self.0.join("images"))
            .arg("--destination")
            .arg(self.0.join("public"))
            .env("RUST_BACKTRACE", "1")
            .output()
            .unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!text.contains("panicked"), "{text}");
        (output.status.success(), text)
    }
    fn assert_failure(&self, path: &str) {
        let (success, text) = self.run();
        assert!(!success, "generation succeeded: {text}");
        assert!(text.contains(path), "missing path {path}: {text}");
        assert!(text.contains("error"), "{text}");
    }
}
impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn malformed_and_missing_configuration_files_are_named() {
    for file in ["main.json", "menu.json", "page.json"] {
        let site = Site::new();
        site.write(&format!("source/{file}"), "{ invalid json");
        site.assert_failure(file);
        assert!(!site.0.join("public").exists());
    }
    for file in ["main.json", "menu.json"] {
        let site = Site::new();
        fs::remove_file(site.0.join("source").join(file)).unwrap();
        site.assert_failure(file);
    }
}

#[test]
fn reports_multiple_bad_files_and_all_missing_main_fields_together() {
    let site = Site::new();
    site.write("source/main.json", "{}");
    site.write("source/menu.json", "{}");
    site.write("source/blog/page.json", "{");
    let (success, text) = site.run();
    assert!(!success);
    for name in [
        "main.json",
        "menu.json",
        "blog/page.json",
        "siteTitle",
        "footerContent",
        "jsFiles",
        "cssFiles",
        "mainMenu",
        "socials",
    ] {
        assert!(text.contains(name), "missing {name}: {text}");
    }
    assert!(!site.0.join("public").exists());
}

#[test]
fn broken_page_leaves_every_existing_output_file_untouched() {
    let site = Site::new();
    assert!(site.run().0);
    let before: Vec<_> = ["index.html", ".vados-manifest.json", "robots.txt"]
        .map(|name| (name, fs::read(site.0.join("public").join(name)).unwrap()))
        .into();
    site.write("source/deep/broken/page.json", "{");
    site.write(
        "source/page.json",
        r#"{"title":"Changed","content":"<p>changed</p>"}"#,
    );
    site.assert_failure("deep/broken/page.json");
    for (name, contents) in before {
        assert_eq!(
            contents,
            fs::read(site.0.join("public").join(name)).unwrap()
        );
    }
    assert!(!site.0.join("public/deep").exists());
}

#[test]
fn unrecognised_content_and_incomplete_socials_are_named() {
    let site = Site::new();
    site.write(
        "source/blog/page.json",
        r#"{"title":"Blog","content":"unknown content"}"#,
    );
    site.assert_failure("blog/page.json");
    let site = Site::new();
    site.write("source/page.json", r#"{"title":"Home","content":"<p>Home</p>","leftNotifications":[{"content":"unsupported"}]}"#);
    site.assert_failure("page.json");
    let site = Site::new();
    site.write(
        "source/main.json",
        r#"{"siteTitle":"Test","jsFiles":[],"cssFiles":[],"footerContent":"unsupported"}"#,
    );
    site.assert_failure("main.json");
    let site = Site::new();
    site.write(
        "source/menu.json",
        r#"{"mainMenu":[],"socials":[{"url":"https://example.com/me"}]}"#,
    );
    site.assert_failure("menu.json");
    let site = Site::new();
    site.write(
        "source/menu.json",
        r#"{"mainMenu":[{"url":"https://example.com"}],"socials":[]}"#,
    );
    site.assert_failure("menu.json");
}

#[test]
fn bad_image_manifest_missing_image_and_corrupt_image_fail() {
    let site = Site::new();
    site.write("images/images.json", "{");
    site.assert_failure("images.json");
    let site = Site::new();
    site.write(
        "images/images.json",
        r#"{"list":[{"fileName":"broken.png","altText":"A picture"}]}"#,
    );
    site.assert_failure("broken.png");
    site.write("images/broken.png", "not an image");
    site.assert_failure("broken.png");
}

#[test]
fn output_creation_writing_and_manifest_errors_name_the_path() {
    let site = Site::new();
    site.write("public", "a file blocks the destination");
    site.assert_failure("public");
    let site = Site::new();
    site.write("public/js", "a file blocks a directory");
    site.assert_failure("public/js");
    let site = Site::new();
    fs::create_dir_all(site.0.join("public/index.html")).unwrap();
    site.assert_failure("index.html");
    let site = Site::new();
    site.write("public/.vados-manifest.json", "{");
    site.assert_failure(".vados-manifest.json");
}

#[test]
fn missing_or_invalid_source_directories_are_named() {
    let site = Site::new();
    fs::remove_dir(site.0.join("images")).unwrap();
    site.assert_failure("images");
    site.write("images", "not a directory");
    site.assert_failure("images");
}

#[cfg(unix)]
#[test]
fn unreadable_inputs_and_directories_are_named() {
    use std::os::unix::fs::PermissionsExt;
    for path in [
        "source/main.json",
        "source/menu.json",
        "source/page.json",
        "source/private",
        "images/private",
        "images/images.json",
    ] {
        let site = Site::new();
        let full = site.0.join(path);
        if path.ends_with("private") {
            fs::create_dir_all(&full).unwrap();
        } else if path == "images/images.json" {
            site.write(path, "{\"list\":[]}");
        }
        let original = fs::metadata(&full).unwrap().permissions();
        fs::set_permissions(&full, fs::Permissions::from_mode(0o000)).unwrap();
        // Some privileged environments can read mode-000 files.
        let blocked = if full.is_dir() {
            fs::read_dir(&full).is_err()
        } else {
            fs::read(&full).is_err()
        };
        // Restore permissions even when an assertion fails, so cleanup works.
        let output = site.run();
        fs::set_permissions(&full, original).unwrap();
        if !blocked {
            continue;
        }
        assert!(!output.0, "{path}: {}", output.1);
        assert!(output.1.contains(path), "{path}: {}", output.1);
    }
}

#[test]
fn invalid_events_are_omitted_and_returned_as_counted_warnings() {
    let site = Site::new();
    site.write("source/main.json", r#"{"siteTitle":"Test","jsFiles":[],"cssFiles":[],"footerContent":"<p>Footer</p>","siteUrl":"https://example.com"}"#);
    site.write("source/page.json", r#"{"title":"Home","content":"<p>Home</p>","event":{"start":"bad","timeZone":"UTC"},"lumaEvents":[{"eventId":"bad","title":"Bad"}]}"#);
    let report = generate(
        site.0.join("source").to_str().unwrap(),
        site.0.join("images").to_str().unwrap(),
        site.0.join("public").to_str().unwrap(),
    )
    .unwrap();
    assert!(report.warnings.iter().any(|w| w.message.contains("event")));
    assert!(report.warnings.iter().any(|w| w.message.contains("Luma")));
    let (success, text) = site.run();
    assert!(success, "{text}");
    assert!(
        text.contains(&format!(
            "generation passed: {} warning(s)",
            report.warnings.len()
        )),
        "{text}"
    );
    let page = fs::read_to_string(site.0.join("public/index.html")).unwrap();
    assert!(!page.contains("\"@type\":\"Event\""));
    assert!(!site.0.join("public/event.ics").exists());
}

#[test]
fn cleanup_failure_is_reported_without_advancing_the_manifest() {
    let site = Site::new();
    site.write(
        "source/gone/page.json",
        r#"{"title":"Gone","content":"<p>Gone</p>"}"#,
    );
    assert!(site.run().0);
    let manifest = fs::read(site.0.join("public/.vados-manifest.json")).unwrap();
    fs::remove_dir_all(site.0.join("source/gone")).unwrap();
    fs::remove_file(site.0.join("public/gone.html")).unwrap();
    fs::create_dir(site.0.join("public/gone.html")).unwrap();
    site.assert_failure("gone.html");
    assert_eq!(
        manifest,
        fs::read(site.0.join("public/.vados-manifest.json")).unwrap()
    );
}
