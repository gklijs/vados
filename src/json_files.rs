//! Reading and rewriting the JSON files a maintainer authors: main.json,
//! menu.json, page.json and images.json. `init` writes these fresh; the
//! content-authoring commands (`page new`, `image add`, `page add-image`,
//! `social add`/`update`/`remove`, `footer set`, `menu add-item`) read one
//! back, change the one thing they were asked to, and write the whole thing
//! out again. The literal mechanics of doing so are deliberately not domain
//! -- see `vados.allium`'s Excludes section -- so this is the one place they
//! live, instead of being repeated once per command.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Serializes `value` as pretty JSON and writes it to `path`, creating any
/// missing parent directories first.
pub(crate) fn write_json_pretty<T: Serialize>(path: PathBuf, value: &T) -> io::Result<()> {
    let contents = serde_json::to_string_pretty(value)
        .expect("a config value built by this crate is always representable as JSON");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

/// Reads and parses `path`, failing if it is missing or malformed. Used
/// where the file is expected to already exist -- main.json and menu.json are
/// written by `init` before any content-authoring command can run.
pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    let file = fs::File::open(path)?;
    serde_json::from_reader(file).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Like `read_json`, but a missing file falls back to `default()` instead of
/// failing -- for images.json, which `init` never creates and which is
/// legitimately absent from a directory that has no images yet.
pub(crate) fn read_json_or<T: DeserializeOwned>(
    path: &Path,
    default: impl FnOnce() -> T,
) -> io::Result<T> {
    match fs::File::open(path) {
        Ok(file) => {
            serde_json::from_reader(file).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(default()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    fn temp_path(name: &str) -> PathBuf {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("vados_json_files_test_{name}_{id}.json"))
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Sample {
        a: u32,
        b: String,
    }

    #[test]
    fn write_then_read_json_round_trips() {
        let path = temp_path("round_trip");
        let value = Sample {
            a: 1,
            b: "hi".to_string(),
        };

        write_json_pretty(path.clone(), &value).unwrap();
        let read_back: Sample = read_json(&path).unwrap();

        assert_eq!(read_back, value);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_json_fails_on_a_missing_file() {
        let path = temp_path("missing");
        let result: io::Result<Sample> = read_json(&path);
        assert!(result.is_err());
    }

    #[test]
    fn read_json_or_falls_back_to_the_default_on_a_missing_file() {
        let path = temp_path("missing_default");
        let value: Sample = read_json_or(&path, || Sample {
            a: 0,
            b: "default".to_string(),
        })
        .unwrap();
        assert_eq!(value.b, "default");
    }

    #[test]
    fn read_json_or_still_reads_a_present_file() {
        let path = temp_path("present_default");
        write_json_pretty(
            path.clone(),
            &Sample {
                a: 9,
                b: "real".to_string(),
            },
        )
        .unwrap();

        let value: Sample = read_json_or(&path, || Sample {
            a: 0,
            b: "default".to_string(),
        })
        .unwrap();

        assert_eq!(value.a, 9);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_json_reports_malformed_content_as_an_error_rather_than_panicking() {
        let path = temp_path("malformed");
        fs::write(&path, "{ not valid json").unwrap();

        let result: io::Result<Sample> = read_json(&path);

        assert!(result.is_err());
        let _ = fs::remove_file(&path);
    }
}
