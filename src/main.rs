mod config_files;

use crate::config_files::MainConfig;
use build_html::{Container, ContainerType, Html, HtmlContainer, HtmlPage};
use std::fs;
use std::fs::File;
use std::path::Path;

fn main() {
    let config_file_path = Path::new("config.json");
    let file = File::open(config_file_path).unwrap();
    let config: MainConfig = serde_json::from_reader(file).expect("JSON was not well-formatted");
    let mut page = HtmlPage::new().with_title(config.site_title);
    for js_link in config.js_files.iter() {
        page.add_script_link(js_link)
    }
    for css_link in config.css_files.iter() {
        page.add_stylesheet(css_link)
    }
    page.add_container(
        Container::new(ContainerType::Article)
            .with_attributes([("id", "article1")])
            .with_header_attr(
                2,
                "Hello, World",
                [("id", "article-head"), ("class", "header")],
            )
            .with_paragraph("This is a simple HTML demo"),
    );
    fs::write("index.html", page.to_html_string()).expect("Unable to write file");
}
