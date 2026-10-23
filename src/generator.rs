extern crate core;

use crate::content::{to_content_items, ContentHelper, GenericContent};
use crate::feed::write_feeds;
use crate::files::{
    copy_static_files, get_all_directory_paths, get_hero_image_references, get_image_list,
    get_main_config, get_menu_config, get_page_directory_paths, write_html,
};
use crate::image::{prune_orphaned_variants, ImageProcessor};
use crate::sitemap::write_sitemap;
use crate::structure::Structure;

/// The core function to call, if the files at the source are valid, the static site will be
/// generated at the destination location. Please make sure the files and/or directories have the proper ownership.
pub fn generate(source: &str, img_source: &str, destination: &str) {
    let main_config = get_main_config(source);
    let menu_config = get_menu_config(source);

    // Link previews need the site URL to point at their image, so only then
    // is it worth making their crops.
    let preview_keys = if main_config.get_site_url().is_some() {
        let mut keys = get_hero_image_references(source, main_config.static_dir.as_deref());
        keys.extend(main_config.default_image.clone());
        keys
    } else {
        Default::default()
    };
    let mut image_processor = ImageProcessor::new(img_source, destination, preview_keys);
    for directory_path in get_all_directory_paths(img_source) {
        if let Some(l) = get_image_list(&directory_path) {
            image_processor.process_list(&directory_path, l.list)
        }
    }
    prune_orphaned_variants(destination, &image_processor.meta_cache);

    let structure = Structure::new(image_processor.meta_cache);
    let mut all_paths = vec![];
    // Static files go first, so a generated page at the same path wins.
    let static_files = match &main_config.static_dir {
        Some(static_dir) => copy_static_files(source, static_dir, destination),
        None => vec![],
    };

    for directory_path in get_page_directory_paths(source, main_config.static_dir.as_deref()) {
        let content_items = to_content_items(
            source,
            directory_path,
            &structure,
            main_config.highlights_code(),
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
    structure.sort();

    let generic_content = GenericContent::new(source, destination, &main_config, &menu_config);
    for path in &all_paths {
        let content_helper = ContentHelper::new(path, &structure);
        let navigation = content_helper.get_navigation(&main_config, &menu_config);
        let main_content = content_helper.get_main_content(source, &generic_content);
        let page = content_helper.get_page(&navigation, &main_content, &generic_content);

        //generating the end html and writing it to file
        write_html(destination, path, &page);
    }
    write_feeds(source, destination, &main_config, &structure);
    if let Some(site_url) = main_config.get_site_url() {
        write_sitemap(destination, site_url, &structure, &static_files);
    }
}
