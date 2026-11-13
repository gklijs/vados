use std::path::{Path, PathBuf};

/// Whether a stylesheet link names Bulma before 1.0, as a CDN link carries
/// it (`bulma@0.9.4`, `bulma@~0.9`, `bulma/0.9.4`). Only the version right
/// after the name counts. See vados.allium's `outdated_bulma_stylesheet`.
pub(super) fn is_outdated_bulma_link(link: &str) -> bool {
    let link = link.to_ascii_lowercase();
    let version = ["bulma@", "bulma/"]
        .iter()
        .find_map(|prefix| link.split(prefix).nth(1));
    version.and_then(leading_version_major) == Some(0)
}

/// The `package.json` next to the source tree, when its Bulma dependency
/// asks for a version before 1.0, with the manifest's location and the
/// version spec. The manifest is in the source directory's parent, or the
/// working directory when that has none. See vados.allium's
/// `outdated_bulma_dependency`.
pub(super) fn outdated_bulma_dependency(source: &str) -> Option<(String, String)> {
    let path = project_dir(source).join("package.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
    for section in ["dependencies", "devDependencies"] {
        let Some(spec) = manifest
            .get(section)
            .and_then(|dependencies| dependencies.get("bulma"))
            .and_then(|version| version.as_str())
        else {
            continue;
        };
        if spec_is_outdated(spec) {
            return Some((path.display().to_string(), spec.to_string()));
        }
    }
    None
}

/// The directory the site's build files (`package.json`, `sass/`) are in:
/// the source directory's parent, or the working directory when that has
/// none.
pub(super) fn project_dir(source: &str) -> PathBuf {
    Path::new(source)
        .canonicalize()
        .ok()
        .and_then(|source| source.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}

/// The first line of the project's `sass/main.scss` that loads Bulma with
/// `@import`, with the file's location. See vados.allium's
/// `bulma_sass_import`.
pub(super) fn bulma_sass_import(source: &str) -> Option<(String, String)> {
    let path = project_dir(source).join("sass").join("main.scss");
    let line = std::fs::read_to_string(&path)
        .ok()?
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("@import") && line.to_ascii_lowercase().contains("bulma"))?
        .to_string();
    Some((path.display().to_string(), line))
}

/// Whether a dependency spec only allows versions before 1.0: every
/// alternative (`a || b`) must start below 1.0, so a range that also allows
/// 1.x is not outdated. Specs without a number (`latest`, `*`) are unknown.
pub(super) fn spec_is_outdated(spec: &str) -> bool {
    let majors: Vec<Option<u32>> = spec
        .split("||")
        .map(|alternative| {
            let start = alternative.find(|c: char| c.is_ascii_digit())?;
            leading_version_major(&alternative[start..])
        })
        .collect();
    !majors.is_empty() && majors.iter().all(|major| *major == Some(0))
}

/// The major of a version at the start of `text` (`0.9.4/css`, `~0.9`, `1`),
/// after any range operator; `None` when it doesn't start with a number.
pub(super) fn leading_version_major(text: &str) -> Option<u32> {
    let text = text.trim_start_matches(['^', '~', '=', 'v', ' ']);
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::test_support::*;
    use crate::check::{FindingKind, Severity};

    #[test]
    fn a_bulma_0_stylesheet_is_an_error() {
        assert!(is_outdated_bulma_link(
            "https://cdn.jsdelivr.net/npm/bulma@0.9.4/css/bulma.min.css"
        ));
        assert!(is_outdated_bulma_link(
            "https://cdnjs.cloudflare.com/ajax/libs/bulma/0.9.3/css/bulma.min.css"
        ));
        assert!(is_outdated_bulma_link(
            "https://cdn.jsdelivr.net/npm/bulma@~0.9.4/css/bulma.min.css"
        ));
        assert!(is_outdated_bulma_link(
            "https://cdn.jsdelivr.net/npm/bulma@0/css/bulma.min.css"
        ));
        assert!(!is_outdated_bulma_link(
            "https://cdn.jsdelivr.net/npm/bulma@1.0.4/css/bulma.min.css"
        ));
        assert!(!is_outdated_bulma_link(
            "https://cdn.jsdelivr.net/npm/bulma@latest/css/bulma.min.css"
        ));
        assert!(!is_outdated_bulma_link("/css/main.min.css"));
        // Only the version right after the name counts.
        assert!(!is_outdated_bulma_link("/css/bulma/theme.css?v=0"));
        assert!(!is_outdated_bulma_link("/css/bulma/bulma.min.css"));
        assert_eq!(
            FindingKind::OutdatedBulmaStylesheet.severity(),
            Severity::Error
        );
    }

    #[test]
    fn a_dependency_spec_is_outdated_only_when_it_cannot_reach_bulma_1() {
        for outdated in ["^0.9.4", "~0.9", "0.x", "=0.9.4", "v0.9.4", "^0.8 || ^0.9"] {
            assert!(spec_is_outdated(outdated), "{outdated}");
        }
        for fine in ["^1.0.4", "1", ">=1.0", "^0.9 || ^1.0", "latest", "*", ""] {
            assert!(!spec_is_outdated(fine), "{fine}");
        }
    }

    #[test]
    fn the_manifest_is_found_from_a_source_directory_given_as_a_relative_path() {
        let site = TestSite::new("bulma_0_package_relative");
        site.write_project("package.json", r#"{"dependencies":{"bulma":"^0.9.4"}}"#);
        let source = site.source.join("..").join("source");
        let found = outdated_bulma_dependency(source.to_str().unwrap());
        assert_eq!(found.map(|(_, spec)| spec).as_deref(), Some("^0.9.4"));
    }

    #[test]
    fn a_bulma_0_stylesheet_in_main_json_is_an_error() {
        let site = TestSite::new("bulma_0_stylesheet");
        let main_json = VALID_MAIN_JSON.replace(
            r#""cssFiles": []"#,
            r#""cssFiles": ["https://cdn.jsdelivr.net/npm/bulma@0.9.4/css/bulma.min.css"]"#,
        );
        site.write_source("main.json", &main_json);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::OutdatedBulmaStylesheet), 1);
        assert!(!report.passed());
    }

    #[test]
    fn a_bulma_0_package_dependency_is_an_error() {
        let site = TestSite::new("bulma_0_package");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_project(
            "package.json",
            r#"{"devDependencies":{"sass":"^1.77.6","bulma":"^0.9.4"}}"#,
        );

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::OutdatedBulmaDependency), 1);
        assert!(has_kind_at(
            &report,
            FindingKind::OutdatedBulmaDependency,
            site.root.join("package.json").to_str().unwrap()
        ));
        assert!(!report.passed());
    }

    #[test]
    fn an_import_of_bulma_in_main_scss_is_a_warning() {
        let site = TestSite::new("bulma_sass_import");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_project(
            "sass/main.scss",
            "$primary: #ff00aa;\n\n  @import \"bulma/bulma\";\n",
        );

        let report = site.check();

        assert_eq!(count_kind(&report, FindingKind::BulmaSassImport), 1);
        assert!(has_kind_at(
            &report,
            FindingKind::BulmaSassImport,
            site.root.join("sass/main.scss").to_str().unwrap()
        ));
        assert!(report.passed(), "a warning does not fail the check");
        assert_eq!(FindingKind::BulmaSassImport.severity(), Severity::Warning);
    }

    #[test]
    fn a_use_of_bulma_or_an_import_of_something_else_is_clean() {
        let site = TestSite::new("bulma_sass_use");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_project(
            "sass/main.scss",
            "@use \"bulma/sass\" with ($primary: #ff00aa);\n@import \"extras\";\n",
        );

        assert!(site.check().findings.is_empty());
    }

    #[test]
    fn a_bulma_1_package_dependency_is_clean() {
        let site = TestSite::new("bulma_1_package");
        site.write_source("main.json", VALID_MAIN_JSON);
        site.write_source("menu.json", VALID_MENU_JSON);
        site.write_source("page.json", VALID_PAGE_JSON);
        site.write_project(
            "package.json",
            r#"{"devDependencies":{"sass":"^1.77.6","bulma":"^1.0.4"}}"#,
        );

        let report = site.check();

        assert!(report.findings.is_empty(), "{:?}", report.findings);
    }
}
