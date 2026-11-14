//! Serial/parallel publication must agree on bytes, diagnostics and failures.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Site(PathBuf);
impl Site {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "vados_parallel_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir_all(root.join("images")).unwrap();
        Self(root)
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn run(&self, jobs: usize, dest: &str) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_vados"))
            .args(["generate", "--source"])
            .arg(self.0.join("source"))
            .arg("--img-source")
            .arg(self.0.join("images"))
            .arg("--destination")
            .arg(self.0.join(dest))
            .arg("--jobs")
            .arg(jobs.to_string())
            .output()
            .unwrap()
    }
}
impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            (
                entry.path().strip_prefix(root).unwrap().to_owned(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

#[test]
fn serial_parallel_and_warm_publications_match() {
    let site = Site::new();
    site.write("source/main.json", r#"{"siteTitle":"Test","footerContent":"<p>Footer</p>","siteUrl":"https://example.com","markdown":true,"mermaid":true,"search":{},"staticDir":"static"}"#);
    // Missing menu target produces a renderer diagnostic for every page.
    site.write(
        "source/menu.json",
        r#"{"mainMenu":[{"url":"/missing"}],"socials":[]}"#,
    );
    site.write(
        "source/page.json",
        r#"{"title":"Home","content":"body.md","image":"/a"}"#,
    );
    site.write(
        "source/body.md",
        "```rust\nfn main() {}\n```\n\n```mermaid\ngraph TD; A-->B\n```\n",
    );
    site.write("source/static/index.html", "static loses");
    for i in 0..12 {
        site.write(
            &format!("source/p{i}/page.json"),
            &format!(r#"{{"title":"Page {i}","content":"missing.md","image":"/b"}}"#),
        );
    }
    site.write(
        "images/images.json",
        r#"{"list":[{"fileName":"a.png","altText":"a"},{"fileName":"b.png","altText":"b"}]}"#,
    );
    for name in ["a", "b"] {
        image::RgbaImage::from_pixel(64, 48, image::Rgba([30, 150, 200, 128]))
            .save(site.0.join(format!("images/{name}.png")))
            .unwrap();
    }
    let serial = site.run(1, "serial");
    let parallel = site.run(4, "parallel");
    assert!(serial.status.success(), "{:?}", serial);
    assert!(parallel.status.success(), "{:?}", parallel);
    assert_eq!(
        String::from_utf8_lossy(&serial.stdout),
        String::from_utf8_lossy(&parallel.stdout)
    );
    assert_eq!(serial.stderr, parallel.stderr);
    let expected = tree(&site.0.join("serial"));
    assert_eq!(expected, tree(&site.0.join("parallel")));
    let warm = site.run(4, "parallel");
    assert!(warm.status.success());
    assert_eq!(serial.stdout, warm.stdout);
    assert_eq!(expected, tree(&site.0.join("parallel")));

    // Exercise both historical output collisions with distinguishable metadata.
    site.write(
        "source/index/page.json",
        r#"{"title":"Index collision","content":"<p>Index</p>"}"#,
    );
    site.write("images/images.json", r#"{"list":[{"fileName":"a.png","altText":"first"},{"fileName":"a.jpg","altText":"last"}]}"#);
    fs::copy(site.0.join("images/a.png"), site.0.join("images/a.jpg")).unwrap();
    assert!(site.run(1, "serial").status.success());
    assert!(site.run(4, "parallel").status.success());
    assert_eq!(tree(&site.0.join("serial")), tree(&site.0.join("parallel")));
}

#[test]
fn parallel_image_failures_are_ordered_and_do_not_advance_manifest() {
    let site = Site::new();
    site.write(
        "source/main.json",
        r#"{"siteTitle":"Test","footerContent":"<p>Footer</p>"}"#,
    );
    site.write("source/menu.json", r#"{"mainMenu":[],"socials":[]}"#);
    site.write(
        "source/page.json",
        r#"{"title":"Home","content":"<p>Home</p>"}"#,
    );
    assert!(site.run(1, "public").status.success());
    let manifest = fs::read(site.0.join("public/.vados-manifest.json")).unwrap();
    site.write("public/untracked.txt", "keep");
    site.write("images/images.json", r#"{"list":[{"fileName":"first.png","altText":"first"},{"fileName":"second.png","altText":"second"}]}"#);
    site.write("images/first.png", "invalid pixels");
    site.write("images/second.png", "also invalid");
    let serial = site.run(1, "public");
    for _ in 0..3 {
        let parallel = site.run(4, "public");
        assert!(!parallel.status.success());
        assert_eq!(serial.stderr, parallel.stderr);
        assert_eq!(
            manifest,
            fs::read(site.0.join("public/.vados-manifest.json")).unwrap()
        );
        assert_eq!(
            "keep",
            fs::read_to_string(site.0.join("public/untracked.txt")).unwrap()
        );
    }
    let invalid_jobs = site.run(0, "public");
    assert!(!invalid_jobs.status.success());
}
