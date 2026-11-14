mod cli;

use clap::{Parser, Subcommand};
use cli::footer::FooterCommand;
use cli::image::ImageCommand;
use cli::menu::MenuCommand;
use cli::navbar::NavbarCommand;
use cli::page::PageCommand;
use cli::search::SearchCommand;
use cli::social::SocialCommand;
use cli::spare_tile::SpareTileCommand;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "vados",
    version,
    about = "A static site generator built around Bulma"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read the source and image trees and publish a site at destination.
    Generate {
        #[arg(long)]
        source: String,
        #[arg(long = "img-source")]
        img_source: String,
        #[arg(long)]
        destination: String,
        /// Maximum generation workers (defaults to available CPUs); use 1 for serial work.
        #[arg(long)]
        jobs: Option<std::num::NonZeroUsize>,
        /// Check the links of the published site, and fail if any are broken.
        #[arg(long = "check-links")]
        check_links: bool,
        /// With --check-links, also ask other sites whether the pages linked
        /// to exist. Only ever warnings.
        #[arg(long, requires = "check_links")]
        external: bool,
    },
    /// Read the source and image trees and report every problem found,
    /// without writing anything. Exits non-zero iff at least one error was
    /// found; warnings are reported but never fail the check.
    Check {
        #[arg(long)]
        source: String,
        #[arg(long = "img-source")]
        img_source: String,
        /// Also check the links of a site already published at this directory.
        /// Other sites are never contacted.
        #[arg(long)]
        output: Option<String>,
        /// With --output, also ask other sites whether the pages linked to
        /// exist. Only ever warnings.
        #[arg(long, requires = "output")]
        external: bool,
    },
    /// Scaffold a fresh vados project in the current directory: gathers a
    /// few basics interactively and creates a starting source tree, image
    /// tree and Netlify deployment setup. Refuses to run if anything it
    /// would create already exists.
    Init,
    /// Create or attach an image to a page in an existing project.
    Page {
        #[command(subcommand)]
        command: PageCommand,
    },
    /// Register an image in an existing project.
    Image {
        #[command(subcommand)]
        command: ImageCommand,
    },
    /// Add, update or remove a social link in an existing project.
    Social {
        #[command(subcommand)]
        command: SocialCommand,
    },
    /// Change the site-wide footer of an existing project.
    Footer {
        #[command(subcommand)]
        command: FooterCommand,
    },
    /// Author the main menu of an existing project.
    Menu {
        #[command(subcommand)]
        command: MenuCommand,
    },
    /// Turn on the site search of an existing project.
    Search {
        #[command(subcommand)]
        command: SearchCommand,
    },
    /// Set the block of links that fills spare cells next to child tiles.
    SpareTile {
        #[command(subcommand)]
        command: SpareTileCommand,
    },
    /// Change how the navbar behaves in an existing project.
    Navbar {
        #[command(subcommand)]
        command: NavbarCommand,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Generate {
            source,
            img_source,
            destination,
            check_links,
            external,
            jobs,
        } => cli::generate::run(source, img_source, destination, check_links, external, jobs),
        Command::Check {
            source,
            img_source,
            output,
            external,
        } => cli::check::run(source, img_source, output, external),
        Command::Init => cli::init::run_init(),
        Command::Page { command } => cli::page::run(command),
        Command::Search { command } => cli::search::run(command),
        Command::SpareTile { command } => cli::spare_tile::run(command),
        Command::Navbar { command } => cli::navbar::run(command),
        Command::Image { command } => cli::image::run(command),
        Command::Social { command } => cli::social::run(command),
        Command::Footer { command } => cli::footer::run(command),
        Command::Menu { command } => cli::menu::run(command),
    }
}
