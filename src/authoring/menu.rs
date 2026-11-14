//! Footer and main-menu authoring: adjusts main.json's footer content
//! reference and adds, updates or removes menu.json entries. See
//! `vados.allium`'s `FooterUpdateRun` and `MenuLinkAdditionRun`.

use super::page::page_exists_at;
use crate::config_files::{MainConfig, MenuConfig, RawMenuItem};
use crate::json_files::{read_json_for_edit as read_json, write_json_pretty};
use std::fmt;
use std::io;
use std::path::Path;

/// Replaces main.json's single footer content reference. Nothing about a
/// footer content reference can be rejected that `check` doesn't already
/// classify for itself (an unrecognised reference is reported there, not
/// blocked here), so there is no rejection to model -- see `vados.allium`'s
/// comment on `FooterUpdateRun`.
pub fn set_footer(source: &str, footer_content: String) -> io::Result<String> {
    let path = Path::new(source).join("main.json");
    let mut config: MainConfig = read_json(&path)?;
    config.footer_content = footer_content.clone();
    write_json_pretty(path, &config)?;
    Ok(footer_content)
}

/// An entry pointing off-site is external; anything else names a path
/// within this site. Classified the same way `RegisterMainMenuLink`
/// classifies a declared link, so add-time judgement and read-time judgement
/// never drift apart. See `vados.allium`'s `MenuLinkKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuLinkKind {
    Internal,
    External,
}

pub fn menu_link_kind_of(url: &str) -> MenuLinkKind {
    if url.starts_with("https://") {
        MenuLinkKind::External
    } else {
        MenuLinkKind::Internal
    }
}

/// Why a `menu add-item` request was rejected. See `vados.allium`'s
/// `MenuLinkAdditionRejectionReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuLinkAdditionRejectionReason {
    ExternalLinkWithoutTitle,
    UrlAlreadyInMenu,
}

impl fmt::Display for MenuLinkAdditionRejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MenuLinkAdditionRejectionReason::ExternalLinkWithoutTitle => {
                "an external menu link needs a title"
            }
            MenuLinkAdditionRejectionReason::UrlAlreadyInMenu => {
                "this url is already in the main menu"
            }
        })
    }
}

/// What `menu add-item` added, once it has. `would_resolve` tells the
/// maintainer, at the moment the entry is added, whether it will actually
/// show up in the published menu -- distinct from a rejection, since an
/// internal link naming a path with no page yet is allowed; it just will not
/// resolve until that page exists. See `vados.allium`'s
/// `MenuLinkAddedOutcome`.
#[derive(Debug)]
pub struct MenuLinkAddedOutcome {
    pub url: String,
    pub title: Option<String>,
    pub icon: Option<String>,
    pub kind: MenuLinkKind,
    pub would_resolve: bool,
}

fn url_already_in_menu(menu_config: &MenuConfig, url: &str) -> bool {
    menu_config.main_menu.iter().any(|item| item.url == url)
}

/// Fields omitted from an update keep their current values. The nested
/// options allow an explicit clearing of a title or icon.
#[derive(Debug, Default)]
pub struct MenuLinkUpdate {
    pub url: Option<String>,
    pub title: Option<Option<String>>,
    pub icon: Option<Option<String>>,
    /// Zero-based final position in the full menu, after moving the entry.
    pub position: Option<usize>,
}

fn invalid_request(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn select_menu_link(menu: &MenuConfig, pattern: &str, index: Option<i64>) -> io::Result<usize> {
    let matches: Vec<_> = menu
        .main_menu
        .iter()
        .enumerate()
        .filter(|(_, item)| item.url.contains(pattern))
        .map(|(i, _)| i)
        .collect();
    if matches.is_empty() {
        return Err(invalid_request("no matching menu item; nothing to change"));
    }
    match index {
        Some(i) if i < 0 || i as usize >= matches.len() => Err(invalid_request(
            "--index does not name one of the matching menu items",
        )),
        Some(i) => Ok(matches[i as usize]),
        None if matches.len() > 1 => Err(invalid_request(
            "more than one menu item matches; use --index to pick one",
        )),
        None => Ok(matches[0]),
    }
}

/// Removes a URL-substring match. Index is zero-based among matches, as in
/// `social remove`. Rejections leave the file untouched.
pub fn remove_menu_link(source: &str, pattern: &str, index: Option<i64>) -> io::Result<String> {
    let path = Path::new(source).join("menu.json");
    let mut menu: MenuConfig = read_json(&path)?;
    let selected = select_menu_link(&menu, pattern, index)?;
    let removed = menu.main_menu.remove(selected);
    write_json_pretty(path, &menu)?;
    Ok(removed.url)
}

/// Updates only the supplied fields and optionally moves the selected entry.
/// All validation finishes before writing the file.
pub fn update_menu_link(
    source: &str,
    pattern: &str,
    index: Option<i64>,
    update: MenuLinkUpdate,
) -> io::Result<String> {
    let path = Path::new(source).join("menu.json");
    let mut menu: MenuConfig = read_json(&path)?;
    let selected = select_menu_link(&menu, pattern, index)?;
    if update.url.is_none()
        && update.title.is_none()
        && update.icon.is_none()
        && update.position.is_none()
    {
        return Err(invalid_request(
            "give a URL, title, icon, clearing flag or position to update",
        ));
    }
    let position = update.position.unwrap_or(selected);
    if position >= menu.main_menu.len() {
        return Err(invalid_request("--position is outside the main menu"));
    }
    let mut item = menu.main_menu.remove(selected);
    if let Some(url) = update.url {
        item.url = url;
    }
    if let Some(title) = update.title {
        item.title = title;
    }
    if let Some(icon) = update.icon {
        item.icon = icon;
    }
    if menu_link_kind_of(&item.url) == MenuLinkKind::External && item.title.is_none() {
        return Err(invalid_request("an external menu link needs a title"));
    }
    if url_already_in_menu(&menu, &item.url) {
        return Err(invalid_request("this url is already in the main menu"));
    }
    let url = item.url.clone();
    menu.main_menu.insert(position, item);
    write_json_pretty(path, &menu)?;
    Ok(url)
}

/// `menu add-item`. See `vados.allium`'s `ApplyMenuLinkAddition`. Single-phase
/// like `social add`/`update`/`remove`: nothing about a menu entry is
/// expensive enough to check that it's worth a separate prompt phase first.
pub fn add_menu_link(
    source: &str,
    url: String,
    title: Option<String>,
    icon: Option<String>,
) -> io::Result<Result<MenuLinkAddedOutcome, MenuLinkAdditionRejectionReason>> {
    let path = Path::new(source).join("menu.json");
    let mut menu_config: MenuConfig = read_json(&path)?;

    let kind = menu_link_kind_of(&url);
    if kind == MenuLinkKind::External && title.is_none() {
        return Ok(Err(
            MenuLinkAdditionRejectionReason::ExternalLinkWithoutTitle,
        ));
    }
    if url_already_in_menu(&menu_config, &url) {
        return Ok(Err(MenuLinkAdditionRejectionReason::UrlAlreadyInMenu));
    }

    let would_resolve = kind == MenuLinkKind::External || page_exists_at(source, &url);

    menu_config.main_menu.push(RawMenuItem {
        url: url.clone(),
        title: title.clone(),
        icon: icon.clone(),
    });
    write_json_pretty(path, &menu_config)?;

    Ok(Ok(MenuLinkAddedOutcome {
        url,
        title,
        icon,
        kind,
        would_resolve,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    struct TestSource {
        path: std::path::PathBuf,
    }

    impl TestSource {
        fn new(name: &str) -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!("vados_menu_authoring_test_{name}_{id}"));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            TestSource { path }
        }

        fn root(&self) -> &str {
            self.path.to_str().unwrap()
        }

        fn write_main_json(&self, footer_content: &str) {
            fs::write(
                self.path.join("main.json"),
                format!(
                    r#"{{"siteTitle":"Site","jsFiles":[],"includeDefaultJs":null,"cssFiles":[],"includeDefaultCss":null,"backgroundClass":null,"navbarColor":null,"footerContent":"{footer_content}"}}"#
                ),
            )
            .unwrap();
        }

        fn write_menu_json(&self, contents: &str) {
            fs::write(self.path.join("menu.json"), contents).unwrap();
        }

        fn mkdir(&self, rel_path: &str) {
            fs::create_dir_all(self.path.join(rel_path.trim_start_matches('/'))).unwrap();
        }
    }

    impl Drop for TestSource {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn set_footer_replaces_only_the_footer_content() {
        let source = TestSource::new("footer_set");
        source.write_main_json("<p>old</p>");

        set_footer(source.root(), "<p>new</p>".to_string()).unwrap();

        let config: MainConfig = read_json(&Path::new(source.root()).join("main.json")).unwrap();
        assert_eq!(config.footer_content, "<p>new</p>");
        assert_eq!(config.site_title, "Site");
    }

    #[test]
    fn menu_link_kind_of_classifies_by_scheme() {
        assert_eq!(
            menu_link_kind_of("https://example.com"),
            MenuLinkKind::External
        );
        assert_eq!(menu_link_kind_of("/blog"), MenuLinkKind::Internal);
    }

    #[test]
    fn an_external_link_without_a_title_is_rejected() {
        let source = TestSource::new("external_no_title");
        source.write_menu_json(r#"{"mainMenu":[],"socials":[]}"#);

        let result =
            add_menu_link(source.root(), "https://example.com".to_string(), None, None).unwrap();

        assert_eq!(
            result.unwrap_err(),
            MenuLinkAdditionRejectionReason::ExternalLinkWithoutTitle
        );
    }

    #[test]
    fn a_duplicate_url_is_rejected() {
        let source = TestSource::new("duplicate");
        source.write_menu_json(
            r#"{"mainMenu":[{"url":"/blog","title":null,"icon":null}],"socials":[]}"#,
        );

        let result = add_menu_link(source.root(), "/blog".to_string(), None, None).unwrap();

        assert_eq!(
            result.unwrap_err(),
            MenuLinkAdditionRejectionReason::UrlAlreadyInMenu
        );
    }

    #[test]
    fn an_internal_link_naming_an_existing_page_resolves() {
        let source = TestSource::new("internal_resolves");
        source.write_menu_json(r#"{"mainMenu":[],"socials":[]}"#);
        source.mkdir("blog");

        let outcome = add_menu_link(source.root(), "/blog".to_string(), None, None)
            .unwrap()
            .unwrap();

        assert!(outcome.would_resolve);
        assert_eq!(outcome.kind, MenuLinkKind::Internal);
    }

    #[test]
    fn an_internal_link_naming_no_page_yet_is_added_but_would_not_resolve() {
        let source = TestSource::new("internal_forward_reference");
        source.write_menu_json(r#"{"mainMenu":[],"socials":[]}"#);

        let outcome = add_menu_link(source.root(), "/not-yet".to_string(), None, None)
            .unwrap()
            .unwrap();

        assert!(!outcome.would_resolve);

        let menu_config: MenuConfig =
            read_json(&Path::new(source.root()).join("menu.json")).unwrap();
        assert_eq!(menu_config.main_menu.len(), 1);
    }

    #[test]
    fn an_external_link_with_a_title_always_resolves() {
        let source = TestSource::new("external_with_title");
        source.write_menu_json(r#"{"mainMenu":[],"socials":[]}"#);

        let outcome = add_menu_link(
            source.root(),
            "https://example.com".to_string(),
            Some("Example".to_string()),
            None,
        )
        .unwrap()
        .unwrap();

        assert!(outcome.would_resolve);
        assert_eq!(outcome.title, Some("Example".to_string()));
    }
}
