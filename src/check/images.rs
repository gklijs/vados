use super::config::read_json;
use super::{checked_directories, Finding, FindingKind};

use crate::config_files::ImageList;
use crate::files::get_all_directory_paths;
use crate::image::reference_key_for;
use std::collections::HashMap;
use std::path::Path;

pub(super) struct CheckedImage {
    pub(super) reference_key: String,
    pub(super) source_file: String,
    pub(super) alt_text: String,
}

pub(super) fn collect_images(img_source: &str, findings: &mut Vec<Finding>) -> Vec<CheckedImage> {
    let mut images = Vec::new();
    for dir in checked_directories(get_all_directory_paths(img_source), findings) {
        let path = format!("{}/images.json", dir);
        if !Path::new(&path).try_exists().unwrap_or(true) {
            continue;
        }
        let Some(list) =
            read_json::<ImageList>(&path, FindingKind::ImagesManifestUnreadable, findings)
        else {
            continue;
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

pub(super) fn duplicate_image_findings(images: &[CheckedImage]) -> Vec<Finding> {
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

#[cfg(test)]
mod tests {
    use crate::check::test_support::*;
    use crate::check::FindingKind;

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
    fn images_manifest_unreadable_is_an_error() {
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
        assert!(!report.passed());
    }
}
