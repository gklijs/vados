use crate::config_files::{ImageList, MainConfig, MenuConfig};
use crate::generator::GenerateError;
use crate::json_files::{read_json, read_json_or};
use std::collections::BTreeSet;
use std::fs;
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
    pub(crate) fn new(destination: &str) -> Result<Self, GenerateError> {
        let root = PathBuf::from(destination);
        fs::create_dir_all(&root).map_err(|e| GenerateError::at(&root, e))?;
        let root = root
            .canonicalize()
            .map_err(|e| GenerateError::at(&root, e))?;
        let manifest = root.join(MANIFEST);
        if manifest.is_symlink() {
            return Err(GenerateError::at(
                &manifest,
                "output manifest must not be a symlink",
            ));
        }
        let previous =
            read_json_or(&manifest, BTreeSet::new).map_err(|e| GenerateError::at(&manifest, e))?;
        let output = Self {
            root,
            previous,
            current: Mutex::new(BTreeSet::new()),
        };
        // Validate before writing anything, including paths no longer in use.
        for path in &output.previous {
            output.path(path)?;
        }
        Ok(output)
    }

    fn path(&self, relative: &str) -> Result<PathBuf, GenerateError> {
        if relative.is_empty()
            || relative.contains('\\')
            || relative
                .split('/')
                .any(|c| c.is_empty() || c == "." || c == "..")
            || relative.split('/').next() == Some(MANIFEST)
        {
            return Err(GenerateError::at(
                self.root.join(relative),
                "invalid output path",
            ));
        }
        let mut full = self.root.clone();
        for component in relative.split('/') {
            full.push(component);
            if full.is_symlink() {
                return Err(GenerateError::at(
                    &full,
                    "output path must not traverse a symlink",
                ));
            }
        }
        Ok(full)
    }

    /// Cached image variants belong to this run even when no write is needed.
    pub(crate) fn record(&self, path: &str) -> Result<(), GenerateError> {
        let relative = path
            .strip_prefix('/')
            .ok_or_else(|| GenerateError::at(path, "output path must start with /"))?;
        self.path(relative)?;
        self.current.lock().unwrap().insert(relative.to_owned());
        Ok(())
    }

    pub(crate) fn finish(&self) -> Result<(), GenerateError> {
        let current = self.current.lock().unwrap();
        for relative in self.previous.difference(&current) {
            let path = self.path(relative)?;
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(GenerateError::at(&path, e)),
            }
            // Only remove now-empty ancestors of a tracked file. Other build
            // steps can keep files in the same directories.
            let mut parent = path.parent();
            while let Some(dir) = parent.filter(|dir| *dir != self.root) {
                match fs::remove_dir(dir) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => break,
                    Err(e) => return Err(GenerateError::at(dir, e)),
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
                Err(e) => return Err(GenerateError::at(&temporary, e)),
            };
            serde_json::to_writer_pretty(file, &*current)
                .map_err(|e| GenerateError::at(&temporary, e))?;
            fs::rename(&temporary, self.root.join(MANIFEST))
                .map_err(|e| GenerateError::at(self.root.join(MANIFEST), e))?;
            break;
        }
        Ok(())
    }
}

pub(crate) fn get_main_config(source: &str) -> Result<MainConfig, GenerateError> {
    let path = Path::new(source).join("main.json");
    read_json(&path).map_err(|e| GenerateError::at(&path, e))
}

pub(crate) fn get_menu_config(source: &str) -> Result<MenuConfig, GenerateError> {
    let path = Path::new(source).join("menu.json");
    read_json(&path).map_err(|e| GenerateError::at(&path, e))
}

pub(crate) fn get_image_list(source: &str) -> Result<Option<ImageList>, GenerateError> {
    let path = Path::new(source).join("images.json");
    read_json_or(&path, || None).map_err(|e| GenerateError::at(&path, e))
}

pub(crate) fn write_html(
    destination: &Output,
    path: &str,
    html: &str,
) -> Result<(), GenerateError> {
    let path = if path == "/" {
        String::from("/index.html")
    } else {
        format!("{}.html", path)
    };
    write_raw(destination, &path, minifier::html::minify(html))
}

pub(crate) fn write_raw<C: AsRef<[u8]>>(
    destination: &Output,
    path: &str,
    contents: C,
) -> Result<(), GenerateError> {
    let relative = path
        .strip_prefix('/')
        .ok_or_else(|| GenerateError::at(path, "output path must start with /"))?;
    let full_path = destination.path(relative)?;
    fs::create_dir_all(full_path.parent().unwrap())
        .map_err(|e| GenerateError::at(full_path.parent().unwrap(), e))?;
    fs::write(&full_path, contents).map_err(|e| GenerateError::at(&full_path, e))?;
    destination.record(path)
}

pub(crate) fn empty(destination: &Output, path: &str) -> Result<bool, GenerateError> {
    Ok(!destination.path(path.trim_start_matches('/'))?.exists())
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
pub(crate) fn get_page_directory_paths(
    source: &str,
    static_dir: Option<&str>,
) -> Result<Vec<String>, GenerateError> {
    let excluded = static_dir.and_then(|d| static_dir_path(source, d).canonicalize().ok());
    directories(source, excluded.as_deref())
}

fn directories(source: &str, excluded: Option<&Path>) -> Result<Vec<String>, GenerateError> {
    let root = Path::new(source);
    if !fs::metadata(root)
        .map_err(|e| GenerateError::at(root, e))?
        .is_dir()
    {
        return Err(GenerateError::at(root, "source must be a directory"));
    }
    WalkDir::new(source)
        .into_iter()
        .filter_entry(|e| {
            e.file_type().is_dir()
                && excluded.is_none_or(|x| e.path().canonicalize().ok().as_deref() != Some(x))
        })
        .map(|entry| {
            entry
                .map(|e| e.path().display().to_string())
                .map_err(|e| GenerateError::at(e.path().unwrap_or(Path::new(source)), &e))
        })
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
) -> Result<Vec<String>, GenerateError> {
    let root = static_dir_path(source, static_dir);
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut published = vec![];
    for entry in WalkDir::new(&root) {
        let entry = entry.map_err(|e| GenerateError::at(e.path().unwrap_or(&root), &e))?;
        if !entry.file_type().is_file() || entry.file_name() == GITKEEP {
            continue;
        }
        let path = entry.path();
        let href = format!("/{}", path.strip_prefix(&root).unwrap().to_string_lossy());
        let contents = fs::read(path).map_err(|e| GenerateError::at(path, e))?;
        write_raw(destination, &href, contents)?;
        published.push(href);
    }
    Ok(published)
}

pub(crate) fn get_all_directory_paths(source: &str) -> Result<Vec<String>, GenerateError> {
    directories(source, None)
}
