//! The `check` command: a pre-flight validation pass over the source and
//! image trees. It walks exactly the trees `generate` would and declares the
//! same things from them, per `vados.allium`'s `SourceTree` surface, but it
//! never writes anything and never decodes an image. It also reads the
//! package manifest next to the source tree, to tell whether the site's
//! compiled stylesheet still uses Bulma before 1.0. Its whole point is to
//! find every problem in one pass instead of stopping at the first one, so
//! every check below runs regardless of what earlier checks found.

use crate::config_files::{
    is_site_url, ChildOrder, ImageList, MainConfig, MenuConfig, Notification, PageConfig,
};
use crate::content::{
    classify_content_reference, get_file_path, has_top_level_heading, ContentKind,
};
use crate::files::{
    get_all_directory_paths, get_page_directory_paths, get_static_files, static_dir_path,
};
use crate::icons::is_known_icon;
use crate::image::reference_key_for;
use crate::luma::{is_luma_event_id, LumaEvent};
use crate::page_date::PageDate;
use crate::structure::{is_recognized_social_provider, parent_path};
use crate::ui_text::is_translated;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};

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
    OutdatedBulmaStylesheet,
    OutdatedBulmaDependency,
    BulmaSassImport,
}

impl FindingKind {
    pub fn severity(self) -> Severity {
        use FindingKind::*;
        match self {
            SiteConfigUnreadable
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
            | OutdatedBulmaDependency => Severity::Error,
            ImagesManifestUnreadable
            | MissingContentFile
            | TopLevelHeadingInContent
            | UnresolvedMenuLink
            | UnknownPageImageReference
            | UnknownNotificationImageReference
            | MissingImageSourceFile
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
            | BulmaSassImport => Severity::Warning,
        }
    }
}

impl fmt::Display for FindingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use FindingKind::*;
        f.write_str(match self {
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

struct CheckedImage {
    reference_key: String,
    source_file: String,
    alt_text: String,
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
    let page_paths: HashSet<&str> = pages.keys().map(|p| p.as_str()).collect();

    if let Some(menu_config) = &menu_config {
        for item in &menu_config.main_menu {
            if item.url.starts_with("https://") {
                if item.title.is_none() {
                    findings.push(Finding {
                        kind: FindingKind::ExternalMenuLinkWithoutTitle,
                        location: item.url.clone(),
                        detail: None,
                    });
                }
            } else if !page_paths.contains(item.url.as_str()) {
                findings.push(Finding {
                    kind: FindingKind::UnresolvedMenuLink,
                    location: item.url.clone(),
                    detail: None,
                });
            }
        }
        let menu_icons = menu_config
            .main_menu
            .iter()
            .map(|i| (&i.url, &i.icon))
            .chain(menu_config.socials.iter().map(|s| (&s.url, &s.icon)));
        for (url, icon) in menu_icons {
            // A recognised provider's icon is its own, whatever menu.json says.
            let shown = !menu_config.socials.iter().any(|s| &s.url == url)
                || !is_recognized_social_provider(url);
            if let Some(icon) = icon.as_ref().filter(|i| shown && !is_known_icon(i)) {
                findings.push(Finding {
                    kind: FindingKind::UnknownIcon,
                    location: url.clone(),
                    detail: Some(icon.clone()),
                });
            }
        }
        for social in &menu_config.socials {
            if !is_recognized_social_provider(&social.url)
                && (social.icon.is_none() || social.color.is_none() || social.label.is_none())
            {
                findings.push(Finding {
                    kind: FindingKind::UnrecognisedSocialLinkIncomplete,
                    location: social.url.clone(),
                    detail: None,
                });
            }
        }
    }

    for (path, page_config) in &pages {
        check_content_reference(source, path, &page_config.content, &mut findings);
        if let Some(icon) = page_config.icon.as_ref().filter(|i| !is_known_icon(i)) {
            findings.push(Finding {
                kind: FindingKind::UnknownIcon,
                location: path.clone(),
                detail: Some(icon.clone()),
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
        check_content_reference(source, "/", &main_config.footer_content, &mut findings);
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
        if let Some(static_dir) = &main_config.static_dir {
            if !static_dir_path(source, static_dir).is_dir() {
                findings.push(Finding {
                    kind: FindingKind::MissingStaticDir,
                    location: format!("{}/main.json", source),
                    detail: Some(static_dir.clone()),
                });
            }
            let mut shadowed: Vec<(String, String)> = get_static_files(source, static_dir)
                .into_iter()
                .filter_map(|(published, from)| {
                    let page = match published.as_str() {
                        "/index.html" => "/",
                        p => p.strip_suffix(".html")?,
                    };
                    pages
                        .contains_key(page)
                        .then(|| (from.display().to_string(), String::from(page)))
                })
                .collect();
            shadowed.sort();
            for (file, page) in shadowed {
                findings.push(Finding {
                    kind: FindingKind::StaticFileShadowedByPage,
                    location: file,
                    detail: Some(page),
                });
            }
        }
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

fn read_json<T: serde::de::DeserializeOwned>(
    path: &str,
    unreadable_kind: FindingKind,
    findings: &mut Vec<Finding>,
) -> Option<T> {
    let result = File::open(path)
        .map_err(|e| e.to_string())
        .and_then(|f| serde_json::from_reader(f).map_err(|e| e.to_string()));
    match result {
        Ok(value) => Some(value),
        Err(detail) => {
            findings.push(Finding {
                kind: unreadable_kind,
                location: String::from(path),
                detail: Some(detail),
            });
            None
        }
    }
}

fn collect_images(img_source: &str, findings: &mut Vec<Finding>) -> Vec<CheckedImage> {
    let mut images = Vec::new();
    for dir in get_all_directory_paths(img_source) {
        let path = format!("{}/images.json", dir);
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => continue, // images.json is optional per directory
        };
        let list: ImageList = match serde_json::from_reader(file) {
            Ok(l) => l,
            Err(e) => {
                findings.push(Finding {
                    kind: FindingKind::ImagesManifestUnreadable,
                    location: path,
                    detail: Some(e.to_string()),
                });
                continue;
            }
        };
        let path_start = &dir[img_source.len()..];
        for reference in list.list {
            let source_file = format!("{}/{}", dir, reference.file_name);
            match reference_key_for(path_start, &reference.file_name) {
                Some(reference_key) => images.push(CheckedImage {
                    reference_key,
                    source_file,
                    alt_text: reference.alt_text,
                }),
                None => findings.push(Finding {
                    kind: FindingKind::ImagesManifestUnreadable,
                    location: source_file,
                    detail: Some(String::from("file name has no extension")),
                }),
            }
        }
    }
    images
}

fn duplicate_image_findings(images: &[CheckedImage]) -> Vec<Finding> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for image in images {
        *counts.entry(image.reference_key.as_str()).or_insert(0) += 1;
    }
    images
        .iter()
        .filter(|i| counts[i.reference_key.as_str()] > 1)
        .map(|i| Finding {
            kind: FindingKind::DuplicateImageReference,
            location: i.source_file.clone(),
            detail: Some(i.reference_key.clone()),
        })
        .collect()
}

fn collect_pages(
    source: &str,
    static_dir: Option<&str>,
    findings: &mut Vec<Finding>,
) -> HashMap<String, PageConfig> {
    let mut pages = HashMap::new();
    for dir in get_page_directory_paths(source, static_dir) {
        let page_path = if dir == source {
            String::from("/")
        } else {
            dir[source.len()..].to_string()
        };
        let page_json = format!("{}/page.json", dir);
        let config = match File::open(&page_json) {
            Ok(f) => match serde_json::from_reader(f) {
                Ok(config) => config,
                Err(e) => {
                    findings.push(Finding {
                        kind: FindingKind::PageConfigUnreadable,
                        location: page_json,
                        detail: Some(e.to_string()),
                    });
                    PageConfig::new(&dir)
                }
            },
            Err(_) => PageConfig::new(&dir), // page.json is optional per directory
        };
        pages.insert(page_path, config);
    }
    pages
}

fn check_content_reference(
    source: &str,
    location: &str,
    reference: &str,
    findings: &mut Vec<Finding>,
) {
    match classify_content_reference(reference) {
        ContentKind::Unrecognised => findings.push(Finding {
            kind: FindingKind::UnrecognisedContentReference,
            location: String::from(location),
            detail: Some(String::from(reference)),
        }),
        ContentKind::InlineHtml => {}
        ContentKind::MarkdownFile | ContentKind::HtmlFile => {
            let file_path = get_file_path(source, location, reference);
            if !Path::new(&file_path).exists() {
                findings.push(Finding {
                    kind: FindingKind::MissingContentFile,
                    location: String::from(location),
                    detail: Some(String::from(reference)),
                });
            }
        }
    }
}

fn check_notification(
    source: &str,
    page_path: &str,
    notification: &Notification,
    page_paths: &HashSet<&str>,
    image_keys: &HashSet<&str>,
    findings: &mut Vec<Finding>,
) {
    check_content_reference(source, page_path, &notification.content, findings);
    if let Some(image_ref) = &notification.image {
        if !image_keys.contains(image_ref.as_str()) {
            findings.push(Finding {
                kind: FindingKind::UnknownNotificationImageReference,
                location: String::from(page_path),
                detail: Some(image_ref.clone()),
            });
        }
    }
    if let Some(url) = &notification.url {
        if url.starts_with('/') && !page_paths.contains(url.as_str()) {
            findings.push(Finding {
                kind: FindingKind::DeadInternalNotificationLink,
                location: String::from(page_path),
                detail: Some(url.clone()),
            });
        }
    }
}

fn check_page_date(
    page_path: &str,
    page_config: &PageConfig,
    pages: &HashMap<String, PageConfig>,
    findings: &mut Vec<Finding>,
) {
    if let Some(updated) = &page_config.updated {
        if updated.parse::<PageDate>().is_err() {
            findings.push(Finding {
                kind: FindingKind::UnreadablePageDate,
                location: String::from(page_path),
                detail: Some(format!("updated: {updated}")),
            });
        }
    }
    match &page_config.date {
        Some(date) => {
            if date.parse::<PageDate>().is_err() {
                findings.push(Finding {
                    kind: FindingKind::UnreadablePageDate,
                    location: String::from(page_path),
                    detail: Some(date.clone()),
                });
            }
            let has_summary = page_config
                .summary
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty());
            if !has_summary {
                findings.push(Finding {
                    kind: FindingKind::DatedPageWithoutSummary,
                    location: String::from(page_path),
                    detail: None,
                });
            }
        }
        None => {
            let parent = parent_path(page_path).and_then(|p| pages.get(&p));
            let parent_orders_by_date = parent
                .and_then(|p| p.child_order)
                .is_some_and(|o| o != ChildOrder::Order);
            let parent_publishes_feed = parent.is_some_and(|p| p.get_feed().is_some());
            // One warning per undated page: being left out of a feed is the
            // more consequential of the two.
            let kind = if parent_publishes_feed {
                Some(FindingKind::UndatedPageLeftOutOfFeed)
            } else if parent_orders_by_date {
                Some(FindingKind::UndatedPageInDateOrderedSection)
            } else {
                None
            };
            if let Some(kind) = kind {
                findings.push(Finding {
                    kind,
                    location: String::from(page_path),
                    detail: None,
                });
            }
        }
    }
}

fn check_luma_events(page_path: &str, events: &[LumaEvent], findings: &mut Vec<Finding>) {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for event in events {
        *counts.entry(event.event_id.as_str()).or_insert(0) += 1;
    }
    for event in events {
        let mut report = |kind| {
            findings.push(Finding {
                kind,
                location: String::from(page_path),
                detail: Some(event.event_id.clone()),
            })
        };
        if !is_luma_event_id(&event.event_id) {
            report(FindingKind::MalformedLumaEventId);
        }
        if event.effective_title().is_none() {
            report(FindingKind::LumaEventWithoutTitle);
        }
        if counts[event.event_id.as_str()] > 1 {
            report(FindingKind::DuplicateLumaEvent);
        }
    }
}

/// Whether a stylesheet link names Bulma before 1.0, as a CDN link carries
/// it (`bulma@0.9.4`, `bulma@~0.9`, `bulma/0.9.4`). Only the version right
/// after the name counts. See vados.allium's `outdated_bulma_stylesheet`.
fn is_outdated_bulma_link(link: &str) -> bool {
    let link = link.to_ascii_lowercase();
    let version = ["bulma@", "bulma/"]
        .iter()
        .find_map(|prefix| link.split(prefix).nth(1));
    version.and_then(leading_version_major) == Some(0)
}

/// The `package.json` next to the source tree, when its Bulma dependency
/// asks for a version before 1.0, with the manifest's location and the
/// version spec. The manifest is in the source directory's parent, or the
/// working directory when that has none. See vados.allium's
/// `outdated_bulma_dependency`.
fn outdated_bulma_dependency(source: &str) -> Option<(String, String)> {
    let path = project_dir(source).join("package.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
    for section in ["dependencies", "devDependencies"] {
        let Some(spec) = manifest
            .get(section)
            .and_then(|dependencies| dependencies.get("bulma"))
            .and_then(|version| version.as_str())
        else {
            continue;
        };
        if spec_is_outdated(spec) {
            return Some((path.display().to_string(), spec.to_string()));
        }
    }
    None
}

/// The directory the site's build files (`package.json`, `sass/`) are in:
/// the source directory's parent, or the working directory when that has
/// none.
fn project_dir(source: &str) -> PathBuf {
    Path::new(source)
        .canonicalize()
        .ok()
        .and_then(|source| source.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}

/// The first line of the project's `sass/main.scss` that loads Bulma with
/// `@import`, with the file's location. See vados.allium's
/// `bulma_sass_import`.
fn bulma_sass_import(source: &str) -> Option<(String, String)> {
    let path = project_dir(source).join("sass").join("main.scss");
    let line = std::fs::read_to_string(&path)
        .ok()?
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("@import") && line.to_ascii_lowercase().contains("bulma"))?
        .to_string();
    Some((path.display().to_string(), line))
}

/// Whether a dependency spec only allows versions before 1.0: every
/// alternative (`a || b`) must start below 1.0, so a range that also allows
/// 1.x is not outdated. Specs without a number (`latest`, `*`) are unknown.
fn spec_is_outdated(spec: &str) -> bool {
    let majors: Vec<Option<u32>> = spec
        .split("||")
        .map(|alternative| {
            let start = alternative.find(|c: char| c.is_ascii_digit())?;
            leading_version_major(&alternative[start..])
        })
        .collect();
    !majors.is_empty() && majors.iter().all(|major| *major == Some(0))
}

/// The major of a version at the start of `text` (`0.9.4/css`, `~0.9`, `1`),
/// after any range operator; `None` when it doesn't start with a number.
fn leading_version_major(text: &str) -> Option<u32> {
    let text = text.trim_start_matches(['^', '~', '=', 'v', ' ']);
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    const VALID_MAIN_JSON: &str = r#"{
        "siteTitle": "Test site",
        "jsFiles": [],
        "includeDefaultJs": null,
        "cssFiles": [],
        "includeDefaultCss": null,
        "backgroundClass": null,
        "navbarColor": null,
        "footerContent": "<p>footer</p>"
    }"#;

    const VALID_MENU_JSON: &str = r#"{
        "mainMenu": [],
        "socials": []
    }"#;

    const VALID_PAGE_JSON: &str = r#"{
        "title": "Home",
        "subTitle": null,
        "image": null,
        "icon": null,
        "summary": null,
        "content": "<p>hi</p>",
        "order": null,
        "leftNotifications": null,
        "rightNotifications": null
    }"#;

    /// A disposable source/image tree for one test. Every test gets its own
    /// directories under the OS temp dir, named after the test plus a counter
    /// so parallel test runs never collide, and cleaned up on drop.
    struct TestSite {
        root: PathBuf,
        source: PathBuf,
        img_source: PathBuf,
    }

    impl TestSite {
        fn new(name: &str) -> Self {
            let id = NEXT_ID.fetch_add(1, AtomicOrdering::SeqCst);
            let root = std::env::temp_dir().join(format!("vados_check_test_{}_{}", name, id));
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

        fn write_source(&self, rel_path: &str, contents: &str) {
            let path = self.source.join(rel_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        fn write_image(&self, rel_path: &str, contents: &str) {
            let path = self.img_source.join(rel_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        fn write_project(&self, rel_path: &str, contents: &str) {
            let path = self.root.join(rel_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        fn check(&self) -> CheckReport {
            check(
                self.source.to_str().unwrap(),
                self.img_source.to_str().unwrap(),
            )
        }
    }

    impl Drop for TestSite {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn count_kind(report: &CheckReport, kind: FindingKind) -> usize {
        report.findings.iter().filter(|f| f.kind == kind).count()
    }

    fn has_kind_at(report: &CheckReport, kind: FindingKind, location: &str) -> bool {
        report
            .findings
            .iter()
            .any(|f| f.kind == kind && f.location == location)
    }

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
    fn missing_main_and_menu_json_are_errors() {
        let site = TestSite::new("missing_main_menu");
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::SiteConfigUnreadable), 1);
        assert_eq!(count_kind(&report, FindingKind::MenuUnreadable), 1);
        assert!(!report.passed());
        assert_eq!(report.error_count(), 2);
    }

    #[test]
    fn page_config_unreadable_is_an_error_but_check_continues() {
        let site = TestSite::new("page_unreadable");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", "{ not valid json");

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::PageConfigUnreadable), 1);
        assert!(!report.passed());
        // The unreadable page.json still yields a page (falling back to the
        // directory-derived default), so the footer's content reference is
        // still checked rather than the whole run being stranded.
        assert_eq!(
            count_kind(&report, FindingKind::UnrecognisedContentReference),
            0
        );
    }

    #[test]
    fn unresolved_internal_menu_link_is_a_warning() {
        let site = TestSite::new("unresolved_menu_link");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[{"url":"/missing","title":null,"icon":null}],"socials":[]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnresolvedMenuLink,
            "/missing"
        ));
        assert_eq!(
            FindingKind::UnresolvedMenuLink.severity(),
            Severity::Warning
        );
        assert!(report.passed());
    }

    #[test]
    fn external_menu_link_without_title_is_an_error() {
        let site = TestSite::new("external_link_no_title");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[{"url":"https://example.com","title":null,"icon":null}],"socials":[]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::ExternalMenuLinkWithoutTitle,
            "https://example.com"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn unrecognised_social_provider_missing_icon_and_color_is_an_error() {
        let site = TestSite::new("social_unrecognised");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://mastodon.social/@someone","icon":null,"color":null}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnrecognisedSocialLinkIncomplete,
            "https://mastodon.social/@someone"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn unrecognised_social_provider_missing_only_the_label_is_an_error() {
        // icon and color are given; the accessible-name label is not. It is
        // just as required as the other two -- see
        // vados.allium's UnrecognisedSocialLinkIsFullyDescribed.
        let site = TestSite::new("social_missing_label");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://mastodon.social/@someone","icon":"mastodon","color":"6364FF"}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnrecognisedSocialLinkIncomplete,
            "https://mastodon.social/@someone"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn unrecognised_social_provider_with_icon_color_and_label_is_not_flagged() {
        let site = TestSite::new("social_fully_described");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://mastodon.social/@someone","icon":"mastodon","color":"6364FF","label":"Mastodon"}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::UnrecognisedSocialLinkIncomplete),
            0
        );
        assert!(report.passed());
    }

    #[test]
    fn recognized_social_provider_without_icon_is_not_flagged() {
        let site = TestSite::new("social_recognized");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source(
            "menu.json",
            r#"{"mainMenu":[],"socials":[{"url":"https://github.com/someone","icon":null,"color":null}]}"#,
        );
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::UnrecognisedSocialLinkIncomplete),
            0
        );
        assert!(report.passed());
    }

    #[test]
    fn unrecognised_content_reference_is_an_error() {
        let site = TestSite::new("content_unrecognised");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"not-a-recognised-reference","order":null,
               "leftNotifications":null,"rightNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnrecognisedContentReference,
            "/"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn missing_content_file_is_a_warning() {
        let site = TestSite::new("content_missing_file");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"missing.md","order":null,
               "leftNotifications":null,"rightNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(&report, FindingKind::MissingContentFile, "/"));
        assert!(report.passed());
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
    fn duplicate_image_reference_is_an_error_for_each_offender() {
        let site = TestSite::new("image_duplicate");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_image(
            "images.json",
            r#"{"title":null,"list":[
                {"title":null,"fileName":"photo.jpg","altText":"a photo"},
                {"title":null,"fileName":"photo.png","altText":"a photo again"}
            ]}"#,
        );
        site.write_image("photo.jpg", "not really a jpg");
        site.write_image("photo.png", "not really a png");

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::DuplicateImageReference), 2);
        assert!(!report.passed());
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
    fn missing_image_source_file_is_a_warning() {
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
        assert!(report.passed());
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

    #[test]
    fn unknown_notification_image_reference_is_a_warning() {
        let site = TestSite::new("notification_image_unknown");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"<p>hi</p>","order":null,
               "leftNotifications":[{"content":"<p>n</p>","title":null,"image":"/nope","url":null,"color":null}],
               "rightNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnknownNotificationImageReference,
            "/"
        ));
        assert!(report.passed());
    }

    #[test]
    fn dead_internal_notification_link_is_a_warning() {
        let site = TestSite::new("notification_dead_link");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            r#"{"title":"Home","subTitle":null,"image":null,"icon":null,"summary":null,
               "content":"<p>hi</p>","order":null,
               "rightNotifications":[{"content":"<p>n</p>","title":null,"image":null,"url":"/missing","color":null}],
               "leftNotifications":null}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::DeadInternalNotificationLink,
            "/"
        ));
        assert!(report.passed());
    }

    fn page_with_luma_events(events: &str) -> String {
        format!(r#"{{"title":"Home","content":"<p>hi</p>","lumaEvents":{events}}}"#)
    }

    #[test]
    fn well_formed_titled_luma_events_have_no_findings() {
        let site = TestSite::new("luma_valid");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            &page_with_luma_events(
                r#"[{"eventId":"evt-a1","title":"Jam","display":"both"},
                    {"eventId":"evt-b2","title":"Gig","buttonLabel":"Sign up"}]"#,
            ),
        );

        let report = site.check();

        assert!(report.findings.is_empty(), "{:?}", report.findings);
    }

    #[test]
    fn malformed_luma_event_id_is_an_error() {
        let site = TestSite::new("luma_malformed");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            &page_with_luma_events(r#"[{"eventId":"https://luma.com/jam","title":"Jam"}]"#),
        );

        let report = site.check();

        assert!(has_kind_at(&report, FindingKind::MalformedLumaEventId, "/"));
        assert!(!report.passed());
    }

    #[test]
    fn luma_event_without_title_is_an_error() {
        let site = TestSite::new("luma_untitled");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            &page_with_luma_events(
                r#"[{"eventId":"evt-a1","title":null},{"eventId":"evt-b2","title":" "}]"#,
            ),
        );

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::LumaEventWithoutTitle), 2);
        assert!(!report.passed());
    }

    #[test]
    fn duplicate_luma_event_is_a_warning_for_each_occurrence() {
        let site = TestSite::new("luma_duplicate");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source(
            "page.json",
            &page_with_luma_events(
                r#"[{"eventId":"evt-a1","title":"Jam"},{"eventId":"evt-a1","title":"Jam again"}]"#,
            ),
        );

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::DuplicateLumaEvent), 2);
        assert!(report.passed());
    }

    #[test]
    fn images_manifest_unreadable_is_a_warning() {
        let site = TestSite::new("images_manifest_unreadable");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_image("images.json", "{ not valid json");

        let report = site.check();

        assert_eq!(
            count_kind(&report, FindingKind::ImagesManifestUnreadable),
            1
        );
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
    fn dated_page_without_summary_is_a_warning_and_unreadable_updated_an_error() {
        let site = date_ordered_blog(
            r#"{"title":"Post","content":"<p>hi</p>","date":"2026-10-22","updated":"soon"}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::DatedPageWithoutSummary,
            "/blog/post"
        ));
        assert!(has_kind_at(
            &report,
            FindingKind::UnreadablePageDate,
            "/blog/post"
        ));
        assert!(!report.passed());
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

    fn date_ordered_blog(post_json: &str) -> TestSite {
        let site = TestSite::new("date_ordered_blog");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_source(
            "blog/page.json",
            r#"{"title":"Blog","content":"<p>hi</p>","childOrder":"dateDesc"}"#,
        );
        site.write_source("blog/post/page.json", post_json);
        site
    }

    #[test]
    fn dated_page_in_a_date_ordered_section_has_no_findings() {
        let site = date_ordered_blog(
            r#"{"title":"Post","content":"<p>hi</p>","date":"2026-10-22T09:30","summary":"Hi"}"#,
        );

        let report = site.check();

        assert!(report.findings.is_empty(), "{:?}", report.findings);
    }

    #[test]
    fn unreadable_page_date_is_an_error() {
        let site =
            date_ordered_blog(r#"{"title":"Post","content":"<p>hi</p>","date":"22-10-2026"}"#);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UnreadablePageDate,
            "/blog/post"
        ));
        assert!(!report.passed());
    }

    #[test]
    fn undated_page_in_a_date_ordered_section_is_a_warning() {
        let site = date_ordered_blog(r#"{"title":"Post","content":"<p>hi</p>"}"#);

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UndatedPageInDateOrderedSection,
            "/blog/post"
        ));
        assert!(report.passed());
    }

    #[test]
    fn undated_page_under_an_order_ordered_parent_has_no_findings() {
        let site = TestSite::new("undated_order_parent");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_source(
            "about/page.json",
            r#"{"title":"About","content":"<p>hi</p>"}"#,
        );

        assert!(site.check().findings.is_empty());
    }

    fn main_json_with_site_url(site_url: &str) -> String {
        VALID_MAIN_JSON.replacen(
            r#""footerContent""#,
            &format!(r#""siteUrl": "{site_url}", "footerContent""#),
            1,
        )
    }

    fn feed_site(name: &str, main_json: &str, post_json: &str) -> TestSite {
        let site = TestSite::new(name);
        site.write_source("main.json", main_json);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_source(
            "blog/page.json",
            r#"{"title":"Blog","content":"<p>hi</p>","childOrder":"dateDesc","feed":true}"#,
        );
        site.write_source("blog/post/page.json", post_json);
        site
    }

    const DATED_POST: &str =
        r#"{"title":"Post","content":"<p>hi</p>","date":"2026-10-22","summary":"Hi"}"#;

    #[test]
    fn feed_with_a_usable_site_url_and_dated_posts_has_no_findings() {
        let site = feed_site(
            "feed_ok",
            &main_json_with_site_url("https://example.com"),
            DATED_POST,
        );

        let report = site.check();

        assert!(report.findings.is_empty(), "{:?}", report.findings);
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
    fn a_bulma_0_stylesheet_is_an_error() {
        assert!(is_outdated_bulma_link(
            "https://cdn.jsdelivr.net/npm/bulma@0.9.4/css/bulma.min.css"
        ));
        assert!(is_outdated_bulma_link(
            "https://cdnjs.cloudflare.com/ajax/libs/bulma/0.9.3/css/bulma.min.css"
        ));
        assert!(is_outdated_bulma_link(
            "https://cdn.jsdelivr.net/npm/bulma@~0.9.4/css/bulma.min.css"
        ));
        assert!(is_outdated_bulma_link(
            "https://cdn.jsdelivr.net/npm/bulma@0/css/bulma.min.css"
        ));
        assert!(!is_outdated_bulma_link(
            "https://cdn.jsdelivr.net/npm/bulma@1.0.4/css/bulma.min.css"
        ));
        assert!(!is_outdated_bulma_link(
            "https://cdn.jsdelivr.net/npm/bulma@latest/css/bulma.min.css"
        ));
        assert!(!is_outdated_bulma_link("/css/main.min.css"));
        // Only the version right after the name counts.
        assert!(!is_outdated_bulma_link("/css/bulma/theme.css?v=0"));
        assert!(!is_outdated_bulma_link("/css/bulma/bulma.min.css"));
        assert_eq!(
            FindingKind::OutdatedBulmaStylesheet.severity(),
            Severity::Error
        );
    }

    #[test]
    fn a_dependency_spec_is_outdated_only_when_it_cannot_reach_bulma_1() {
        for outdated in ["^0.9.4", "~0.9", "0.x", "=0.9.4", "v0.9.4", "^0.8 || ^0.9"] {
            assert!(spec_is_outdated(outdated), "{outdated}");
        }
        for fine in ["^1.0.4", "1", ">=1.0", "^0.9 || ^1.0", "latest", "*", ""] {
            assert!(!spec_is_outdated(fine), "{fine}");
        }
    }

    #[test]
    fn the_manifest_is_found_from_a_source_directory_given_as_a_relative_path() {
        let site = TestSite::new("bulma_0_package_relative");
        site.write_project("package.json", r#"{"dependencies":{"bulma":"^0.9.4"}}"#);
        let source = site.source.join("..").join("source");
        let found = outdated_bulma_dependency(source.to_str().unwrap());
        assert_eq!(found.map(|(_, spec)| spec).as_deref(), Some("^0.9.4"));
    }

    #[test]
    fn a_bulma_0_stylesheet_in_main_json_is_an_error() {
        let site = TestSite::new("bulma_0_stylesheet");
        let main_json = VALID_MAIN_JSON.replace(
            r#""cssFiles": []"#,
            r#""cssFiles": ["https://cdn.jsdelivr.net/npm/bulma@0.9.4/css/bulma.min.css"]"#,
        );
        site.write_source("main.json", &main_json);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::OutdatedBulmaStylesheet), 1);
        assert!(!report.passed());
    }

    #[test]
    fn a_bulma_0_package_dependency_is_an_error() {
        let site = TestSite::new("bulma_0_package");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_project(
            "package.json",
            r#"{"devDependencies":{"sass":"^1.77.6","bulma":"^0.9.4"}}"#,
        );

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::OutdatedBulmaDependency), 1);
        assert!(has_kind_at(
            &report,
            FindingKind::OutdatedBulmaDependency,
            site.root.join("package.json").to_str().unwrap()
        ));
        assert!(!report.passed());
    }

    #[test]
    fn an_import_of_bulma_in_main_scss_is_a_warning() {
        let site = TestSite::new("bulma_sass_import");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_project(
            "sass/main.scss",
            "$primary: #ff00aa;\n\n  @import \"bulma/bulma\";\n",
        );

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::BulmaSassImport), 1);
        assert!(has_kind_at(
            &report,
            FindingKind::BulmaSassImport,
            site.root.join("sass/main.scss").to_str().unwrap()
        ));
        assert!(report.passed(), "a warning does not fail the check");
        assert_eq!(FindingKind::BulmaSassImport.severity(), Severity::Warning);
    }

    #[test]
    fn a_use_of_bulma_or_an_import_of_something_else_is_clean() {
        let site = TestSite::new("bulma_sass_use");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_project(
            "sass/main.scss",
            "@use \"bulma/sass\" with ($primary: #ff00aa);\n@import \"extras\";\n",
        );

        assert!(site.check().findings.is_empty());
    }

    #[test]
    fn a_bulma_1_package_dependency_is_clean() {
        let site = TestSite::new("bulma_1_package");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_project(
            "package.json",
            r#"{"devDependencies":{"sass":"^1.77.6","bulma":"^1.0.4"}}"#,
        );

        let report = site.check();

        assert!(report.findings.is_empty(), "{:?}", report.findings);
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

    #[test]
    fn undated_page_under_a_feed_gets_one_warning_about_the_feed() {
        let site = feed_site(
            "feed_undated",
            &main_json_with_site_url("https://example.com"),
            r#"{"title":"Post","content":"<p>hi</p>"}"#,
        );

        let report = site.check();

        assert!(has_kind_at(
            &report,
            FindingKind::UndatedPageLeftOutOfFeed,
            "/blog/post"
        ));
        assert_eq!(
            count_kind(&report, FindingKind::UndatedPageInDateOrderedSection),
            0
        );
        assert!(report.passed());
    }
}
