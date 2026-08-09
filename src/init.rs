//! The `init` command: scaffolds a fresh, deployable vados project into the
//! maintainer's current directory. Unlike `generate`/`check`, which read an
//! existing source and image tree, `init` creates one; see vados.allium's
//! `InitializationRun`.
//!
//! This module holds everything testable without a terminal: conflict
//! detection and the actual scaffolding. The interactive prompt sequence
//! that gathers `ProjectBasics` lives in `main.rs`, the same way the spec
//! excludes the prompt sequence itself from the domain while keeping the
//! bundle of basics gathered as a first-class type here.

use crate::bulma::default_css_links;
// Re-exported so callers outside this crate (the CLI in `main.rs`) can name
// the provider type through `vados::init` without reaching into the
// otherwise-private `structure` module.
pub use crate::structure::RecognizedSocialProvider;
use serde::Serialize;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where `init` puts the source tree, the image tree and, per
/// `netlify.toml`, where a build publishes to. Named once here so the
/// scaffolded files and the conflict check always agree.
pub const SOURCE_DIR: &str = "root";
pub const IMAGE_DIR: &str = "imgroot";
pub const DESTINATION_DIR: &str = "public";

const DEFAULT_PRIMARY_COLOR: &str = "#00d1b2";
const DEFAULT_FOOTER_TEXT: &str = "Built with vados.";

fn default_home_intro_for(site_title: &str) -> String {
    format!("Welcome to {}.", site_title)
}

/// What `init` would create in the maintainer's current directory. Named so
/// a conflict can say which artifact is in the way, not just that something
/// is; see vados.allium's `ScaffoldArtifactKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaffoldArtifactKind {
    SiteConfig,
    MenuConfig,
    HomePage,
    ImageTree,
    StylesheetEntry,
    PackageManifest,
    NetlifyConfig,
    Readme,
}

impl ScaffoldArtifactKind {
    pub fn all() -> [ScaffoldArtifactKind; 8] {
        use ScaffoldArtifactKind::*;
        [
            SiteConfig,
            MenuConfig,
            HomePage,
            ImageTree,
            StylesheetEntry,
            PackageManifest,
            NetlifyConfig,
            Readme,
        ]
    }

    /// Where this artifact lives, relative to the maintainer's current
    /// directory.
    fn relative_path(self) -> String {
        match self {
            ScaffoldArtifactKind::SiteConfig => format!("{SOURCE_DIR}/main.json"),
            ScaffoldArtifactKind::MenuConfig => format!("{SOURCE_DIR}/menu.json"),
            ScaffoldArtifactKind::HomePage => format!("{SOURCE_DIR}/page.json"),
            ScaffoldArtifactKind::ImageTree => IMAGE_DIR.to_string(),
            ScaffoldArtifactKind::StylesheetEntry => String::from("sass/main.scss"),
            ScaffoldArtifactKind::PackageManifest => String::from("package.json"),
            ScaffoldArtifactKind::NetlifyConfig => String::from("netlify.toml"),
            ScaffoldArtifactKind::Readme => String::from("README.md"),
        }
    }
}

impl fmt::Display for ScaffoldArtifactKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ScaffoldArtifactKind::SiteConfig => "site configuration (main.json)",
            ScaffoldArtifactKind::MenuConfig => "menu configuration (menu.json)",
            ScaffoldArtifactKind::HomePage => "home page (page.json)",
            ScaffoldArtifactKind::ImageTree => "image tree root directory",
            ScaffoldArtifactKind::StylesheetEntry => "Sass stylesheet entry",
            ScaffoldArtifactKind::PackageManifest => "package manifest (package.json)",
            ScaffoldArtifactKind::NetlifyConfig => "Netlify build configuration (netlify.toml)",
            ScaffoldArtifactKind::Readme => "README",
        })
    }
}

/// An artifact `init` would create whose path was already occupied.
/// Recorded rather than raised, so every occupied path is found in one pass
/// instead of stopping at the first; see vados.allium's `ScaffoldConflict`.
#[derive(Debug)]
pub struct ScaffoldConflict {
    pub kind: ScaffoldArtifactKind,
    pub path: String,
}

/// Checks every artifact `init` would create in one pass, before the
/// maintainer is asked anything. See vados.allium's `DetectScaffoldConflicts`.
pub fn detect_conflicts(dir: &Path) -> Vec<ScaffoldConflict> {
    ScaffoldArtifactKind::all()
        .into_iter()
        .filter_map(|kind| {
            let path = kind.relative_path();
            dir.join(&path)
                .exists()
                .then_some(ScaffoldConflict { kind, path })
        })
        .collect()
}

/// One social handle the maintainer gave `init` for a recognised provider.
/// See vados.allium's `SocialHandle` value type.
#[derive(Debug, Clone)]
pub struct SocialHandle {
    pub provider: RecognizedSocialProvider,
    pub handle: String,
}

/// The bundle of basics gathered by `InitBasics`, all at once -- mirroring
/// how `SiteConfigurationDeclared` already bundles main.json's fields.
/// `site_title` is the one basic with no default; the rest are `None` when
/// the maintainer accepted the default rather than typing one.
#[derive(Debug, Clone)]
pub struct ProjectBasics {
    pub site_title: String,
    pub home_intro: Option<String>,
    pub primary_color: Option<String>,
    pub footer_text: Option<String>,
    pub socials: Vec<SocialHandle>,
}

/// What `init` created, once it has. See vados.allium's `InitOutcome`
/// surface.
#[derive(Debug)]
pub struct InitOutcome {
    pub site_title: String,
    pub home_intro: String,
    pub primary_color: String,
    pub footer_text: String,
    pub socials: Vec<SocialHandle>,
    pub repository_initialized: bool,
    pub gitignore_created: bool,
}

/// Scaffolds a fresh vados project into `dir`. Callers are expected to have
/// already run `detect_conflicts` and found it empty -- exactly like
/// `ScaffoldProject`'s `requires: run.status = init_requested`, reachable
/// only once `DetectScaffoldConflicts` found nothing occupied.
pub fn scaffold(dir: &Path, basics: ProjectBasics) -> io::Result<InitOutcome> {
    let home_intro = basics
        .home_intro
        .clone()
        .unwrap_or_else(|| default_home_intro_for(&basics.site_title));
    let primary_color = basics
        .primary_color
        .clone()
        .unwrap_or_else(|| DEFAULT_PRIMARY_COLOR.to_string());
    let footer_text = basics
        .footer_text
        .clone()
        .unwrap_or_else(|| DEFAULT_FOOTER_TEXT.to_string());

    write_site_config(dir, &basics.site_title, &footer_text)?;
    write_menu_config(dir, &basics.socials)?;
    write_home_page(dir, &basics.site_title, &home_intro)?;
    fs::create_dir_all(dir.join(IMAGE_DIR))?;
    write_stylesheet_entry(dir, &primary_color)?;
    write_package_manifest(dir, &basics.site_title)?;
    write_netlify_config(dir)?;
    write_readme(dir, &basics.site_title)?;

    let repository_initialized = initialize_repository(dir);
    let gitignore_created = write_or_merge_gitignore(dir)?;

    Ok(InitOutcome {
        site_title: basics.site_title,
        home_intro,
        primary_color,
        footer_text,
        socials: basics.socials,
        repository_initialized,
        gitignore_created,
    })
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn write_json_pretty<T: Serialize>(path: PathBuf, value: &T) -> io::Result<()> {
    let contents = serde_json::to_string_pretty(value)
        .expect("a scaffolded config is always representable as JSON");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MainJson {
    site_title: String,
    js_files: Vec<String>,
    include_default_js: Option<bool>,
    css_files: Vec<String>,
    include_default_css: Option<bool>,
    background_class: Option<String>,
    navbar_color: Option<String>,
    footer_content: String,
}

fn write_site_config(dir: &Path, site_title: &str, footer_text: &str) -> io::Result<()> {
    // The default MDI link, kept in sync with the same one `generate` uses
    // for a site that leaves `includeDefaultCss` at its own default. Bulma
    // itself is compiled locally instead, so `$primary` can be overridden.
    let mdi_link = default_css_links().remove(0);
    let json = MainJson {
        site_title: site_title.to_string(),
        js_files: vec![],
        include_default_js: None,
        css_files: vec![String::from("/css/main.min.css"), mdi_link],
        include_default_css: Some(false),
        background_class: None,
        navbar_color: None,
        footer_content: format!("<p>{}</p>", escape_html(footer_text)),
    };
    write_json_pretty(dir.join(SOURCE_DIR).join("main.json"), &json)
}

#[derive(Serialize)]
struct SocialEntryJson {
    url: String,
    icon: Option<String>,
    color: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MenuJson {
    main_menu: Vec<serde_json::Value>,
    socials: Vec<SocialEntryJson>,
}

fn write_menu_config(dir: &Path, socials: &[SocialHandle]) -> io::Result<()> {
    // No main-menu entries: the maintainer adds those by hand once there are
    // pages to link to. Every social handle is for a recognised provider, so
    // its icon and color are left for `generate`/`check` to fill in
    // canonically rather than repeated here.
    let socials_json = socials
        .iter()
        .map(|s| SocialEntryJson {
            url: s.provider.profile_url(s.handle.trim()),
            icon: None,
            color: None,
        })
        .collect();
    let json = MenuJson {
        main_menu: vec![],
        socials: socials_json,
    };
    write_json_pretty(dir.join(SOURCE_DIR).join("menu.json"), &json)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HomePageJson {
    title: String,
    sub_title: Option<String>,
    image: Option<String>,
    icon: Option<String>,
    summary: Option<String>,
    content: String,
    order: Option<u32>,
    left_notifications: Option<Vec<serde_json::Value>>,
    right_notifications: Option<Vec<serde_json::Value>>,
}

fn write_home_page(dir: &Path, site_title: &str, home_intro: &str) -> io::Result<()> {
    let json = HomePageJson {
        title: site_title.to_string(),
        sub_title: None,
        image: None,
        icon: None,
        summary: None,
        content: format!("<p>{}</p>", escape_html(home_intro)),
        order: None,
        left_notifications: None,
        right_notifications: None,
    };
    write_json_pretty(dir.join(SOURCE_DIR).join("page.json"), &json)
}

fn write_stylesheet_entry(dir: &Path, primary_color: &str) -> io::Result<()> {
    let contents = format!(
        "// Generated by `vados init`.\n\
         // Bulma variables must be set before Bulma itself is imported.\n\
         $primary: {primary_color};\n\
         \n\
         @import \"bulma/bulma\";\n"
    );
    let path = dir.join("sass").join("main.scss");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

/// Turns a site title into a lowercase, hyphenated `package.json` name.
fn slugify(title: &str) -> String {
    let mut out = String::new();
    for c in title.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        String::from("vados-site")
    } else {
        out
    }
}

fn write_package_manifest(dir: &Path, site_title: &str) -> io::Result<()> {
    let name = slugify(site_title);
    let contents = format!(
        "{{\n\
         \x20\x20\"name\": \"{name}\",\n\
         \x20\x20\"private\": true,\n\
         \x20\x20\"version\": \"1.0.0\",\n\
         \x20\x20\"scripts\": {{\n\
         \x20\x20\x20\x20\"build\": \"sass sass/main.scss:{DESTINATION_DIR}/css/main.min.css --style=compressed --no-source-map\"\n\
         \x20\x20}},\n\
         \x20\x20\"dependencies\": {{\n\
         \x20\x20\x20\x20\"bulma\": \"^0.9.4\"\n\
         \x20\x20}},\n\
         \x20\x20\"devDependencies\": {{\n\
         \x20\x20\x20\x20\"sass\": \"^1.77.6\"\n\
         \x20\x20}}\n\
         }}\n"
    );
    fs::write(dir.join("package.json"), contents)
}

fn write_netlify_config(dir: &Path) -> io::Result<()> {
    // Rust and `vados` are installed fresh on every build rather than
    // pinned, so deploys pick up vados fixes automatically; see
    // vados.allium's `AlwaysInstallsCurrentVados` guarantee.
    let contents = format!(
        "[build]\n\
         \x20\x20publish = \"{DESTINATION_DIR}\"\n\
         \x20\x20command = \"curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable && source \\\"$HOME/.cargo/env\\\" && cargo install vados && npm install && npm run build && vados generate --source {SOURCE_DIR} --img-source {IMAGE_DIR} --destination {DESTINATION_DIR}\"\n"
    );
    fs::write(dir.join("netlify.toml"), contents)
}

fn write_readme(dir: &Path, site_title: &str) -> io::Result<()> {
    let contents = format!(
        "# {site_title}\n\
         \n\
         A static site built with [vados](https://github.com/gklijs/vados), scaffolded by `vados init`.\n\
         \n\
         ## Content\n\
         \n\
         - `{SOURCE_DIR}/` -- the source tree: `main.json`, `menu.json`, and one `page.json` per page directory.\n\
         - `{IMAGE_DIR}/` -- the image tree: one `images.json` per directory of images to publish.\n\
         - `sass/main.scss` -- overrides Bulma's `$primary` before importing it; rebuilt by `npm run build` into `{DESTINATION_DIR}/css/main.min.css`.\n\
         \n\
         ## Local development\n\
         \n\
         ```\n\
         cargo install vados\n\
         npm install\n\
         npm run build\n\
         vados check --source {SOURCE_DIR} --img-source {IMAGE_DIR}\n\
         vados generate --source {SOURCE_DIR} --img-source {IMAGE_DIR} --destination {DESTINATION_DIR}\n\
         ```\n\
         \n\
         ## Deployment\n\
         \n\
         This project deploys to [Netlify](https://www.netlify.com/) with no further setup: connect the repository and `netlify.toml` takes care of installing Rust, `vados`, and the Sass build.\n"
    );
    fs::write(dir.join("README.md"), contents)
}

/// Whether `dir` is already inside a git work tree -- its own repository or
/// one belonging to an ancestor directory.
fn git_repository_present(dir: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Initializes a git repository in `dir` if one isn't already present.
/// Returns whether it did. A `git` invocation that fails or a missing `git`
/// binary is reported but does not fail the whole scaffold -- version
/// control is a courtesy here, not a requirement.
fn initialize_repository(dir: &Path) -> bool {
    if git_repository_present(dir) {
        return false;
    }
    match Command::new("git").arg("init").current_dir(dir).output() {
        Ok(output) if output.status.success() => true,
        Ok(output) => {
            eprintln!(
                "warning: `git init` did not succeed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
            false
        }
        Err(e) => {
            eprintln!("warning: could not run `git init` ({e}); skipping version control setup.");
            false
        }
    }
}

fn default_gitignore_contents() -> String {
    format!("node_modules/\n{DESTINATION_DIR}/\n.netlify/\n")
}

/// Creates `.gitignore` fresh if absent, or merges in whichever of our
/// default entries aren't already there. Returns whether it was created
/// fresh (`false` means it existed and was merged into, whether or not that
/// merge actually added anything).
fn write_or_merge_gitignore(dir: &Path) -> io::Result<bool> {
    let path = dir.join(".gitignore");
    if !path.exists() {
        fs::write(&path, default_gitignore_contents())?;
        return Ok(true);
    }

    let existing = fs::read_to_string(&path)?;
    let existing_lines: HashSet<&str> = existing.lines().collect();
    let defaults = default_gitignore_contents();
    let missing: Vec<&str> = defaults
        .lines()
        .filter(|line| !existing_lines.contains(line))
        .collect();

    if !missing.is_empty() {
        let mut merged = existing;
        if !merged.is_empty() && !merged.ends_with('\n') {
            merged.push('\n');
        }
        merged.push_str("# Added by `vados init`\n");
        for line in missing {
            merged.push_str(line);
            merged.push('\n');
        }
        fs::write(&path, merged)?;
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    /// A disposable directory for one test, cleaned up on drop.
    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!("vados_init_test_{name}_{id}"));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            TestDir { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn basics(site_title: &str) -> ProjectBasics {
        ProjectBasics {
            site_title: site_title.to_string(),
            home_intro: None,
            primary_color: None,
            footer_text: None,
            socials: vec![],
        }
    }

    #[test]
    fn empty_directory_has_no_conflicts() {
        let dir = TestDir::new("no_conflicts");
        assert!(detect_conflicts(&dir.path).is_empty());
    }

    #[test]
    fn every_scaffold_artifact_is_detected_as_a_conflict_in_one_pass() {
        let dir = TestDir::new("all_conflicts");
        fs::create_dir_all(dir.path.join(SOURCE_DIR)).unwrap();
        fs::write(dir.path.join(SOURCE_DIR).join("main.json"), "{}").unwrap();
        fs::write(dir.path.join(SOURCE_DIR).join("menu.json"), "{}").unwrap();
        fs::write(dir.path.join(SOURCE_DIR).join("page.json"), "{}").unwrap();
        fs::create_dir_all(dir.path.join(IMAGE_DIR)).unwrap();
        fs::create_dir_all(dir.path.join("sass")).unwrap();
        fs::write(dir.path.join("sass/main.scss"), "").unwrap();
        fs::write(dir.path.join("package.json"), "{}").unwrap();
        fs::write(dir.path.join("netlify.toml"), "").unwrap();
        fs::write(dir.path.join("README.md"), "").unwrap();

        let conflicts = detect_conflicts(&dir.path);

        assert_eq!(conflicts.len(), 8, "expected every kind to conflict, got {:?}", conflicts);
    }

    #[test]
    fn a_single_occupied_path_is_reported_without_hiding_the_rest() {
        let dir = TestDir::new("one_conflict");
        fs::write(dir.path.join("README.md"), "existing readme").unwrap();

        let conflicts = detect_conflicts(&dir.path);

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, ScaffoldArtifactKind::Readme);
        assert_eq!(conflicts[0].path, "README.md");
    }

    #[test]
    fn scaffold_writes_every_artifact() {
        let dir = TestDir::new("scaffold_writes_everything");

        let outcome = scaffold(&dir.path, basics("My Site")).unwrap();

        assert!(dir.path.join(SOURCE_DIR).join("main.json").exists());
        assert!(dir.path.join(SOURCE_DIR).join("menu.json").exists());
        assert!(dir.path.join(SOURCE_DIR).join("page.json").exists());
        assert!(dir.path.join(IMAGE_DIR).is_dir());
        assert!(dir.path.join("sass/main.scss").exists());
        assert!(dir.path.join("package.json").exists());
        assert!(dir.path.join("netlify.toml").exists());
        assert!(dir.path.join("README.md").exists());
        assert!(detect_conflicts(&dir.path).len() == 8);

        assert_eq!(outcome.site_title, "My Site");
        assert_eq!(outcome.home_intro, "Welcome to My Site.");
        assert_eq!(outcome.primary_color, DEFAULT_PRIMARY_COLOR);
        assert_eq!(outcome.footer_text, DEFAULT_FOOTER_TEXT);
        assert!(outcome.socials.is_empty());
    }

    #[test]
    fn given_basics_override_the_defaults() {
        let dir = TestDir::new("given_basics");
        let mut b = basics("Bass Log");
        b.home_intro = Some("Hi there".to_string());
        b.primary_color = Some("#ff00aa".to_string());
        b.footer_text = Some("(c) me".to_string());
        b.socials = vec![SocialHandle {
            provider: RecognizedSocialProvider::Github,
            handle: "gklijs".to_string(),
        }];

        let outcome = scaffold(&dir.path, b).unwrap();

        assert_eq!(outcome.home_intro, "Hi there");
        assert_eq!(outcome.primary_color, "#ff00aa");
        assert_eq!(outcome.footer_text, "(c) me");
        assert_eq!(outcome.socials.len(), 1);

        let main_json =
            fs::read_to_string(dir.path.join(SOURCE_DIR).join("main.json")).unwrap();
        assert!(main_json.contains("(c) me"));

        let menu_json =
            fs::read_to_string(dir.path.join(SOURCE_DIR).join("menu.json")).unwrap();
        assert!(menu_json.contains("https://github.com/gklijs"));

        let stylesheet = fs::read_to_string(dir.path.join("sass/main.scss")).unwrap();
        assert!(stylesheet.contains("#ff00aa"));
    }

    #[test]
    fn menu_json_starts_with_no_main_menu_entries() {
        let dir = TestDir::new("empty_main_menu");
        scaffold(&dir.path, basics("Site")).unwrap();

        let menu_json: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(dir.path.join(SOURCE_DIR).join("menu.json")).unwrap(),
        )
        .unwrap();

        assert_eq!(menu_json["mainMenu"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn gitignore_is_created_fresh_when_absent() {
        let dir = TestDir::new("gitignore_fresh");
        let outcome = scaffold(&dir.path, basics("Site")).unwrap();

        assert!(outcome.gitignore_created);
        let contents = fs::read_to_string(dir.path.join(".gitignore")).unwrap();
        assert!(contents.contains("node_modules/"));
        assert!(contents.contains(&format!("{DESTINATION_DIR}/")));
    }

    #[test]
    fn gitignore_is_merged_rather_than_overwritten_when_present() {
        let dir = TestDir::new("gitignore_merge");
        fs::write(dir.path.join(".gitignore"), "my-existing-entry/\n").unwrap();

        let outcome = scaffold(&dir.path, basics("Site")).unwrap();

        assert!(!outcome.gitignore_created);
        let contents = fs::read_to_string(dir.path.join(".gitignore")).unwrap();
        assert!(contents.contains("my-existing-entry/"));
        assert!(contents.contains("node_modules/"));
    }

    #[test]
    fn gitignore_merge_does_not_duplicate_entries_already_present() {
        let dir = TestDir::new("gitignore_no_dupe");
        fs::write(dir.path.join(".gitignore"), "node_modules/\n").unwrap();

        scaffold(&dir.path, basics("Site")).unwrap();

        let contents = fs::read_to_string(dir.path.join(".gitignore")).unwrap();
        assert_eq!(contents.matches("node_modules/").count(), 1);
    }

    #[test]
    fn repository_is_initialized_only_when_absent() {
        if Command::new("git").arg("--version").output().is_err() {
            eprintln!("skipping: git binary not available");
            return;
        }
        let dir = TestDir::new("git_repo_fresh");
        let outcome = scaffold(&dir.path, basics("Site")).unwrap();
        assert!(outcome.repository_initialized);
        assert!(dir.path.join(".git").is_dir());
    }

    #[test]
    fn slugify_produces_a_lowercase_hyphenated_name() {
        assert_eq!(slugify("Bass - gklijs!"), "bass-gklijs");
        assert_eq!(slugify(""), "vados-site");
        assert_eq!(slugify("###"), "vados-site");
    }
}
