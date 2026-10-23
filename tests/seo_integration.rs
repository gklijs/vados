//! End-to-end coverage for what a published site tells search engines, link
//! previews and feed readers about itself: generates a small hand-written
//! site with a site URL, a dated post with a hero image and a default image,
//! and reads back the published HTML and files.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use vados::check::check;
use vados::generator::generate;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct TestSite {
    root: PathBuf,
    source: PathBuf,
    img_source: PathBuf,
    destination: PathBuf,
}

impl TestSite {
    fn new(name: &str) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("vados_seo_integration_{name}_{id}"));
        let _ = fs::remove_dir_all(&root);
        let site = TestSite {
            source: root.join("root"),
            img_source: root.join("imgroot"),
            destination: root.join("public"),
            root,
        };
        fs::create_dir_all(&site.source).unwrap();
        fs::create_dir_all(&site.img_source).unwrap();
        site
    }

    fn write_source(&self, rel_path: &str, contents: &str) {
        write(&self.source.join(rel_path), contents);
    }

    /// A real, decodable image of the given size, declared in images.json.
    fn write_image(&self, file_name: &str, width: u32, height: u32, alt_text: &str) {
        let image = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        image.save(self.img_source.join(file_name)).unwrap();
        let manifest = self.img_source.join("images.json");
        let mut list: serde_json::Value = fs::read_to_string(&manifest)
            .map(|s| serde_json::from_str(&s).unwrap())
            .unwrap_or_else(|_| serde_json::json!({ "title": null, "list": [] }));
        list["list"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "title": null, "fileName": file_name, "altText": alt_text
            }));
        fs::write(manifest, list.to_string()).unwrap();
    }

    fn check(&self) -> vados::check::CheckReport {
        check(
            self.source.to_str().unwrap(),
            self.img_source.to_str().unwrap(),
        )
    }

    fn generate(&self) {
        generate(
            self.source.to_str().unwrap(),
            self.img_source.to_str().unwrap(),
            self.destination.to_str().unwrap(),
        );
    }

    fn published(&self, rel_path: &str) -> String {
        fs::read_to_string(self.destination.join(rel_path))
            .unwrap_or_else(|e| panic!("{rel_path} was not published: {e}"))
    }
}

impl Drop for TestSite {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

const MENU_JSON: &str = r#"{ "mainMenu": [], "socials": [{ "url": "https://github.com/jo" }] }"#;

/// A blog with a dated post that has a hero image, and a site-wide default
/// image for every other page.
fn blog(name: &str, main_json_extra: &str) -> TestSite {
    let site = TestSite::new(name);
    site.write_source(
        "main.json",
        &format!(
            r#"{{
                "siteTitle": "My site",
                "jsFiles": [], "cssFiles": [],
                "language": "en-GB",
                "footerContent": "<p>footer</p>"
                {main_json_extra}
            }}"#
        ),
    );
    site.write_source("menu.json", MENU_JSON);
    site.write_source("page.json", r#"{"title":"Home","content":"<p>home</p>"}"#);
    site.write_source(
        "blog/page.json",
        r#"{"title":"Blog","content":"<p>posts</p>","childOrder":"dateDesc"}"#,
    );
    site.write_source(
        "blog/post/page.json",
        r#"{"title":"A post","content":"<p>text</p>","date":"2026-10-22T09:30",
            "summary":"What the post is about","image":"/batman"}"#,
    );
    site.write_image("batman.png", 1600, 1000, "Batman up close");
    site.write_image("logo.png", 400, 400, "The site's logo");
    site
}

#[test]
fn pages_describe_themselves_to_link_previews() {
    let site = blog(
        "link_previews",
        r#", "siteUrl": "https://example.com/", "defaultImage": "/logo""#,
    );
    let report = site.check();
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    site.generate();

    let post = site.published("blog/post.html");
    for tag in [
        r#"<link rel="canonical" href="https://example.com/blog/post">"#,
        r#"<meta property="og:url" content="https://example.com/blog/post""#,
        r#"<meta property="og:type" content="article""#,
        r#"<meta property="article:published_time" content="2026-10-22T09:30:00Z""#,
        r#"<meta property="og:locale" content="en_GB""#,
        r#"<meta property="og:image" content="https://example.com/img/batman-og.jpg""#,
        r#"<meta property="og:image:width" content="1200""#,
        r#"<meta property="og:image:height" content="630""#,
        r#"<meta property="og:image:alt" content="Batman up close""#,
        r#"<meta name="twitter:card" content="summary_large_image""#,
        r#"<meta name="robots" content="max-image-preview:large""#,
    ] {
        assert!(post.contains(tag), "missing {tag} in {post}");
    }
    let preview = image::open(site.destination.join("img/batman-og.jpg")).unwrap();
    assert_eq!((preview.width(), preview.height()), (1200, 630));

    // A page without a hero image of its own falls back to the default
    // image, which is smaller than a full preview and kept at its own size.
    let home = site.published("index.html");
    assert!(home.contains(r#"<link rel="canonical" href="https://example.com/">"#));
    assert!(home.contains(r#"<meta property="og:type" content="website""#));
    assert!(
        home.contains(r#"<meta property="og:image" content="https://example.com/img/logo-og.jpg""#)
    );
    assert!(home.contains(r#"<meta property="og:image:width" content="400""#));
    assert!(home.contains(r#"<meta property="og:image:height" content="210""#));
}

#[test]
fn without_a_site_url_pages_give_no_absolute_links() {
    let site = blog("no_site_url", "");
    let report = site.check();
    assert_eq!(
        report.findings.iter().map(|f| f.kind).collect::<Vec<_>>(),
        [vados::check::FindingKind::PreviewImageWithoutSiteUrl]
    );
    site.generate();

    let post = site.published("blog/post.html");
    assert!(!post.contains("canonical"));
    assert!(!post.contains("og:url"));
    assert!(!post.contains("og:image"));
    assert!(post.contains(r#"<meta name="twitter:card" content="summary""#));
    assert!(!site.destination.join("img/batman-og.jpg").exists());
}

/// The JSON-LD a published page carries, parsed.
fn structured_data(html: &str) -> serde_json::Value {
    let start = html
        .find("<script type='application/ld+json'>")
        .expect("the page carries structured data")
        + "<script type='application/ld+json'>".len();
    let end = start + html[start..].find("</script>").unwrap();
    serde_json::from_str(&html[start..end]).unwrap()
}

#[test]
fn dated_pages_and_the_home_page_carry_structured_data() {
    let site = blog(
        "structured_data",
        r#", "siteUrl": "https://example.com", "author": { "name": "Jo" }"#,
    );
    site.write_source(
        "blog/post/page.json",
        r#"{"title":"A post","content":"<p>text</p>","date":"2026-10-22T09:30",
            "updated":"2026-10-24","summary":"What the post is about","image":"/batman"}"#,
    );
    site.generate();

    let post = structured_data(&site.published("blog/post.html"));
    assert_eq!(post["@type"], "BlogPosting");
    assert_eq!(post["headline"], "A post");
    assert_eq!(post["description"], "What the post is about");
    assert_eq!(post["datePublished"], "2026-10-22T09:30:00Z");
    assert_eq!(post["dateModified"], "2026-10-24T00:00:00Z");
    assert_eq!(post["image"][0], "https://example.com/img/batman-og.jpg");
    assert_eq!(post["author"]["name"], "Jo");
    assert_eq!(post["author"]["url"], "https://example.com/");
    assert_eq!(post["author"]["sameAs"][0], "https://github.com/jo");
    assert_eq!(post["mainEntityOfPage"], "https://example.com/blog/post");

    let home = structured_data(&site.published("index.html"));
    assert_eq!(home["@graph"][0]["@type"], "WebSite");
    assert_eq!(home["@graph"][0]["name"], "My site");
    assert_eq!(home["@graph"][1]["name"], "Jo");

    assert!(!site.published("blog.html").contains("application/ld+json"));
}
