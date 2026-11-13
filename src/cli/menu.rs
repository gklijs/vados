use super::prompt::require_flag;
use clap::Subcommand;
use std::process::ExitCode;
use vados::authoring::menu::{self as menu_authoring, MenuLinkKind};

#[derive(Subcommand)]
pub enum MenuCommand {
    /// Appends one entry to menu.json's main menu. There is no
    /// `remove-item`/`update-item` yet -- edit menu.json by hand for that.
    AddItem {
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
}

pub fn run(command: MenuCommand) -> ExitCode {
    match command {
        MenuCommand::AddItem {
            source,
            url,
            title,
            icon,
        } => run_menu_add_item(source, url, title, icon),
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
