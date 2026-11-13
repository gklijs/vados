use super::prompt::{optional_flag, require_flag};
use clap::Subcommand;
use std::process::ExitCode;
use vados::authoring::image_registry;

#[derive(Subcommand)]
pub enum ImageCommand {
    /// Registers one image in an images.json, creating the file if the
    /// directory has none yet.
    Add {
        #[arg(long = "img-source")]
        img_source: String,
        /// Relative to `--img-source`; omit for the image root.
        #[arg(long)]
        dir: Option<String>,
        #[arg(long = "file-name")]
        file_name: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long = "alt-text")]
        alt_text: Option<String>,
    },
}

pub fn run(command: ImageCommand) -> ExitCode {
    match command {
        ImageCommand::Add {
            img_source,
            dir,
            file_name,
            title,
            alt_text,
        } => run_image_add(img_source, dir, file_name, title, alt_text),
    }
}

// -- image add -----------------------------------------------------------

fn run_image_add(
    img_source: String,
    dir: Option<String>,
    file_name: String,
    title: Option<String>,
    alt_text: Option<String>,
) -> ExitCode {
    let dir = dir.unwrap_or_default();

    // Every blocker is checked in one pass before the maintainer is asked
    // for a title or alt text; see vados.allium's
    // `DetectImageRegistrationBlockers`.
    let blocks = image_registry::detect_registration_blockers(&img_source, &dir, &file_name);
    if !blocks.is_empty() {
        println!("`image add` can't register {file_name} yet:\n");
        for b in &blocks {
            println!("  {:<32} {}", b.reason, b.detail);
        }
        return ExitCode::FAILURE;
    }

    let alt_text = match require_flag(alt_text, "alt-text", "Alternative text") {
        Some(a) => a,
        None => return ExitCode::FAILURE,
    };
    let title = optional_flag(title, "Title [default: the file name]");

    match image_registry::register_image(&img_source, &dir, &file_name, title, alt_text) {
        Ok(outcome) => {
            println!("\nRegistered image {}:", outcome.reference_key);
            println!("  title:    {}", outcome.title);
            println!("  alt text: {}", outcome.alt_text);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("image add failed while writing the project: {e}");
            ExitCode::FAILURE
        }
    }
}
