//! End-to-end coverage for the six content-authoring subcommands
//! (`page new`, `image add`, `page add-image`, `social
//! add`/`update`/`remove`, `footer set`, `menu add-item`): scaffolds a fresh
//! project with `init`, authors content into it one piece at a time exactly
//! the way each subcommand's library function would, and checks the result
//! with `check` -- the same validation a maintainer would run before
//! publishing. Each unit test elsewhere in this crate exercises one function
//! in isolation; this exercises the same functions chained together against
//! one real tree, the way a maintainer actually uses them.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use vados::authoring::image_registry;
use vados::authoring::menu as menu_authoring;
use vados::authoring::page;
use vados::authoring::social::{self, GivenSocialLink, RecognizedSocialProvider};
use vados::check::check;
use vados::init::{self, ProjectBasics};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct TestProject {
    dir: PathBuf,
    source: String,
    img_source: String,
}

impl TestProject {
    /// Scaffolds a fresh project with `init`, the same starting point every
    /// content-authoring command assumes: main.json, menu.json and a home
    /// page.json already exist.
    fn new(name: &str) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("vados_authoring_integration_{name}_{id}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        init::scaffold(
            &dir,
            ProjectBasics {
                site_title: "Test Site".to_string(),
                home_intro: None,
                primary_color: None,
                footer_text: None,
                language: None,
                socials: vec![],
            },
        )
        .unwrap();

        let source = dir.join(init::SOURCE_DIR).to_str().unwrap().to_string();
        let img_source = dir.join(init::IMAGE_DIR).to_str().unwrap().to_string();
        TestProject {
            dir,
            source,
            img_source,
        }
    }

    fn write_image_file(&self, rel_path: &str) {
        let path = PathBuf::from(&self.img_source).join(rel_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "not really image bytes, but check never decodes them").unwrap();
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn a_freshly_scaffolded_project_already_checks_clean() {
    let project = TestProject::new("fresh_scaffold");

    let report = check(&project.source, &project.img_source);

    assert!(
        report.findings.is_empty(),
        "expected no findings, got {:?}",
        report.findings
    );
}

#[test]
fn authoring_every_kind_of_content_still_checks_clean() {
    let project = TestProject::new("full_chain");

    // `image add`: register a hero image.
    project.write_image_file("team/alice.jpg");
    assert!(
        image_registry::detect_registration_blockers(&project.img_source, "team", "alice.jpg")
            .is_empty()
    );
    let image_outcome = image_registry::register_image(
        &project.img_source,
        "team",
        "alice.jpg",
        Some("Alice".to_string()),
        "Alice smiling at the camera".to_string(),
    )
    .unwrap();
    assert_eq!(image_outcome.reference_key, "/team/alice");

    // `page new`: create a page that uses the freshly registered image as
    // its hero.
    assert!(page::detect_page_creation_blockers(
        &project.source,
        Some(&project.img_source),
        "/team",
        Some(&image_outcome.reference_key),
    )
    .is_empty());
    let page_outcome = page::create_page(
        &project.source,
        "/team",
        Some(image_outcome.reference_key.clone()),
        page::PageCreationDetails {
            title: "Team".to_string(),
            sub_title: Some("Who's behind this".to_string()),
            icon: None,
            summary: Some("Meet the team".to_string()),
            order: None,
            content: None,
        },
    )
    .unwrap();
    assert!(page_outcome.content_was_defaulted);

    // `page new` again: a child page, so `page add-image` below has
    // somewhere to attach a second image as a notification instead of a
    // hero.
    page::create_page(
        &project.source,
        "/team/alice",
        None,
        page::PageCreationDetails {
            title: "Alice".to_string(),
            sub_title: None,
            icon: None,
            summary: None,
            order: None,
            content: None,
        },
    )
    .unwrap();

    // `page add-image`: attach the same declared image to the child page.
    // The child page has no hero yet, so this becomes its hero rather than a
    // notification.
    let attach_request = page::PageImageAttachmentRequest {
        given_path: "/team/alice".to_string(),
        source: page::GivenImageSource::Existing {
            reference: image_outcome.reference_key.clone(),
        },
        given_as: None,
    };
    assert!(page::detect_attachment_blockers(
        &project.source,
        &project.img_source,
        &attach_request
    )
    .is_empty());
    let attach_outcome = page::attach_image(
        &project.source,
        &project.img_source,
        attach_request,
        page::PageImageAttachmentDetails::default(),
    )
    .unwrap();
    assert_eq!(attach_outcome.role, page::PageImageRole::Hero);

    // `page add-image` again, this time forced to `notification`, with a
    // plain-text caption -- not inline HTML, a .md file, or an .html file.
    // `Notification.content` is a `ContentReference` underneath, so this
    // must come out wrapped as inline HTML rather than tripping `check`'s
    // `UnrecognisedContentReference` finding.
    let second_attach_outcome = page::attach_image(
        &project.source,
        &project.img_source,
        page::PageImageAttachmentRequest {
            given_path: "/team/alice".to_string(),
            source: page::GivenImageSource::Existing {
                reference: image_outcome.reference_key.clone(),
            },
            given_as: Some(page::PageImageRole::Notification),
        },
        page::PageImageAttachmentDetails {
            caption: Some("Say hi to Alice".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        second_attach_outcome.content,
        Some("<p>Say hi to Alice</p>".to_string())
    );

    // `social add`: a recognised provider by handle.
    social::add_social_link(
        &project.source,
        GivenSocialLink {
            handle: Some((RecognizedSocialProvider::Github, "gklijs".to_string())),
            url: None,
            icon: None,
            brand_color: None,
            label: None,
        },
    )
    .unwrap()
    .unwrap();

    // `footer set`: replace the scaffolded footer with a markdown file, and
    // actually create that file so `check` finds it.
    fs::write(
        PathBuf::from(&project.source).join("footer.md"),
        "Built by the team.",
    )
    .unwrap();
    menu_authoring::set_footer(&project.source, "footer.md".to_string()).unwrap();

    // `menu add-item`: point the main menu at the new page.
    let menu_outcome =
        menu_authoring::add_menu_link(&project.source, "/team".to_string(), None, None)
            .unwrap()
            .unwrap();
    assert!(menu_outcome.would_resolve);

    let report = check(&project.source, &project.img_source);
    assert!(
        report.findings.is_empty(),
        "expected the fully authored project to check clean, got {:?}",
        report.findings
    );
}

#[test]
fn a_page_creation_block_is_reported_without_writing_anything() {
    let project = TestProject::new("blocked_page");

    // The home page already exists at "/".
    let blocks = page::detect_page_creation_blockers(&project.source, None, "/", None);

    assert_eq!(blocks.len(), 1);
    assert_eq!(
        blocks[0].reason,
        page::PageCreationBlockReason::PathAlreadyAPage
    );
}
