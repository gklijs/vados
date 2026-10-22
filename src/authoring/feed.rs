//! `page enable-feed`: turns on the Atom feed of a page that's already
//! there, and records the site URL in main.json if the feed would otherwise
//! have none to build its links from. See `vados.allium`'s
//! `FeedEnablementRun`.

use super::page::{page_dir, page_exists_at};
use crate::config_files::{is_site_url, FeedConfig, FeedSetting, MainConfig, PageConfig};
use crate::feed::feed_href;
use crate::json_files::{read_json, read_json_or, write_json_pretty};
use std::fmt;
use std::io;
use std::path::Path;

/// Why a `page enable-feed` request couldn't proceed. See `vados.allium`'s
/// `FeedEnablementBlockReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedEnablementBlockReason {
    TargetPageMissing,
    MalformedSiteUrl,
}

impl fmt::Display for FeedEnablementBlockReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FeedEnablementBlockReason::TargetPageMissing => "the target page does not exist",
            FeedEnablementBlockReason::MalformedSiteUrl => {
                "not an absolute http(s) URL, e.g. https://example.com"
            }
        })
    }
}

#[derive(Debug)]
pub struct FeedEnablementBlock {
    pub reason: FeedEnablementBlockReason,
    pub detail: String,
}

/// main.json's site URL, if it has a usable one. A main.json that can't be
/// read has none as far as this is concerned; the write that follows will
/// surface the problem instead.
pub fn usable_site_url(source: &str) -> Option<String> {
    read_json::<MainConfig>(&Path::new(source).join("main.json"))
        .ok()
        .and_then(|c| c.get_site_url().map(String::from))
}

/// Checks every way a `page enable-feed` request could fail in one pass. See
/// `vados.allium`'s `DetectFeedEnablementBlockers`.
pub fn detect_feed_enablement_blockers(
    source: &str,
    path: &str,
    given_site_url: Option<&str>,
) -> Vec<FeedEnablementBlock> {
    let mut blocks = Vec::new();
    if !page_exists_at(source, path) {
        blocks.push(FeedEnablementBlock {
            reason: FeedEnablementBlockReason::TargetPageMissing,
            detail: String::from(path),
        });
    }
    if let Some(url) = given_site_url {
        if !is_site_url(url) {
            blocks.push(FeedEnablementBlock {
                reason: FeedEnablementBlockReason::MalformedSiteUrl,
                detail: String::from(url),
            });
        }
    }
    blocks
}

/// Gathered once the blocker check has passed. `site_url` is written to
/// main.json when given; it's required only when main.json has no usable one.
#[derive(Debug, Clone, Default)]
pub struct FeedEnablementDetails {
    pub title: Option<String>,
    pub limit: Option<u32>,
    pub site_url: Option<String>,
}

/// What `page enable-feed` did, once it has. See `vados.allium`'s
/// `FeedEnabledOutcome`.
#[derive(Debug)]
pub struct FeedEnabledOutcome {
    pub path: String,
    /// Where the feed will be published, in full.
    pub feed_url: Option<String>,
    pub title: Option<String>,
    pub limit: u32,
    /// The site URL, when this run is what wrote it to main.json.
    pub site_url_written: Option<String>,
}

/// Turns on the page's feed, replacing any title or limit it already had.
/// Callers are expected to have already run
/// `detect_feed_enablement_blockers` and found it empty.
pub fn enable_feed(
    source: &str,
    path: &str,
    details: FeedEnablementDetails,
) -> io::Result<FeedEnabledOutcome> {
    let title = details
        .title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    let site_url_written = details.site_url.map(|u| u.trim().to_string());

    if let Some(url) = &site_url_written {
        let main_json = Path::new(source).join("main.json");
        let mut main_config: MainConfig = read_json(&main_json)?;
        main_config.site_url = Some(url.clone());
        write_json_pretty(main_json, &main_config)?;
    }

    let page_json = page_dir(source, path).join("page.json");
    let mut config = read_json_or(&page_json, || PageConfig::new(path))?;
    let feed = FeedConfig {
        title: title.clone(),
        limit: details.limit,
    };
    // Plain `true` unless there's something to say, so page.json reads the
    // way the README shows it.
    config.feed = Some(if feed == FeedConfig::default() {
        FeedSetting::Enabled(true)
    } else {
        FeedSetting::Configured(feed.clone())
    });
    write_json_pretty(page_json, &config)?;

    let feed_url = usable_site_url(source).map(|u| format!("{u}{}", feed_href(path)));
    Ok(FeedEnabledOutcome {
        path: String::from(path),
        feed_url,
        title,
        limit: feed.get_limit(),
        site_url_written,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    const MAIN_JSON: &str =
        r#"{"siteTitle":"Test","jsFiles":[],"cssFiles":[],"footerContent":"<p>f</p>"}"#;

    struct TestSite {
        root: PathBuf,
    }

    impl TestSite {
        fn new(name: &str, main_json: &str) -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir().join(format!("vados_feed_authoring_test_{name}_{id}"));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("blog")).unwrap();
            fs::write(root.join("main.json"), main_json).unwrap();
            TestSite { root }
        }
        fn source(&self) -> &str {
            self.root.to_str().unwrap()
        }
        fn read(&self, rel: &str) -> String {
            fs::read_to_string(self.root.join(rel)).unwrap()
        }
    }

    impl Drop for TestSite {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn blocks_a_missing_page_and_a_malformed_site_url_together() {
        let site = TestSite::new("blocked", MAIN_JSON);
        let blocks = detect_feed_enablement_blockers(site.source(), "/news", Some("example.com"));
        let reasons: Vec<_> = blocks.iter().map(|b| b.reason).collect();
        assert_eq!(
            reasons,
            [
                FeedEnablementBlockReason::TargetPageMissing,
                FeedEnablementBlockReason::MalformedSiteUrl
            ]
        );
        assert!(detect_feed_enablement_blockers(site.source(), "/blog", None).is_empty());
    }

    #[test]
    fn enabling_without_settings_writes_plain_true_and_the_site_url() {
        let site = TestSite::new("plain", MAIN_JSON);
        assert_eq!(usable_site_url(site.source()), None);

        let outcome = enable_feed(
            site.source(),
            "/blog",
            FeedEnablementDetails {
                site_url: Some("https://example.com/".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(site.read("blog/page.json").contains(r#""feed": true"#));
        assert!(site
            .read("main.json")
            .contains(r#""siteUrl": "https://example.com/""#));
        assert_eq!(
            outcome.feed_url.as_deref(),
            Some("https://example.com/blog/feed.xml")
        );
        assert_eq!(outcome.limit, FeedConfig::DEFAULT_LIMIT);
    }

    #[test]
    fn enabling_with_a_title_and_limit_writes_them_and_keeps_the_site_url() {
        let site = TestSite::new(
            "configured",
            r#"{"siteTitle":"Test","jsFiles":[],"cssFiles":[],"footerContent":"<p>f</p>","siteUrl":"https://example.com"}"#,
        );

        let outcome = enable_feed(
            site.source(),
            "/blog",
            FeedEnablementDetails {
                title: Some(" Notes ".to_string()),
                limit: Some(5),
                site_url: None,
            },
        )
        .unwrap();

        let config: PageConfig = read_json(&site.root.join("blog/page.json")).unwrap();
        assert_eq!(
            config.get_feed(),
            Some(FeedConfig {
                title: Some("Notes".to_string()),
                limit: Some(5)
            })
        );
        assert_eq!(outcome.site_url_written, None);
        assert_eq!(outcome.limit, 5);
    }
}
