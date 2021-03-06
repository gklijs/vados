use crate::config_files::{ImageList, MainConfig, MenuConfig};
use std::fs;
use std::fs::File;
use std::path::Path;
use walkdir::WalkDir;

fn get_html_destination(destination: &str, path: &str) -> (String, String) {
    if path == "/" {
        (String::from(destination), String::from("index.html"))
    } else {
        match String::from(path).rsplit_once('/') {
            None => panic!("path didn't contain a separator"),
            Some((first, second)) => (
                format!("{}{}", destination, first),
                format!("{}.html", second),
            ),
        }
    }
}

fn get_destination(destination: &str, path: &str) -> (String, String) {
    match String::from(path).rsplit_once('/') {
        None => panic!("path didn't contain a separator"),
        Some((first, second)) => (format!("{}{}", destination, first), String::from(second)),
    }
}

pub(crate) fn get_main_config(source: &str) -> MainConfig {
    let path = format!("{}/main.json", source);
    let config_file_path = Path::new(&path);
    let file = File::open(config_file_path).expect("JSON file main.json could not be read");
    serde_json::from_reader(file).expect("JSON file main.json was not well-formatted")
}

pub(crate) fn get_menu_config(source: &str) -> MenuConfig {
    let path = format!("{}/menu.json", source);
    let config_file_path = Path::new(&path);
    let file = File::open(config_file_path).expect("JSON file menu.json could not be read");
    serde_json::from_reader(file).expect("JSON file menu.json was not well-formatted")
}

pub(crate) fn get_image_list(source: &str) -> Option<ImageList> {
    let path = format!("{}/images.json", source);
    let config_file_path = Path::new(&path);
    let file = match File::open(config_file_path) {
        Ok(f) => f,
        Err(_) => return None,
    };
    match serde_json::from_reader(file) {
        Ok(list) => Some(list),
        Err(e) => {
            println!("JSON file images.json was not well-formatted:\n{}", e);
            None
        }
    }
}

pub(crate) fn write_html(destination: &str, path: &str, html: &str) {
    let (path, file) = get_html_destination(destination, path);
    let contents = minifier::html::minify(html);
    fs::create_dir_all(&path).unwrap();
    fs::write(format!("{}/{}", &path, file), contents).expect("Unable to write file");
}

pub(crate) fn write_raw<C: AsRef<[u8]>>(destination: &str, path: &str, contents: C) {
    let (path, file) = get_destination(destination, path);
    fs::create_dir_all(&path).unwrap();
    fs::write(format!("{}/{}", &path, file), contents).expect("Unable to write file");
}

pub(crate) fn empty(destination: &str, path: &str) -> bool {
    let full_path = format!("{}/{}", destination, path);
    !Path::new(&full_path).exists()
}

pub(crate) fn get_all_directory_paths(source: &str) -> Vec<String> {
    WalkDir::new(source)
        .into_iter()
        .filter_entry(|e| e.metadata().unwrap().is_dir())
        .map(|e| e.unwrap().path().display().to_string())
        .collect()
}
