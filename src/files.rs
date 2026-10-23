use crate::config_files::{ImageList, MainConfig, MenuConfig, PageConfig};
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
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

/// Every hero image a page.json names, read ahead of the pages themselves so
/// the images can be processed first. A page.json that can't be read names
/// none here; reading the pages reports it.
pub(crate) fn get_hero_image_references(source: &str, static_dir: Option<&str>) -> HashSet<String> {
    get_page_directory_paths(source, static_dir)
        .iter()
        .filter_map(|dir| File::open(format!("{}/page.json", dir)).ok())
        .filter_map(|file| serde_json::from_reader::<_, PageConfig>(file).ok())
        .filter_map(|config| config.image)
        .collect()
}

pub(crate) fn write_html(destination: &str, path: &str, html: &str) {
    let (path, file) = get_html_destination(destination, path);
    let contents = minifier::html::minify(html);
    fs::create_dir_all(&path).unwrap();
    fs::write(format!("{}/{}", path, file), contents).expect("Unable to write file");
}

pub(crate) fn write_raw<C: AsRef<[u8]>>(destination: &str, path: &str, contents: C) {
    let (path, file) = get_destination(destination, path);
    fs::create_dir_all(&path).unwrap();
    fs::write(format!("{}/{}", path, file), contents).expect("Unable to write file");
}

pub(crate) fn empty(destination: &str, path: &str) -> bool {
    let full_path = format!("{}/{}", destination, path);
    !Path::new(&full_path).exists()
}

/// The placeholder that keeps an empty directory in git; never published.
pub(crate) const GITKEEP: &str = ".gitkeep";

/// Where main.json's `staticDir` points: relative to the source root, so
/// `"static"` sits inside it and `"../static"` next to it.
pub(crate) fn static_dir_path(source: &str, static_dir: &str) -> PathBuf {
    Path::new(source).join(static_dir)
}

/// Every directory under the source root that is a page: all of them, except
/// the static directory and everything in it when it sits inside the root.
pub(crate) fn get_page_directory_paths(source: &str, static_dir: Option<&str>) -> Vec<String> {
    let excluded = static_dir.and_then(|d| static_dir_path(source, d).canonicalize().ok());
    WalkDir::new(source)
        .into_iter()
        .filter_entry(|e| {
            e.metadata().unwrap().is_dir()
                && excluded
                    .as_ref()
                    .is_none_or(|x| e.path().canonicalize().ok().as_ref() != Some(x))
        })
        .map(|e| e.unwrap().path().display().to_string())
        .collect()
}

/// Every file in the static directory, as the site-root path it's published
/// at (`/robots.txt`, `/.well-known/security.txt`), with where it's read
/// from. Empty when there is no such directory. A `.gitkeep` is left out: it
/// only keeps an otherwise empty directory in git.
pub(crate) fn get_static_files(source: &str, static_dir: &str) -> Vec<(String, PathBuf)> {
    let root = static_dir_path(source, static_dir);
    WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.file_name() != GITKEEP)
        .filter_map(|e| {
            let relative = e.path().strip_prefix(&root).ok()?;
            let published = format!("/{}", relative.to_string_lossy().replace('\\', "/"));
            Some((published, e.path().to_path_buf()))
        })
        .collect()
}

/// Copies the static directory into the destination as it is. Returns the
/// site-root paths it published.
pub(crate) fn copy_static_files(source: &str, static_dir: &str, destination: &str) -> Vec<String> {
    let files = get_static_files(source, static_dir);
    if files.is_empty() && !static_dir_path(source, static_dir).is_dir() {
        println!(
            "staticDir {} is not a directory, so no static files are published.",
            static_dir
        );
    }
    files
        .into_iter()
        .filter_map(|(published, from)| match fs::read(&from) {
            Ok(contents) => {
                write_raw(destination, &published, contents);
                Some(published)
            }
            Err(e) => {
                println!(
                    "static file {} could not be read, skipping it: {}",
                    from.display(),
                    e
                );
                None
            }
        })
        .collect()
}

pub(crate) fn get_all_directory_paths(source: &str) -> Vec<String> {
    WalkDir::new(source)
        .into_iter()
        .filter_entry(|e| e.metadata().unwrap().is_dir())
        .map(|e| e.unwrap().path().display().to_string())
        .collect()
}
