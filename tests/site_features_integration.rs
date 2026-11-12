//! End-to-end coverage for what a site can turn on: generates small sites and
//! reads back the published files, and the link check over them.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use vados::check::{check, CheckReport, FindingKind};
use vados::generator::generate;
use vados::link_check::check_output;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct Site {
    root: PathBuf,
}

const MENU: &str = r#"{"mainMenu":[],"socials":[]}"#;

impl Site {
    fn new(main_extra: &str) -> Site {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("vados_site_features_it_{id}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("root")).unwrap();
        fs::create_dir_all(root.join("img")).unwrap();
        let site = Site { root };
        site.write(
            "main.json",
            &format!(
                r#"{{"siteTitle":"T","jsFiles":[],"cssFiles":[],"siteUrl":"https://example.com","language":"nl","footerContent":"<p>f</p>"{main_extra}}}"#
            ),
        );
        site.write("menu.json", MENU);
        site.write("page.json", r#"{"title":"Home","content":"<p>home</p>"}"#);
        site
    }
    fn write(&self, path: &str, contents: &str) {
        let file = self.root.join("root").join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, contents).unwrap();
    }
    fn try_generate(
        &self,
    ) -> Result<vados::generator::GenerateReport, vados::generator::GenerateError> {
        generate(
            self.root.join("root").to_str().unwrap(),
            self.root.join("img").to_str().unwrap(),
            self.root.join("public").to_str().unwrap(),
        )
    }
    fn generate(&self) {
        self.try_generate().unwrap();
    }
    fn published(&self, path: &str) -> String {
        fs::read_to_string(self.root.join("public").join(path))
            .unwrap_or_else(|e| panic!("{path}: {e}"))
    }
    fn exists(&self, path: &str) -> bool {
        self.root.join("public").join(path).exists()
    }
    fn check(&self) -> CheckReport {
        check(
            self.root.join("root").to_str().unwrap(),
            self.root.join("img").to_str().unwrap(),
        )
    }
    fn assert_links_are_fine(&self) {
        let findings = check_output(
            self.root.join("public").to_str().unwrap(),
            Some("https://example.com"),
        );
        let found: Vec<String> = findings.iter().map(|f| f.to_string()).collect();
        assert!(found.is_empty(), "{found:#?}");
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn kinds(report: &CheckReport) -> Vec<FindingKind> {
    report.findings.iter().map(|f| f.kind).collect()
}

fn page(title: &str, extra: &str) -> String {
    format!(r#"{{"title":"{title}","content":"<p>{title} body</p>"{extra}}}"#)
}

#[test]
fn a_dropdown_gets_a_translated_hidden_button_and_the_hover_fallback() {
    let site = Site::new("");
    site.write("menu.json", r#"{"mainMenu":[{"url":"/s","title":null,"icon":"no-such-icon"},{"url":"/t","title":null,"icon":"no-such-icon"}],"socials":[]}"#);
    site.write("s/page.json", &page("Sectie", ""));
    site.write("s/a/page.json", &page("A", r#","icon":"no-such-icon""#));
    site.write("t/page.json", &page("Plat", ""));
    site.generate();
    let html = site.published("index.html");
    assert!(
        html.contains(r#"has-dropdown is-hoverable" data-nav-disclosure"#),
        "{html}"
    );
    assert!(
        html.contains(r#"aria-controls="nav-subpages-0" aria-label="Subpagina&#39;s van Sectie""#),
        "{html}"
    );
    assert!(html.contains(r#"id="nav-subpages-0""#));
    assert!(html.contains("hidden"));
    // Unknown icons leave no empty box, at any level of the menu.
    assert!(!html.contains(r#"<span class="icon">"#), "{html}");
    assert!(!site.published("js/vados.js").is_empty());
}

#[test]
fn long_dropdowns_are_cut_short_with_a_link_to_the_section() {
    let site = Site::new(r#","menuDropdownLimit":2"#);
    site.write(
        "menu.json",
        r#"{"mainMenu":[{"url":"/s","title":null,"icon":null}],"socials":[]}"#,
    );
    site.write("s/page.json", &page("Sectie", ""));
    for c in ["a", "b", "c"] {
        site.write(&format!("s/{c}/page.json"), &page(c, ""));
    }
    site.generate();
    let html = site.published("index.html");
    assert!(html.contains("Meer in Sectie"), "{html}");
    assert_eq!(html.matches(r#"href="/s/"#).count(), 2, "{html}");
}

#[test]
fn navbar_theme_replaces_the_colours_default_theme() {
    let site = Site::new(r#","navbarColor":"Warning","navbarTheme":"dark""#);
    site.generate();
    assert!(site
        .published("index.html")
        .contains("is-warning theme-dark"));
}

#[test]
fn a_feed_can_select_nested_and_unlisted_pages() {
    let site = Site::new("");
    site.write("page.json", &page("Home", r#","feed":{"from":"/i","depth":1,"limit":2,"includeUnlisted":true,"content":"summary"}"#));
    site.write("i/page.json", &page("I", r#","listed":false"#));
    site.write(
        "i/one/page.json",
        &page(
            "One",
            r#","listed":false,"date":"2026-01-01","summary":"First""#,
        ),
    );
    site.write(
        "i/two/page.json",
        &page(
            "Two",
            r#","listed":false,"date":"2026-01-02","summary":"Second""#,
        ),
    );
    site.write(
        "i/three/page.json",
        &page("Three", r#","listed":false,"date":"2026-01-03""#),
    );
    site.write(
        "i/one/deep/page.json",
        &page("Deep", r#","date":"2026-02-01","summary":"S""#),
    );
    site.write("about/page.json", &page("About", ""));
    site.generate();
    let xml = site.published("feed.xml");
    let entries: Vec<&str> = xml.split("<entry>").skip(1).collect();
    assert_eq!(entries.len(), 2, "{xml}");
    assert!(entries[0].contains("<title>Three</title>"));
    // No summary to carry, so it keeps its content.
    assert!(entries[0].contains("<content"));
    assert!(entries[1].contains("<summary>Second</summary>"));
    assert!(!entries[1].contains("<content"));
    assert!(!kinds(&site.check()).contains(&FindingKind::UndatedPageLeftOutOfFeed));
    assert!(!kinds(&site.check()).contains(&FindingKind::EmptyFeed));
}

#[test]
fn an_empty_feed_and_an_unknown_source_are_warned_about() {
    let site = Site::new("");
    site.write("page.json", &page("Home", r#","feed":{"from":"/nope"}"#));
    let report = site.check();
    assert!(kinds(&report).contains(&FindingKind::EmptyFeed));
    assert!(kinds(&report).contains(&FindingKind::UnresolvedSelectionSource));
    assert!(report.passed());
}

#[test]
fn feed_true_still_lists_direct_listed_children_only() {
    let site = Site::new("");
    site.write("page.json", &page("Home", r#","feed":true"#));
    site.write(
        "a/page.json",
        &page("A", r#","date":"2026-01-01","summary":"S""#),
    );
    site.write(
        "a/b/page.json",
        &page("B", r#","date":"2026-01-02","summary":"S""#),
    );
    site.write(
        "c/page.json",
        &page("C", r#","date":"2026-01-03","summary":"S","listed":false"#),
    );
    site.generate();
    let xml = site.published("feed.xml");
    assert_eq!(xml.matches("<entry>").count(), 1, "{xml}");
    assert!(xml.contains("<title>A</title>"));
    assert!(xml.contains("<content"));
}

#[test]
fn a_noindexed_page_says_so_and_leaves_the_sitemap_but_stays_in_the_menu_when_asked() {
    let site = Site::new("");
    site.write("hidden/page.json", &page("Hidden", r#","noindex":true"#));
    site.write(
        "quiet/page.json",
        &page("Quiet", r#","noindex":true,"listed":false"#),
    );
    site.generate();
    assert!(site
        .published("hidden.html")
        .contains(r#"content="noindex, follow""#));
    assert!(site
        .published("index.html")
        .contains("max-image-preview:large"));
    let sitemap = site.published("sitemap.xml");
    assert!(!sitemap.contains("hidden"), "{sitemap}");
    let report = site.check();
    let flagged: Vec<&str> = report
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::NoindexedPageIsListed)
        .map(|f| f.location.as_str())
        .collect();
    assert_eq!(flagged, ["/hidden"]);
}

#[test]
fn search_is_off_by_default_and_changes_nothing() {
    let site = Site::new("");
    site.generate();
    assert!(!site.exists("search-index.json"));
    assert!(!site.exists("search.html"));
    assert!(!site.exists("js/vados-search.js"));
    assert!(!site.published("index.html").contains("search"));
}

#[test]
fn search_true_publishes_a_translated_page_index_and_menu_entry() {
    let site = Site::new(r#","search":true"#);
    site.write("menu.json", MENU);
    site.write("blog/page.json", &page("Blog", ""));
    site.write(
        "blog/post/page.json",
        &page("Café", r#","date":"2026-01-02","summary":"Over koffie""#),
    );
    site.write("secret/page.json", &page("Secret", r#","listed":false"#));
    site.write(
        "gone/page.json",
        &page("Gone", r#","noindex":true,"listed":false"#),
    );
    assert!(site.check().passed());
    site.generate();
    let search = site.published("search.html");
    assert!(
        search.contains(r#"src="/js/vados-search.js""#)
            || search.contains("src=/js/vados-search.js"),
        "{search}"
    );
    assert!(search.contains("Zoektermen"), "{search}");
    assert!(search.contains("noindex, follow"));
    assert!(!site.published("index.html").contains("vados-search"));
    assert!(site.published("index.html").contains(r#"href="/search""#));
    assert!(!site.published("sitemap.xml").contains("search"));
    let index: serde_json::Value =
        serde_json::from_str(&site.published("search-index.json")).unwrap();
    let records = index.as_array().unwrap();
    let urls: Vec<&str> = records.iter().map(|r| r["url"].as_str().unwrap()).collect();
    assert_eq!(urls, ["/", "/blog", "/blog/post"]);
    assert_eq!(records[2]["text"], "Over koffie");
    assert_eq!(records[2]["section"], "Blog");
    assert_eq!(records[2]["dateText"], "2 januari 2026");
    assert_eq!(records[1]["text"], "Blog body");
    site.assert_links_are_fine();
}

#[test]
fn a_sites_own_search_page_keeps_its_own_say() {
    let site = Site::new(r#","search":{"from":"/blog","includeUnlisted":true,"menu":false}"#);
    site.write("search/page.json", &page("Vinden", r#","listed":false"#));
    site.write("blog/page.json", &page("Blog", ""));
    site.write("blog/a/page.json", &page("A", r#","listed":false"#));
    site.generate();
    let search = site.published("search.html");
    assert!(search.contains("Vinden"));
    assert!(search.contains("search-form"));
    assert!(!search.contains("noindex"));
    assert!(!site.published("index.html").contains(r#"href="/search""#));
    let index = site.published("search-index.json");
    assert!(
        index.contains("/blog/a") && !index.contains(r#""url":"/""#),
        "{index}"
    );
}

const EVENT: &str = r#","summary":"Samen spelen","event":{"start":"2026-11-05T14:00:00+01:00","end":"2026-11-05T15:30:00+01:00","timeZone":"Europe/Amsterdam","attendance":"inPerson","status":"cancelled","venue":"Zaal <1>","registrationUrl":"https://example.org/r","sequence":2}"#;

#[test]
fn an_event_page_shows_its_details_and_describes_itself_as_an_event() {
    let site = Site::new("");
    site.write(
        "jam/page.json",
        &page("Jam", &format!("{EVENT},\"date\":\"2026-10-01\"")),
    );
    assert!(site.check().passed());
    site.generate();
    let html = site.published("jam.html");
    assert!(
        html.contains("5 november 2026, 14:00–15:30 (Europe/Amsterdam)"),
        "{html}"
    );
    assert!(html.contains("Afgelast"));
    assert!(html.contains("Zaal &#60;1&#62;"), "{html}");
    assert!(html.contains("Niet bevestigd"), "cost is unknown, not free");
    assert!(html.contains(r#""@type":"Event""#), "{html}");
    assert!(!html.contains("BlogPosting"));
    assert!(html.contains("/jam.ics"));
    let ics = site.published("jam.ics");
    assert!(ics.contains("UID:https://example.com/jam#event\r\n"));
    assert!(ics.contains("SEQUENCE:2\r\n"));
    assert!(ics.contains("STATUS:CANCELLED\r\n"));
    assert!(ics.contains("DTSTAMP:20261001T000000Z\r\n"));
    site.assert_links_are_fine();
}

#[test]
fn an_invalid_event_is_an_error_and_left_off_the_page() {
    let site = Site::new("");
    site.write(
        "jam/page.json",
        &page(
            "Jam",
            r#","event":{"start":"2026-07-05T14:00:00+01:00","timeZone":"Europe/Amsterdam"}"#,
        ),
    );
    let report = site.check();
    assert!(kinds(&report).contains(&FindingKind::InvalidEvent));
    assert!(!report.passed());
    site.generate();
    assert!(!site.published("jam.html").contains("event-details"));
    assert!(!site.exists("jam.ics"));
}

#[test]
fn events_without_a_site_url_are_warned_about_and_get_no_calendar_file() {
    let site = Site::new("");
    site.write(
        "main.json",
        r#"{"siteTitle":"T","jsFiles":[],"cssFiles":[],"footerContent":"<p>f</p>"}"#,
    );
    site.write("jam/page.json", &page("Jam", EVENT));
    assert!(kinds(&site.check()).contains(&FindingKind::EventWithoutSiteUrl));
    site.generate();
    assert!(!site.exists("jam.ics"));
    assert!(!site.published("jam.html").contains(".ics"));
}

#[test]
fn an_agenda_lists_events_soonest_first_with_a_calendar_of_them_all() {
    let site = Site::new("");
    site.write("agenda/page.json", &page("Agenda", r#","agenda":true"#));
    site.write("agenda/later/page.json", &page("Later", EVENT));
    site.write(
        "agenda/sooner/page.json",
        &page(
            "Sooner",
            r#","event":{"start":"2026-10-01T10:00:00+02:00","timeZone":"Europe/Amsterdam"}"#,
        ),
    );
    site.write("agenda/not-an-event/page.json", &page("Plain", ""));
    assert!(site.check().passed());
    site.generate();
    let html = site.published("agenda.html");
    let sooner = html.find("Sooner</a>").expect(&html);
    let later = html.find("Later</a>").expect(&html);
    assert!(sooner < later);
    assert!(!html.contains("Plain</a></h2>"));
    assert!(
        html.contains(r#"data-event-end="2026-11-05T14:30:00Z""#),
        "{html}"
    );
    assert!(html.contains("vados-agenda.js"));
    assert!(!site.published("index.html").contains("vados-agenda.js"));
    assert_eq!(
        site.published("agenda/calendar.ics")
            .matches("BEGIN:VEVENT")
            .count(),
        2
    );
    assert!(!site.published("js/vados-agenda.js").is_empty());
    site.assert_links_are_fine();
}

#[test]
fn a_spare_tile_fills_the_shorter_column_for_real() {
    let site = Site::new(
        r#","spareTile":{"title":"Meer","links":[{"url":"/a","title":"A"},{"url":"/nope","title":"Weg"},{"url":"/feed.xml","title":"Feed"}]}"#,
    );
    site.write("a/page.json", &page("A", ""));
    site.write("page.json", r#"{"title":"Home","content":"<p>x</p>","leftNotifications":[{"content":"<p>1</p>"},{"content":"<p>2</p>"}],"rightNotifications":[{"content":"<p>3</p>"}]}"#);
    let report = site.check();
    let flagged: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::SpareTileLinkMatchesNoPage)
        .map(|f| f.detail.clone().unwrap())
        .collect();
    assert_eq!(flagged, ["/nope"]);
    site.generate();
    let html = site.published("index.html");
    assert!(html.contains(r#"id="spare-tile""#), "{html}");
    assert!(!html.contains("is-row-span-2-tablet\" id=\"sub"), "{html}");
}

#[test]
fn a_generated_site_passes_its_own_link_check_and_a_broken_one_does_not() {
    let site = Site::new(r#","search":true"#);
    site.write("page.json", r#"{"title":"Home","content":"<p><a href=\"/about#team\">a</a> <a href=\"/missing\">b</a></p>"}"#);
    site.write("about/page.json", &page("About", ""));
    site.generate();
    let found = check_output(
        site.root.join("public").to_str().unwrap(),
        Some("https://example.com"),
    );
    let mut details: Vec<(FindingKind, String)> = found
        .iter()
        .map(|f| (f.kind, f.detail.clone().unwrap()))
        .collect();
    details.sort_by(|a, b| a.1.cmp(&b.1));
    assert_eq!(
        details,
        [
            (
                FindingKind::MissingLinkFragment,
                String::from("/about#team")
            ),
            (FindingKind::BrokenInternalLink, String::from("/missing"))
        ]
    );
    assert!(found.iter().all(|f| f.location == "/"));
    site.write(
        "page.json",
        r#"{"title":"Home","content":"<p><a href=\"/about\">a</a></p>"}"#,
    );
    site.generate();
    site.assert_links_are_fine();
}

#[test]
fn a_menu_item_is_active_by_whole_path_segments_and_marks_the_current_page() {
    let site = Site::new("");
    site.write(
        "menu.json",
        r#"{"mainMenu":[{"url":"/blog","title":null,"icon":null},{"url":"/blogging","title":null,"icon":null},{"url":"/","title":"Start","icon":null}],"socials":[]}"#,
    );
    site.write("blog/page.json", &page("Blog", ""));
    site.write("blogging/page.json", &page("Blogging", ""));
    site.write("blog/post/page.json", &page("Post", ""));
    site.generate();
    let blogging = site.published("blogging.html");
    assert!(
        blogging.contains(r#"class="navbar-link" href="/blog""#)
            || blogging.contains(r#"class="navbar-item is-tab" href="/blog""#),
        "{blogging}"
    );
    assert!(blogging.contains(r#"class="navbar-item is-tab is-active" href="/blogging" target="_self" aria-current="page""#), "{blogging}");
    // The home entry isn't active just because everything is below it.
    assert!(
        blogging.contains(r#"class="navbar-link" href="/" target="_self""#),
        "{blogging}"
    );
    let post = site.published("blog/post.html");
    assert!(
        post.contains(r#"class="navbar-link is-active" href="/blog" target="_self">"#),
        "{post}"
    );
    assert!(
        post.contains(r#"href="/blog/post" target="_self" aria-current="page""#),
        "{post}"
    );
    assert!(site
        .published("index.html")
        .contains(r#"id="nav-site-title" aria-current="page""#));
}

#[test]
fn dated_luma_events_join_an_agenda_and_its_calendar_and_undated_ones_do_not() {
    let site = Site::new("");
    site.write("agenda/page.json", &page("Agenda", r#","agenda":true,"lumaEvents":[{"eventId":"evt-own","title":"Op de agenda","start":"2026-09-01T10:00:00+02:00","timeZone":"Europe/Amsterdam"}]"#));
    site.write("agenda/jam/page.json", &page("Jam", r#","event":{"start":"2026-10-01T10:00:00+02:00","timeZone":"Europe/Amsterdam"},"lumaEvents":[{"eventId":"evt-dated","title":"Gedateerd","start":"2026-11-05T14:00:00+01:00","timeZone":"Europe/Amsterdam"},{"eventId":"evt-undated","title":"Ongedateerd"}]"#));
    assert!(site.check().passed());
    site.generate();
    let html = site.published("agenda.html");
    let position = |needle: &str| {
        html.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {html}"))
    };
    assert!(position("Op de agenda</a></h2>") < position("Jam</a></h2>"));
    assert!(position("Jam</a></h2>") < position("Gedateerd</a></h2>"));
    assert!(!html.contains("Ongedateerd</a></h2>"));
    assert!(html.contains(r#"href="https://luma.com/event/evt-dated""#));
    let ics = site.published("agenda/calendar.ics");
    assert_eq!(ics.matches("BEGIN:VEVENT").count(), 3);
    assert!(ics.contains("UID:https://luma.com/event/evt-dated\r\n"));
}

#[test]
fn a_luma_event_with_an_invalid_time_is_an_error_but_stays_on_its_page() {
    let site = Site::new("");
    site.write("agenda/page.json", &page("Agenda", r#","agenda":true,"lumaEvents":[{"eventId":"evt-a","title":"Mis","start":"2026-07-05T14:00:00+01:00","timeZone":"Europe/Amsterdam"}]"#));
    assert!(kinds(&site.check()).contains(&FindingKind::InvalidEvent));
    site.generate();
    let html = site.published("agenda.html");
    assert!(html.contains("luma.com/embed/event/evt-a"));
    assert!(!html.contains("Mis</a></h2><p><time"));
    assert_eq!(
        site.published("agenda/calendar.ics")
            .matches("BEGIN:VEVENT")
            .count(),
        0
    );
}

#[test]
fn check_finds_dead_page_links_in_content_without_a_build() {
    let site = Site::new(r#","search":true"#);
    site.write("main.json", r#"{"siteTitle":"T","jsFiles":[],"cssFiles":[],"search":true,"footerContent":"<p><a href=\"/footer-gone\">f</a></p>"}"#);
    site.write("page.json", r#"{"title":"Home","content":"<p><a href=\"/about\">ok</a> <a href=\"/about/\">ok</a> <a href=\"/search\">ok</a> <a href=\"/gone#x\">no</a> <a href=\"/file.pdf\">skip</a> <a href=\"https://x.test/gone\">skip</a></p>","leftNotifications":[{"content":"<p><a href=\"/tile-gone\">t</a></p>"}]}"#);
    site.write(
        "about/page.json",
        r#"{"title":"About","content":"about.md"}"#,
    );
    site.write("about/about.md", "[home](/) and [gone](/md-gone)\n");
    let report = site.check();
    let mut flagged: Vec<String> = report
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::BrokenContentLink)
        .map(|f| f.detail.clone().unwrap())
        .collect();
    flagged.sort();
    assert_eq!(flagged, ["/footer-gone", "/gone", "/md-gone", "/tile-gone"]);
    assert!(report.passed());
}

#[test]
fn an_empty_feed_does_not_change_between_builds() {
    let site = Site::new("");
    site.write("page.json", &page("Home", r#","feed":true"#));
    site.generate();
    let xml = site.published("feed.xml");
    assert!(
        xml.contains("<updated>1970-01-01T00:00:00Z</updated>"),
        "{xml}"
    );
    site.write(
        "page.json",
        &page("Home", r#","feed":true,"updated":"2026-03-01""#),
    );
    site.generate();
    assert!(site
        .published("feed.xml")
        .contains("<updated>2026-03-01T00:00:00Z</updated>"));
}

#[test]
fn check_warns_about_a_static_headers_file_a_host_may_reject() {
    let site = Site::new(r#","staticDir":"static""#);
    site.write(
        "static/_headers",
        "# a comment\n/*\n  X-Frame-Options: DENY\n",
    );
    let report = site.check();
    assert!(kinds(&report).contains(&FindingKind::HeadersFileProblem));
    assert!(report.passed());
}

#[test]
fn a_selection_from_the_root_takes_root_level_pages() {
    let site = Site::new(r#","search":{"from":"/","depth":1}"#);
    site.write(
        "page.json",
        &page(
            "Home",
            r#","feed":{"from":"/","depth":1},"agenda":{"from":"/","depth":1}"#,
        ),
    );
    site.write(
        "post/page.json",
        &page("Post", r#","date":"2026-01-02","summary":"S""#),
    );
    site.write(
        "jam/page.json",
        &page(
            "Jam",
            r#","event":{"start":"2026-11-05T14:00:00+01:00","timeZone":"Europe/Amsterdam"}"#,
        ),
    );
    site.write(
        "post/deep/page.json",
        &page("Deep", r#","date":"2026-01-03","summary":"S""#),
    );
    let report = site.check();
    assert!(!kinds(&report).contains(&FindingKind::EmptyFeed));
    site.generate();
    let feed = site.published("feed.xml");
    assert!(
        feed.contains("<title>Post</title>") && !feed.contains("<title>Deep</title>"),
        "{feed}"
    );
    assert!(site.published("index.html").contains("Jam</a></h2>"));
    let index = site.published("search-index.json");
    assert!(
        index.contains(r#""url":"/post""#) && !index.contains(r#""url":"/post/deep""#),
        "{index}"
    );
}

#[test]
fn a_content_link_to_the_generated_search_page_is_fine_only_with_search_on() {
    let content = r#"{"title":"Home","content":"<p><a href=\"/search\">zoek</a></p>"}"#;
    let on = Site::new(r#","search":true"#);
    on.write("page.json", content);
    assert!(!kinds(&on.check()).contains(&FindingKind::BrokenContentLink));
    let off = Site::new("");
    off.write("page.json", content);
    assert!(kinds(&off.check()).contains(&FindingKind::BrokenContentLink));
}

#[test]
fn an_unreadable_event_costs_only_itself_in_generate_and_is_an_error_in_check() {
    let site = Site::new("");
    site.write("jam/page.json", &page("Jam", r#","event":{"start":"2026-11-05T14:00:00+01:00","timeZone":"UTC","status":"canceled"}"#));
    site.write(
        "gig/page.json",
        &page("Gig", r#","event":{"venue":"Hall"}"#),
    );
    let report = site.check();
    assert_eq!(
        kinds(&report)
            .iter()
            .filter(|k| **k == FindingKind::InvalidEvent)
            .count(),
        2
    );
    assert!(!kinds(&report).contains(&FindingKind::PageConfigUnreadable));
    assert!(!report.passed());
    site.generate();
    assert!(site.published("jam.html").contains("Jam body"));
    assert!(!site.published("jam.html").contains("event-details"));
}

#[test]
fn the_same_luma_event_on_two_pages_is_one_entry_in_an_agenda() {
    let luma = r#","lumaEvents":[{"eventId":"evt-a","title":"Gig","start":"2026-11-05T14:00:00+01:00","timeZone":"Europe/Amsterdam"}]"#;
    let site = Site::new("");
    site.write("agenda/page.json", &page("Agenda", r#","agenda":true"#));
    site.write("agenda/a/page.json", &page("A", luma));
    site.write("agenda/b/page.json", &page("B", luma));
    site.generate();
    assert_eq!(
        site.published("agenda/calendar.ics")
            .matches("BEGIN:VEVENT")
            .count(),
        1
    );
}

#[test]
fn an_event_page_at_an_agendas_calendar_address_is_warned_about_and_the_agenda_wins() {
    let site = Site::new("");
    site.write(
        "agenda/page.json",
        &page("Agenda", r#","agenda":{"from":"/other"}"#),
    );
    site.write("other/page.json", &page("Other", ""));
    site.write(
        "agenda/calendar/page.json",
        &page(
            "Calendar",
            r#","event":{"start":"2026-11-05T14:00:00+01:00","timeZone":"Europe/Amsterdam"}"#,
        ),
    );
    assert!(kinds(&site.check()).contains(&FindingKind::CalendarFileCollision));
    site.generate();
    assert!(!site
        .published("agenda/calendar.ics")
        .contains("BEGIN:VEVENT"));
}

#[test]
fn a_depth_of_zero_is_warned_about_and_search_depth_works_without_from() {
    let site = Site::new(r#","search":{"depth":1}"#);
    site.write(
        "page.json",
        &page("Home", r#","agenda":{"from":"/a","depth":0}"#),
    );
    site.write("a/page.json", &page("A", ""));
    site.write("a/b/page.json", &page("B", ""));
    assert!(kinds(&site.check()).contains(&FindingKind::SelectionDepthOfZero));
    site.generate();
    let index = site.published("search-index.json");
    assert!(
        index.contains(r#""url":"/a""#) && !index.contains(r#""url":"/a/b""#),
        "{index}"
    );
}

#[test]
fn an_agenda_without_a_site_url_is_warned_about() {
    let site = Site::new("");
    site.write(
        "main.json",
        r#"{"siteTitle":"T","jsFiles":[],"cssFiles":[],"footerContent":"<p>f</p>"}"#,
    );
    site.write("page.json", &page("Home", r#","agenda":true"#));
    assert!(kinds(&site.check()).contains(&FindingKind::EventWithoutSiteUrl));
}

#[test]
fn a_content_link_to_a_static_file_without_an_extension_is_fine() {
    let site = Site::new(r#","staticDir":"static""#);
    site.write("static/LICENSE", "text");
    site.write(
        "page.json",
        r#"{"title":"Home","content":"<p><a href=\"/LICENSE\">l</a></p>"}"#,
    );
    assert!(!kinds(&site.check()).contains(&FindingKind::BrokenContentLink));
}

#[test]
fn a_social_link_with_an_unknown_icon_shows_its_label() {
    let site = Site::new("");
    site.write("menu.json", r#"{"mainMenu":[],"socials":[{"url":"https://example.org/me","icon":"no-such-icon","color":"000000","label":"Elsewhere"}]}"#);
    site.generate();
    assert!(site
        .published("index.html")
        .contains("<span>Elsewhere</span>"));
}

#[test]
fn an_agendas_own_dated_luma_events_join_whatever_from_says() {
    let site = Site::new("");
    site.write("agenda/page.json", &page("Agenda", r#","agenda":{"from":"/events"},"event":{"start":"2026-08-01T10:00:00+02:00","timeZone":"Europe/Amsterdam"},"lumaEvents":[{"eventId":"evt-own","title":"Eigen","start":"2026-09-01T10:00:00+02:00","timeZone":"Europe/Amsterdam"},{"eventId":"evt-out","title":"Zonder datum"}]"#));
    site.write(
        "events/jam/page.json",
        &page(
            "Jam",
            r#","event":{"start":"2026-10-01T10:00:00+02:00","timeZone":"Europe/Amsterdam"}"#,
        ),
    );
    site.write("elsewhere/page.json", &page("Elders", r#","event":{"start":"2026-10-02T10:00:00+02:00","timeZone":"Europe/Amsterdam"},"lumaEvents":[{"eventId":"evt-far","title":"Ver weg","start":"2026-10-03T10:00:00+02:00","timeZone":"Europe/Amsterdam"}]"#));
    site.generate();
    let html = site.published("agenda.html");
    assert!(html.contains("Eigen</a></h2>"), "{html}");
    assert!(html.contains("Jam</a></h2>"), "{html}");
    assert!(!html.contains("Zonder datum</a></h2><p>"), "{html}");
    assert!(!html.contains("Elders</a></h2>"), "{html}");
    assert!(!html.contains("Ver weg</a></h2>"), "{html}");
    // Its own `event` is the page itself, so it isn't one of the listed ones.
    let ics = site.published("agenda/calendar.ics");
    assert_eq!(ics.matches("BEGIN:VEVENT").count(), 2, "{ics}");
}

#[test]
fn markdown_is_opt_in_and_preserves_authored_content_with_resolved_urls() {
    let disabled = Site::new("");
    disabled.generate();
    assert!(!disabled.exists("index.md"));
    assert!(!disabled.published("index.html").contains("text/markdown"));

    let site = Site::new(r#", "markdown":true"#);
    site.write("blog/post/page.json", r##"{"title":"Post","subTitle":"Notes","date":"2026-11-01","updated":"2026-11-02","content":"post.md"}"##);
    site.write("blog/post/post.md", "## Authored heading\n\nA **bold** word.\n\n[Other](../other?x=1#part)\n\n![Alt](photo.png)\n\n[Ref][r]\n\n[r]: /about \"About\"\n\n<a href=\"relative\">HTML link</a>\n\n```rust\nlet url = \"[unchanged](relative)\";\n```\n");
    site.generate();
    let markdown = site.published("blog/post.md");
    for expected in [
        "Canonical: https://example.com/blog/post",
        "# Post",
        "Notes",
        "2026-11-01",
        "Updated: 2026-11-02",
        "## Authored heading",
        "A **bold** word.",
        "[Other](/other?x=1#part)",
        "![Alt](/blog/photo.png)",
        "[Ref][r]",
        "[r]: /about \"About\"",
        "<a href=\"/blog/relative\">HTML link</a>",
        "```rust\nlet url = \"[unchanged](relative)\";\n```",
    ] {
        assert!(
            markdown.contains(expected),
            "missing {expected:?}: {markdown}"
        );
    }
    assert!(!markdown.contains("PLACEHOLDER"));
    assert!(site.published("blog/post.html").contains("text/markdown"));
    assert!(site.published("blog/post.html").contains("/blog/post.md"));
    assert!(site.exists("index.md"));
}

#[test]
fn markdown_converts_html_and_keeps_generated_sections_without_controls() {
    let site = Site::new(r#", "markdown":true,"search":true"#);
    site.write("page.json", r#"{"title":"Home","content":"<h2>HTML section</h2><p>Read <a href='next'>next</a>.</p><ul><li>One</li><li>Two</li></ul><pre><code class='language-rust'>let x = 1;</code></pre><script>secret_script()</script>","agenda":true,"lumaEvents":[{"eventId":"evt-AbC123","title":"External jam"}]}"#);
    site.write("next/page.json", &page("Next", ""));
    let mut home: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(site.root.join("root/page.json")).unwrap())
            .unwrap();
    home["leftNotifications"] = serde_json::json!([{
        "title": "Notice", "content": "<p>A useful notice.</p>"
    }]);
    home["latest"] = serde_json::json!({"from":"/news","limit":1});
    site.write("page.json", &home.to_string());
    site.write(
        "news/post/page.json",
        &page(
            "Latest post",
            r#", "date":"2026-11-01","summary":"Newest news""#,
        ),
    );
    site.write("jam/page.json", &page("Jam", r#", "event":{"start":"2026-11-15T14:00:00+01:00","timeZone":"Europe/Amsterdam","venue":"Club"}"#));
    site.generate();
    let markdown = site.published("index.md");
    for expected in [
        "## HTML section",
        "[next](</next>)",
        "- One",
        "- Two",
        "```rust\nlet x = 1;\n```",
        "[Next](</next>)",
        "## Notice",
        "A useful notice.",
        "[Latest post](</news/post>)",
        "Newest news",
        "[Jam](</jam>)",
        "External jam",
        "https://luma.com/event/evt-AbC123",
    ] {
        assert!(
            markdown.contains(expected),
            "missing {expected:?}: {markdown}"
        );
    }
    for excluded in [
        "<iframe",
        "secret_script",
        "<script",
        "<svg",
        "<form",
        "PLACEHOLDER",
        "hoofdinhoud",
        "Geen komende",
    ] {
        assert!(
            !markdown.contains(excluded),
            "unexpected {excluded:?}: {markdown}"
        );
    }
    assert!(site.published("jam.md").contains("Club"));
    assert!(site.exists("search.md"));
    site.assert_links_are_fine();
}

#[test]
fn markdown_static_collisions_are_reported_and_generated_content_wins() {
    let site = Site::new(r#", "markdown":true,"staticDir":"static""#);
    site.write("static/index.md", "old static content");
    assert!(kinds(&site.check()).contains(&FindingKind::StaticFileShadowedByPage));
    site.generate();
    assert!(!site.published("index.md").contains("old static content"));
}

#[test]
fn markdown_without_site_url_converts_html_files_and_keeps_unlisted_pages() {
    let site = Site::new(r#", "markdown":true,"staticDir":"static","search":true"#);
    let mut config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(site.root.join("root/main.json")).unwrap())
            .unwrap();
    config.as_object_mut().unwrap().remove("siteUrl");
    site.write("main.json", &config.to_string());
    site.write(
        "private/page.json",
        r#"{"title":"Private","content":"body.html","listed":false,"noindex":true}"#,
    );
    site.write("private/body.html", "<p>A <strong>strong</strong> word and <a href='#part'>fragment</a>.</p><h2 id='part'>Part</h2><iframe src='https://example.com/video' title='Video'></iframe>");
    site.write("static/search.md", "shadowed");
    assert!(kinds(&site.check()).contains(&FindingKind::StaticFileShadowedByPage));
    site.generate();
    let markdown = site.published("private.md");
    assert!(markdown.contains("Canonical: /private"), "{markdown}");
    assert!(markdown.contains("**strong**"), "{markdown}");
    assert!(
        markdown.contains("[fragment](</private#part>)"),
        "{markdown}"
    );
    assert!(
        markdown.contains("[Video](<https://example.com/video>)"),
        "{markdown}"
    );
    assert!(!site.published("index.md").contains("Private"));
    assert!(site.published("private.html").contains("/private.md"));
}

#[test]
fn removed_and_renamed_pages_are_pruned_with_empty_directories() {
    let site = Site::new("");
    site.write("gone/child/page.json", &page("Gone", ""));
    site.write("old/page.json", &page("Renamed", ""));
    site.generate();
    fs::remove_dir_all(site.root.join("root/gone")).unwrap();
    fs::rename(site.root.join("root/old"), site.root.join("root/new")).unwrap();
    site.generate();
    for path in ["gone.html", "gone", "old.html"] {
        assert!(!site.exists(path), "stale output: {path}");
    }
    assert!(site.published("new.html").contains("Renamed"));
    site.assert_links_are_fine();
}

#[test]
fn disabling_features_removes_their_files_and_preserves_other_build_output() {
    let site =
        Site::new(r#", "search":true,"markdown":true,"staticDir":"static","favicon":"icon.svg""#);
    site.write(
        "icon.svg",
        "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>",
    );
    site.write("static/nested/file.txt", "static content");
    site.write("page.json", &page("Home", r#", "feed":true,"agenda":true"#));
    site.write(
        "post/page.json",
        &page("Post", &format!(r#"{EVENT},"date":"2026-10-01""#)),
    );
    site.generate();
    let generated = [
        "feed.xml",
        "calendar.ics",
        "post.ics",
        "search.html",
        "search-index.json",
        "js/vados-search.js",
        "js/vados-agenda.js",
        "index.md",
        "post.md",
        "nested/file.txt",
    ];
    // All the enabled features really published something on the first run.
    for path in &generated {
        assert!(site.exists(path), "missing first-run output: {path}");
    }
    let manifest: Vec<String> =
        serde_json::from_str(&site.published(".vados-manifest.json")).unwrap();
    let favicon = manifest
        .iter()
        .find(|p| p.ends_with(".svg"))
        .unwrap()
        .clone();
    for path in ["css/custom.css", "nested/other.txt", "img/manual-w159.webp"] {
        let file = site.root.join("public").join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, "another build step").unwrap();
    }
    site.write(
        "main.json",
        r#"{"siteTitle":"T","jsFiles":[],"cssFiles":[],"footerContent":"<p>f</p>"}"#,
    );
    site.write("page.json", &page("Home", ""));
    site.write("post/page.json", &page("Post", ""));
    site.generate();
    assert!(!site.exists(&favicon));
    for path in generated {
        assert!(!site.exists(path), "stale feature output: {path}");
    }
    assert!(!site.exists("sitemap.xml"));
    for path in ["css/custom.css", "nested/other.txt", "img/manual-w159.webp"] {
        assert_eq!(site.published(path), "another build step");
    }
    // The next identical build neither forgets files nor tries to delete them again.
    let manifest = site.published(".vados-manifest.json");
    site.generate();
    assert_eq!(manifest, site.published(".vados-manifest.json"));
}

#[test]
fn without_a_previous_manifest_existing_files_are_preserved() {
    let site = Site::new("");
    for path in ["old.html", "img/old-w159.webp", "css/custom.css"] {
        let file = site.root.join("public").join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, "untracked").unwrap();
    }
    site.generate();
    for path in ["old.html", "img/old-w159.webp", "css/custom.css"] {
        assert_eq!(site.published(path), "untracked");
    }
    assert!(site.exists(".vados-manifest.json"));
}

#[test]
fn a_path_can_change_between_a_page_and_a_static_file() {
    let site = Site::new(r#", "staticDir":"static""#);
    site.write("same/page.json", &page("Generated", ""));
    site.generate();
    fs::remove_dir_all(site.root.join("root/same")).unwrap();
    site.write("static/same.html", "static replacement");
    site.generate();
    assert_eq!(site.published("same.html"), "static replacement");
    fs::remove_file(site.root.join("root/static/same.html")).unwrap();
    site.write("same/page.json", &page("Generated again", ""));
    site.generate();
    assert!(site.published("same.html").contains("Generated again"));
}

#[test]
fn invalid_manifest_paths_do_not_delete_files_outside_the_destination() {
    let site = Site::new("");
    fs::create_dir_all(site.root.join("public")).unwrap();
    fs::write(site.root.join("keep.txt"), "keep").unwrap();
    fs::write(
        site.root.join("public/.vados-manifest.json"),
        r#"["../keep.txt"]"#,
    )
    .unwrap();
    assert!(site.try_generate().is_err());
    assert_eq!(
        fs::read_to_string(site.root.join("keep.txt")).unwrap(),
        "keep"
    );
}

#[cfg(unix)]
#[test]
fn a_manifest_path_cannot_delete_through_a_symlink() {
    let site = Site::new("");
    fs::create_dir_all(site.root.join("public")).unwrap();
    fs::create_dir_all(site.root.join("other")).unwrap();
    fs::write(site.root.join("other/keep.txt"), "keep").unwrap();
    std::os::unix::fs::symlink(site.root.join("other"), site.root.join("public/link")).unwrap();
    fs::write(
        site.root.join("public/.vados-manifest.json"),
        r#"["link/keep.txt"]"#,
    )
    .unwrap();
    assert!(site.try_generate().is_err());
    assert_eq!(
        fs::read_to_string(site.root.join("other/keep.txt")).unwrap(),
        "keep"
    );
}

#[test]
fn a_failed_generation_keeps_the_previous_manifest_and_stale_files() {
    let site = Site::new("");
    site.write("gone/page.json", &page("Gone", ""));
    site.generate();
    let manifest = site.published(".vados-manifest.json");
    fs::remove_dir_all(site.root.join("root/gone")).unwrap();
    // A failure before publication completes must not advance cleanup state.
    site.write("page.json", "invalid json");
    assert!(site.try_generate().is_err());
    assert_eq!(manifest, site.published(".vados-manifest.json"));
    assert!(site.exists("gone.html"));
    site.write("page.json", &page("Home", ""));
    site.generate();
    assert!(!site.exists("gone.html"));
}
