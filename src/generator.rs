extern crate core;

use crate::content::{to_content_items, ContentHelper, GenericContent};
use crate::files::{
    get_all_directory_paths, get_image_list, get_main_config, get_menu_config, write_html,
};
use crate::image::ImageProcessor;
use crate::structure::Structure;

/// The core function to call, if the files at the source are valid, the static site will be
/// generated at the destination location. Please make sure the files and/or directories have the proper ownership.
pub fn generate(source: &str, img_source: &str, destination: &str) {
    let main_config = get_main_config(source);
    let menu_config = get_menu_config(source);

    let mut image_processor = ImageProcessor::new(img_source, destination);
    for directory_path in get_all_directory_paths(img_source) {
        if let Some(l) = get_image_list(&directory_path) {
            image_processor.process_list(&directory_path, l.list)
        }
    }

    let structure = Structure::new(image_processor.meta_cache);
    let mut all_paths = vec![];
    for directory_path in get_all_directory_paths(source) {
        let content_items = to_content_items(source, directory_path, &structure);
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

    let generic_content = GenericContent::new(source, destination, &main_config);
    for path in &all_paths {
        let content_helper = ContentHelper::new(path, &structure);
        let navigation = content_helper.get_navigation(&main_config, &menu_config);
        let main_content = content_helper.get_main_content(source);
        let page = content_helper.get_page(&navigation, &main_content, &generic_content);

        //generating the end html and writing it to file
        write_html(destination, path, &page);
    }
}
