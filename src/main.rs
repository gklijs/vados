use clap::{Parser, Subcommand};
use dialoguer::Input;
use std::path::Path;
use std::process::ExitCode;
use vados::check::check;
use vados::generator::generate;
use vados::init::{self, ProjectBasics, RecognizedSocialProvider, SocialHandle};

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
    /// Scaffold a fresh vados project in the current directory: gathers a
    /// few basics interactively and creates a starting source tree, image
    /// tree and Netlify deployment setup. Refuses to run if anything it
    /// would create already exists.
    Init,
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
        Command::Init => run_init(),
    }
}

fn run_init() -> ExitCode {
    let dir = Path::new(".");

    // Every artifact `init` would create is checked in one pass before the
    // maintainer is asked anything; see vados.allium's
    // `DetectScaffoldConflicts`.
    let conflicts = init::detect_conflicts(dir);
    if !conflicts.is_empty() {
        println!(
            "init found {} existing path(s) it would need to create:\n",
            conflicts.len()
        );
        for conflict in &conflicts {
            println!("  {:<45} {}", conflict.kind, conflict.path);
        }
        println!("\nMove or remove them, then run `vados init` again.");
        return ExitCode::FAILURE;
    }

    // The wizard reads answers interactively; without a real terminal on
    // both ends a prompt can never be answered, so fail fast with a clear
    // message instead of leaving a `Site title` prompt blocked forever.
    if !dialoguer::console::user_attended() {
        eprintln!("init needs an interactive terminal to ask its questions; none was found.");
        return ExitCode::FAILURE;
    }

    println!("Scaffolding a new vados project in the current directory.\n");

    let site_title = prompt_required("Site title");
    let home_intro = prompt_optional(&format!(
        "Home page intro [default: \"Welcome to {}.\"]",
        site_title
    ));
    let primary_color = prompt_optional("Primary color [default: #00d1b2, Bulma's own]");
    let footer_text = prompt_optional("Footer text [default: \"Built with vados.\"]");

    println!("\nSocial links -- leave any of these blank to skip it.");
    let mut socials: Vec<SocialHandle> = Vec::new();
    for provider in RecognizedSocialProvider::all() {
        if let Some(handle) = prompt_optional(&format!("{} handle", provider.label())) {
            socials.push(SocialHandle { provider, handle });
        }
    }

    let basics = ProjectBasics {
        site_title,
        home_intro,
        primary_color,
        footer_text,
        socials,
    };

    match init::scaffold(dir, basics) {
        Ok(outcome) => {
            println!("\nScaffolded a new vados project:\n");
            println!("  site title:    {}", outcome.site_title);
            println!("  home intro:    {}", outcome.home_intro);
            println!("  primary color: {}", outcome.primary_color);
            println!("  footer text:   {}", outcome.footer_text);
            if outcome.socials.is_empty() {
                println!("  socials:       none");
            } else {
                println!("  socials:");
                for social in &outcome.socials {
                    println!("    {}: {}", social.provider.label(), social.handle);
                }
            }
            println!(
                "  git repo:      {}",
                if outcome.repository_initialized {
                    "initialized"
                } else {
                    "already present"
                }
            );
            println!(
                "  .gitignore:    {}",
                if outcome.gitignore_created {
                    "created"
                } else {
                    "merged into the existing one"
                }
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("init failed while writing the project: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Prompts until a non-blank answer is given. Used only for `site_title`,
/// the one basic with no default.
fn prompt_required(prompt: &str) -> String {
    loop {
        let answer: String = Input::new()
            .with_prompt(prompt)
            .interact_text()
            .unwrap_or_default();
        let trimmed = answer.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        println!("This can't be left blank.");
    }
}

/// Prompts for an answer the maintainer can skip. A blank answer means the
/// default applies, so it is returned as `None` rather than the default's
/// own text -- the default itself is resolved downstream in `init::scaffold`.
fn prompt_optional(prompt: &str) -> Option<String> {
    let answer: String = Input::new()
        .with_prompt(prompt)
        .allow_empty(true)
        .interact_text()
        .unwrap_or_default();
    let trimmed = answer.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
