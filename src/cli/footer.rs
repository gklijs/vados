use super::prompt::require_flag;
use clap::Subcommand;
use std::process::ExitCode;
use vados::authoring::menu as menu_authoring;

#[derive(Subcommand)]
pub enum FooterCommand {
    /// Replaces main.json's footer content reference.
    Set {
        #[arg(long)]
        source: String,
        /// A `.md`/`.html` file, or inline HTML.
        #[arg(long)]
        content: Option<String>,
    },
}

pub fn run(command: FooterCommand) -> ExitCode {
    match command {
        FooterCommand::Set { source, content } => run_footer_set(source, content),
    }
}

// -- footer set ------------------------------------------------------------

fn run_footer_set(source: String, content: Option<String>) -> ExitCode {
    let content = match require_flag(
        content,
        "content",
        "Footer content reference (a .md/.html file, or inline HTML)",
    ) {
        Some(c) => c,
        None => return ExitCode::FAILURE,
    };
    match menu_authoring::set_footer(&source, content) {
        Ok(content) => {
            println!("Footer content set to: {content}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("footer set failed while writing the project: {e}");
            ExitCode::FAILURE
        }
    }
}
