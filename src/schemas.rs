//! The JSON Schemas in `schemas/`, generated from the configuration structs
//! so they can't drift from what vados reads. Editors use them to complete
//! keys and flag mistakes while a site's author types; `check` keeps its own
//! findings and doesn't read them.
//!
//! Only built for tests, so `schemars` stays out of the binary. The test
//! fails when a committed schema differs from what the structs generate;
//! run it with `VADOS_UPDATE_SCHEMAS=1` to rewrite them.

use crate::config_files::{ImageList, MainConfig, MenuConfig, PageConfig};
use schemars::generate::SchemaSettings;
use schemars::JsonSchema;
use serde_json::Value;
use std::path::PathBuf;

/// Draft 7: the newest draft that every common editor understands.
fn generate<T: JsonSchema>() -> String {
    let schema = SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<T>();
    let mut value = schema.to_value();
    unwrap_descriptions(&mut value);
    let mut text = serde_json::to_string_pretty(&value).expect("a schema is JSON");
    text.push('\n');
    text
}

/// Joins the lines of each doc comment, which editors would otherwise show
/// with the source's line breaks. A blank line stays a paragraph break.
fn unwrap_descriptions(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                match child {
                    Value::String(text) if key == "description" => {
                        *text = text
                            .split("\n\n")
                            .map(|paragraph| paragraph.split('\n').collect::<Vec<_>>().join(" "))
                            .collect::<Vec<_>>()
                            .join("\n\n");
                    }
                    _ => unwrap_descriptions(child),
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(unwrap_descriptions),
        _ => {}
    }
}

/// Each file's schema, by the file name it describes.
pub(crate) fn schemas() -> Vec<(&'static str, String)> {
    vec![
        ("main.json", generate::<MainConfig>()),
        ("menu.json", generate::<MenuConfig>()),
        ("page.json", generate::<PageConfig>()),
        ("images.json", generate::<ImageList>()),
    ]
}

/// Where the committed schema for `file` lives.
pub(crate) fn schema_path(file: &str) -> PathBuf {
    let stem = file.trim_end_matches(".json");
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("schemas")
        .join(format!("{stem}.schema.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_files::schema_url;
    use std::fs;

    #[test]
    fn committed_schemas_match_the_configuration_structs() {
        let update = std::env::var_os("VADOS_UPDATE_SCHEMAS").is_some();
        let mut stale = vec![];
        for (file, generated) in schemas() {
            let path = schema_path(file);
            if fs::read_to_string(&path).ok().as_deref() == Some(generated.as_str()) {
                continue;
            }
            if update {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, generated).unwrap();
            } else {
                stale.push(path.display().to_string());
            }
        }
        assert!(
            stale.is_empty(),
            "out of date, rerun with VADOS_UPDATE_SCHEMAS=1: {}",
            stale.join(", ")
        );
    }

    // Follow references and union branches so nested accepted keys are covered too.
    fn configuration_paths(
        node: &Value,
        root: &Value,
        prefix: &str,
        paths: &mut std::collections::BTreeSet<String>,
    ) {
        if let Some(reference) = node["$ref"].as_str() {
            configuration_paths(
                root.pointer(reference.trim_start_matches('#')).unwrap(),
                root,
                prefix,
                paths,
            );
        }
        if let Some(properties) = node["properties"].as_object() {
            for (key, child) in properties {
                let path = format!("{prefix}{key}");
                paths.insert(path.clone());
                configuration_paths(child, root, &format!("{path}."), paths);
            }
        }
        if let Some(items) = node.get("items") {
            configuration_paths(
                items,
                root,
                &format!("{}[].", prefix.trim_end_matches('.')),
                paths,
            );
        }
        for branch in ["anyOf", "oneOf", "allOf"] {
            if let Some(children) = node[branch].as_array() {
                for child in children {
                    configuration_paths(child, root, prefix, paths);
                }
            }
        }
    }

    #[test]
    fn readme_configuration_reference_covers_accepted_keys() {
        let readme =
            fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md"))
                .unwrap();
        let reference = readme
            .split("## Configuration reference\n")
            .nth(1)
            .unwrap()
            .split("## Luma events")
            .next()
            .unwrap();
        for (file, generated) in schemas() {
            let root: Value = serde_json::from_str(&generated).unwrap();
            let mut expected = std::collections::BTreeSet::new();
            configuration_paths(&root, &root, "", &mut expected);
            let heading = format!("### `{file}`\n");
            let table = reference
                .split(&heading)
                .nth(1)
                .unwrap()
                .split("### ")
                .next()
                .unwrap();
            let actual = table
                .lines()
                .filter_map(|line| {
                    line.strip_prefix("| `")
                        .and_then(|row| row.split_once("` |").map(|(key, _)| key.to_owned()))
                })
                .collect();
            assert_eq!(expected, actual, "configuration reference for {file}");
        }
    }

    #[test]
    fn the_schema_url_names_this_version_and_the_schema_file() {
        assert_eq!(
            schema_url("page.json"),
            format!(
                "https://raw.githubusercontent.com/gklijs/vados/v{}/schemas/page.schema.json",
                env!("CARGO_PKG_VERSION")
            )
        );
        for (file, _) in schemas() {
            let url = schema_url(file);
            let name = schema_path(file);
            assert!(url.ends_with(name.file_name().unwrap().to_str().unwrap()));
        }
    }
}
