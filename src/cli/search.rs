use super::prompt::report_change;
use clap::Subcommand;
use std::process::ExitCode;
use vados::authoring::site_features;

#[derive(Subcommand)]
pub enum SearchCommand {
    /// Publishes a site search at `/search`, with an entry in the main menu.
    Enable {
        #[arg(long)]
        source: String,
        /// Only search below this page; give it more than once for several.
        /// Every page when not given.
        #[arg(long)]
        from: Vec<String>,
        /// How many levels below `--from` to look. Any depth when not given.
        #[arg(long)]
        depth: Option<u32>,
        /// Also search pages with `"listed": false`.
        #[arg(long = "include-unlisted")]
        include_unlisted: bool,
        /// Leave the search out of the main menu.
        #[arg(long = "no-menu")]
        no_menu: bool,
    },
}

pub fn run(command: SearchCommand) -> ExitCode {
    match command {
        SearchCommand::Enable {
            source,
            from,
            depth,
            include_unlisted,
            no_menu,
        } => report_change(
            site_features::enable_search(&source, from, depth, include_unlisted, !no_menu)
                .map(|_| String::from("The site now publishes a search at /search.")),
        ),
    }
}
