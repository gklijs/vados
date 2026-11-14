use super::prompt::{report_change, require_flag};
use clap::Subcommand;
use std::process::ExitCode;
use vados::authoring::menu::{self as menu_authoring, MenuLinkKind};

#[derive(Subcommand)]
pub enum MenuCommand {
    /// Appends one entry to menu.json's main menu.
    #[command(name = "add-item")]
    Add {
        #[arg(long)]
        source: String,
        /// A site-relative path (e.g. `/blog`) or an `https://` url.
        #[arg(long)]
        url: String,
        /// Required for an external (`https://`) url; optional otherwise,
        /// where it falls back to the target page's own title.
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        icon: Option<String>,
    },
    /// Removes a main-menu entry selected by a substring of its URL.
    #[command(name = "remove-item")]
    Remove {
        #[arg(long)]
        source: String,
        #[arg(long = "match")]
        match_value: Option<String>,
        /// Zero-based index among matching entries.
        #[arg(long)]
        index: Option<i64>,
    },
    /// Changes supplied fields of an entry, keeping omitted fields.
    #[command(name = "update-item")]
    Update {
        #[arg(long)]
        source: String,
        #[arg(long = "match")]
        match_value: Option<String>,
        /// Zero-based index among matching entries.
        #[arg(long)]
        index: Option<i64>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long, conflicts_with = "clear_title")]
        title: Option<String>,
        #[arg(long, conflicts_with = "clear_icon")]
        icon: Option<String>,
        /// Falls back to the target page's title; requires an internal URL.
        #[arg(long)]
        clear_title: bool,
        #[arg(long)]
        clear_icon: bool,
        /// Zero-based final position in the full main menu.
        #[arg(long)]
        position: Option<usize>,
    },
}

pub fn run(command: MenuCommand) -> ExitCode {
    match command {
        MenuCommand::Add {
            source,
            url,
            title,
            icon,
        } => run_menu_add_item(source, url, title, icon),
        MenuCommand::Remove {
            source,
            match_value,
            index,
        } => {
            let Some(pattern) = require_flag(match_value, "match", "Menu URL substring") else {
                return ExitCode::FAILURE;
            };
            report_change(
                menu_authoring::remove_menu_link(&source, &pattern, index)
                    .map(|url| format!("Removed menu link {url}"))
                    .map_err(|e| e.to_string()),
            )
        }
        MenuCommand::Update {
            source,
            match_value,
            index,
            url,
            title,
            icon,
            clear_title,
            clear_icon,
            position,
        } => {
            let Some(pattern) = require_flag(match_value, "match", "Menu URL substring") else {
                return ExitCode::FAILURE;
            };
            report_change(
                menu_authoring::update_menu_link(
                    &source,
                    &pattern,
                    index,
                    menu_authoring::MenuLinkUpdate {
                        url,
                        title: if clear_title {
                            Some(None)
                        } else {
                            title.map(Some)
                        },
                        icon: if clear_icon {
                            Some(None)
                        } else {
                            icon.map(Some)
                        },
                        position,
                    },
                )
                .map(|url| format!("Updated menu link {url}"))
                .map_err(|e| e.to_string()),
            )
        }
    }
}

// -- menu add-item ---------------------------------------------------------

fn run_menu_add_item(
    source: String,
    url: String,
    title: Option<String>,
    icon: Option<String>,
) -> ExitCode {
    let title =
        if menu_authoring::menu_link_kind_of(&url) == MenuLinkKind::External && title.is_none() {
            // Genuinely required here -- `add_menu_link` rejects an external
            // link with no title outright -- so this loops/fails like every
            // other required-conditional-on-context field (`run_page_new`'s
            // `title`, `run_footer_set`'s `content`), rather than
            // `optional_flag`, which would accept a blank answer only for
            // the rejection to arrive right after.
            match require_flag(None, "title", "Title (required for an external link)") {
                Some(t) => Some(t),
                None => return ExitCode::FAILURE,
            }
        } else {
            title
        };

    match menu_authoring::add_menu_link(&source, url, title, icon) {
        Ok(Ok(outcome)) => {
            println!("\nAdded menu link {}:", outcome.url);
            println!(
                "  kind:          {}",
                match outcome.kind {
                    MenuLinkKind::Internal => "internal",
                    MenuLinkKind::External => "external",
                }
            );
            println!("  would resolve: {}", outcome.would_resolve);
            ExitCode::SUCCESS
        }
        Ok(Err(reason)) => {
            eprintln!("menu add-item rejected: {reason}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("menu add-item failed while writing the project: {e}");
            ExitCode::FAILURE
        }
    }
}
