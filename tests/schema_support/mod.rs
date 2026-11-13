//! Validates configuration files against the committed JSON Schemas in
//! `schemas/`, the way an editor would. A unit test in `src/schemas.rs`
//! keeps those files in step with the configuration structs.
#![allow(dead_code)]

use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

/// The files a schema is published for.
pub const FILES: [&str; 4] = ["main.json", "menu.json", "page.json", "images.json"];

fn validators() -> &'static HashMap<&'static str, jsonschema::Validator> {
    static VALIDATORS: OnceLock<HashMap<&'static str, jsonschema::Validator>> = OnceLock::new();
    VALIDATORS.get_or_init(|| {
        FILES
            .iter()
            .map(|file| {
                let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("schemas")
                    .join(file.replace(".json", ".schema.json"));
                let schema: Value =
                    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
                (*file, jsonschema::draft7::new(&schema).unwrap())
            })
            .collect()
    })
}

/// Each place where `value` breaks the schema for `file`, as `path: message`.
pub fn schema_errors(file: &str, value: &Value) -> Vec<String> {
    validators()[file]
        .iter_errors(value)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

/// Every schema error in the configuration files below `dir`, each prefixed
/// with the file it is in.
pub fn tree_errors(dir: &Path) -> Vec<String> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name().to_str().unwrap_or_default();
            entry.file_type().is_file() && FILES.contains(&name)
        })
        .flat_map(|entry| {
            let name = entry.file_name().to_str().unwrap().to_string();
            let value: Value =
                serde_json::from_str(&std::fs::read_to_string(entry.path()).unwrap()).unwrap();
            schema_errors(&name, &value)
                .into_iter()
                .map(move |error| format!("{}: {error}", entry.path().display()))
        })
        .collect()
}
