use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MainConfig {
    pub site_title: String,
    pub js_files: Vec<String>,
    pub css_files: Vec<String>,
}
