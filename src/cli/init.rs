use super::prompt::{prompt_favicon, prompt_optional, prompt_optional_site_url, prompt_required};
use std::path::Path;
use std::process::ExitCode;
use vados::init::{self, ProjectBasics, RecognizedSocialProvider, SocialHandle};

pub fn run_init() -> ExitCode {
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
    let language = prompt_optional("Site language, as a BCP 47 tag [default: \"en\"]");
    let favicon = prompt_favicon();
    let site_url = prompt_optional_site_url();

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
        language,
        favicon,
        site_url,
        socials,
    };

    match init::scaffold(dir, basics) {
        Ok(outcome) => {
            println!("\nScaffolded a new vados project:\n");
            println!("  site title:    {}", outcome.site_title);
            println!("  home intro:    {}", outcome.home_intro);
            println!("  primary color: {}", outcome.primary_color);
            println!("  footer text:   {}", outcome.footer_text);
            println!("  language:      {}", outcome.language);
            println!(
                "  favicon:       {}",
                outcome.favicon.as_deref().unwrap_or("none")
            );
            println!(
                "  site URL:      {}",
                outcome.site_url.as_deref().unwrap_or("none yet")
            );
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
