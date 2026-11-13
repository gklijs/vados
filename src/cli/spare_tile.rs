use super::prompt::report_change;
use clap::Subcommand;
use std::process::ExitCode;
use vados::authoring::site_features;

#[derive(Subcommand)]
pub enum SpareTileCommand {
    /// Replaces the spare tile. Shown on a tablet and wider, in the cells the
    /// shorter column of child tiles leaves over.
    Set {
        #[arg(long)]
        source: String,
        #[arg(long)]
        title: Option<String>,
        /// A link as `/url=Title`; give it once per link.
        #[arg(long = "link", required = true)]
        links: Vec<String>,
    },
    /// Removes the spare tile.
    Remove {
        #[arg(long)]
        source: String,
    },
}

pub fn run(command: SpareTileCommand) -> ExitCode {
    match command {
        SpareTileCommand::Set {
            source,
            title,
            links,
        } => report_change(
            site_features::set_spare_tile(&source, title, &links)
                .map(|_| String::from("The spare tile is set.")),
        ),
        SpareTileCommand::Remove { source } => report_change(
            site_features::remove_spare_tile(&source)
                .map(|_| String::from("The spare tile is removed.")),
        ),
    }
}
