//! `image add`: registers one entry in an images.json under the image tree,
//! creating the file if the directory has none yet. Also the mechanics
//! `page add-image` reuses when attaching a freshly registered image rather
//! than one already declared -- see `vados.allium`'s `ImageRegistrationRun`
//! and the two `ImageRegistrationBlockReason` variants it shares with
//! `PageImageAttachmentBlockReason`.

use crate::config_files::{ImageList, ImageReference};
use crate::files::get_all_directory_paths;
use crate::image::reference_key_for;
use crate::json_files::{read_json_or, write_json_pretty};
use std::collections::HashSet;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// Why an `image add` (or the image-registering form of `page add-image`)
/// request couldn't proceed. See `vados.allium`'s
/// `ImageRegistrationBlockReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageRegistrationBlockReason {
    SourceFileUnreadable,
    ReferenceKeyAlreadyUsed,
}

impl fmt::Display for ImageRegistrationBlockReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ImageRegistrationBlockReason::SourceFileUnreadable => {
                "image source file could not be read (or its name has no extension)"
            }
            ImageRegistrationBlockReason::ReferenceKeyAlreadyUsed => {
                "an image is already declared with this reference key"
            }
        })
    }
}

/// One reason an image-registration request couldn't proceed. Recorded
/// rather than raised, so every problem is found in one pass; see
/// `vados.allium`'s `ImageRegistrationBlock`.
#[derive(Debug)]
pub struct ImageRegistrationBlock {
    pub reason: ImageRegistrationBlockReason,
    pub detail: String,
}

/// Normalizes a maintainer-given directory (e.g. `""`, `"team"`, `"/team/"`)
/// into the `path_start` form the rest of this crate already uses: empty for
/// the image root, or a single leading slash with no trailing one otherwise.
fn normalize_dir(given_dir: &str) -> String {
    let trimmed = given_dir.trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("/{trimmed}")
    }
}

/// Strips a trailing slash from a maintainer-given `--img-source`. Every
/// function below either concatenates this onto `given_dir` or slices a
/// `WalkDir` entry at `image_source.len()`; both assume `image_source` has
/// no trailing separator. A shell that tab-completes a directory naturally
/// produces one (`imgroot/`), which would otherwise shift every subdirectory
/// key by one character -- `"team/photo"` instead of `"/team/photo"` -- and
/// make otherwise-valid image references look unresolved.
fn normalize_image_source(image_source: &str) -> &str {
    image_source.trim_end_matches('/')
}

/// Where the actual image bytes for a registration request live on disk.
pub fn image_source_path(image_source: &str, given_dir: &str, file_name: &str) -> PathBuf {
    let image_source = normalize_image_source(image_source);
    let path_start = normalize_dir(given_dir);
    if path_start.is_empty() {
        Path::new(image_source).join(file_name)
    } else {
        Path::new(&format!("{image_source}{path_start}")).join(file_name)
    }
}

fn images_json_path(image_source: &str, given_dir: &str) -> PathBuf {
    let image_source = normalize_image_source(image_source);
    let path_start = normalize_dir(given_dir);
    if path_start.is_empty() {
        Path::new(image_source).join("images.json")
    } else {
        Path::new(&format!("{image_source}{path_start}")).join("images.json")
    }
}

/// How this image would be looked up from a page or notification, once
/// registered. `None` when `file_name` has no extension -- `reference_key_for`
/// can't derive a key without one, and a source file with no extension can
/// never actually be read as an image either.
pub fn reference_key_for_registration(given_dir: &str, file_name: &str) -> Option<String> {
    reference_key_for(&normalize_dir(given_dir), file_name)
}

/// Whether a file exists, can be opened as an ordinary file, and has a name
/// `reference_key_for` can actually derive a key from. Used for
/// `source_readable` in `vados.allium`; deliberately shallow otherwise -- it
/// does not decode the image, the same way `check`'s `MissingImageSourceFile`
/// finding only asks whether the file is there.
///
/// The extension check matters: without it, a present-but-extensionless file
/// passes this check, `reference_key_for_registration` then returns `None`
/// for it, and the `ReferenceKeyAlreadyUsed` check in
/// `detect_registration_blockers` (guarded on that same `Some`) silently
/// never fires -- so the request looks unblocked right up until
/// `register_image` panics on the very same `None`, after already having
/// written the malformed entry to disk. Folding the extension requirement in
/// here means a missing extension is reported as `SourceFileUnreadable`
/// (whose own message already says "or its name has no extension") before
/// anything is written, the same as any other unreadable source.
pub fn source_readable(path: &Path) -> bool {
    let has_extension = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.contains('.'));
    path.is_file() && has_extension
}

/// Every reference key already declared anywhere in the image tree. Backs
/// both `image_reference_key_used` (registering a new image must not repeat
/// one) and `image_reference_resolves` (attaching an existing image must name
/// one that's really there).
pub fn collect_declared_image_keys(image_source: &str) -> HashSet<String> {
    let image_source = normalize_image_source(image_source);
    let mut keys = HashSet::new();
    for dir in get_all_directory_paths(image_source) {
        let images_json = Path::new(&dir).join("images.json");
        let list: ImageList = match read_json_or(&images_json, || ImageList {
            title: None,
            list: vec![],
        }) {
            Ok(l) => l,
            Err(_) => continue, // unreadable images.json elsewhere: not this run's problem to fix
        };
        let path_start = &dir[image_source.len()..];
        for reference in list.list {
            if let Some(key) = reference_key_for(path_start, &reference.file_name) {
                keys.insert(key);
            }
        }
    }
    keys
}

/// The alternative text a declared image carries, found by its reference
/// key. Backs `declared_image_alt_text`, used when `page add-image` attaches
/// an already-declared image: its caption defaults to that image's own
/// alt text rather than asking the maintainer to repeat it.
pub fn declared_image_alt_text(image_source: &str, reference_key: &str) -> Option<String> {
    let image_source = normalize_image_source(image_source);
    for dir in get_all_directory_paths(image_source) {
        let images_json = Path::new(&dir).join("images.json");
        let list: ImageList = match read_json_or(&images_json, || ImageList {
            title: None,
            list: vec![],
        }) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let path_start = &dir[image_source.len()..];
        for reference in list.list {
            if reference_key_for(path_start, &reference.file_name).as_deref() == Some(reference_key)
            {
                return Some(reference.alt_text);
            }
        }
    }
    None
}

/// Whether a reference key names an image actually declared somewhere in the
/// tree. Backs `image_reference_resolves`.
pub fn image_reference_resolves(image_source: &str, reference_key: &str) -> bool {
    collect_declared_image_keys(image_source).contains(reference_key)
}

/// The title an image falls back to when the maintainer didn't give one: the
/// file name without its extension. Mirrors `ProcessedImage`'s own fallback
/// in `src/image.rs`.
pub fn base_name(file_name: &str) -> String {
    file_name
        .rsplit_once('.')
        .map(|(base, _)| base.to_string())
        .unwrap_or_else(|| file_name.to_string())
}

/// Checks every way an `image add` request could fail in one pass, before
/// the maintainer is asked for a title or alt text. See `vados.allium`'s
/// `DetectImageRegistrationBlockers`.
pub fn detect_registration_blockers(
    image_source: &str,
    given_dir: &str,
    file_name: &str,
) -> Vec<ImageRegistrationBlock> {
    let mut blocks = Vec::new();

    let source_path = image_source_path(image_source, given_dir, file_name);
    if !source_readable(&source_path) {
        blocks.push(ImageRegistrationBlock {
            reason: ImageRegistrationBlockReason::SourceFileUnreadable,
            detail: file_name.to_string(),
        });
    }

    if let Some(key) = reference_key_for_registration(given_dir, file_name) {
        if collect_declared_image_keys(image_source).contains(&key) {
            blocks.push(ImageRegistrationBlock {
                reason: ImageRegistrationBlockReason::ReferenceKeyAlreadyUsed,
                detail: key,
            });
        }
    }

    blocks
}

/// What `image add` registered, once it has -- including the reference key a
/// page or notification would use to point at it. See `vados.allium`'s
/// `ImageRegisteredOutcome`.
#[derive(Debug, Clone)]
pub struct ImageRegistrationOutcome {
    pub reference_key: String,
    pub title: String,
    pub alt_text: String,
}

/// Registers one image. Callers are expected to have already run
/// `detect_registration_blockers` and found it empty -- exactly like
/// `ProvideImageRegistrationDetails`'s `requires: run.status =
/// image_registration_requested`, reachable only once no blocker fired.
pub fn register_image(
    image_source: &str,
    given_dir: &str,
    file_name: &str,
    title: Option<String>,
    alt_text: String,
) -> io::Result<ImageRegistrationOutcome> {
    let path = images_json_path(image_source, given_dir);
    let mut list: ImageList = read_json_or(&path, || ImageList {
        title: None,
        list: vec![],
    })?;
    list.list.push(ImageReference {
        title: title.clone(),
        file_name: file_name.to_string(),
        alt_text: alt_text.clone(),
    });
    write_json_pretty(path, &list)?;

    let reference_key = reference_key_for_registration(given_dir, file_name)
        .expect("a blocker would already have fired for a file name with no extension");
    let title = title.unwrap_or_else(|| base_name(file_name));
    Ok(ImageRegistrationOutcome {
        reference_key,
        title,
        alt_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    struct TestTree {
        path: PathBuf,
    }

    impl TestTree {
        fn new(name: &str) -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!("vados_image_registry_test_{name}_{id}"));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            TestTree { path }
        }

        fn write_image(&self, rel_path: &str, contents: &str) {
            let path = self.path.join(rel_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        fn write_images_json(&self, dir: &str, contents: &str) {
            let path = if dir.is_empty() {
                self.path.join("images.json")
            } else {
                self.path.join(dir).join("images.json")
            };
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        fn root(&self) -> &str {
            self.path.to_str().unwrap()
        }
    }

    impl Drop for TestTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn a_readable_unclaimed_source_has_no_blockers() {
        let tree = TestTree::new("no_blockers");
        tree.write_image("photo.jpg", "bytes");

        let blocks = detect_registration_blockers(tree.root(), "", "photo.jpg");

        assert!(blocks.is_empty());
    }

    #[test]
    fn an_unreadable_source_file_is_blocked() {
        let tree = TestTree::new("unreadable");

        let blocks = detect_registration_blockers(tree.root(), "", "ghost.jpg");

        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].reason,
            ImageRegistrationBlockReason::SourceFileUnreadable
        );
    }

    #[test]
    fn a_present_file_with_no_extension_is_blocked_rather_than_panicking() {
        // A regression test: reference_key_for_registration can't derive a
        // key without an extension, so this used to sail through the
        // blocker check (source_readable only asked whether the file
        // existed) and panic inside register_image after already writing a
        // malformed entry to images.json.
        let tree = TestTree::new("no_extension");
        tree.write_image("photo", "bytes");

        let blocks = detect_registration_blockers(tree.root(), "", "photo");

        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].reason,
            ImageRegistrationBlockReason::SourceFileUnreadable
        );
        assert!(!tree.path.join("images.json").exists());
    }

    #[test]
    fn a_reused_reference_key_is_blocked() {
        let tree = TestTree::new("reused_key");
        tree.write_image("photo.jpg", "bytes");
        tree.write_images_json(
            "",
            r#"{"title":null,"list":[{"title":null,"fileName":"photo.png","altText":"already here"}]}"#,
        );

        let blocks = detect_registration_blockers(tree.root(), "", "photo.jpg");

        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].reason,
            ImageRegistrationBlockReason::ReferenceKeyAlreadyUsed
        );
        assert_eq!(blocks[0].detail, "/photo");
    }

    #[test]
    fn both_blockers_are_found_in_one_pass() {
        let tree = TestTree::new("both_blockers");
        tree.write_images_json(
            "",
            r#"{"title":null,"list":[{"title":null,"fileName":"ghost.png","altText":"a"}]}"#,
        );
        // Deliberately not writing the ghost.jpg source file.

        let blocks = detect_registration_blockers(tree.root(), "", "ghost.jpg");

        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn register_image_creates_images_json_when_absent() {
        let tree = TestTree::new("create_fresh");
        tree.write_image("photo.jpg", "bytes");

        let outcome =
            register_image(tree.root(), "", "photo.jpg", None, "a photo".to_string()).unwrap();

        assert_eq!(outcome.reference_key, "/photo");
        assert_eq!(outcome.title, "photo");
        assert_eq!(outcome.alt_text, "a photo");
        assert!(tree.path.join("images.json").exists());
    }

    #[test]
    fn register_image_appends_to_an_existing_images_json_without_disturbing_it() {
        let tree = TestTree::new("append");
        tree.write_image("first.jpg", "bytes");
        tree.write_image("second.jpg", "bytes");
        tree.write_images_json(
            "",
            r#"{"title":"Gallery","list":[{"title":null,"fileName":"first.jpg","altText":"first"}]}"#,
        );

        register_image(
            tree.root(),
            "",
            "second.jpg",
            Some("Second".to_string()),
            "second".to_string(),
        )
        .unwrap();

        let list: ImageList = read_json_or(&tree.path.join("images.json"), || ImageList {
            title: None,
            list: vec![],
        })
        .unwrap();
        assert_eq!(list.title, Some("Gallery".to_string()));
        assert_eq!(list.list.len(), 2);
        assert_eq!(list.list[0].file_name, "first.jpg");
        assert_eq!(list.list[1].file_name, "second.jpg");
        assert_eq!(list.list[1].title, Some("Second".to_string()));
    }

    #[test]
    fn register_image_under_a_subdirectory_uses_a_nested_reference_key() {
        let tree = TestTree::new("nested");
        tree.write_image("team/alice.jpg", "bytes");

        let outcome =
            register_image(tree.root(), "team", "alice.jpg", None, "alice".to_string()).unwrap();

        assert_eq!(outcome.reference_key, "/team/alice");
        assert!(tree.path.join("team/images.json").exists());
    }

    #[test]
    fn collect_declared_image_keys_walks_every_directory() {
        let tree = TestTree::new("collect_keys");
        tree.write_images_json(
            "",
            r#"{"title":null,"list":[{"title":null,"fileName":"a.jpg","altText":"a"}]}"#,
        );
        tree.write_images_json(
            "team",
            r#"{"title":null,"list":[{"title":null,"fileName":"b.jpg","altText":"b"}]}"#,
        );

        let keys = collect_declared_image_keys(tree.root());

        assert!(keys.contains("/a"));
        assert!(keys.contains("/team/b"));
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn a_trailing_slash_on_img_source_does_not_shift_subdirectory_keys() {
        // A shell that tab-completes a directory naturally appends a
        // trailing slash. Regression test: this used to shift every
        // subdirectory key by one character ("team/b" instead of "/team/b"),
        // making an image that plainly exists look unresolved.
        let tree = TestTree::new("trailing_slash");
        tree.write_images_json(
            "team",
            r#"{"title":null,"list":[{"title":null,"fileName":"b.jpg","altText":"b"}]}"#,
        );
        let with_trailing_slash = format!("{}/", tree.root());

        let keys = collect_declared_image_keys(&with_trailing_slash);

        assert!(
            keys.contains("/team/b"),
            "expected /team/b among {:?}",
            keys
        );
    }

    #[test]
    fn declared_image_alt_text_finds_a_match_by_reference_key() {
        let tree = TestTree::new("alt_text_lookup");
        tree.write_images_json(
            "team",
            r#"{"title":null,"list":[{"title":null,"fileName":"alice.jpg","altText":"Alice smiling"}]}"#,
        );

        assert_eq!(
            declared_image_alt_text(tree.root(), "/team/alice"),
            Some("Alice smiling".to_string())
        );
        assert_eq!(declared_image_alt_text(tree.root(), "/nope"), None);
    }

    #[test]
    fn declared_image_alt_text_also_tolerates_a_trailing_slash_on_img_source() {
        let tree = TestTree::new("alt_text_trailing_slash");
        tree.write_images_json(
            "team",
            r#"{"title":null,"list":[{"title":null,"fileName":"alice.jpg","altText":"Alice smiling"}]}"#,
        );
        let with_trailing_slash = format!("{}/", tree.root());

        assert_eq!(
            declared_image_alt_text(&with_trailing_slash, "/team/alice"),
            Some("Alice smiling".to_string())
        );
    }

    #[test]
    fn base_name_strips_the_extension() {
        assert_eq!(base_name("photo.jpg"), "photo");
        assert_eq!(base_name("no-extension"), "no-extension");
    }
}
