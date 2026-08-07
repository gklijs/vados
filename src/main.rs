use clap::{Parser, Subcommand};
use std::process::ExitCode;
use vados::check::check;
use vados::generator::generate;

#[derive(Parser)]
#[command(name = "vados", version, about = "A static site generator built around Bulma")]
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
    },
    /// Read the source and image trees and report every problem found,
    /// without writing anything. Exits non-zero iff at least one error was
    /// found; warnings are reported but never fail the check.
    Check {
        #[arg(long)]
        source: String,
        #[arg(long = "img-source")]
        img_source: String,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Generate {
            source,
            img_source,
            destination,
        } => {
            generate(&source, &img_source, &destination);
            ExitCode::SUCCESS
        }
        Command::Check { source, img_source } => {
            let report = check(&source, &img_source);
            for finding in &report.findings {
                println!("{}", finding);
            }
            println!();
            println!(
                "{} error(s), {} warning(s)",
                report.error_count(),
                report.warning_count()
            );
            if report.passed() {
                println!("check passed");
                ExitCode::SUCCESS
            } else {
                println!("check failed");
                ExitCode::FAILURE
            }
        }
    }
}
