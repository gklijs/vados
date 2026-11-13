//! The `check` command: a pre-flight validation pass over the source and
//! image trees. It walks exactly the trees `generate` would and declares the
//! same things from them, per `vados.allium`'s `SourceTree` surface, but it
//! never writes anything and never decodes an image. It also reads the
//! package manifest next to the source tree, to tell whether the site's
//! compiled stylesheet still uses Bulma before 1.0. Its whole point is to
//! find every problem in one pass instead of stopping at the first one, so
//! every check below runs regardless of what earlier checks found.

use crate::config_files::{is_site_url, MainConfig, MenuConfig};
use crate::content::has_top_level_heading;
use crate::files::{get_static_files, static_dir_path};
use crate::icons::{is_known_icon, suggest_icons};
use crate::search::SEARCH_PATH;
use crate::ui_text::is_translated;
use std::collections::HashSet;
use std::fmt;
use std::fs::File;
use std::path::Path;

mod bulma;
mod config;
mod content_links;
mod events;
mod feeds_and_latest;
mod hosting;
mod images;
mod menu_and_social;
mod pages;
#[cfg(test)]
mod test_support;

use bulma::{bulma_sass_import, is_outdated_bulma_link, outdated_bulma_dependency};
use config::read_json;
use content_links::check_content_links;
use events::check_luma_events;
use feeds_and_latest::check_page_selections;
use hosting::check_static_dir;
use images::{collect_images, duplicate_image_findings};
use menu_and_social::check_menu;
use pages::{
    check_content_reference, check_mermaid_reference, check_noindex, check_notification,
    check_page_date, collect_pages,
};

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
    GenerationFailed,
    SourceUnreadable,
    InvalidMermaidScript,
    EmptyMermaidDiagram,
    MermaidWithoutAlternativeText,
    MermaidDisabled,
    SiteConfigUnreadable,
    MenuUnreadable,
    PageConfigUnreadable,
    ImagesManifestUnreadable,
    UnrecognisedContentReference,
    MissingContentFile,
    TopLevelHeadingInContent,
    ExternalMenuLinkWithoutTitle,
    UnresolvedMenuLink,
    UnrecognisedSocialLinkIncomplete,
    DuplicateImageReference,
    ImageWithoutAlternativeText,
    UnknownPageImageReference,
    UnknownNotificationImageReference,
    MissingImageSourceFile,
    DeadInternalNotificationLink,
    MalformedLumaEventId,
    LumaEventWithoutTitle,
    DuplicateLumaEvent,
    MissingFaviconFile,
    UntranslatedInterfaceLanguage,
    UnreadablePageDate,
    UndatedPageInDateOrderedSection,
    MalformedSiteUrl,
    FeedWithoutSiteUrl,
    UndatedPageLeftOutOfFeed,
    PreviewImageWithoutSiteUrl,
    UnknownDefaultImageReference,
    DatedPageWithoutSummary,
    MissingStaticDir,
    StaticFileShadowedByPage,
    UnresolvedLatestSource,
    UnknownIcon,
    UnknownConfigKey,
    OutdatedBulmaStylesheet,
    OutdatedBulmaDependency,
    BulmaSassImport,
    EmptyFeed,
    UnresolvedSelectionSource,
    SpareTileLinkMatchesNoPage,
    NoindexedPageIsListed,
    InvalidEvent,
    EventWithoutSiteUrl,
    BrokenInternalLink,
    MissingLinkFragment,
    MissingAsset,
    BrokenExternalLink,
    BrokenContentLink,
    HeadersFileProblem,
    OutputUnreadable,
    SelectionDepthOfZero,
    CalendarFileCollision,
}

impl FindingKind {
    pub fn severity(self) -> Severity {
        use FindingKind::*;
        match self {
            GenerationFailed
            | SourceUnreadable
            | ImagesManifestUnreadable
            | MissingImageSourceFile
            | InvalidMermaidScript
            | SiteConfigUnreadable
            | MenuUnreadable
            | PageConfigUnreadable
            | UnrecognisedContentReference
            | ExternalMenuLinkWithoutTitle
            | UnrecognisedSocialLinkIncomplete
            | DuplicateImageReference
            | ImageWithoutAlternativeText
            | MalformedLumaEventId
            | LumaEventWithoutTitle
            | UnreadablePageDate
            | MalformedSiteUrl
            | FeedWithoutSiteUrl
            | OutdatedBulmaStylesheet
            | OutdatedBulmaDependency
            | InvalidEvent
            | BrokenInternalLink
            | MissingLinkFragment
            | MissingAsset
            | OutputUnreadable => Severity::Error,
            EmptyMermaidDiagram
            | MermaidWithoutAlternativeText
            | MermaidDisabled
            | MissingContentFile
            | TopLevelHeadingInContent
            | UnresolvedMenuLink
            | UnknownPageImageReference
            | UnknownNotificationImageReference
            | DeadInternalNotificationLink
            | DuplicateLumaEvent
            | MissingFaviconFile
            | UntranslatedInterfaceLanguage
            | UndatedPageInDateOrderedSection
            | UndatedPageLeftOutOfFeed
            | PreviewImageWithoutSiteUrl
            | UnknownDefaultImageReference
            | DatedPageWithoutSummary
            | MissingStaticDir
            | StaticFileShadowedByPage
            | UnresolvedLatestSource
            | UnknownIcon
            | UnknownConfigKey
            | BulmaSassImport
            | EmptyFeed
            | UnresolvedSelectionSource
            | SpareTileLinkMatchesNoPage
            | NoindexedPageIsListed
            | EventWithoutSiteUrl
            | BrokenExternalLink
            | BrokenContentLink
            | HeadersFileProblem
            | SelectionDepthOfZero
            | CalendarFileCollision => Severity::Warning,
        }
    }
}

impl fmt::Display for FindingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use FindingKind::*;
        f.write_str(match self {
            UnknownConfigKey => "unknown configuration key",
            GenerationFailed => "generation failed",
            SourceUnreadable => "source directory could not be read",
            InvalidMermaidScript => "mermaidScript must name an HTTP(S) module or an existing root-relative file in staticDir",
            EmptyMermaidDiagram => "Mermaid diagram is empty",
            MermaidWithoutAlternativeText => "Mermaid diagram has no accTitle or accDescr text alternative",
            MermaidDisabled => "Mermaid rendering is disabled; this diagram will show as source code",
            SiteConfigUnreadable => "main.json could not be read",
            MenuUnreadable => "menu.json could not be read",
            PageConfigUnreadable => "page.json could not be read",
            ImagesManifestUnreadable => "images.json could not be read",
            UnrecognisedContentReference => {
                "content reference is not a Markdown file, an HTML file, or inline HTML"
            }
            MissingContentFile => "content file does not exist",
            TopLevelHeadingInContent => {
                "content has a top-level heading, a second one after the page title"
            }
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
            MalformedLumaEventId => "Luma event ID is not of the form evt-...",
            LumaEventWithoutTitle => "Luma event has no title",
            DuplicateLumaEvent => "the same Luma event is linked more than once on this page",
            MissingFaviconFile => "favicon file does not exist",
            UntranslatedInterfaceLanguage => {
                "vados has no interface text in the site language, so it falls back to English"
            }
            UnreadablePageDate => "page date is not of the form YYYY-MM-DD or YYYY-MM-DDTHH:MM",
            UndatedPageInDateOrderedSection => {
                "page has no date, but its parent orders its children by date, so it is listed last"
            }
            MalformedSiteUrl => "site URL is not an absolute http(s) URL",
            FeedWithoutSiteUrl => {
                "page publishes a feed, but main.json has no usable siteUrl, so no feed is written"
            }
            UndatedPageLeftOutOfFeed => {
                "page has no date, so it is left out of its parent's feed"
            }
            PreviewImageWithoutSiteUrl => {
                "pages have images, but main.json has no usable siteUrl, so link previews show none"
            }
            UnknownDefaultImageReference => "default image reference matches no declared image",
            MissingStaticDir => "static directory does not exist",
            UnknownIcon => "icon is not a Material Design Icons name, so it is left out",
            OutdatedBulmaStylesheet => {
                "stylesheet loads Bulma before 1.0, which lacks the classes vados now lays pages out with"
            }
            BulmaSassImport => {
                "sass/main.scss loads Bulma with the deprecated @import; Bulma 1.0 is loaded with @use \"bulma/sass\" with (...)"
            }
            OutdatedBulmaDependency => {
                "package.json asks for Bulma before 1.0, so the compiled stylesheet lacks the classes vados now lays pages out with"
            }
            UnresolvedLatestSource => "page shows the latest pages from a path that matches no page",
            StaticFileShadowedByPage => {
                "static file has the same path as a generated page, which replaces it"
            }
            EmptyFeed => "page publishes a feed, but no dated page matches it, so it has no entries",
            UnresolvedSelectionSource => {
                "a feed, agenda or search draws from a path that matches no page"
            }
            SpareTileLinkMatchesNoPage => "spare tile link matches no page",
            NoindexedPageIsListed => {
                "page is kept out of search engines but is still in a menu or side tiles; set \"listed\": false if that isn't meant"
            }
            InvalidEvent => "event is not valid, so it is left off the page",
            EventWithoutSiteUrl => {
                "site has events or agendas, but main.json has no usable siteUrl, so no calendar files are written"
            }
            BrokenInternalLink => "link points at a page that was not generated",
            MissingLinkFragment => "link points at an anchor that isn't on the target page",
            MissingAsset => "page or stylesheet refers to a file that was not generated",
            BrokenExternalLink => "link to another site looks broken",
            BrokenContentLink => "content links to a page that doesn't exist",
            HeadersFileProblem => "_headers file has something a host may reject",
            OutputUnreadable => "generated site could not be read",
            SelectionDepthOfZero => "a feed, agenda or search with depth 0 selects no pages below its sources",
            CalendarFileCollision => {
                "event page's calendar file has the same address as an agenda's, which replaces it"
            }
            DatedPageWithoutSummary => {
                "dated page has no summary, so search engines and feeds have no description of it"
            }
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

    let static_dir = main_config.as_ref().and_then(|c| c.static_dir.as_deref());
    let pages = collect_pages(source, static_dir, &mut findings);
    let mut page_paths: HashSet<&str> = pages.keys().map(|p| p.as_str()).collect();
    // `generate` publishes a search page when the site has none of its own.
    let search = main_config.as_ref().and_then(MainConfig::get_search);
    if search.is_some() {
        page_paths.insert(SEARCH_PATH);
    }

    // What a content link can point at: a page, or a static file, some of which
    // have no extension, like `_redirects` or `LICENSE`.
    let static_paths: Vec<String> = static_dir
        .map(|d| get_static_files(source, d))
        .unwrap_or_default()
        .into_iter()
        .map(|(published, _)| published)
        .collect();
    let link_targets: HashSet<&str> = page_paths
        .iter()
        .copied()
        .chain(static_paths.iter().map(String::as_str))
        .collect();

    if let Some(menu_config) = &menu_config {
        check_menu(menu_config, &page_paths, &mut findings);
    }

    for (path, page_config) in &pages {
        check_content_reference(source, path, &page_config.content, &mut findings);
        let references = std::iter::once(&page_config.content).chain(
            page_config
                .left_notifications
                .iter()
                .flatten()
                .chain(page_config.right_notifications.iter().flatten())
                .map(|n| &n.content),
        );
        for reference in references {
            check_mermaid_reference(source, path, reference, main_config.as_ref(), &mut findings);
            check_content_links(source, path, path, reference, &link_targets, &mut findings);
        }
        if let Some(icon) = page_config.icon.as_ref().filter(|i| !is_known_icon(i)) {
            findings.push(Finding {
                kind: FindingKind::UnknownIcon,
                location: path.clone(),
                detail: Some(unknown_icon_detail(icon)),
            });
        }
        if has_top_level_heading(source, path, &page_config.content) {
            findings.push(Finding {
                kind: FindingKind::TopLevelHeadingInContent,
                location: path.clone(),
                detail: Some(page_config.content.clone()),
            });
        }
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
        check_luma_events(
            path,
            page_config.luma_events.as_deref().unwrap_or_default(),
            &mut findings,
        );
        check_page_date(path, page_config, &pages, &mut findings);
        check_page_selections(path, page_config, &pages, &page_paths, &mut findings);
        check_noindex(path, page_config, menu_config.as_ref(), &mut findings);
        if let Some(Err(problems)) = page_config.event.as_ref().map(|e| e.resolve()) {
            for problem in problems {
                findings.push(Finding {
                    kind: FindingKind::InvalidEvent,
                    location: path.clone(),
                    detail: Some(problem),
                });
            }
        }
        for source in page_config.latest.iter().flat_map(|l| l.sources()) {
            if !page_paths.contains(source) {
                findings.push(Finding {
                    kind: FindingKind::UnresolvedLatestSource,
                    location: path.clone(),
                    detail: Some(source.to_string()),
                });
            }
        }
    }

    if let Some(main_config) = &main_config {
        check_mermaid_reference(
            source,
            "/",
            &main_config.footer_content,
            Some(main_config),
            &mut findings,
        );
        if let Some(script) = &main_config.mermaid_script {
            let valid = match crate::mermaid::script_location(script) {
                Some(None) => true,
                Some(Some(path)) => main_config.static_dir.as_deref().is_some_and(|dir| {
                    Path::new(&static_dir_path(source, dir))
                        .join(path)
                        .is_file()
                }),
                None => false,
            };
            if !valid {
                findings.push(Finding {
                    kind: FindingKind::InvalidMermaidScript,
                    location: format!("{source}/main.json"),
                    detail: Some(script.clone()),
                });
            }
        }
        let before_footer = findings.len();
        check_content_reference(source, "/", &main_config.footer_content, &mut findings);
        for finding in &mut findings[before_footer..] {
            if finding.kind == FindingKind::UnrecognisedContentReference {
                finding.location = format!("{source}/main.json");
            }
        }
        check_content_links(
            source,
            "/",
            &format!("{}/main.json", source),
            &main_config.footer_content,
            &link_targets,
            &mut findings,
        );
        if let Some(favicon) = &main_config.favicon {
            if !Path::new(&format!("{}/{}", source, favicon)).is_file() {
                findings.push(Finding {
                    kind: FindingKind::MissingFaviconFile,
                    location: format!("{}/main.json", source),
                    detail: Some(favicon.clone()),
                });
            }
        }
        for css in main_config
            .css_files
            .iter()
            .filter(|f| is_outdated_bulma_link(f))
        {
            findings.push(Finding {
                kind: FindingKind::OutdatedBulmaStylesheet,
                location: format!("{}/main.json", source),
                detail: Some(css.clone()),
            });
        }
        if let Some(url) = &main_config.site_url {
            if !is_site_url(url) {
                findings.push(Finding {
                    kind: FindingKind::MalformedSiteUrl,
                    location: format!("{}/main.json", source),
                    detail: Some(url.clone()),
                });
            }
        }
        if main_config.get_site_url().is_none() {
            let mut feed_paths: Vec<&String> = pages
                .iter()
                .filter(|(_, c)| c.get_feed().is_some())
                .map(|(p, _)| p)
                .collect();
            feed_paths.sort();
            for path in feed_paths {
                findings.push(Finding {
                    kind: FindingKind::FeedWithoutSiteUrl,
                    location: path.clone(),
                    detail: None,
                });
            }
        }
        check_static_dir(source, main_config, &page_paths, &mut findings);
        if let Some(image_ref) = &main_config.default_image {
            if !image_keys.contains(image_ref.as_str()) {
                findings.push(Finding {
                    kind: FindingKind::UnknownDefaultImageReference,
                    location: format!("{}/main.json", source),
                    detail: Some(image_ref.clone()),
                });
            }
        }
        // Reported once for the site rather than once per page: the fix is
        // the one siteUrl. A malformed one is already an error of its own.
        let has_preview_image =
            main_config.default_image.is_some() || pages.values().any(|p| p.image.is_some());
        if main_config.site_url.is_none() && has_preview_image {
            findings.push(Finding {
                kind: FindingKind::PreviewImageWithoutSiteUrl,
                location: format!("{}/main.json", source),
                detail: None,
            });
        }
        let has_calendars = pages
            .values()
            .any(|p| p.event.is_some() || p.get_agenda().is_some());
        if main_config.get_site_url().is_none() && has_calendars {
            findings.push(Finding {
                kind: FindingKind::EventWithoutSiteUrl,
                location: format!("{}/main.json", source),
                detail: None,
            });
        }
        for link in main_config.spare_tile.iter().flat_map(|s| &s.links) {
            // A file at a fixed address (`/feed.xml`) isn't a page.
            let names_a_file = link.url.rsplit('/').next().is_some_and(|l| l.contains('.'));
            if link.url.starts_with('/') && !names_a_file && !page_paths.contains(link.url.as_str())
            {
                findings.push(Finding {
                    kind: FindingKind::SpareTileLinkMatchesNoPage,
                    location: format!("{}/main.json", source),
                    detail: Some(link.url.clone()),
                });
            }
        }
        if search.as_ref().is_some_and(|s| s.depth == Some(0)) {
            findings.push(Finding {
                kind: FindingKind::SelectionDepthOfZero,
                location: format!("{}/main.json", source),
                detail: None,
            });
        }
        for from in search
            .iter()
            .flat_map(|s| s.from.iter())
            .flat_map(|f| f.paths())
        {
            if !page_paths.contains(from) {
                findings.push(Finding {
                    kind: FindingKind::UnresolvedSelectionSource,
                    location: format!("{}/main.json", source),
                    detail: Some(from.to_string()),
                });
            }
        }
        if !is_translated(main_config.get_language()) {
            findings.push(Finding {
                kind: FindingKind::UntranslatedInterfaceLanguage,
                location: format!("{}/main.json", source),
                detail: Some(main_config.get_language().to_string()),
            });
        }
    }

    // The manifest sits next to the source tree, not in it. A site that
    // compiles Bulma through `npm run build` has no `cssFiles` link naming
    // Bulma, so this is the only place its version shows up in check.
    if let Some((location, spec)) = outdated_bulma_dependency(source) {
        findings.push(Finding {
            kind: FindingKind::OutdatedBulmaDependency,
            location,
            detail: Some(spec),
        });
    }

    if let Some((location, line)) = bulma_sass_import(source) {
        findings.push(Finding {
            kind: FindingKind::BulmaSassImport,
            location,
            detail: Some(line),
        });
    }

    CheckReport { findings }
}

/// The usable `siteUrl` in the source's main.json, if it can be read.
pub fn usable_site_url(source: &str) -> Option<String> {
    let file = File::open(format!("{}/main.json", source)).ok()?;
    let config: MainConfig = serde_json::from_reader(file).ok()?;
    config.get_site_url().map(String::from)
}

/// The name `check` couldn't find, and what to use instead when something
/// close exists.
fn unknown_icon_detail(icon: &str) -> String {
    match suggest_icons(icon).as_slice() {
        [] => {
            format!("{icon}; pick a name from https://pictogrammers.com/library/mdi/ or remove it")
        }
        suggestions => format!("{icon}; did you mean {}?", suggestions.join(", ")),
    }
}

fn checked_directories(
    result: Result<Vec<String>, crate::generator::GenerateError>,
    findings: &mut Vec<Finding>,
) -> Vec<String> {
    match result {
        Ok(dirs) => dirs,
        Err(error) => {
            findings.extend(error.findings.into_iter().map(|mut f| {
                f.kind = FindingKind::SourceUnreadable;
                f
            }));
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::test_support::*;

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
    fn top_level_heading_in_page_content_is_a_warning() {
        let site = TestSite::new("content_top_level_heading");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", r#"{"title":"Home","content":"home.md"}"#);
        site.write_source("home.md", "Intro\n\n# Home again\n\n## Fine\n");
        site.write_source(
            "about/page.json",
            r#"{"title":"About","content":"<h2>Fine</h2><p>```\n# not a heading\n```</p>"}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::TopLevelHeadingInContent,
            "/"
        ));
        assert!(!has_kind_at(
            &report,
            FindingKind::TopLevelHeadingInContent,
            "/about"
        ));
        assert!(report.passed());
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
    fn missing_image_source_file_is_an_error() {
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

        assert_eq!(count_kind(&report, FindingKind::MissingImageSourceFile), 1);
        assert!(!report.passed());
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

    fn site_with_main_json_extra(name: &str, extra: &str) -> TestSite {
        let site = TestSite::new(name);
        site.write_source(
            "main.json",
            &VALID_MAIN_JSON.replacen('{', &format!("{{ {},", extra), 1),
        );
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site
    }

    #[test]
    fn latest_pages_from_a_path_with_no_page_is_a_warning() {
        let site = date_ordered_blog(r#"{"title":"Post","content":"<p>hi</p>"}"#);
        site.write_source(
            "page.json",
            r#"{"title":"Home","content":"<p>home</p>","latest":{"from":["/blog","/nope"]}}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnresolvedLatestSource,
            "/"
        ));
        assert_eq!(count_kind(&report, FindingKind::UnresolvedLatestSource), 1);
        assert!(report.passed());
    }

    #[test]
    fn unknown_icons_are_a_warning() {
        let site = TestSite::new("icons_unknown");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[{"url":"/","icon":"hom"}],"socials":[
                {"url":"https://github.com/x","icon":"whatever"},
                {"url":"https://hachyderm.io/@x","icon":"mastodon","color":"6364FF","label":"Mastodon"},
                {"url":"https://example.com/x","icon":"nope","color":"000000","label":"X"}]}"#,
        );
        site.write_source(
            "page.json",
            r#"{"title":"Home","content":"<p>hi</p>","icon":"mdi-home"}"#,
        );
        site.write_source(
            "about/page.json",
            r#"{"title":"About","content":"<p>hi</p>","icon":"no-such-icon"}"#,
        );

        let report = site.check();

        assert!(has_kind_at(&report, FindingKind::UnknownIcon, "/"));
        assert!(has_kind_at(
            &report,
            FindingKind::UnknownIcon,
            "https://example.com/x"
        ));
        assert!(has_kind_at(&report, FindingKind::UnknownIcon, "/about"));
        // The home page's own icon is fine, so is a recognised provider's,
        // whatever menu.json says.
        assert_eq!(count_kind(&report, FindingKind::UnknownIcon), 3);
        assert!(report.passed());
    }

    #[test]
    fn unknown_default_image_reference_is_a_warning() {
        let site = site_with_main_json_extra(
            "default_image_unknown",
            r#""siteUrl": "https://example.com", "defaultImage": "/nope""#,
        );

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::UnknownDefaultImageReference),
            1
        );
        assert_eq!(
            count_kind(&report, FindingKind::PreviewImageWithoutSiteUrl),
            0
        );
        assert!(report.passed());
    }

    #[test]
    fn missing_favicon_file_is_a_warning() {
        let site = site_with_main_json_extra("favicon_missing", r#""favicon": "favicon.svg""#);

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::MissingFaviconFile), 1);
        assert!(report.passed());
    }

    #[test]
    fn present_favicon_file_is_not_flagged() {
        let site = site_with_main_json_extra("favicon_present", r#""favicon": "favicon.svg""#);
        site.write_source("favicon.svg", "<svg/>");

        assert!(site.check().findings.is_empty());
    }

    #[test]
    fn untranslated_site_language_is_a_warning() {
        let site = site_with_main_json_extra("language_untranslated", r#""language": "pl""#);

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::UntranslatedInterfaceLanguage),
            1
        );
        assert!(report.passed());
    }

    #[test]
    fn translated_site_language_is_not_flagged() {
        let site = site_with_main_json_extra("language_translated", r#""language": "nl-BE""#);

        assert!(site.check().findings.is_empty());
    }

    #[test]
    fn feed_without_a_site_url_is_an_error() {
        let site = feed_site("feed_no_url", VALID_MAIN_JSON, DATED_POST);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::FeedWithoutSiteUrl,
            "/blog"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn malformed_site_url_is_an_error_and_leaves_feeds_without_one() {
        let site = feed_site(
            "feed_bad_url",
            &main_json_with_site_url("example.com"),
            DATED_POST,
        );

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::MalformedSiteUrl), 1);
        assert!(has_kind_at(
            &report,
            FindingKind::FeedWithoutSiteUrl,
            "/blog"
        ));
        assert!(!report.passed());
    }
}
