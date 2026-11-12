extern crate core;

use crate::content::{
    to_content_items, ContentHelper, GenericContent, MarkdownOptions, AGENDA_SCRIPT_HREF,
};
use crate::event::write_calendars;
use crate::feed::write_feeds;
use crate::files::{
    copy_static_files, get_all_directory_paths, get_image_list, get_main_config, get_menu_config,
    get_page_directory_paths, write_html, write_raw,
};
use crate::image::ImageProcessor;
use crate::search::{generated_search_page, write_search, SEARCH_PATH};
use crate::sitemap::{write_robots, write_sitemap};
use crate::structure::Structure;
use crate::ui_text::ui_text;

/// Warnings about source content omitted or degraded during publication.
#[derive(Debug)]
pub struct GenerateReport {
    pub warnings: Vec<GenerateWarning>,
}

#[derive(Debug)]
pub struct GenerateWarning {
    pub location: String,
    pub message: String,
}

impl std::fmt::Display for GenerateWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "warning {} ({})", self.message, self.location)
    }
}

/// File-specific failures that prevented a complete publication.
#[derive(Debug)]
pub struct GenerateError {
    pub findings: Vec<crate::check::Finding>,
}

impl GenerateError {
    pub(crate) fn at(path: impl AsRef<std::path::Path>, problem: impl std::fmt::Display) -> Self {
        Self {
            findings: vec![crate::check::Finding {
                kind: crate::check::FindingKind::GenerationFailed,
                location: path.as_ref().display().to_string(),
                detail: Some(problem.to_string()),
            }],
        }
    }
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, finding) in self.findings.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{finding}")?;
        }
        Ok(())
    }
}
impl std::error::Error for GenerateError {}

/// Publishes a site, validating inputs before touching the destination.
/// Invalid events and other tolerated content problems are returned as warnings.
/// Image processing and output failures return an error; cleanup and the output
/// manifest advance only after a complete publication. An output I/O failure can
/// leave partially written files, so the caller should retry after fixing it.
pub fn generate(
    source: &str,
    img_source: &str,
    destination: &str,
) -> Result<GenerateReport, GenerateError> {
    use crate::check::FindingKind::*;
    let checked = crate::check::check(source, img_source);
    let mut errors = vec![];
    let mut warnings = vec![];
    for mut finding in checked.findings {
        match finding.kind {
            ExternalMenuLinkWithoutTitle | UnrecognisedSocialLinkIncomplete => {
                finding.detail = Some(finding.location.clone());
                finding.location = format!("{source}/menu.json");
            }
            UnrecognisedContentReference if finding.location != format!("{source}/main.json") => {
                finding.location = if finding.location == "/" {
                    format!("{source}/page.json")
                } else {
                    format!("{source}{}/page.json", finding.location)
                };
            }
            _ => {}
        }
        // Preserve generation's content-level fallbacks, including invalid
        // events, dates and feeds. These are still errors in a source check.
        if matches!(
            finding.kind,
            SiteConfigUnreadable
                | MenuUnreadable
                | PageConfigUnreadable
                | ImagesManifestUnreadable
                | SourceUnreadable
                | MissingImageSourceFile
                | UnrecognisedContentReference
                | ExternalMenuLinkWithoutTitle
                | UnrecognisedSocialLinkIncomplete
        ) {
            errors.push(finding);
        } else {
            warnings.push(GenerateWarning {
                location: finding.location.clone(),
                message: match &finding.detail {
                    Some(detail) => format!("{}: {}", finding.kind, detail),
                    None => finding.kind.to_string(),
                },
            });
        }
    }
    if !errors.is_empty() {
        return Err(GenerateError { findings: errors });
    }
    let main_config = get_main_config(source)?;
    let menu_config = get_menu_config(source)?;
    // Hold every parsed page and image manifest before creating output. A
    // second read failure is also returned, rather than hidden or panicked.
    let pages = get_page_directory_paths(source, main_config.static_dir.as_deref())?
        .into_iter()
        .map(|dir| {
            let path = std::path::Path::new(&dir).join("page.json");
            let config = crate::json_files::read_json_or(&path, || {
                crate::config_files::PageConfig::new(&dir)
            })
            .map_err(|e| GenerateError::at(&path, e))?;
            Ok((dir, config))
        })
        .collect::<Result<Vec<_>, GenerateError>>()?;
    let images = get_all_directory_paths(img_source)?
        .into_iter()
        .map(|dir| {
            let list = get_image_list(&dir)?;
            Ok((dir, list))
        })
        .collect::<Result<Vec<_>, GenerateError>>()?;
    let output = crate::files::Output::new(destination)?;
    let destination = &output;

    // Link previews need the site URL to point at their image, so only then
    // is it worth making their crops.
    let preview_keys = if main_config.get_site_url().is_some() {
        let mut keys: std::collections::HashSet<String> =
            pages.iter().filter_map(|(_, p)| p.image.clone()).collect();
        keys.extend(main_config.default_image.clone());
        keys
    } else {
        Default::default()
    };
    let mut image_processor = ImageProcessor::new(img_source, destination, preview_keys);
    for (directory_path, list) in images {
        if let Some(l) = list {
            image_processor.process_list(&directory_path, l.list)?
        }
    }

    let structure = Structure::new(image_processor.meta_cache);
    let mut all_paths = vec![];
    // Static files go first, so a generated page at the same path wins.
    let static_files = match &main_config.static_dir {
        Some(static_dir) => copy_static_files(source, static_dir, destination)?,
        None => vec![],
    };

    for (directory_path, page_config) in pages {
        let content_items = to_content_items(
            source,
            directory_path,
            page_config,
            &structure,
            MarkdownOptions::for_site(&main_config),
        );
        let path = content_items.item.path.clone();
        if let Some(notifications) = content_items.left_sub_notifications {
            structure.add_left_sub_notifications(&path, notifications)
        }
        if let Some(notifications) = content_items.right_sub_notifications {
            structure.add_right_sub_notifications(&path, notifications)
        }
        structure.add_item(content_items.item);
        all_paths.push(path)
    }
    // The search needs a page; a site's own at the same path keeps its say.
    let search = main_config.get_search();
    let ui = ui_text(main_config.get_language());
    if search.is_some() && structure.find_item(SEARCH_PATH).is_none() {
        structure.add_item(generated_search_page(ui));
        all_paths.push(String::from(SEARCH_PATH));
    }
    structure.sort();

    let generic_content = GenericContent::new(source, destination, &main_config, &menu_config)?;
    let mut uses_mermaid = false;
    for path in &all_paths {
        let content_helper = ContentHelper::new(path, &structure);
        let navigation = content_helper
            .get_navigation(&main_config, &menu_config)
            .map_err(|e| GenerateError::at(format!("{source}/menu.json"), e))?;
        let main_content = content_helper.get_main_content(source, &generic_content);
        let page = content_helper.get_page(&navigation, &main_content, &generic_content);

        uses_mermaid |= page.contains("data-mermaid-script=");

        //generating the end html and writing it to file
        write_html(destination, path, &page)?;
        if main_config.markdown == Some(true) {
            let markdown = content_helper.get_markdown(source, &generic_content);
            write_raw(destination, &crate::markdown::href(path), markdown)?;
        }
    }
    if uses_mermaid {
        write_raw(
            destination,
            crate::mermaid::LOADER_PATH,
            include_str!("../assets/js/mermaid.js"),
        )?;
    }
    write_feeds(source, destination, &main_config, &structure)?;
    write_calendars(destination, &main_config, &structure)?;
    if let Some(search) = &search {
        write_search(source, destination, search, &structure, ui)?;
    }
    if structure.has_agenda() {
        write_raw(
            destination,
            AGENDA_SCRIPT_HREF,
            include_str!("../assets/js/agenda.js"),
        )?;
    }
    if let Some(site_url) = main_config.get_site_url() {
        write_sitemap(destination, site_url, &structure)?;
    }
    write_robots(destination, main_config.get_site_url(), &static_files)?;
    output.finish()?;
    Ok(GenerateReport { warnings })
}
