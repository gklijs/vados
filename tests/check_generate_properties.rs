//! Property-based fuzzing over randomly generated source/image trees.
//!
//! Unlike `check.rs`'s hand-written unit tests (one input per finding kind)
//! and the `vados_bass` integration test (one fixed, real tree), this throws
//! hundreds of small, schema-valid-but-otherwise-arbitrary trees at both
//! commands and holds them to the two promises `vados.allium` makes about
//! how they relate:
//!
//! 1. `check` never panics, on anything -- panicking is a `generate` failure
//!    mode, not a `check` one; `check`'s whole job is to turn failures into
//!    findings instead.
//! 2. When `check` reports zero errors, `generate` does not panic. That is
//!    the substance of "check tells you what generate would abort on"; if it
//!    ever stopped holding, check would be lying to the maintainer.
//!
//! Images are kept to real, tiny (1x1) PNG/JPEG files so the pipeline is
//! exercised for real without spending the fuzzing budget on image decoding.

use proptest::prelude::*;
use serde_json::json;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use vados::check::check;
use vados::generator::generate;

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// One of the four ways a content reference can resolve, plus the two ways
/// it can fail to. Mirrors `ContentKind` plus "the file is missing".
#[derive(Debug, Clone)]
enum ContentSpec {
    InlineHtml,
    RealFile { markdown: bool, body: String },
    MissingFile { markdown: bool },
    Unrecognised(String),
}

fn content_spec() -> impl Strategy<Value = ContentSpec> {
    prop_oneof![
        Just(ContentSpec::InlineHtml),
        (any::<bool>(), short_text())
            .prop_map(|(markdown, body)| ContentSpec::RealFile { markdown, body }),
        any::<bool>().prop_map(|markdown| ContentSpec::MissingFile { markdown }),
        "[a-z]{1,8}".prop_map(ContentSpec::Unrecognised),
    ]
}

/// Short, but deliberately unrestricted text: proptest's `String` strategy
/// covers the unicode, empty-string and control-character edge cases a
/// hand-written fixture would not think to try. serde_json takes care of
/// escaping it correctly regardless of content.
fn short_text() -> impl Strategy<Value = String> {
    any::<String>().prop_filter("keep it short", |s| s.chars().count() <= 24)
}

#[derive(Debug, Clone)]
enum ImageRefSpec {
    None,
    Existing(usize),
    Dangling(String),
}

fn image_ref_spec(image_count: usize) -> impl Strategy<Value = ImageRefSpec> {
    if image_count == 0 {
        prop_oneof![
            Just(ImageRefSpec::None),
            "[a-z]{1,6}".prop_map(ImageRefSpec::Dangling),
        ]
        .boxed()
    } else {
        prop_oneof![
            Just(ImageRefSpec::None),
            (0..image_count).prop_map(ImageRefSpec::Existing),
            "[a-z]{1,6}".prop_map(ImageRefSpec::Dangling),
        ]
        .boxed()
    }
}

#[derive(Debug, Clone)]
enum LinkTargetSpec {
    None,
    ExistingPage(usize),
    DanglingPage(String),
    External(String),
}

fn notification_url_spec(page_count: usize) -> impl Strategy<Value = LinkTargetSpec> {
    prop_oneof![
        Just(LinkTargetSpec::None),
        (0..page_count.max(1)).prop_map(LinkTargetSpec::ExistingPage),
        "[a-z]{1,6}".prop_map(LinkTargetSpec::DanglingPage),
        "[a-z]{1,6}".prop_map(|s| LinkTargetSpec::External(format!("https://example.com/{}", s))),
    ]
}

#[derive(Debug, Clone)]
struct NotificationSpec {
    content: ContentSpec,
    image: ImageRefSpec,
    url: LinkTargetSpec,
}

fn notification_spec(
    page_count: usize,
    image_count: usize,
) -> impl Strategy<Value = NotificationSpec> {
    (
        content_spec(),
        image_ref_spec(image_count),
        notification_url_spec(page_count),
    )
        .prop_map(|(content, image, url)| NotificationSpec {
            content,
            image,
            url,
        })
}

#[derive(Debug, Clone)]
struct PageSpec {
    /// Path segments below the root, e.g. `["a", "b"]` for `/a/b`. Empty for
    /// the root page itself.
    segments: Vec<String>,
    has_config: bool,
    title: String,
    content: ContentSpec,
    image: ImageRefSpec,
    notifications: Vec<NotificationSpec>,
}

fn path_segment() -> impl Strategy<Value = String> {
    prop_oneof![Just("a"), Just("b"), Just("c"), Just("d")].prop_map(String::from)
}

#[derive(Debug, Clone)]
struct ImageSpec {
    file_stem: String,
    extension: &'static str,
    alt_text: String,
    file_exists: bool,
}

fn image_spec() -> impl Strategy<Value = ImageSpec> {
    (
        prop_oneof![Just("img1"), Just("img2"), Just("img3")],
        prop_oneof![Just("png"), Just("jpg")],
        short_text(),
        any::<bool>(),
    )
        .prop_map(|(file_stem, extension, alt_text, file_exists)| ImageSpec {
            file_stem: file_stem.to_string(),
            extension,
            alt_text,
            file_exists,
        })
}

#[derive(Debug, Clone)]
enum MenuLinkSpec {
    Internal(LinkTargetSpec),
    External { has_title: bool },
}

fn menu_link_spec(page_count: usize) -> impl Strategy<Value = MenuLinkSpec> {
    prop_oneof![
        notification_url_spec(page_count).prop_map(MenuLinkSpec::Internal),
        any::<bool>().prop_map(|has_title| MenuLinkSpec::External { has_title }),
    ]
}

#[derive(Debug, Clone)]
enum SocialSpec {
    Github,
    LinkedIn,
    Other { has_icon: bool, has_color: bool },
}

fn social_spec() -> impl Strategy<Value = SocialSpec> {
    prop_oneof![
        Just(SocialSpec::Github),
        Just(SocialSpec::LinkedIn),
        (any::<bool>(), any::<bool>()).prop_map(|(has_icon, has_color)| SocialSpec::Other {
            has_icon,
            has_color
        }),
    ]
}

#[derive(Debug, Clone)]
struct SiteSpec {
    site_title: String,
    footer: ContentSpec,
    pages: Vec<PageSpec>,
    images: Vec<ImageSpec>,
    main_menu: Vec<MenuLinkSpec>,
    socials: Vec<SocialSpec>,
}

/// The whole tree, built bottom-up: image count first (menu/notification
/// targets can dangle regardless, but existing-image references need to know
/// how many images there are to pick a valid index from), then pages
/// (existing-page references likewise need the page count), then the two
/// site-wide files that reference both.
fn site_spec() -> impl Strategy<Value = SiteSpec> {
    let images = proptest::collection::vec(image_spec(), 0..4);
    images.prop_flat_map(|images| {
        let image_count = images.len();
        let page = (
            proptest::collection::vec(path_segment(), 0..3),
            any::<bool>(),
            short_text(),
            content_spec(),
            image_ref_spec(image_count),
            proptest::collection::vec(notification_spec(4, image_count), 0..3),
        )
            .prop_map(
                move |(segments, has_config, title, content, image, notifications)| PageSpec {
                    segments,
                    has_config,
                    title,
                    content,
                    image,
                    notifications,
                },
            );
        let pages = proptest::collection::vec(page, 1..5);
        let images = Just(images);
        (pages, images).prop_flat_map(move |(pages, images)| {
            let page_count = pages.len();
            (
                Just(pages),
                Just(images),
                short_text(),
                content_spec(),
                proptest::collection::vec(menu_link_spec(page_count), 0..4),
                proptest::collection::vec(social_spec(), 0..3),
            )
                .prop_map(|(pages, images, site_title, footer, main_menu, socials)| {
                    SiteSpec {
                        site_title,
                        footer,
                        pages,
                        images,
                        main_menu,
                        socials,
                    }
                })
        })
    })
}

// ---------------------------------------------------------------------------
// Materialisation: SiteSpec -> real files on disk
// ---------------------------------------------------------------------------

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct Materialized {
    root: PathBuf,
    source: PathBuf,
    img_source: PathBuf,
}

impl Drop for Materialized {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn page_path(segments: &[String]) -> String {
    if segments.is_empty() {
        String::from("/")
    } else {
        format!("/{}", segments.join("/"))
    }
}

fn content_ref(spec: &ContentSpec, dir: &std::path::Path, unique: &str) -> String {
    match spec {
        ContentSpec::InlineHtml => String::from("<p>content</p>"),
        ContentSpec::RealFile { markdown, body } => {
            let ext = if *markdown { "md" } else { "html" };
            let name = format!("{}.{}", unique, ext);
            let contents = if *markdown {
                body.clone()
            } else {
                format!("<p>{}</p>", body)
            };
            let _ = std::fs::write(dir.join(&name), contents);
            name
        }
        ContentSpec::MissingFile { markdown } => {
            format!(
                "does-not-exist-{}.{}",
                unique,
                if *markdown { "md" } else { "html" }
            )
        }
        ContentSpec::Unrecognised(s) => s.clone(),
    }
}

fn image_key(stem: &str, path_start: &str) -> String {
    if path_start.is_empty() {
        format!("/{}", stem)
    } else {
        format!("{}/{}", path_start, stem)
    }
}

fn image_ref(spec: &ImageRefSpec, image_keys: &[String]) -> Option<String> {
    match spec {
        ImageRefSpec::None => None,
        ImageRefSpec::Existing(i) => image_keys.get(*i).cloned(),
        ImageRefSpec::Dangling(s) => Some(format!("/does-not-exist-{}", s)),
    }
}

fn link_target(spec: &LinkTargetSpec, page_paths: &[String]) -> Option<String> {
    match spec {
        LinkTargetSpec::None => None,
        LinkTargetSpec::ExistingPage(i) => page_paths.get(i % page_paths.len().max(1)).cloned(),
        LinkTargetSpec::DanglingPage(s) => Some(format!("/does-not-exist-{}", s)),
        LinkTargetSpec::External(url) => Some(url.clone()),
    }
}

fn notification_json(
    spec: &NotificationSpec,
    dir: &std::path::Path,
    image_keys: &[String],
    page_paths: &[String],
    unique: &str,
) -> serde_json::Value {
    json!({
        "content": content_ref(&spec.content, dir, unique),
        "title": Option::<String>::None,
        "image": image_ref(&spec.image, image_keys),
        "url": link_target(&spec.url, page_paths),
        "color": Option::<String>::None,
    })
}

fn materialize(spec: &SiteSpec) -> Materialized {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("vados_prop_{}_{}", std::process::id(), id));
    let _ = std::fs::remove_dir_all(&root);
    let source = root.join("source");
    let img_source = root.join("images");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&img_source).unwrap();

    // Images first: every declared image gets a reference key, real file or
    // not, before anything can refer to it.
    let mut image_keys = vec![];
    let mut image_entries = vec![];
    for (i, image) in spec.images.iter().enumerate() {
        let file_name = format!("{}.{}", image.file_stem, image.extension);
        image_keys.push(image_key(&image.file_stem, ""));
        if image.file_exists {
            write_tiny_image(&img_source.join(&file_name), image.extension);
        }
        image_entries.push(json!({
            "title": Option::<String>::None,
            "fileName": file_name,
            "altText": image.alt_text,
        }));
        let _ = i;
    }
    if !image_entries.is_empty() {
        let _ = std::fs::write(
            img_source.join("images.json"),
            json!({ "title": Option::<String>::None, "list": image_entries }).to_string(),
        );
    }

    // Pages next, deduplicated by path (proptest can and does generate the
    // same segments twice) so every directory is written exactly once.
    let mut seen = std::collections::HashSet::new();
    let mut ordered_pages = vec![];
    for page in &spec.pages {
        let path = page_path(&page.segments);
        if seen.insert(path.clone()) {
            ordered_pages.push(page.clone());
        }
    }
    let page_paths: Vec<String> = ordered_pages
        .iter()
        .map(|p| page_path(&p.segments))
        .collect();

    for (i, page) in ordered_pages.iter().enumerate() {
        let dir = if page.segments.is_empty() {
            source.clone()
        } else {
            source.join(page.segments.join("/"))
        };
        std::fs::create_dir_all(&dir).unwrap();
        if page.has_config {
            let unique = format!("p{}", i);
            let notifications: Vec<_> = page
                .notifications
                .iter()
                .enumerate()
                .map(|(j, n)| {
                    notification_json(
                        n,
                        &dir,
                        &image_keys,
                        &page_paths,
                        &format!("{}n{}", unique, j),
                    )
                })
                .collect();
            let doc = json!({
                "title": page.title,
                "subTitle": Option::<String>::None,
                "image": image_ref(&page.image, &image_keys),
                "icon": Option::<String>::None,
                "summary": Option::<String>::None,
                "content": content_ref(&page.content, &dir, &unique),
                "order": Option::<u32>::None,
                "leftNotifications": if notifications.is_empty() { serde_json::Value::Null } else { serde_json::Value::Array(notifications) },
                "rightNotifications": Option::<Vec<serde_json::Value>>::None,
            });
            let _ = std::fs::write(dir.join("page.json"), doc.to_string());
        }
    }

    // Site-wide files last, since the footer and the menu can both refer to
    // pages and images that now definitely exist as entries.
    let footer_ref = content_ref(&spec.footer, &source, "footer");
    let main_json = json!({
        "siteTitle": spec.site_title,
        "jsFiles": Vec::<String>::new(),
        "includeDefaultJs": false,
        "cssFiles": Vec::<String>::new(),
        "includeDefaultCss": false,
        "backgroundClass": Option::<String>::None,
        "navbarColor": Option::<String>::None,
        "footerContent": footer_ref,
    });
    let _ = std::fs::write(source.join("main.json"), main_json.to_string());

    let main_menu: Vec<_> = spec
        .main_menu
        .iter()
        .map(|link| match link {
            MenuLinkSpec::Internal(target) => json!({
                "url": link_target(target, &page_paths).unwrap_or_else(|| String::from("/")),
                "title": Option::<String>::None,
                "icon": Option::<String>::None,
            }),
            MenuLinkSpec::External { has_title } => json!({
                "url": "https://example.com/",
                "title": if *has_title { Some("External") } else { None },
                "icon": Option::<String>::None,
            }),
        })
        .collect();
    let socials: Vec<_> = spec
        .socials
        .iter()
        .map(|s| match s {
            SocialSpec::Github => json!({
                "url": "https://github.com/someone",
                "icon": Option::<String>::None,
                "color": Option::<String>::None,
            }),
            SocialSpec::LinkedIn => json!({
                "url": "https://www.linkedin.com/in/someone",
                "icon": Option::<String>::None,
                "color": Option::<String>::None,
            }),
            SocialSpec::Other {
                has_icon,
                has_color,
            } => json!({
                "url": "https://mastodon.example/@someone",
                "icon": if *has_icon { Some("mastodon") } else { None },
                "color": if *has_color { Some("6364FF") } else { None },
            }),
        })
        .collect();
    let menu_json = json!({ "mainMenu": main_menu, "socials": socials });
    let _ = std::fs::write(source.join("menu.json"), menu_json.to_string());

    Materialized {
        root,
        source,
        img_source,
    }
}

/// Writes a real, decodable 1x1 image, so the resize pipeline runs for real
/// without spending the fuzzing time budget decoding large photos -- that is
/// what the vados_bass integration test is for.
fn write_tiny_image(path: &std::path::Path, extension: &str) {
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([200, 100, 50, 255]));
    match extension {
        "png" => {
            let _ = img.save_with_format(path, image::ImageFormat::Png);
        }
        _ => {
            let _ = image::DynamicImage::ImageRgba8(img)
                .to_rgb8()
                .save_with_format(path, image::ImageFormat::Jpeg);
        }
    }
}

// ---------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 150, .. ProptestConfig::default() })]

    /// `check` never panics. Its entire reason to exist is to turn "generate
    /// would fall over here" into a reported finding; a panic inside check
    /// itself is always a bug in check, whatever the input looks like.
    #[test]
    fn check_never_panics(spec in site_spec()) {
        let site = materialize(&spec);
        let result = catch_unwind(AssertUnwindSafe(|| {
            check(site.source.to_str().unwrap(), site.img_source.to_str().unwrap())
        }));
        prop_assert!(result.is_ok(), "check() panicked on {:#?}", spec);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 60, .. ProptestConfig::default() })]

    /// When `check` reports zero errors, `generate` does not panic. This is
    /// the substance of the spec's promise that check tells the maintainer
    /// what generate would abort on: if a tree checks clean and generate
    /// still panics, check was wrong to call it clean.
    #[test]
    fn generate_does_not_panic_when_check_reports_no_errors(spec in site_spec()) {
        let site = materialize(&spec);
        let report = check(site.source.to_str().unwrap(), site.img_source.to_str().unwrap());
        prop_assume!(report.passed());

        let dest = site.root.join("dest");
        let result = catch_unwind(AssertUnwindSafe(|| {
            generate(
                site.source.to_str().unwrap(),
                site.img_source.to_str().unwrap(),
                dest.to_str().unwrap(),
            )
        }));
        prop_assert!(
            result.is_ok(),
            "generate() panicked on a tree check() called clean: {:#?}\nfindings={:#?}",
            spec,
            report.findings
        );
    }
}
