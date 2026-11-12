use crate::config_files::{ImageList, MainConfig, MenuConfig, PageConfig};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use walkdir::WalkDir;

/// Relative paths owned by the last completed run. Keep this alongside the
/// output so moving or deleting the destination also moves or clears its state.
const MANIFEST: &str = ".vados-manifest.json";

pub(crate) struct Output {
    root: PathBuf,
    previous: BTreeSet<String>,
    current: Mutex<BTreeSet<String>>,
}

impl Output {
    pub(crate) fn new(destination: &str) -> Self {
        let root = PathBuf::from(destination);
        fs::create_dir_all(&root).expect("Unable to create destination");
        let root = root.canonicalize().expect("Unable to resolve destination");
        let manifest = root.join(MANIFEST);
        assert!(
            !manifest.is_symlink(),
            "Output manifest must not be a symlink"
        );
        let previous = match fs::read(&manifest) {
            Ok(bytes) => serde_json::from_slice::<BTreeSet<String>>(&bytes)
                .expect("Unable to parse output manifest"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeSet::new(),
            Err(e) => panic!("Unable to read output manifest: {e}"),
        };
        let output = Self {
            root,
            previous,
            current: Mutex::new(BTreeSet::new()),
        };
        // Validate before writing anything, including paths no longer in use.
        for path in &output.previous {
            output.path(path);
        }
        output
    }

    fn path(&self, relative: &str) -> PathBuf {
        assert!(
            !relative.is_empty()
                && !relative.contains('\\')
                && relative
                    .split('/')
                    .all(|c| !c.is_empty() && c != "." && c != "..")
                && relative.split('/').next() != Some(MANIFEST),
            "Invalid output path: {relative}"
        );
        let mut full = self.root.clone();
        for component in relative.split('/') {
            full.push(component);
            assert!(
                !full.is_symlink(),
                "Output path must not traverse a symlink: {}",
                full.display()
            );
        }
        full
    }

    /// Cached image variants belong to this run even when no write is needed.
    pub(crate) fn record(&self, path: &str) {
        let relative = path
            .strip_prefix('/')
            .expect("output path must start with /");
        self.path(relative);
        self.current.lock().unwrap().insert(relative.to_owned());
    }

    pub(crate) fn finish(&self) {
        let current = self.current.lock().unwrap();
        for relative in self.previous.difference(&current) {
            let path = self.path(relative);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => panic!("Unable to remove stale output {}: {e}", path.display()),
            }
            // Only remove now-empty ancestors of a tracked file. Other build
            // steps can keep files in the same directories.
            let mut parent = path.parent();
            while let Some(dir) = parent.filter(|dir| *dir != self.root) {
                match fs::remove_dir(dir) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => break,
                    Err(e) => panic!(
                        "Unable to remove empty output directory {}: {e}",
                        dir.display()
                    ),
                }
                parent = dir.parent();
            }
        }
        // Replace atomically so an interrupted write cannot truncate the last
        // completed manifest. create_new leaves any existing file untouched.
        for attempt in 0.. {
            let temporary = self
                .root
                .join(format!("{MANIFEST}.{}.{attempt}.tmp", std::process::id()));
            let file = match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
            {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("Unable to create output manifest: {e}"),
            };
            serde_json::to_writer_pretty(file, &*current).expect("Unable to write output manifest");
            fs::rename(temporary, self.root.join(MANIFEST))
                .expect("Unable to replace output manifest");
            break;
        }
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

pub(crate) fn write_html(destination: &Output, path: &str, html: &str) {
    let path = if path == "/" {
        String::from("/index.html")
    } else {
        format!("{}.html", path)
    };
    write_raw(destination, &path, minifier::html::minify(html));
}

pub(crate) fn write_raw<C: AsRef<[u8]>>(destination: &Output, path: &str, contents: C) {
    let relative = path
        .strip_prefix('/')
        .expect("output path must start with /");
    let full_path = destination.path(relative);
    fs::create_dir_all(full_path.parent().unwrap()).expect("Unable to create output directory");
    fs::write(full_path, contents).expect("Unable to write file");
    destination.record(path);
}

pub(crate) fn empty(destination: &Output, path: &str) -> bool {
    !destination.path(path.trim_start_matches('/')).exists()
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
pub(crate) fn copy_static_files(
    source: &str,
    static_dir: &str,
    destination: &Output,
) -> Vec<String> {
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
