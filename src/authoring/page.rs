//! `page new` and `page add-image`: authors a page.json into an existing
//! source tree, or attaches an image to a page that's already there. See
//! `vados.allium`'s `PageCreationRun` and `PageImageAttachmentRun`.

use super::image_registry;
use crate::config_files::{Notification, PageConfig};
use crate::content::ensure_content_reference;
use crate::json_files::{read_json, read_json_or, write_json_pretty};
use crate::structure::parent_path;
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

/// The directory a page's path maps to under `source`. The root page ("/")
/// is `source` itself; anything else is `source` plus the path.
pub fn page_dir(source: &str, path: &str) -> PathBuf {
    if path == "/" {
        PathBuf::from(source)
    } else {
        PathBuf::from(format!("{source}{path}"))
    }
}

/// Whether a directory tree already has a page at `path`. Every directory
/// under the source root is a page whether or not it has its own page.json
/// (see `vados.allium`'s `DiscoverPage`), so this asks about the directory,
/// not the file.
pub fn page_exists_at(source: &str, path: &str) -> bool {
    page_dir(source, path).is_dir()
}

/// A page.json with no content reference gets the same fallback a directory
/// with no page.json gets at generate/check time: a bare heading named after
/// the page's own last path segment. See `vados.allium`'s
/// `default_page_content` and `DiscoverPage`.
pub fn default_page_content(path: &str) -> String {
    let name = path.rsplit('/').find(|s| !s.is_empty()).unwrap_or(path);
    format!("<h1>{}</h1>", name)
}

/// Why a `page new` request couldn't proceed. See `vados.allium`'s
/// `PageCreationBlockReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageCreationBlockReason {
    PathAlreadyAPage,
    ParentPathIsNotAPage,
    ImageReferenceUnresolved,
}

impl fmt::Display for PageCreationBlockReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PageCreationBlockReason::PathAlreadyAPage => "a page already exists at this path",
            PageCreationBlockReason::ParentPathIsNotAPage => {
                "the parent path is not itself a page yet"
            }
            PageCreationBlockReason::ImageReferenceUnresolved => {
                "the given image reference matches no declared image"
            }
        })
    }
}

/// One reason a page-creation request couldn't proceed. Recorded rather than
/// raised, so every problem is found in one pass; see `vados.allium`'s
/// `PageCreationBlock`.
#[derive(Debug)]
pub struct PageCreationBlock {
    pub reason: PageCreationBlockReason,
    pub detail: String,
}

/// Checks every way a `page new` request could fail in one pass, before the
/// maintainer is asked for a title or anything else. See `vados.allium`'s
/// `DetectPageCreationBlockers`.
///
/// `image_source` is optional the same way `PageCreationRun.image_source_location`
/// is in `vados.allium`: a page created without an image reference never
/// needs to know where the image tree lives. Given an image reference with no
/// image source to check it against, the reference can never resolve.
pub fn detect_page_creation_blockers(
    source: &str,
    image_source: Option<&str>,
    given_path: &str,
    given_image_reference: Option<&str>,
) -> Vec<PageCreationBlock> {
    let mut blocks = Vec::new();

    if page_exists_at(source, given_path) {
        blocks.push(PageCreationBlock {
            reason: PageCreationBlockReason::PathAlreadyAPage,
            detail: given_path.to_string(),
        });
    }
    if let Some(parent) = parent_path(given_path) {
        if !page_exists_at(source, &parent) {
            blocks.push(PageCreationBlock {
                reason: PageCreationBlockReason::ParentPathIsNotAPage,
                detail: parent,
            });
        }
    }
    if let Some(image_ref) = given_image_reference {
        let resolves = image_source
            .map(|is| image_registry::image_reference_resolves(is, image_ref))
            .unwrap_or(false);
        if !resolves {
            blocks.push(PageCreationBlock {
                reason: PageCreationBlockReason::ImageReferenceUnresolved,
                detail: image_ref.to_string(),
            });
        }
    }

    blocks
}

/// The rest of a new page's details, gathered once the blocker check has
/// passed. See `vados.allium`'s `PageCreationDetailsProvided`.
#[derive(Debug, Clone)]
pub struct PageCreationDetails {
    pub title: String,
    pub sub_title: Option<String>,
    pub icon: Option<String>,
    pub summary: Option<String>,
    pub order: Option<u32>,
    pub content: Option<String>,
}

/// What `page new` created, once it has. See `vados.allium`'s
/// `PageCreatedOutcome`.
#[derive(Debug)]
pub struct PageCreatedOutcome {
    pub path: String,
    pub title: String,
    pub sub_title: Option<String>,
    pub icon: Option<String>,
    pub summary: Option<String>,
    pub order: Option<u32>,
    pub content: String,
    pub content_was_defaulted: bool,
    pub image_reference: Option<String>,
}

/// Writes a page.json for `given_path`. Callers are expected to have already
/// run `detect_page_creation_blockers` and found it empty -- exactly like
/// `ProvidePageCreationDetails`'s `requires: run.status =
/// page_creation_requested`, reachable only once no blocker fired.
pub fn create_page(
    source: &str,
    given_path: &str,
    given_image_reference: Option<String>,
    details: PageCreationDetails,
) -> io::Result<PageCreatedOutcome> {
    let content_was_defaulted = details.content.is_none();
    let content = details
        .content
        .unwrap_or_else(|| default_page_content(given_path));

    let page_config = PageConfig {
        title: details.title.clone(),
        sub_title: details.sub_title.clone(),
        image: given_image_reference.clone(),
        icon: details.icon.clone(),
        summary: details.summary.clone(),
        content: content.clone(),
        order: details.order,
        left_notifications: None,
        right_notifications: None,
    };

    let dir = page_dir(source, given_path);
    fs::create_dir_all(&dir)?;
    write_json_pretty(dir.join("page.json"), &page_config)?;

    Ok(PageCreatedOutcome {
        path: given_path.to_string(),
        title: details.title,
        sub_title: details.sub_title,
        icon: details.icon,
        summary: details.summary,
        order: details.order,
        content,
        content_was_defaulted,
        image_reference: given_image_reference,
    })
}

/// Why a `page add-image` request couldn't proceed. Shares its middle two
/// reasons with `image_registry::ImageRegistrationBlockReason` -- registering
/// a new image is the same problem however it's reached. See
/// `vados.allium`'s `PageImageAttachmentBlockReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageImageAttachmentBlockReason {
    TargetPageMissing,
    SourceFileUnreadable,
    ReferenceKeyAlreadyUsed,
    ImageReferenceUnresolved,
    HeroAlreadyPresent,
}

impl fmt::Display for PageImageAttachmentBlockReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PageImageAttachmentBlockReason::TargetPageMissing => "the target page does not exist",
            PageImageAttachmentBlockReason::SourceFileUnreadable => {
                "image source file could not be read (or its name has no extension)"
            }
            PageImageAttachmentBlockReason::ReferenceKeyAlreadyUsed => {
                "an image is already declared with this reference key"
            }
            PageImageAttachmentBlockReason::ImageReferenceUnresolved => {
                "the given image reference matches no declared image"
            }
            PageImageAttachmentBlockReason::HeroAlreadyPresent => {
                "the page already has a hero image"
            }
        })
    }
}

#[derive(Debug)]
pub struct PageImageAttachmentBlock {
    pub reason: PageImageAttachmentBlockReason,
    pub detail: String,
}

/// Whether an attached image fills a page's one hero slot or joins its
/// notifications instead. See `vados.allium`'s `PageImageRole`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageImageRole {
    Hero,
    Notification,
}

/// Which side of the page a new notification is added to. See
/// `vados.allium`'s `NotificationSide`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationSide {
    Left,
    Right,
}

/// Exactly one of these two names the image being attached: a freshly
/// registered file, or one already declared elsewhere in the tree. See
/// `vados.allium`'s `PageImageAttachmentNamesExactlyOneImageSource`.
#[derive(Debug, Clone)]
pub enum GivenImageSource {
    New { dir: String, file_name: String },
    Existing { reference: String },
}

/// A `page add-image` request. See `vados.allium`'s `PageImageAttachmentRun`.
#[derive(Debug, Clone)]
pub struct PageImageAttachmentRequest {
    pub given_path: String,
    pub source: GivenImageSource,
    pub given_as: Option<PageImageRole>,
}

/// The reference key of the image currently filling a page's hero slot, if
/// any. Backs `existing_page_image_reference`.
pub fn existing_page_image_reference(source: &str, path: &str) -> Option<String> {
    let page_json = page_dir(source, path).join("page.json");
    let config: PageConfig = read_json(&page_json).ok()?;
    config.image
}

/// Whether a page's hero slot is free. A page has exactly one hero image; the
/// first image attached to it fills that slot, every one after joins its
/// notifications instead.
pub fn hero_available(source: &str, path: &str) -> bool {
    existing_page_image_reference(source, path).is_none()
}

/// The role an attachment actually takes: the maintainer's override if they
/// gave one, otherwise hero when the slot is free and notification
/// otherwise.
pub fn effective_role(given_as: Option<PageImageRole>, hero_is_available: bool) -> PageImageRole {
    given_as.unwrap_or(if hero_is_available {
        PageImageRole::Hero
    } else {
        PageImageRole::Notification
    })
}

/// Checks every way a `page add-image` request could fail in one pass,
/// before the maintainer is asked for anything further. See
/// `vados.allium`'s `DetectPageImageAttachmentBlockers`.
pub fn detect_attachment_blockers(
    source: &str,
    image_source: &str,
    request: &PageImageAttachmentRequest,
) -> Vec<PageImageAttachmentBlock> {
    let mut blocks = Vec::new();

    if !page_exists_at(source, &request.given_path) {
        blocks.push(PageImageAttachmentBlock {
            reason: PageImageAttachmentBlockReason::TargetPageMissing,
            detail: request.given_path.clone(),
        });
    }

    match &request.source {
        GivenImageSource::New { dir, file_name } => {
            let source_path = image_registry::image_source_path(image_source, dir, file_name);
            if !image_registry::source_readable(&source_path) {
                blocks.push(PageImageAttachmentBlock {
                    reason: PageImageAttachmentBlockReason::SourceFileUnreadable,
                    detail: file_name.clone(),
                });
            }
            if let Some(key) = image_registry::reference_key_for_registration(dir, file_name) {
                if image_registry::collect_declared_image_keys(image_source).contains(&key) {
                    blocks.push(PageImageAttachmentBlock {
                        reason: PageImageAttachmentBlockReason::ReferenceKeyAlreadyUsed,
                        detail: key,
                    });
                }
            }
        }
        GivenImageSource::Existing { reference } => {
            if !image_registry::image_reference_resolves(image_source, reference) {
                blocks.push(PageImageAttachmentBlock {
                    reason: PageImageAttachmentBlockReason::ImageReferenceUnresolved,
                    detail: reference.clone(),
                });
            }
        }
    }

    if request.given_as == Some(PageImageRole::Hero) && !hero_available(source, &request.given_path)
    {
        blocks.push(PageImageAttachmentBlock {
            reason: PageImageAttachmentBlockReason::HeroAlreadyPresent,
            detail: request.given_path.clone(),
        });
    }

    blocks
}

/// Gathered once the blocker check has passed. `alt_text` is required only
/// when a new image is being registered; `side`/`caption` matter only once
/// the role resolves to notification. See `vados.allium`'s
/// `PageImageAttachmentDetailsProvided`.
#[derive(Debug, Clone, Default)]
pub struct PageImageAttachmentDetails {
    pub alt_text: Option<String>,
    pub side: Option<NotificationSide>,
    pub caption: Option<String>,
}

/// What `page add-image` attached, once it has. See `vados.allium`'s
/// `PageImageAttachedOutcome`.
#[derive(Debug)]
pub struct PageImageAttachedOutcome {
    pub path: String,
    pub reference_key: String,
    pub role: PageImageRole,
    pub side: Option<NotificationSide>,
    pub content: Option<String>,
}

/// Attaches an image to a page. Callers are expected to have already run
/// `detect_attachment_blockers` and found it empty. Always the mechanism
/// that writes page.json, even when the role resolves to hero and there is
/// nothing beyond a new image's own alt text to gather -- the same shape
/// `vados.allium`'s `ProvidePageImageAttachmentDetails` uses.
pub fn attach_image(
    source: &str,
    image_source: &str,
    request: PageImageAttachmentRequest,
    details: PageImageAttachmentDetails,
) -> io::Result<PageImageAttachedOutcome> {
    let role = effective_role(
        request.given_as,
        hero_available(source, &request.given_path),
    );

    let (reference_key, resolved_alt_text) = match &request.source {
        GivenImageSource::Existing { reference } => (
            reference.clone(),
            image_registry::declared_image_alt_text(image_source, reference).unwrap_or_default(),
        ),
        GivenImageSource::New { dir, file_name } => {
            // A page-image attachment never registers a new image without the
            // alternative text `EveryImageHasAlternativeText` will go on to
            // require of it once a generate/check run reads the tree back;
            // see vados.allium's `PageImageAttachmentRegistersAltTextForNewImages`.
            let alt_text = details
                .alt_text
                .clone()
                .expect("callers must gather alt text before attaching a freshly registered image");
            let outcome = image_registry::register_image(
                image_source,
                dir,
                file_name,
                None,
                alt_text.clone(),
            )?;
            (outcome.reference_key, alt_text)
        }
    };

    let page_json_path = page_dir(source, &request.given_path).join("page.json");
    let mut config: PageConfig =
        read_json_or(&page_json_path, || PageConfig::new(&request.given_path))?;

    let (side, content) = match role {
        PageImageRole::Hero => {
            config.image = Some(reference_key.clone());
            (None, None)
        }
        PageImageRole::Notification => {
            let side = details.side.unwrap_or(NotificationSide::Right);
            // Neither a given caption nor an image's own alt text is
            // written with a content reference's mechanics in mind -- both
            // are just descriptive text. Coerced into one here, the same way
            // `init` already wraps the plain text it gathers (site title,
            // home intro, footer text) into inline HTML before writing it,
            // so a caption checks clean without the maintainer needing to
            // know `Notification.content` is a `ContentReference` underneath.
            let content =
                ensure_content_reference(details.caption.clone().unwrap_or(resolved_alt_text));
            let notification = Notification {
                content: content.clone(),
                title: None,
                image: Some(reference_key.clone()),
                url: None,
                color: None,
            };
            match side {
                NotificationSide::Left => config
                    .left_notifications
                    .get_or_insert_with(Vec::new)
                    .push(notification),
                NotificationSide::Right => config
                    .right_notifications
                    .get_or_insert_with(Vec::new)
                    .push(notification),
            }
            (Some(side), Some(content))
        }
    };

    write_json_pretty(page_json_path, &config)?;

    Ok(PageImageAttachedOutcome {
        path: request.given_path,
        reference_key,
        role,
        side,
        content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    struct TestSite {
        root: PathBuf,
        source: PathBuf,
        img_source: PathBuf,
    }

    impl TestSite {
        fn new(name: &str) -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir().join(format!("vados_page_authoring_test_{name}_{id}"));
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

        fn source(&self) -> &str {
            self.source.to_str().unwrap()
        }

        fn img_source(&self) -> &str {
            self.img_source.to_str().unwrap()
        }

        fn mkdir(&self, rel_path: &str) {
            fs::create_dir_all(self.source.join(rel_path.trim_start_matches('/'))).unwrap();
        }

        fn write_page(&self, rel_path: &str, contents: &str) {
            let path = self.source.join(rel_path.trim_start_matches('/'));
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("page.json"), contents).unwrap();
        }

        fn write_image(&self, rel_path: &str, contents: &str) {
            let path = self.img_source.join(rel_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        fn write_images_json(&self, dir: &str, contents: &str) {
            let path = if dir.is_empty() {
                self.img_source.join("images.json")
            } else {
                self.img_source.join(dir).join("images.json")
            };
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
    }

    impl Drop for TestSite {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    // -- page new -----------------------------------------------------------

    #[test]
    fn default_page_content_is_a_bare_heading_named_after_the_last_segment() {
        assert_eq!(default_page_content("/blog/post-one"), "<h1>post-one</h1>");
        assert_eq!(default_page_content("/blog"), "<h1>blog</h1>");
    }

    #[test]
    fn a_fresh_path_under_an_existing_parent_has_no_blockers() {
        let site = TestSite::new("no_blockers");

        let blocks =
            detect_page_creation_blockers(site.source(), Some(site.img_source()), "/blog", None);

        assert!(blocks.is_empty());
    }

    #[test]
    fn an_already_existing_path_is_blocked() {
        let site = TestSite::new("already_exists");
        site.mkdir("blog");

        let blocks =
            detect_page_creation_blockers(site.source(), Some(site.img_source()), "/blog", None);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].reason, PageCreationBlockReason::PathAlreadyAPage);
    }

    #[test]
    fn a_parent_that_is_not_yet_a_page_is_blocked() {
        let site = TestSite::new("missing_parent");

        let blocks = detect_page_creation_blockers(
            site.source(),
            Some(site.img_source()),
            "/blog/post-one",
            None,
        );

        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].reason,
            PageCreationBlockReason::ParentPathIsNotAPage
        );
        assert_eq!(blocks[0].detail, "/blog");
    }

    #[test]
    fn an_unresolved_image_reference_is_blocked() {
        let site = TestSite::new("unresolved_image");

        let blocks = detect_page_creation_blockers(
            site.source(),
            Some(site.img_source()),
            "/blog",
            Some("/nope"),
        );

        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].reason,
            PageCreationBlockReason::ImageReferenceUnresolved
        );
    }

    #[test]
    fn every_blocker_is_found_in_one_pass() {
        // Directories nest, so `path_already_a_page` and
        // `parent_path_is_not_a_page` can never both fire: a path that
        // exists necessarily has an existing parent. This combines the two
        // reasons that genuinely can co-occur -- a missing parent and an
        // unresolved image reference -- to check both are still reported
        // together rather than the first one short-circuiting the rest.
        let site = TestSite::new("all_blockers");

        let blocks = detect_page_creation_blockers(
            site.source(),
            Some(site.img_source()),
            "/blog/post-one",
            Some("/nope"),
        );

        assert_eq!(blocks.len(), 2);
        assert!(blocks
            .iter()
            .any(|b| b.reason == PageCreationBlockReason::ParentPathIsNotAPage));
        assert!(blocks
            .iter()
            .any(|b| b.reason == PageCreationBlockReason::ImageReferenceUnresolved));
    }

    #[test]
    fn create_page_writes_the_given_details() {
        let site = TestSite::new("create_given");
        site.mkdir(""); // root already exists as the source dir itself

        let outcome = create_page(
            site.source(),
            "/blog",
            None,
            PageCreationDetails {
                title: "Blog".to_string(),
                sub_title: Some("Notes".to_string()),
                icon: Some("pencil".to_string()),
                summary: Some("A blog".to_string()),
                order: Some(1),
                content: Some("posts.md".to_string()),
            },
        )
        .unwrap();

        assert_eq!(outcome.content, "posts.md");
        assert!(!outcome.content_was_defaulted);
        assert!(site.source.join("blog/page.json").is_file());

        let config: PageConfig =
            crate::json_files::read_json(&site.source.join("blog/page.json")).unwrap();
        assert_eq!(config.title, "Blog");
        assert_eq!(config.content, "posts.md");
    }

    #[test]
    fn create_page_defaults_content_when_none_is_given() {
        let site = TestSite::new("create_defaulted");

        let outcome = create_page(
            site.source(),
            "/about",
            None,
            PageCreationDetails {
                title: "About".to_string(),
                sub_title: None,
                icon: None,
                summary: None,
                order: None,
                content: None,
            },
        )
        .unwrap();

        assert!(outcome.content_was_defaulted);
        assert_eq!(outcome.content, "<h1>about</h1>");
    }

    #[test]
    fn create_page_records_the_given_image_reference() {
        let site = TestSite::new("create_with_image");

        let outcome = create_page(
            site.source(),
            "/team",
            Some("/team/alice".to_string()),
            PageCreationDetails {
                title: "Team".to_string(),
                sub_title: None,
                icon: None,
                summary: None,
                order: None,
                content: None,
            },
        )
        .unwrap();

        assert_eq!(outcome.image_reference, Some("/team/alice".to_string()));
        let config: PageConfig =
            crate::json_files::read_json(&site.source.join("team/page.json")).unwrap();
        assert_eq!(config.image, Some("/team/alice".to_string()));
    }

    // -- page add-image -------------------------------------------------

    #[test]
    fn a_missing_target_page_is_blocked() {
        let site = TestSite::new("target_missing");
        site.write_image("photo.jpg", "bytes");

        let request = PageImageAttachmentRequest {
            given_path: "/blog".to_string(),
            source: GivenImageSource::New {
                dir: String::new(),
                file_name: "photo.jpg".to_string(),
            },
            given_as: None,
        };
        let blocks = detect_attachment_blockers(site.source(), site.img_source(), &request);

        assert!(blocks
            .iter()
            .any(|b| b.reason == PageImageAttachmentBlockReason::TargetPageMissing));
    }

    #[test]
    fn an_unresolved_existing_reference_is_blocked() {
        let site = TestSite::new("unresolved_existing");
        site.mkdir("blog");

        let request = PageImageAttachmentRequest {
            given_path: "/blog".to_string(),
            source: GivenImageSource::Existing {
                reference: "/nope".to_string(),
            },
            given_as: None,
        };
        let blocks = detect_attachment_blockers(site.source(), site.img_source(), &request);

        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].reason,
            PageImageAttachmentBlockReason::ImageReferenceUnresolved
        );
    }

    #[test]
    fn forcing_a_second_hero_is_blocked() {
        let site = TestSite::new("hero_present");
        site.write_page(
            "blog",
            r#"{"title":"Blog","subTitle":null,"image":"/existing","icon":null,"summary":null,"content":"<p>hi</p>","order":null,"leftNotifications":null,"rightNotifications":null}"#,
        );
        site.write_images_json(
            "",
            r#"{"title":null,"list":[{"title":null,"fileName":"existing.jpg","altText":"e"},{"title":null,"fileName":"new.jpg","altText":"n"}]}"#,
        );

        let request = PageImageAttachmentRequest {
            given_path: "/blog".to_string(),
            source: GivenImageSource::Existing {
                reference: "/new".to_string(),
            },
            given_as: Some(PageImageRole::Hero),
        };
        let blocks = detect_attachment_blockers(site.source(), site.img_source(), &request);

        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].reason,
            PageImageAttachmentBlockReason::HeroAlreadyPresent
        );
    }

    #[test]
    fn without_an_override_the_first_attachment_becomes_the_hero() {
        let site = TestSite::new("first_is_hero");
        site.mkdir("blog");
        site.write_image("photo.jpg", "bytes");

        let outcome = attach_image(
            site.source(),
            site.img_source(),
            PageImageAttachmentRequest {
                given_path: "/blog".to_string(),
                source: GivenImageSource::New {
                    dir: String::new(),
                    file_name: "photo.jpg".to_string(),
                },
                given_as: None,
            },
            PageImageAttachmentDetails {
                alt_text: Some("a photo".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(outcome.role, PageImageRole::Hero);
        assert_eq!(outcome.reference_key, "/photo");
        let config: PageConfig =
            crate::json_files::read_json(&site.source.join("blog/page.json")).unwrap();
        assert_eq!(config.image, Some("/photo".to_string()));
    }

    #[test]
    fn a_second_attachment_without_an_override_becomes_a_notification() {
        let site = TestSite::new("second_is_notification");
        site.write_page(
            "blog",
            r#"{"title":"Blog","subTitle":null,"image":"/existing","icon":null,"summary":null,"content":"<p>hi</p>","order":null,"leftNotifications":null,"rightNotifications":null}"#,
        );
        site.write_image("second.jpg", "bytes");
        site.write_images_json(
            "",
            r#"{"title":null,"list":[{"title":null,"fileName":"existing.jpg","altText":"e"}]}"#,
        );

        let outcome = attach_image(
            site.source(),
            site.img_source(),
            PageImageAttachmentRequest {
                given_path: "/blog".to_string(),
                source: GivenImageSource::New {
                    dir: String::new(),
                    file_name: "second.jpg".to_string(),
                },
                given_as: None,
            },
            PageImageAttachmentDetails {
                alt_text: Some("second".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(outcome.role, PageImageRole::Notification);
        assert_eq!(outcome.side, Some(NotificationSide::Right));
        assert_eq!(outcome.content, Some("<p>second</p>".to_string()));

        let config: PageConfig =
            crate::json_files::read_json(&site.source.join("blog/page.json")).unwrap();
        assert_eq!(config.image, Some("/existing".to_string()));
        assert_eq!(config.right_notifications.unwrap().len(), 1);
    }

    #[test]
    fn a_notification_caption_defaults_to_the_images_own_alt_text_wrapped_as_inline_html() {
        // Alt text is plain descriptive text, not a ContentReference; the
        // fallback is coerced into inline HTML so a check run reads it back
        // as valid content rather than flagging it as unrecognised.
        let site = TestSite::new("caption_defaults");
        site.mkdir("blog");
        site.write_images_json(
            "",
            r#"{"title":null,"list":[{"title":null,"fileName":"photo.jpg","altText":"Alt text here"}]}"#,
        );

        let outcome = attach_image(
            site.source(),
            site.img_source(),
            PageImageAttachmentRequest {
                given_path: "/blog".to_string(),
                source: GivenImageSource::Existing {
                    reference: "/photo".to_string(),
                },
                given_as: Some(PageImageRole::Notification),
            },
            PageImageAttachmentDetails::default(),
        )
        .unwrap();

        assert_eq!(outcome.content, Some("<p>Alt text here</p>".to_string()));
    }

    #[test]
    fn a_plain_text_caption_is_also_wrapped_as_inline_html() {
        let site = TestSite::new("caption_plain_text");
        site.mkdir("blog");
        site.write_images_json(
            "",
            r#"{"title":null,"list":[{"title":null,"fileName":"photo.jpg","altText":"a"}]}"#,
        );

        let outcome = attach_image(
            site.source(),
            site.img_source(),
            PageImageAttachmentRequest {
                given_path: "/blog".to_string(),
                source: GivenImageSource::Existing {
                    reference: "/photo".to_string(),
                },
                given_as: Some(PageImageRole::Notification),
            },
            PageImageAttachmentDetails {
                caption: Some("Say hi to the team".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            outcome.content,
            Some("<p>Say hi to the team</p>".to_string())
        );
    }

    #[test]
    fn a_caption_that_already_looks_like_a_content_reference_is_left_untouched() {
        let site = TestSite::new("caption_already_reference");
        site.mkdir("blog");
        site.write_images_json(
            "",
            r#"{"title":null,"list":[{"title":null,"fileName":"photo.jpg","altText":"a"}]}"#,
        );

        let outcome = attach_image(
            site.source(),
            site.img_source(),
            PageImageAttachmentRequest {
                given_path: "/blog".to_string(),
                source: GivenImageSource::Existing {
                    reference: "/photo".to_string(),
                },
                given_as: Some(PageImageRole::Notification),
            },
            PageImageAttachmentDetails {
                caption: Some("<strong>Already HTML</strong>".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            outcome.content,
            Some("<strong>Already HTML</strong>".to_string())
        );
    }

    #[test]
    fn a_left_side_notification_is_appended_to_left_notifications() {
        let site = TestSite::new("left_side");
        site.mkdir("blog");
        site.write_images_json(
            "",
            r#"{"title":null,"list":[{"title":null,"fileName":"photo.jpg","altText":"a"}]}"#,
        );

        attach_image(
            site.source(),
            site.img_source(),
            PageImageAttachmentRequest {
                given_path: "/blog".to_string(),
                source: GivenImageSource::Existing {
                    reference: "/photo".to_string(),
                },
                given_as: Some(PageImageRole::Notification),
            },
            PageImageAttachmentDetails {
                side: Some(NotificationSide::Left),
                ..Default::default()
            },
        )
        .unwrap();

        let config: PageConfig =
            crate::json_files::read_json(&site.source.join("blog/page.json")).unwrap();
        assert!(config.left_notifications.is_some());
        assert!(config.right_notifications.is_none());
    }
}
