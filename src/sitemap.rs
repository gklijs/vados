//! `sitemap.xml` and `robots.txt`: what search engines are pointed at to find
//! every page. The sitemap needs the site URL; robots.txt is always written,
//! naming the sitemap when there is one. See `vados.allium`'s `Sitemap` and
//! `Robots`.

use crate::content::escape_html;
use crate::feed::page_url;
use crate::files::write_raw;
use crate::structure::Structure;

/// Writes the sitemap, listing every listed page in full.
pub(crate) fn write_sitemap(destination: &str, site_url: &str, structure: &Structure) {
    write_raw(destination, "/sitemap.xml", sitemap(site_url, structure));
}

/// Writes a robots.txt allowing every crawler, pointing at the sitemap when
/// there is a site URL -- unless the static files already gave one.
pub(crate) fn write_robots(destination: &str, site_url: Option<&str>, static_files: &[String]) {
    if !static_files.iter().any(|f| f == "/robots.txt") {
        write_raw(destination, "/robots.txt", robots(site_url));
    }
}

fn sitemap(site_url: &str, structure: &Structure) -> String {
    let mut items = structure.get_listed_items();
    items.sort_by(|a, b| a.path.cmp(&b.path));
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for item in items {
        xml.push_str("  <url><loc>");
        xml.push_str(&escape_html(&page_url(site_url, &item.path)));
        xml.push_str("</loc>");
        // Only a date the maintainer gave: search engines distrust a lastmod
        // that is just the time of the build, and ignore priority and
        // changefreq altogether.
        if let Some(modified) = item.modified() {
            xml.push_str("<lastmod>");
            xml.push_str(&modified.to_rfc3339());
            xml.push_str("</lastmod>");
        }
        xml.push_str("</url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

fn robots(site_url: Option<&str>) -> String {
    let mut robots = String::from("User-agent: *\nAllow: /\n");
    if let Some(site_url) = site_url {
        robots.push_str(&format!("\nSitemap: {site_url}/sitemap.xml\n"));
    }
    robots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_files::PageConfig;
    use crate::structure::Item;
    use dashmap::DashMap;

    fn page(path: &str, date: Option<&str>, listed: bool) -> Item {
        let mut config = PageConfig::new(path);
        config.date = date.map(String::from);
        config.listed = Some(listed);
        Item::new(String::from(path), config)
    }

    #[test]
    fn the_sitemap_lists_listed_pages_with_their_dates_only() {
        let structure = Structure::new(DashMap::new());
        structure.add_item(page("/", None, true));
        structure.add_item(page("/blog", None, true));
        structure.add_item(page("/blog/a&b", Some("2026-10-22"), true));
        structure.add_item(page("/404", None, false));

        let xml = sitemap("https://example.com", &structure);
        assert_eq!(
            xml,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  \
             <url><loc>https://example.com/</loc></url>\n  \
             <url><loc>https://example.com/blog</loc></url>\n  \
             <url><loc>https://example.com/blog/a&amp;b</loc><lastmod>2026-10-22T00:00:00Z</lastmod></url>\n\
             </urlset>\n"
        );
    }

    #[test]
    fn robots_allows_everything_and_names_the_sitemap() {
        assert_eq!(
            robots(Some("https://example.com")),
            "User-agent: *\nAllow: /\n\nSitemap: https://example.com/sitemap.xml\n"
        );
    }

    #[test]
    fn robots_without_a_site_url_names_no_sitemap() {
        assert_eq!(robots(None), "User-agent: *\nAllow: /\n");
    }
}
