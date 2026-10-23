//! End-to-end coverage for the content-authoring subcommands (`page new`,
//! `image add`, `page add-image`, `social add`/`update`/`remove`, `footer
//! set`, `menu add-item`, `page add-luma-event`): scaffolds a fresh
//! project with `init`, authors content into it one piece at a time exactly
//! the way each subcommand's library function would, and checks the result
//! with `check` -- the same validation a maintainer would run before
//! publishing. Each unit test elsewhere in this crate exercises one function
//! in isolation; this exercises the same functions chained together against
//! one real tree, the way a maintainer actually uses them.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use vados::authoring::feed as feed_authoring;
use vados::authoring::image_registry;
use vados::authoring::luma::{self as luma_authoring, LumaEventDisplay};
use vados::authoring::menu as menu_authoring;
use vados::authoring::page;
use vados::authoring::social::{self, GivenSocialLink, RecognizedSocialProvider};
use vados::check::check;
use vados::generator::generate;
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
                favicon: None,
                site_url: None,
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
            date: None,
            child_order: None,
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
            date: None,
            child_order: None,
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

    // `page add-luma-event`: link an event to the new page.
    assert!(
        luma_authoring::detect_luma_event_blockers(&project.source, "/team", "evt-TeamDay")
            .is_empty()
    );
    luma_authoring::attach_luma_event(
        &project.source,
        "/team",
        "evt-TeamDay",
        luma_authoring::LumaEventAttachmentDetails {
            title: "Team day".to_string(),
            display: Some(LumaEventDisplay::Both),
            button_label: None,
        },
    )
    .unwrap();

    let report = check(&project.source, &project.img_source);
    assert!(
        report.findings.is_empty(),
        "expected the fully authored project to check clean, got {:?}",
        report.findings
    );
}

#[test]
fn a_linked_luma_event_is_published_on_its_own_page_only() {
    let project = TestProject::new("luma_publish");
    page::create_page(
        &project.source,
        "/events",
        None,
        page::PageCreationDetails {
            title: "Events".to_string(),
            sub_title: None,
            icon: None,
            summary: None,
            order: None,
            date: None,
            child_order: None,
            content: None,
        },
    )
    .unwrap();
    page::create_page(
        &project.source,
        "/events/archive",
        None,
        page::PageCreationDetails {
            title: "Archive".to_string(),
            sub_title: None,
            icon: None,
            summary: None,
            order: None,
            date: None,
            child_order: None,
            content: None,
        },
    )
    .unwrap();
    luma_authoring::attach_luma_event(
        &project.source,
        "/events",
        "evt-Jam",
        luma_authoring::LumaEventAttachmentDetails {
            title: "Jam night".to_string(),
            display: Some(LumaEventDisplay::RegisterButton),
            button_label: Some("Sign up".to_string()),
        },
    )
    .unwrap();

    let dest = project.dir.join("public");
    generate(&project.source, &project.img_source, dest.to_str().unwrap());

    let events = fs::read_to_string(dest.join("events.html")).unwrap();
    // The title links to the event on Luma whatever else is shown; see
    // vados.allium's `LumaEventAlwaysReachable`.
    assert!(events.contains(">Jam night</a>"));
    assert!(events.contains(r#"data-luma-event-id="evt-Jam""#));
    assert!(events.contains(">Sign up</a>"));
    assert!(events.contains("checkout-button.js"));
    // Register button only: no embedded event page.
    assert!(!events.contains("<iframe"));

    // Neither inherited by the page beneath, nor loading Luma's script on a
    // page without a button; see `LumaEventsStayOnTheirPage` and
    // `ThirdPartyCodeOnlyWhereUsed`.
    for other in ["index.html", "events/archive.html"] {
        let html = fs::read_to_string(dest.join(other)).unwrap();
        assert!(!html.contains("luma"), "{other} mentions Luma");
    }
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

#[test]
fn a_dutch_site_with_a_description_and_favicon_is_published_in_dutch() {
    let project = TestProject::new("dutch_site");
    let main_json = PathBuf::from(&project.source).join("main.json");
    let mut main: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&main_json).unwrap()).unwrap();
    main["language"] = "nl".into();
    main["description"] = "Een site over neurodiversiteit.".into();
    main["favicon"] = "favicon.svg".into();
    fs::write(&main_json, serde_json::to_string_pretty(&main).unwrap()).unwrap();
    fs::write(
        PathBuf::from(&project.source).join("favicon.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
    )
    .unwrap();
    for (path, title, summary) in [
        ("/over", "Over ons", Some("Wie we zijn.")),
        ("/over/team", "Team", None),
    ] {
        page::create_page(
            &project.source,
            path,
            None,
            page::PageCreationDetails {
                title: title.to_string(),
                sub_title: None,
                icon: None,
                summary: summary.map(String::from),
                order: None,
                date: None,
                child_order: None,
                content: None,
            },
        )
        .unwrap();
    }
    let report = check(&project.source, &project.img_source);
    assert!(report.findings.is_empty(), "{:?}", report.findings);

    let dest = project.dir.join("public");
    generate(&project.source, &project.img_source, dest.to_str().unwrap());

    assert!(dest.join("favicon.svg").is_file());
    let home = fs::read_to_string(dest.join("index.html")).unwrap();
    assert!(home.contains(r#"<link rel="icon" href="/favicon.svg" type="image/svg+xml">"#));
    // No summary of its own, so the site description stands in.
    assert!(home.contains(r#"<meta name="description" content="Een site over neurodiversiteit.""#));
    assert!(home.contains(">Naar de hoofdinhoud</a>"));
    assert!(home.contains(r#"aria-label="Hoofdmenu openen of sluiten""#));
    assert!(!home.contains("Skip to main content"));

    let over = fs::read_to_string(dest.join("over.html")).unwrap();
    assert!(over.contains(r#"<meta name="description" content="Wie we zijn.""#));

    let team = fs::read_to_string(dest.join("over/team.html")).unwrap();
    assert!(team.contains(r#"aria-label="Kruimelpad""#));
    assert!(team.contains(r#"aria-current="page""#));
}

#[test]
fn a_section_ordered_by_date_lists_its_newest_posts_first() {
    let project = TestProject::new("date_ordered_blog");

    // `page new --child-order date-desc`: a section listing its newest post
    // first.
    page::create_page(
        &project.source,
        "/blog",
        None,
        page::PageCreationDetails {
            title: "Blog".to_string(),
            sub_title: None,
            icon: None,
            summary: None,
            order: None,
            date: None,
            child_order: Some(page::ChildOrder::DateDesc),
            content: None,
        },
    )
    .unwrap();
    assert!(page::parent_orders_by_date(
        &project.source,
        "/blog/anything"
    ));

    for (slug, title, date) in [
        ("first", "First post", "2026-01-10"),
        ("third", "Third post", "2026-10-22T09:30"),
        ("second", "Second post", "2026-05-03"),
        ("fourth", "Fourth post", "2026-10-22T17:00"),
    ] {
        page::create_page(
            &project.source,
            &format!("/blog/{slug}"),
            None,
            page::PageCreationDetails {
                title: title.to_string(),
                sub_title: None,
                icon: None,
                summary: None,
                order: None,
                date: Some(date.parse().unwrap()),
                child_order: None,
                content: None,
            },
        )
        .unwrap();
    }

    let report = check(&project.source, &project.img_source);
    assert!(report.findings.is_empty(), "{:?}", report.findings);

    let dest = project.dir.join("public");
    generate(&project.source, &project.img_source, dest.to_str().unwrap());

    // The section page advertises its three newest posts, newest first.
    let blog = fs::read_to_string(dest.join("blog.html")).unwrap();
    let position = |needle: &str| {
        blog.find(needle)
            .unwrap_or_else(|| panic!("{needle} missing"))
    };
    assert!(position("Fourth post") < position("Third post"));
    assert!(position("Third post") < position("Second post"));
    assert!(!blog.contains("First post"));
    assert!(blog.contains(r#"<time datetime="2026-10-22T17:00">22 October 2026</time>"#));

    // A post shows its own date under its title.
    let post = fs::read_to_string(dest.join("blog/second.html")).unwrap();
    assert!(post.contains(r#"<time datetime="2026-05-03">3 May 2026</time>"#));
}

#[test]
fn a_section_with_a_feed_publishes_its_newest_dated_posts_as_atom() {
    let project = TestProject::new("atom_feed");
    let source = PathBuf::from(&project.source);

    page::create_page(
        &project.source,
        "/blog",
        None,
        page::PageCreationDetails {
            title: "Blog".to_string(),
            sub_title: None,
            icon: None,
            summary: None,
            order: None,
            date: None,
            child_order: Some(page::ChildOrder::DateDesc),
            content: None,
        },
    )
    .unwrap();

    // `page enable-feed`, recording the site URL main.json doesn't have yet.
    assert!(
        feed_authoring::detect_feed_enablement_blockers(&project.source, "/blog", None).is_empty()
    );
    assert_eq!(feed_authoring::usable_site_url(&project.source), None);
    let outcome = feed_authoring::enable_feed(
        &project.source,
        "/blog",
        feed_authoring::FeedEnablementDetails {
            title: None,
            limit: Some(2),
            site_url: Some("https://example.com/".to_string()),
        },
    )
    .unwrap();
    assert_eq!(
        outcome.feed_url.as_deref(),
        Some("https://example.com/blog/feed.xml")
    );

    for (slug, title, date, summary) in [
        ("old", "Old post", "2026-01-10", None),
        (
            "new",
            "Fish & <chips>",
            "2026-10-22T09:30",
            Some("Tasty & cheap"),
        ),
        ("mid", "Middle post", "2026-05-03", None),
    ] {
        page::create_page(
            &project.source,
            &format!("/blog/{slug}"),
            None,
            page::PageCreationDetails {
                title: title.to_string(),
                sub_title: None,
                icon: None,
                summary: summary.map(String::from),
                order: None,
                date: Some(date.parse().unwrap()),
                child_order: None,
                content: Some("post.md".to_string()),
            },
        )
        .unwrap();
        fs::write(
            source.join(format!("blog/{slug}/post.md")),
            format!("## {slug}\n\nSome *text* with a [link](/blog) & an ampersand.\n"),
        )
        .unwrap();
    }

    let report = check(&project.source, &project.img_source);
    assert!(report.findings.is_empty(), "{:?}", report.findings);

    let dest = project.dir.join("public");
    generate(&project.source, &project.img_source, dest.to_str().unwrap());

    let xml = fs::read_to_string(dest.join("blog/feed.xml")).unwrap();
    let doc = roxmltree::Document::parse(&xml).expect("the feed is well-formed XML");
    let feed = doc.root_element();
    assert_eq!(
        feed.tag_name().namespace(),
        Some("http://www.w3.org/2005/Atom")
    );
    let child_text = |node: roxmltree::Node, name: &str| {
        node.children()
            .find(|c| c.has_tag_name(name))
            .and_then(|c| c.text())
            .map(String::from)
    };
    assert_eq!(
        child_text(feed, "title").as_deref(),
        Some("Test Site – Blog")
    );
    assert_eq!(
        child_text(feed, "id").as_deref(),
        Some("https://example.com/blog")
    );
    assert_eq!(
        child_text(feed, "updated").as_deref(),
        Some("2026-10-22T09:30:00Z")
    );

    // Newest first, capped at the limit of two.
    let entries: Vec<_> = feed
        .children()
        .filter(|c| c.has_tag_name("entry"))
        .collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(
        child_text(entries[0], "title").as_deref(),
        Some("Fish & <chips>")
    );
    assert_eq!(
        child_text(entries[0], "id").as_deref(),
        Some("https://example.com/blog/new")
    );
    assert_eq!(
        child_text(entries[0], "summary").as_deref(),
        Some("Tasty & cheap")
    );
    let content = child_text(entries[0], "content").unwrap();
    assert!(content.contains("<em>text</em>"), "{content}");
    assert!(content.contains(r#"<a href="/blog">link</a>"#), "{content}");
    assert_eq!(
        child_text(entries[1], "title").as_deref(),
        Some("Middle post")
    );
    assert_eq!(child_text(entries[1], "summary"), None);

    // Feed readers find it from the section and from every page in it; people
    // find it on the section's own page.
    let blog = fs::read_to_string(dest.join("blog.html")).unwrap();
    assert!(blog.contains(r#"rel="alternate" type="application/atom+xml""#));
    assert!(blog.contains(r#"href="/blog/feed.xml""#));
    assert!(blog.contains("Subscribe (Atom feed)"));
    let post = fs::read_to_string(dest.join("blog/old.html")).unwrap();
    assert!(post
        .contains(r#"type="application/atom+xml" title="Test Site – Blog" href="/blog/feed.xml""#));
    assert!(!post.contains("Subscribe (Atom feed)"));
    let home = fs::read_to_string(dest.join("index.html")).unwrap();
    assert!(!home.contains("application/atom+xml"));
}

#[test]
fn without_a_site_url_no_feed_is_published_or_advertised() {
    let project = TestProject::new("feed_without_site_url");
    fs::create_dir_all(PathBuf::from(&project.source).join("blog")).unwrap();
    fs::write(
        PathBuf::from(&project.source).join("blog/page.json"),
        r#"{"title":"Blog","content":"<p>Posts</p>","feed":true}"#,
    )
    .unwrap();

    let report = check(&project.source, &project.img_source);
    assert!(!report.passed());

    let dest = project.dir.join("public");
    generate(&project.source, &project.img_source, dest.to_str().unwrap());

    assert!(!dest.join("blog/feed.xml").exists());
    let blog = fs::read_to_string(dest.join("blog.html")).unwrap();
    assert!(!blog.contains("application/atom+xml"));
}
