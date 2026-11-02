//! Checks the generated site itself: every internal link, fragment, image,
//! stylesheet and script a published page refers to must exist, resolved the
//! way static hosts resolve them. Nothing is requested from the network. See
//! `vados.allium`'s `OutputCheck`.

use crate::check::{Finding, FindingKind};
#[cfg(feature = "external-links")]
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(feature = "external-links")]
use std::{
    net::{IpAddr, Ipv4Addr, ToSocketAddrs},
    sync::Mutex,
    thread,
    time::Duration,
};
use walkdir::WalkDir;

/// One attribute value or `url()` a file refers to.
struct Reference {
    target: String,
    /// Whether it's something the page loads, rather than a link to follow.
    is_asset: bool,
}

/// What one HTML file contains, once scanned.
#[derive(Default)]
struct ScannedPage {
    references: Vec<Reference>,
    ids: HashSet<String>,
}

/// Reads an HTML document's tags, without building a tree: quoted and unquoted
/// attributes alike, since published pages are minified. Comments and what
/// `<script>` and `<style>` hold are skipped, but a `<style>`'s `url()`s count.
fn scan_html(html: &str) -> ScannedPage {
    let mut page = ScannedPage::default();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        rest = &rest[start..];
        if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.find("-->").map_or("", |end| &after[end + 3..]);
            continue;
        }
        let Some((name, attributes, after)) = read_tag(rest) else {
            rest = &rest[1..];
            continue;
        };
        rest = after;
        collect_references(&name, &attributes, &mut page);
        if name == "script" || name == "style" {
            let close = format!("</{name}");
            let lower = rest.to_ascii_lowercase();
            let end = lower.find(&close).unwrap_or(rest.len());
            if name == "style" {
                for target in css_urls(&rest[..end]) {
                    page.references.push(Reference {
                        target,
                        is_asset: true,
                    });
                }
            }
            rest = &rest[end..];
        }
    }
    page
}

/// A tag's lower-case name, its attributes, and the text following it.
type ReadTag<'a> = (String, Vec<(String, String)>, &'a str);

/// The tag at the start of `text`. `None` when this `<` doesn't open a tag.
fn read_tag(text: &str) -> Option<ReadTag<'_>> {
    let body = &text[1..];
    let name_end = body.find(|c: char| c.is_whitespace() || c == '>' || c == '/')?;
    let name = body[..name_end].to_ascii_lowercase();
    if name.is_empty() || !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let mut attributes = vec![];
    let mut rest = &body[name_end..];
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
        if let Some(after) = rest.strip_prefix('>') {
            return Some((name, attributes, after));
        }
        if rest.is_empty() {
            return None;
        }
        let key_end = rest.find(|c: char| c.is_whitespace() || c == '=' || c == '>' || c == '/')?;
        let key = rest[..key_end].to_ascii_lowercase();
        rest = rest[key_end..].trim_start();
        let Some(after_equals) = rest.strip_prefix('=') else {
            if key_end == 0 {
                rest = &rest[1..];
            } else {
                attributes.push((key, String::new()));
            }
            continue;
        };
        let value_start = after_equals.trim_start();
        let (value, after_value) = match value_start.chars().next() {
            Some(quote @ ('"' | '\'')) => {
                let inner = &value_start[1..];
                let end = inner.find(quote)?;
                (&inner[..end], &inner[end + 1..])
            }
            _ => {
                let end = value_start
                    .find(|c: char| c.is_whitespace() || c == '>')
                    .unwrap_or(value_start.len());
                (&value_start[..end], &value_start[end..])
            }
        };
        attributes.push((key, decode_entities(value)));
        rest = after_value;
    }
}

fn decode_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&#38;", "&")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
}

fn collect_references(name: &str, attributes: &[(String, String)], page: &mut ScannedPage) {
    let attribute = |wanted: &str| {
        attributes
            .iter()
            .find(|(key, _)| key == wanted)
            .map(|(_, value)| value.as_str())
    };
    if let Some(id) = attribute("id") {
        page.ids.insert(id.to_string());
    }
    if name == "a" {
        if let Some(anchor) = attribute("name") {
            page.ids.insert(anchor.to_string());
        }
    }
    let mut add = |target: &str, is_asset: bool| {
        page.references.push(Reference {
            target: target.trim().to_string(),
            is_asset,
        })
    };
    match name {
        "a" | "area" => attribute("href").into_iter().for_each(|t| add(t, false)),
        "link" => {
            // A preconnect or dns-prefetch names a host, not a page.
            let rel = attribute("rel").unwrap_or_default().to_ascii_lowercase();
            if !rel.contains("preconnect") && !rel.contains("dns-prefetch") {
                let is_asset = rel.contains("stylesheet") || rel.contains("icon");
                attribute("href").into_iter().for_each(|t| add(t, is_asset));
            }
        }
        "img" | "script" | "source" | "video" | "audio" | "iframe" | "embed" | "track" => {
            attribute("src").into_iter().for_each(|t| add(t, true));
            for url in srcset_urls(attribute("srcset").unwrap_or_default()) {
                add(url, true);
            }
        }
        "form" => attribute("action").into_iter().for_each(|t| add(t, false)),
        "meta" if attribute("property") == Some("og:image") => {
            attribute("content").into_iter().for_each(|t| add(t, true));
        }
        _ => {}
    }
    // vados's own search form names its index this way.
    if name == "form" {
        attribute("data-index")
            .into_iter()
            .for_each(|t| add(t, true));
    }
}

/// The addresses in a `srcset`, read the way browsers do: an address runs to the
/// next whitespace, so a `data:` URL can hold commas, and what follows it up to
/// the next comma describes it.
fn srcset_urls(srcset: &str) -> Vec<&str> {
    let mut urls = vec![];
    let mut rest = srcset;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        if rest.is_empty() {
            return urls;
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let url = &rest[..end];
        rest = &rest[end..];
        if let Some(without_comma) = url.strip_suffix(',') {
            // No descriptor: the comma ended this candidate.
            urls.push(without_comma);
        } else {
            urls.push(url);
            rest = rest.find(',').map_or("", |comma| &rest[comma + 1..]);
        }
    }
}

/// What a stylesheet's `url()`s point at, leaving out inline `data:` URLs.
fn css_urls(css: &str) -> Vec<String> {
    let lower = css.to_ascii_lowercase();
    let mut urls = vec![];
    let mut from = 0;
    while let Some(start) = lower[from..].find("url(") {
        let value_start = from + start + 4;
        let Some(end) = css[value_start..].find(')') else {
            break;
        };
        let url = css[value_start..value_start + end]
            .trim()
            .trim_matches(['"', '\''])
            .trim();
        if !url.is_empty() {
            urls.push(url.to_string());
        }
        from = value_start + end;
    }
    urls
}

/// `%20` and friends, as a host reads a path. Anything malformed stays as is.
fn percent_decoded(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = (bytes[i] == b'%')
            .then(|| path.get(i + 1..i + 3))
            .flatten()
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match hex {
            Some(byte) => {
                out.push(byte);
                i += 3;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `..` and `.` taken out of an absolute path; going above the root stays at
/// the root, as it does in a browser.
fn normalized(path: &str) -> String {
    let mut parts: Vec<&str> = vec![];
    let ends_in_slash = path.ends_with('/') || path.ends_with("/.") || path.ends_with("/..");
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    let mut result = format!("/{}", parts.join("/"));
    if ends_in_slash && result != "/" {
        result.push('/');
    }
    result
}

/// Which published file a site path means, the way static hosts resolve it:
/// `/agenda` is `agenda.html` even when an `agenda/` directory exists too.
fn resolve_file(root: &Path, files: &HashSet<PathBuf>, path: &str) -> Option<PathBuf> {
    let relative = path.trim_start_matches('/');
    let candidates: Vec<PathBuf> = if relative.is_empty() {
        vec![root.join("index.html")]
    } else if let Some(directory) = relative.strip_suffix('/') {
        vec![
            root.join(directory).join("index.html"),
            root.join(format!("{directory}.html")),
        ]
    } else {
        vec![
            root.join(relative),
            root.join(format!("{relative}.html")),
            root.join(relative).join("index.html"),
        ]
    };
    candidates.into_iter().find(|c| files.contains(c))
}

/// The address a published file is served at.
fn file_site_path(root: &Path, file: &Path) -> String {
    let relative = file.strip_prefix(root).unwrap_or(file);
    let path = relative.to_string_lossy().replace('\\', "/");
    if path == "index.html" {
        return String::from("/");
    }
    let path = path
        .strip_suffix("/index.html")
        .map(|d| format!("{d}/"))
        .unwrap_or(path);
    format!("/{}", path.strip_suffix(".html").unwrap_or(&path))
}

/// The internal path and fragment a reference means, or `None` for anything
/// that isn't this site's: other sites, `mailto:`, inline `data:` and so on.
fn internal_target(
    reference: &str,
    from: &str,
    site_origin: Option<&str>,
) -> Option<(String, Option<String>)> {
    let (without_fragment, fragment) = match reference.split_once('#') {
        Some((before, fragment)) => (before, Some(fragment.to_string())),
        None => (reference, None),
    };
    let without_query = without_fragment.split('?').next().unwrap_or_default();
    let path = if without_query.is_empty() {
        from.to_string()
    } else if let Some(rest) = without_query.strip_prefix("//") {
        let origin_host = site_origin?.split_once("://")?.1;
        let (host, path) = rest.split_once('/').map_or((rest, ""), |(h, p)| (h, p));
        if !host.eq_ignore_ascii_case(origin_host.split('/').next()?) {
            return None;
        }
        format!("/{path}")
    } else if without_query.starts_with('/') {
        without_query.to_string()
    } else if has_scheme(without_query) {
        let origin = site_origin?;
        let rest = strip_origin(without_query, origin)?;
        if rest.is_empty() {
            String::from("/")
        } else {
            rest.to_string()
        }
    } else {
        let directory = from.rfind('/').map_or("/", |i| &from[..=i]);
        format!("{directory}{without_query}")
    };
    Some((normalized(&percent_decoded(&path)), fragment))
}

/// Whether `text` starts with a URL scheme, like `https:` or `mailto:`.
fn has_scheme(text: &str) -> bool {
    text.split_once(':').is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    })
}

/// What follows the site's own address in `url`, also accepting the other of
/// http and https, or `None` when it's on another site.
fn strip_origin<'a>(url: &'a str, origin: &str) -> Option<&'a str> {
    let host_and_path = |u: &'a str| {
        u.strip_prefix("https://")
            .or_else(|| u.strip_prefix("http://"))
    };
    let origin_rest = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"))?;
    let rest = host_and_path(url)?;
    let head = rest.get(..origin_rest.len())?;
    if !head.eq_ignore_ascii_case(origin_rest) {
        return None;
    }
    let after = &rest[origin_rest.len()..];
    (after.is_empty() || after.starts_with('/')).then_some(after)
}

/// The URLs a sitemap lists.
fn sitemap_locations(xml: &str) -> Vec<String> {
    xml.split("<loc>")
        .skip(1)
        .filter_map(|part| part.split_once("</loc>"))
        .map(|(location, _)| decode_entities(location.trim()))
        .collect()
}

/// Every problem with the links of the site published at `output`. `site_url`
/// makes absolute links to the site count as internal.
pub fn check_output(output: &str, site_url: Option<&str>) -> Vec<Finding> {
    let root = Path::new(output);
    let mut unreadable = vec![];
    let files: HashSet<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.map_err(|e| unreadable.push(e.to_string())).ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();
    // A mistyped directory must not pass as a site with no broken links.
    if !unreadable.is_empty() {
        return unreadable
            .into_iter()
            .map(|detail| Finding {
                kind: FindingKind::OutputUnreadable,
                location: String::from(output),
                detail: Some(detail),
            })
            .collect();
    }
    let mut sorted: Vec<&PathBuf> = files.iter().collect();
    sorted.sort();

    let mut pages: HashMap<&Path, ScannedPage> = HashMap::new();
    for file in &sorted {
        let is_html = file.extension().is_some_and(|e| e == "html");
        if is_html {
            if let Ok(html) = fs::read_to_string(file) {
                pages.insert(file.as_path(), scan_html(&html));
            }
        }
    }

    let mut findings = vec![];
    let mut report = |kind: FindingKind, location: String, detail: String| {
        findings.push(Finding {
            kind,
            location,
            detail: Some(detail),
        })
    };

    for file in &sorted {
        let Some(page) = pages.get(file.as_path()) else {
            continue;
        };
        let from = file_site_path(root, file);
        for reference in &page.references {
            let Some((path, fragment)) = internal_target(&reference.target, &from, site_url) else {
                continue;
            };
            let Some(target) = resolve_file(root, &files, &path) else {
                let kind = if reference.is_asset {
                    FindingKind::MissingAsset
                } else {
                    FindingKind::BrokenInternalLink
                };
                report(kind, from.clone(), reference.target.clone());
                continue;
            };
            let Some(fragment) = fragment.filter(|f| !f.is_empty() && f != "top") else {
                continue;
            };
            if let Some(target_page) = pages.get(target.as_path()) {
                if !target_page.ids.contains(&percent_decoded(&fragment)) {
                    report(
                        FindingKind::MissingLinkFragment,
                        from.clone(),
                        reference.target.clone(),
                    );
                }
            }
        }
    }

    for file in sorted
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "css"))
    {
        let Ok(css) = fs::read_to_string(file) else {
            continue;
        };
        let from = format!(
            "/{}",
            file.strip_prefix(root)
                .unwrap_or(file)
                .to_string_lossy()
                .replace('\\', "/")
        );
        for url in css_urls(&css) {
            let Some((path, _)) = internal_target(&url, &from, site_url) else {
                continue;
            };
            if resolve_file(root, &files, &path).is_none() {
                report(FindingKind::MissingAsset, from.clone(), url);
            }
        }
    }

    if let Ok(xml) = fs::read_to_string(root.join("sitemap.xml")) {
        for location in sitemap_locations(&xml) {
            // A sitemap can only list its own site, so an address elsewhere
            // is as wrong as a missing page. Without a site URL, nothing says
            // which site that is.
            let found = match internal_target(&location, "/", site_url) {
                Some((path, _)) => resolve_file(root, &files, &path).is_some(),
                None => site_url.is_none(),
            };
            if !found {
                report(
                    FindingKind::BrokenInternalLink,
                    String::from("/sitemap.xml"),
                    location,
                );
            }
        }
    }
    findings.extend(headers_file_findings(&root.join("_headers")));
    findings
}

/// The root-relative links in `html` that can only mean a page: no scheme, no
/// file extension in their last segment.
pub(crate) fn page_links_in(html: &str) -> Vec<String> {
    scan_html(html)
        .references
        .into_iter()
        .filter(|r| !r.is_asset)
        .filter_map(|r| {
            let path = r.target.split(['?', '#']).next().unwrap_or_default();
            let is_page_path = path.starts_with('/')
                && !path.starts_with("//")
                && !path.rsplit('/').next().unwrap_or_default().contains('.');
            is_page_path.then(|| percent_decoded(path))
        })
        .collect()
}

/// About the longest header value Cloudflare Pages accepts.
const MAX_HEADER_VALUE_BYTES: usize = 2000;

/// What a static host may trip over in a `_headers` file. A comment line made
/// every request on Cloudflare Pages answer an empty 404 when tested on
/// 2026-10-30, which is bad enough to warn about. Nothing else about the file's
/// meaning is judged here.
pub(crate) fn headers_file_findings(file: &Path) -> Vec<Finding> {
    let Ok(contents) = fs::read_to_string(file) else {
        return vec![];
    };
    let location = file.display().to_string();
    let mut findings = vec![];
    for (number, line) in contents.lines().enumerate() {
        let line = line.trim();
        let mut problem = |detail: String| {
            findings.push(Finding {
                kind: FindingKind::HeadersFileProblem,
                location: location.clone(),
                detail: Some(format!("line {}: {detail}", number + 1)),
            })
        };
        if line.starts_with('#') {
            problem(String::from(
                "a comment line, which has made Cloudflare Pages answer every request with an empty 404",
            ));
        } else if let Some((name, value)) = line.split_once(':') {
            if value.trim().len() > MAX_HEADER_VALUE_BYTES {
                problem(format!(
                    "{} is over Cloudflare Pages' limit of {MAX_HEADER_VALUE_BYTES} bytes per value",
                    name.trim()
                ));
            }
        }
    }
    findings
}

#[cfg(feature = "external-links")]
/// Every address on other sites that the site at `output` links to or loads,
/// with the first page using it.
fn external_targets(output: &str, site_url: Option<&str>) -> BTreeMap<String, String> {
    let root = Path::new(output);
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x == "html"))
        .map(|e| e.path().to_path_buf())
        .collect();
    files.sort();
    let mut targets = BTreeMap::new();
    for file in files {
        let Ok(html) = fs::read_to_string(&file) else {
            continue;
        };
        let from = file_site_path(root, &file);
        for reference in scan_html(&html).references {
            let is_web =
                reference.target.starts_with("https://") || reference.target.starts_with("http://");
            if is_web && internal_target(&reference.target, &from, site_url).is_none() {
                let target = reference.target.split('#').next().unwrap_or_default();
                targets
                    .entry(target.to_string())
                    .or_insert_with(|| from.clone());
            }
        }
    }
    targets
}

/// Whether an address is on the public internet. A page's links can come from
/// anywhere, so nothing in them may make this machine call its own network, such
/// as a router or a cloud provider's metadata service.
#[cfg(feature = "external-links")]
fn is_public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_public_v4(v4),
            None => {
                let first = v6.segments()[0];
                !(v6.is_loopback()
                    || v6.is_unspecified()
                    || v6.is_multicast()
                    || (first & 0xfe00) == 0xfc00
                    || (first & 0xffc0) == 0xfe80)
            }
        },
    }
}

#[cfg(feature = "external-links")]
fn is_public_v4(v4: Ipv4Addr) -> bool {
    let [a, b, ..] = v4.octets();
    !(v4.is_private()
        || v4.is_loopback()
        || v4.is_link_local()
        || v4.is_unspecified()
        || v4.is_broadcast()
        || v4.is_multicast()
        || v4.is_documentation()
        || a == 0
        || (a == 100 && (64..128).contains(&b)))
}

/// How an address may be reached.
#[cfg(feature = "external-links")]
enum Reach {
    Public,
    /// Somewhere private, which is no business of a link check.
    Private,
    Unknown(String),
}

/// Where `url`'s host resolves to. A host answering differently a moment later
/// could still get past this; it stops plain links and redirects, which are what
/// a page can do by itself.
#[cfg(feature = "external-links")]
fn reach_of(url: &str) -> Reach {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host_and_port = authority.rsplit('@').next().unwrap_or_default();
    let has_port = host_and_port.rsplit_once(':').is_some_and(|(host, port)| {
        !host.ends_with(':') && !port.ends_with(']') && port.bytes().all(|b| b.is_ascii_digit())
    });
    let default_port = if url.starts_with("https:") { 443 } else { 80 };
    let target = if has_port {
        host_and_port.to_string()
    } else {
        format!("{host_and_port}:{default_port}")
    };
    match target.to_socket_addrs() {
        Ok(addresses) => {
            let addresses: Vec<_> = addresses.collect();
            if addresses.is_empty() {
                Reach::Unknown(String::from("the host has no address"))
            } else if addresses.iter().all(|a| is_public_address(a.ip())) {
                Reach::Public
            } else {
                Reach::Private
            }
        }
        Err(e) => Reach::Unknown(e.to_string()),
    }
}

/// The answer to one request, following redirects by hand so that every step is
/// held to the same rule as the first.
#[cfg(feature = "external-links")]
fn final_status(
    agent: &ureq::Agent,
    url: &str,
    head: bool,
    allow_private: bool,
) -> Result<Option<u16>, String> {
    let mut current = url.to_string();
    for _ in 0..MAX_REDIRECTS {
        if !allow_private {
            match reach_of(&current) {
                Reach::Public => {}
                Reach::Private => return Ok(None),
                Reach::Unknown(problem) => return Err(problem),
            }
        }
        let response = if head {
            agent.head(&current).call()
        } else {
            agent.get(&current).call()
        }
        .map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        if !(300..400).contains(&status) {
            return Ok(Some(status));
        }
        let location = response
            .headers()
            .get("location")
            .and_then(|l| l.to_str().ok())
            .map(str::to_string);
        current = match location {
            Some(l) if l.starts_with("https://") || l.starts_with("http://") => l,
            Some(l) if l.starts_with('/') && !l.starts_with("//") => {
                let (scheme, rest) = current.split_once("://").unwrap_or(("https", &current));
                let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
                format!("{scheme}://{host}{l}")
            }
            // Somewhere this can't follow, or that isn't the web: no judgement.
            _ => return Ok(None),
        };
    }
    Ok(None)
}

/// Why `url` looks broken, or `None` when it works or no one can say. A refusal
/// (401, 403, 429) or a server error says nothing about whether the page exists,
/// and many sites send those to anything that isn't a browser, so only an answer
/// of "not there" or no answer at all counts.
#[cfg(feature = "external-links")]
fn external_problem(agent: &ureq::Agent, url: &str, allow_private: bool) -> Option<String> {
    let status = match final_status(agent, url, true, allow_private) {
        // Not every server handles HEAD; ask properly before judging.
        Ok(Some(status)) if ![400, 404, 405, 501].contains(&status) => Some(status),
        Ok(None) => None,
        _ => match final_status(agent, url, false, allow_private) {
            Ok(status) => status,
            Err(error) => return Some(error),
        },
    };
    status
        .filter(|s| [404, 410].contains(s))
        .map(|s| format!("answered {s}"))
}

/// How many sites are asked at the same time.
#[cfg(feature = "external-links")]
const EXTERNAL_WORKERS: usize = 8;

#[cfg(feature = "external-links")]
const MAX_REDIRECTS: usize = 5;

/// Asks other sites whether the pages linked to exist. Only on request, and only
/// ever warnings: their availability must not decide whether a build passes. One
/// worker takes all of a site's addresses, one after another, so no site gets
/// several requests from this at once. Addresses on private networks are skipped,
/// even through a redirect, since a page shouldn't be able to reach those.
#[cfg(feature = "external-links")]
pub fn check_external(output: &str, site_url: Option<&str>) -> Vec<Finding> {
    ask_other_sites(output, site_url, false)
}

#[cfg(feature = "external-links")]
fn ask_other_sites(output: &str, site_url: Option<&str>, allow_private: bool) -> Vec<Finding> {
    let mut by_host: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (url, page) in external_targets(output, site_url) {
        let host = url
            .split_once("://")
            .map(|(_, rest)| {
                rest.split('/')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
            })
            .unwrap_or_default();
        by_host.entry(host).or_default().push((url, page));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .max_redirects(0)
        .user_agent(concat!("vados-link-check/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let queue = Mutex::new(by_host.into_values().collect::<Vec<_>>());
    let found = Mutex::new(vec![]);
    thread::scope(|scope| {
        for _ in 0..EXTERNAL_WORKERS {
            scope.spawn(|| loop {
                let next = queue.lock().expect("no worker panics holding it").pop();
                let Some(site) = next else {
                    return;
                };
                for (url, page) in site {
                    if let Some(problem) = external_problem(&agent, &url, allow_private) {
                        found
                            .lock()
                            .expect("no worker panics holding it")
                            .push(Finding {
                                kind: FindingKind::BrokenExternalLink,
                                location: page,
                                detail: Some(format!("{url}: {problem}")),
                            });
                    }
                }
            });
        }
    });
    let mut findings = found.into_inner().expect("every worker has finished");
    findings.sort_by(|a, b| a.detail.cmp(&b.detail));
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    struct Output(PathBuf);

    impl Output {
        fn new(files: &[(&str, &str)]) -> Output {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir().join(format!("vados_link_check_{id}"));
            let _ = fs::remove_dir_all(&root);
            for (path, contents) in files {
                let file = root.join(path);
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(file, contents).unwrap();
            }
            Output(root)
        }
        fn findings(&self) -> Vec<(FindingKind, String, String)> {
            let mut found: Vec<_> =
                check_output(self.0.to_str().unwrap(), Some("https://example.com"))
                    .into_iter()
                    .map(|f| (f.kind, f.location, f.detail.unwrap()))
                    .collect();
            found.sort_by(|a, b| a.2.cmp(&b.2));
            found
        }
    }

    impl Drop for Output {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_clean_site_passes_without_any_network() {
        let site = Output::new(&[
            ("index.html", r##"<a href=/agenda>a</a><a href="/agenda/jam#when">b</a><a href='#here' id=here>c</a><a href="https://other.example/x">d</a><a href="mailto:a@b.c">e</a><a href="https://example.com/agenda">f</a><link rel=preconnect href=https://cdn.example><img src=/i.webp srcset="/i.webp 100w, /i2.webp 200w"><script src=/a.js></script><link rel=stylesheet href=/s.css><a href="agenda?x=1">g</a><a href=#>h</a>"##),
            ("agenda.html", r#"<a href="../index.html">home</a>"#),
            ("agenda/jam.html", r#"<h2 id="when">x</h2><a href="/agenda/jam.ics">cal</a><a href="..">up</a>"#),
            ("agenda/jam.ics", ""),
            ("agenda/index.html", ""),
            ("i.webp", ""), ("i2.webp", ""), ("a.js", ""),
            ("s.css", r#"a{background:url("/i.webp")}b{background:url(data:image/png;base64,AA)}c{background:url(i2.webp)}"#),
            ("sitemap.xml", "<urlset><url><loc>https://example.com/</loc></url><url><loc>https://example.com/agenda/jam</loc></url></urlset>"),
        ]);
        assert_eq!(site.findings(), vec![]);
    }

    #[test]
    fn every_kind_of_breakage_names_its_page_and_target() {
        let site = Output::new(&[
            (
                "index.html",
                r##"<a href="/gone">a</a><a href="/about#nope">b</a><img srcset="/ok.webp 1x, /gone.webp 2x" src=/ok.webp><a href="https://example.com/also-gone">c</a>"##,
            ),
            ("about.html", r#"<p id="yes"></p>"#),
            ("ok.webp", ""),
            ("s.css", "a{background:url(/gone.png)}"),
            ("sitemap.xml", "<loc>https://example.com/lost</loc>"),
        ]);
        let found = site.findings();
        let expect = |kind, location: &str, detail: &str| {
            assert!(
                found
                    .iter()
                    .any(|f| f.0 == kind && f.1 == location && f.2 == detail),
                "{detail} in {found:?}"
            );
        };
        expect(FindingKind::BrokenInternalLink, "/", "/gone");
        expect(FindingKind::MissingLinkFragment, "/", "/about#nope");
        expect(FindingKind::MissingAsset, "/", "/gone.webp");
        expect(
            FindingKind::BrokenInternalLink,
            "/",
            "https://example.com/also-gone",
        );
        expect(FindingKind::MissingAsset, "/s.css", "/gone.png");
        expect(
            FindingKind::BrokenInternalLink,
            "/sitemap.xml",
            "https://example.com/lost",
        );
        assert_eq!(found.len(), 6);
    }

    #[test]
    fn a_sitemap_address_on_another_host_is_reported() {
        let site = Output::new(&[
            ("index.html", ""),
            (
                "sitemap.xml",
                "<loc>https://example.com/</loc><loc>https://old.example.org/page</loc>",
            ),
        ]);
        let found = site.findings();
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].0, FindingKind::BrokenInternalLink);
        assert_eq!(found[0].2, "https://old.example.org/page");
    }

    #[test]
    fn without_a_site_url_a_sitemaps_addresses_cannot_be_judged() {
        let site = Output::new(&[("sitemap.xml", "<loc>https://example.com/gone</loc>")]);
        assert!(check_output(site.0.to_str().unwrap(), None).is_empty());
    }

    #[test]
    fn a_page_wins_over_a_directory_of_the_same_name() {
        let root = PathBuf::from("/site");
        let files: HashSet<PathBuf> = [root.join("agenda.html"), root.join("agenda/x.html")]
            .into_iter()
            .collect();
        assert_eq!(
            resolve_file(&root, &files, "/agenda"),
            Some(root.join("agenda.html"))
        );
        assert_eq!(
            resolve_file(&root, &files, "/agenda/"),
            Some(root.join("agenda.html"))
        );
        assert_eq!(resolve_file(&root, &files, "/agenda/y"), None);
    }

    #[test]
    fn what_comments_and_scripts_hold_is_not_a_link() {
        let site = Output::new(&[(
            "index.html",
            r#"<!-- <a href="/gone">a</a> --><script>let a = '<a href="/gone">';</script><pre>&lt;a href="/gone"&gt;</pre><a data-x="a>b" href="/">ok</a>"#,
        )]);
        assert_eq!(site.findings(), vec![]);
    }

    #[test]
    fn inline_style_urls_count() {
        let site = Output::new(&[("index.html", "<style>a{background:url(/gone.png)}</style>")]);
        assert_eq!(site.findings()[0].0, FindingKind::MissingAsset);
    }

    #[test]
    fn a_similar_host_is_not_this_site() {
        assert_eq!(
            strip_origin("https://example.com.evil.test/x", "https://example.com"),
            None
        );
        assert_eq!(
            strip_origin("http://example.com/x", "https://example.com"),
            Some("/x")
        );
        assert_eq!(
            strip_origin("https://EXAMPLE.com", "https://example.com"),
            Some("")
        );
    }

    #[test]
    fn a_headers_file_is_warned_about_for_comments_and_long_values() {
        let long = "x".repeat(MAX_HEADER_VALUE_BYTES + 1);
        let site = Output::new(&[(
            "_headers",
            &format!("/*\n  X-Frame-Options: DENY\n# oops\n  Content-Security-Policy: {long}\n"),
        )]);
        let details: Vec<String> = site.findings().into_iter().map(|f| f.2).collect();
        assert_eq!(details.len(), 2, "{details:?}");
        assert!(details.iter().any(|d| d.starts_with("line 3: a comment")));
        assert!(details
            .iter()
            .any(|d| d.starts_with("line 4: Content-Security-Policy")));
    }

    #[test]
    fn page_links_are_only_addresses_that_can_only_be_pages() {
        let links = page_links_in(
            r##"<a href="/about?x=1#y">a</a><a href="/feed.xml">b</a><a href="https://x.test/a">c</a><a href="rel">d</a><a href="#f">e</a><img src="/nope"><a href="/my%20page/">f</a>"##,
        );
        assert_eq!(links, ["/about", "/my page/"]);
    }

    #[cfg(feature = "external-links")]
    /// A tiny server: answers by path, and refuses HEAD on `/no-head`.
    fn start_server() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                use std::io::{BufRead, BufReader, Write};
                let mut stream = stream;
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap_or_default();
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap_or(0) > 2 {
                    line.clear();
                }
                let parts: Vec<&str> = first.split_whitespace().collect();
                let status = match (parts.first().copied(), parts.get(1).copied()) {
                    (Some("HEAD"), Some("/no-head")) => "405 Method Not Allowed",
                    (_, Some("/gone")) => "404 Not Found",
                    (_, Some("/moved")) => "302 Found\r\nLocation: /gone",
                    (_, Some("/forbidden")) => "403 Forbidden",
                    _ => "200 OK",
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
            }
        });
        format!("http://{address}")
    }

    #[cfg(feature = "external-links")]
    #[test]
    fn only_a_page_that_is_not_there_counts_as_a_broken_outside_link() {
        let server = start_server();
        let html = format!(
            r#"<a href="{server}/ok#part">a</a><a href="{server}/gone">b</a><a href="{server}/forbidden">c</a><a href="{server}/no-head">d</a><a href="{server}/gone">again</a><a href="http://127.0.0.1:1/refused">e</a><a href="https://example.com/mine">own site</a>"#
        );
        let site = Output::new(&[("index.html", &html), ("mine.html", "")]);
        // This server is on loopback, which the public entry point refuses.
        assert!(check_external(site.0.to_str().unwrap(), Some("https://example.com")).is_empty());
        let findings = ask_other_sites(site.0.to_str().unwrap(), Some("https://example.com"), true);
        let details: Vec<&str> = findings
            .iter()
            .map(|f| f.detail.as_deref().unwrap())
            .collect();
        assert_eq!(details.len(), 2, "{details:?}");
        assert!(details.iter().any(|d| d.contains("/gone: answered 404")));
        assert!(details.iter().any(|d| d.contains("/refused")));
        assert!(findings
            .iter()
            .all(|f| f.location == "/" && f.kind == FindingKind::BrokenExternalLink));
    }

    #[test]
    fn a_srcset_is_read_like_a_browser_reads_it() {
        assert_eq!(
            srcset_urls("/a.webp 100w, /b.webp 200w,/c.webp"),
            ["/a.webp", "/b.webp", "/c.webp"]
        );
        assert_eq!(
            srcset_urls("data:image/png;base64,AA,BB 1x, /b.webp 2x"),
            ["data:image/png;base64,AA,BB", "/b.webp"]
        );
        assert_eq!(srcset_urls("/a.webp, /b.webp"), ["/a.webp", "/b.webp"]);
        assert!(srcset_urls(" ").is_empty());
    }

    #[test]
    fn a_directory_that_is_not_there_is_an_error() {
        let found = check_output("/no/such/directory/at/all", None);
        assert_eq!(found[0].kind, FindingKind::OutputUnreadable);
        assert_eq!(found[0].severity(), crate::check::Severity::Error);
    }

    #[cfg(feature = "external-links")]
    #[test]
    fn only_public_addresses_may_be_asked() {
        for private in [
            "127.0.0.1",
            "10.0.0.1",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "::1",
            "fc00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(!is_public_address(private.parse().unwrap()), "{private}");
        }
        for public in ["93.184.216.34", "2606:2800:220:1::1"] {
            assert!(is_public_address(public.parse().unwrap()), "{public}");
        }
    }

    #[cfg(feature = "external-links")]
    #[test]
    fn private_addresses_are_skipped_without_a_request_or_a_warning() {
        let server = start_server();
        let html = format!(
            r#"<a href="{server}/gone">a</a><a href="http://169.254.169.254/latest">b</a>"#
        );
        let site = Output::new(&[("index.html", &html)]);
        assert_eq!(check_external(site.0.to_str().unwrap(), None).len(), 0);
    }

    #[cfg(feature = "external-links")]
    #[test]
    fn redirects_are_followed_by_hand() {
        let server = start_server();
        let html = format!(r#"<a href="{server}/moved">a</a>"#);
        let site = Output::new(&[("index.html", &html)]);
        let found = ask_other_sites(site.0.to_str().unwrap(), None, true);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].detail.as_deref().unwrap().contains("answered 404"));
    }

    #[test]
    fn dots_in_a_path_resolve_like_a_browser() {
        assert_eq!(normalized("/a/b/../c"), "/a/c");
        assert_eq!(normalized("/../../x"), "/x");
        assert_eq!(normalized("/a/.."), "/");
    }

    #[test]
    fn escaped_characters_are_read_like_a_host() {
        let site = Output::new(&[
            ("index.html", r#"<a href="/a%20b">x</a>"#),
            ("a b.html", ""),
        ]);
        assert_eq!(site.findings(), vec![]);
    }
}
