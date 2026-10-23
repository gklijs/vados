//! Structured data (schema.org JSON-LD) for search engines: a dated page is
//! described as a `BlogPosting`, the home page as the `WebSite` and the
//! `Person` behind it. See `vados.allium`'s `PagesCarryStructuredData`.

use crate::structure::Item;
use serde_json::{json, Map, Value};

/// What the whole site contributes to every page's structured data.
pub(crate) struct SiteFacts<'a> {
    pub(crate) site_title: &'a str,
    pub(crate) description: Option<&'a str>,
    /// The usable site URL, without a trailing slash.
    pub(crate) site_url: Option<&'a str>,
    pub(crate) author_name: &'a str,
    pub(crate) author_url: Option<&'a str>,
    /// The site owner's own profiles elsewhere: the social links.
    pub(crate) same_as: Vec<&'a str>,
}

/// The page-specific facts, already resolved.
pub(crate) struct PageFacts<'a> {
    pub(crate) url: Option<&'a str>,
    pub(crate) description: Option<&'a str>,
    pub(crate) image_url: Option<&'a str>,
}

/// The page's JSON-LD, ready to sit in a `<script type="application/ld+json">`,
/// or `None` for a page with nothing to say: neither dated nor the home page.
pub(crate) fn structured_data(item: &Item, page: &PageFacts, site: &SiteFacts) -> Option<String> {
    let value = if let Some(date) = item.date {
        let mut posting = Map::new();
        posting.insert("@context".into(), json!("https://schema.org"));
        posting.insert("@type".into(), json!("BlogPosting"));
        posting.insert("headline".into(), json!(item.title));
        insert_some(&mut posting, "description", page.description);
        posting.insert("datePublished".into(), json!(date.to_rfc3339()));
        posting.insert(
            "dateModified".into(),
            json!(item.modified().unwrap_or(date).to_rfc3339()),
        );
        if let Some(image) = page.image_url {
            posting.insert("image".into(), json!([image]));
        }
        posting.insert("author".into(), person(site));
        insert_some(&mut posting, "mainEntityOfPage", page.url);
        Value::Object(posting)
    } else if item.path == "/" {
        let mut website = Map::new();
        website.insert("@type".into(), json!("WebSite"));
        website.insert("name".into(), json!(site.site_title));
        insert_some(&mut website, "description", site.description);
        insert_some(&mut website, "url", page.url);
        json!({
            "@context": "https://schema.org",
            "@graph": [Value::Object(website), person(site)],
        })
    } else {
        return None;
    };
    // `</` would end the script element early, whatever the JSON says.
    Some(value.to_string().replace("</", "<\\/"))
}

fn person(site: &SiteFacts) -> Value {
    let mut person = Map::new();
    person.insert("@type".into(), json!("Person"));
    person.insert("name".into(), json!(site.author_name));
    let home = site.site_url.map(|u| format!("{u}/"));
    insert_some(&mut person, "url", site.author_url.or(home.as_deref()));
    if !site.same_as.is_empty() {
        person.insert("sameAs".into(), json!(site.same_as));
    }
    Value::Object(person)
}

fn insert_some(map: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(v) = value {
        map.insert(key.into(), json!(v));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_files::PageConfig;

    fn site() -> SiteFacts<'static> {
        SiteFacts {
            site_title: "My site",
            description: Some("About things"),
            site_url: Some("https://example.com"),
            author_name: "Jo",
            author_url: None,
            same_as: vec!["https://github.com/jo"],
        }
    }

    fn item(path: &str, date: Option<&str>, updated: Option<&str>) -> Item {
        let mut config = PageConfig::new(path);
        config.title = String::from("A </script> post");
        config.date = date.map(String::from);
        config.updated = updated.map(String::from);
        Item::new(String::from(path), config)
    }

    #[test]
    fn a_dated_page_is_a_blog_posting() {
        let page = PageFacts {
            url: Some("https://example.com/blog/post"),
            description: Some("Summary"),
            image_url: Some("https://example.com/img/post-og.jpg"),
        };
        let json = structured_data(
            &item("/blog/post", Some("2026-10-22"), Some("2026-10-23T10:00")),
            &page,
            &site(),
        )
        .unwrap();
        assert!(!json.contains("</"));
        let value: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["@type"], "BlogPosting");
        assert_eq!(value["headline"], "A </script> post");
        assert_eq!(value["datePublished"], "2026-10-22T00:00:00Z");
        assert_eq!(value["dateModified"], "2026-10-23T10:00:00Z");
        assert_eq!(value["image"][0], "https://example.com/img/post-og.jpg");
        assert_eq!(value["author"]["name"], "Jo");
        assert_eq!(value["author"]["url"], "https://example.com/");
        assert_eq!(value["author"]["sameAs"][0], "https://github.com/jo");
        assert_eq!(value["mainEntityOfPage"], "https://example.com/blog/post");
    }

    #[test]
    fn the_home_page_is_the_website_and_its_author() {
        let page = PageFacts {
            url: Some("https://example.com/"),
            description: None,
            image_url: None,
        };
        let json = structured_data(&item("/", None, None), &page, &site()).unwrap();
        let value: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["@graph"][0]["@type"], "WebSite");
        assert_eq!(value["@graph"][0]["url"], "https://example.com/");
        assert_eq!(value["@graph"][1]["@type"], "Person");
    }

    #[test]
    fn an_undated_page_has_none() {
        let page = PageFacts {
            url: None,
            description: None,
            image_url: None,
        };
        assert_eq!(
            structured_data(&item("/about", None, None), &page, &site()),
            None
        );
    }
}
