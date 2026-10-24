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

const MENU_JSON: &str = r#"{ "mainMenu": [], "socials": [
    { "url": "https://github.com/jo" },
    { "url": "https://hachyderm.io/@jo", "icon": "mastodon", "color": "6364FF", "label": "Mastodon" }
] }"#;

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

#[test]
fn a_site_follows_the_readers_colour_scheme_unless_pinned() {
    let site = blog("colour_scheme_default", "");
    site.generate();
    assert!(!site.published("index.html").contains("data-theme"));

    for scheme in ["light", "dark"] {
        let site = blog(
            &format!("colour_scheme_{scheme}"),
            &format!(r#", "colorScheme": "{scheme}""#),
        );
        site.generate();
        for page in ["index.html", "blog.html", "blog/post.html"] {
            let html = site.published(page);
            assert!(
                html.contains(&format!(r#"lang="en-GB" data-theme="{scheme}">"#)),
                "{page}"
            );
        }
    }
}

#[test]
fn the_last_side_notification_stretches_only_when_asked() {
    let side = r#"<div class="column is-flex is-flex-direction-column"#;
    let site = blog("stretch_default", "");
    site.generate();
    let html = site.published("blog/post.html");
    assert!(html.contains(side) && html.contains(r#"id="side-column""#));
    assert!(!html.contains("stretch-last\" id=\"side-column"));
    assert!(!html.contains("is-flex-grow-1\" href"));

    let site = blog("stretch_on", r#", "stretchSideNotification": true"#);
    site.generate();
    let html = site.published("blog/post.html");
    assert!(html.contains(r#"stretch-last" id="side-column""#));
    assert!(html.contains("#side-column.stretch-last>.notification:last-child{flex-grow:1;}"));
}

#[test]
fn an_unknown_colour_scheme_is_a_site_config_error() {
    let site = blog("colour_scheme_unknown", r#", "colorScheme": "sepia""#);
    let report = site.check();
    assert!(report
        .findings
        .iter()
        .any(|f| f.kind == vados::check::FindingKind::SiteConfigUnreadable));
    assert!(!report.passed());
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

#[test]
fn static_files_are_published_as_they_are_and_unlisted_pages_stay_unlisted() {
    let site = blog(
        "static_and_unlisted",
        r#", "staticDir": "static", "siteUrl": "https://example.com""#,
    );
    site.write_source("static/robots.txt", "User-agent: *\nDisallow: /drafts\n");
    site.write_source(
        "static/.well-known/security.txt",
        "Contact: mailto:jo@example.com\n",
    );
    site.write_source("static/blog.html", "replaced by the generated page");
    site.write_source("static/.gitkeep", "");
    site.write_source(
        "404/page.json",
        r#"{"title":"Not found","content":"<p>Nothing here.</p>","listed":false}"#,
    );
    site.write_source(
        "blog/draft/page.json",
        r#"{"title":"A draft","content":"<p>soon</p>","listed":false}"#,
    );

    let report = site.check();
    let kinds: Vec<_> = report.findings.iter().map(|f| f.kind).collect();
    assert!(kinds.contains(&vados::check::FindingKind::StaticFileShadowedByPage));
    // The static directory sits inside the source root but is no page.
    assert!(!site.destination.join("static.html").exists());
    site.generate();
    assert!(!site.destination.join("static.html").exists());

    assert_eq!(
        site.published("robots.txt"),
        "User-agent: *\nDisallow: /drafts\n"
    );
    assert_eq!(
        site.published(".well-known/security.txt"),
        "Contact: mailto:jo@example.com\n"
    );
    assert!(site.published("blog.html").contains("<h1"));
    assert!(!site.destination.join(".gitkeep").exists());
    // The static robots.txt wins over the generated one; the sitemap
    // leaves out the unlisted pages.
    let sitemap = site.published("sitemap.xml");
    assert!(sitemap.contains("<loc>https://example.com/blog/post</loc>"));
    assert!(!sitemap.contains("/404"));
    assert!(!sitemap.contains("/blog/draft"));

    // Unlisted pages are published in the site's own layout...
    let not_found = site.published("404.html");
    assert!(not_found.contains(r#"<h1 class="title is-1">Not found</h1>"#));
    assert!(not_found.contains("Nothing here."));
    assert!(site.published("blog/draft.html").contains("soon"));
    // ...but listed nowhere.
    for page in ["index.html", "blog.html", "blog/post.html"] {
        let html = site.published(page);
        assert!(
            !html.contains(r#"href="/404""#),
            "{page} links the 404 page"
        );
        assert!(
            !html.contains(r#"href="/blog/draft""#),
            "{page} links the draft"
        );
    }
}

#[test]
fn with_a_site_url_a_sitemap_and_robots_txt_are_published() {
    let site = blog("sitemap", r#", "siteUrl": "https://example.com/""#);
    site.generate();

    let sitemap = site.published("sitemap.xml");
    let doc = roxmltree::Document::parse(&sitemap).expect("the sitemap is well-formed XML");
    let urls: Vec<(String, Option<String>)> = doc
        .descendants()
        .filter(|n| n.has_tag_name("url"))
        .map(|url| {
            let child = |name| {
                url.children()
                    .find(|c| c.has_tag_name(name))
                    .and_then(|c| c.text())
                    .map(String::from)
            };
            (child("loc").unwrap(), child("lastmod"))
        })
        .collect();
    assert_eq!(
        urls,
        [
            ("https://example.com/".to_string(), None),
            ("https://example.com/blog".to_string(), None),
            (
                "https://example.com/blog/post".to_string(),
                Some("2026-10-22T09:30:00Z".to_string())
            ),
        ]
    );
    assert_eq!(
        site.published("robots.txt"),
        "User-agent: *\nAllow: /\n\nSitemap: https://example.com/sitemap.xml\n"
    );
}

#[test]
fn without_a_site_url_there_is_no_sitemap_but_still_a_robots_txt() {
    let site = blog("no_sitemap", "");
    site.generate();
    assert!(!site.destination.join("sitemap.xml").exists());
    assert_eq!(site.published("robots.txt"), "User-agent: *\nAllow: /\n");
}

#[test]
fn social_links_verify_as_the_site_owners_and_mastodon_names_the_creator() {
    let site = blog("fediverse", "");
    site.generate();

    let home = site.published("index.html");
    assert_eq!(
        home.matches(r#"href="https://hachyderm.io/@jo" target="_blank" rel="me noopener""#)
            .count(),
        2,
        "both the brand's and the menu's Mastodon link carry rel=me"
    );
    assert!(home.contains(r#"<meta name="fediverse:creator" content="@jo@hachyderm.io""#));

    // main.json's own handle wins over the one derived from the links.
    let site = blog(
        "fediverse_given",
        r#", "fediverseCreator": "jo@mastodon.social""#,
    );
    site.generate();
    assert!(site
        .published("index.html")
        .contains(r#"<meta name="fediverse:creator" content="@jo@mastodon.social""#));
}

/// The `<img>` tag whose `src` starts with `src_prefix`.
fn img_tag<'a>(html: &'a str, src_prefix: &str) -> &'a str {
    let at = html
        .find(&format!(r#"src="{src_prefix}"#))
        .unwrap_or_else(|| panic!("no <img> with src {src_prefix}..."));
    let start = html[..at].rfind("<img").unwrap();
    let end = at + html[at..].find("/>").unwrap();
    &html[start..end]
}

#[test]
fn the_largest_images_load_first_and_fall_back_to_a_mid_size_variant() {
    let site = blog("lcp", "");
    site.write_source(
        "blog/page.json",
        r#"{"title":"Blog","content":"<p>posts</p>","childOrder":"dateDesc",
            "leftNotifications":[
              {"content":"<p>a</p>","image":"/batman"},
              {"content":"<p>b</p>","image":"/logo"}],
            "rightNotifications":[{"content":"<p>c</p>","image":"/logo"}]}"#,
    );
    site.generate();

    // The 1600 x 1000 hero is cropped to 5:3; its fallback is the 986 wide
    // variant, not the 159 wide thumbnail.
    let post = site.published("blog/post.html");
    let hero = img_tag(&post, "/img/batman-");
    assert!(hero.contains(r#"src="/img/batman-w986.webp""#), "{hero}");
    // The minifier drops the quotes around a bare number.
    assert!(hero.contains("width=986 height=591"), "{hero}");
    assert!(hero.contains(r#"loading="eager""#), "{hero}");
    assert!(hero.contains(r#"fetchpriority="high""#), "{hero}");

    // The first row of child tiles loads eagerly, the rest lazily, and none
    // jumps the queue. The 400 x 400 logo's widest variant up to 986 is 318.
    let blog = site.published("blog.html");
    let first = img_tag(&blog, "/img/batman-");
    assert!(first.contains(r#"loading="eager""#), "{first}");
    assert!(!first.contains("fetchpriority"), "{first}");
    let tiles: Vec<&str> = blog
        .split("<img")
        .skip(1)
        .filter(|t| t.contains(r#"src="/img/logo-w318.webp""#))
        .collect();
    assert_eq!(tiles.len(), 2);
    assert!(tiles.iter().any(|t| t.contains(r#"loading="eager""#)));
    assert!(tiles.iter().any(|t| t.contains(r#"loading="lazy""#)));
}

#[test]
fn a_page_shows_the_latest_dated_pages_from_elsewhere_first() {
    let site = blog("latest", "");
    site.write_source(
        "page.json",
        r#"{"title":"Home","content":"<p>home</p>",
            "latest":{"from":["/blog","/talks"],"limit":3},
            "leftNotifications":[{"content":"<p>own tile</p>","title":"Own"}]}"#,
    );
    site.write_source(
        "blog/2026/page.json",
        r#"{"title":"2026","content":"<p>year</p>"}"#,
    );
    site.write_source(
        "blog/2026/deep/page.json",
        r#"{"title":"Deep post","content":"<p>x</p>","date":"2026-10-23","summary":"Deep & new"}"#,
    );
    site.write_source(
        "blog/old/page.json",
        r#"{"title":"Old post","content":"<p>x</p>","date":"2020-01-01"}"#,
    );
    site.write_source(
        "blog/hidden/page.json",
        r#"{"title":"Hidden post","content":"<p>x</p>","date":"2026-12-01","listed":false}"#,
    );
    site.write_source(
        "talks/one/page.json",
        r#"{"title":"A talk","content":"<p>x</p>","date":"2026-05-01"}"#,
    );
    site.generate();

    let home = site.published("index.html");
    let position = |needle: &str| {
        home.find(needle)
            .unwrap_or_else(|| panic!("{needle} is not on the home page"))
    };
    // Newest first, at any depth, unlisted pages left out, cut at the limit;
    // the newest sits left, the next right, the third left again, before the
    // page's own tile. The right column ends after one tile, so that one
    // stretches down alongside the three on the left.
    let left = "cell mb-0 is-col-start-1-tablet is-col-span-2-mobile";
    let right = "cell mb-0 is-col-start-2-tablet is-col-span-2-mobile";
    assert!(home.contains(&format!(
        r#"class="notification is-link theme-dark {left}" href="/blog/2026/deep" target="_self" id="latest-0""#
    )));
    assert!(home.contains(&format!(
        r#"{right} is-row-span-3-tablet" href="/blog/post" target="_self" id="latest-1""#
    )));
    assert!(home.contains(&format!(
        r#"{left}" href="/talks/one" target="_self" id="latest-2""#
    )));
    assert!(home.contains(&format!(r#"{left}" id="sub-l-0""#)));
    assert!(!home.contains("order:"));
    assert!(home.contains("<p>Deep &amp; new</p>"));
    assert!(home.contains(r#"<time datetime="2026-10-23">"#));
    assert!(!home.contains(">Old post<"));
    assert!(!home.contains(">Hidden post<"));

    // Row by row, which is also the order a phone stacks them in.
    let tiles = &home[position(r#"id="child-tiles""#)..];
    let at = |needle: &str| {
        tiles
            .find(needle)
            .unwrap_or_else(|| panic!("{needle} is not a child tile"))
    };
    assert!(at(">Deep post<") < at(">A post<"));
    assert!(at(">A post<") < at(">A talk<"));
    assert!(at(">A talk<") < at(">Own<"));
}

#[cfg(feature = "highlight")]
#[test]
fn code_blocks_are_highlighted_and_only_their_pages_load_the_stylesheet() {
    let code_post = |site: &TestSite| {
        site.write_source(
            "blog/post/page.json",
            r#"{"title":"A post","content":"post.md","date":"2026-10-22","summary":"Code"}"#,
        );
        site.write_source(
            "blog/post/post.md",
            "## Code\n\n```rust\nfn main() { println!(\"hi\"); }\n```\n\n```\nplain\n```\n",
        );
    };
    let site = blog("highlight", "");
    code_post(&site);
    site.generate();

    let post = site.published("blog/post.html");
    assert!(post.contains(r#"<pre class="highlight"><code class="language-rust">"#));
    assert!(post.contains(r#"<span class="hl-"#));
    assert!(post.contains(r#"<link href="/css/vados-highlight.css" rel="stylesheet">"#));
    assert!(post.contains("<pre><code>plain"));
    assert!(!site.published("index.html").contains("vados-highlight.css"));
    assert!(site.published("css/vados-highlight.css").contains(".hl-"));

    let site = blog("no_highlight", r#", "codeHighlight": false"#);
    code_post(&site);
    site.generate();
    let post = site.published("blog/post.html");
    assert!(post.contains(r#"<pre><code class="language-rust">"#));
    assert!(!post.contains("hl-"));
    assert!(!post.contains("vados-highlight.css"));
    assert!(!site.destination.join("css/vados-highlight.css").exists());
}
