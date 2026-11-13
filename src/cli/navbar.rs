use super::prompt::report_change;
use clap::{Subcommand, ValueEnum};
use std::process::ExitCode;
use vados::authoring::site_features;

#[derive(Copy, Clone, ValueEnum)]
pub enum NavbarThemeArg {
    Light,
    Dark,
}

#[derive(Subcommand)]
pub enum NavbarCommand {
    /// Changes the given settings; anything else stays as it is.
    Set {
        #[arg(long)]
        source: String,
        /// What sits on the navbar, for a colour your stylesheet recolours to
        /// the opposite brightness.
        #[arg(long)]
        theme: Option<NavbarThemeArg>,
        /// How many children a dropdown lists before a "more" link.
        #[arg(long = "dropdown-limit", value_parser = clap::value_parser!(u32).range(1..))]
        dropdown_limit: Option<u32>,
    },
}

pub fn run(command: NavbarCommand) -> ExitCode {
    match command {
        NavbarCommand::Set {
            source,
            theme,
            dropdown_limit,
        } => report_change(
            site_features::set_navbar(
                &source,
                theme.map(|t| matches!(t, NavbarThemeArg::Dark)),
                dropdown_limit,
            )
            .map(|_| String::from("The navbar settings are updated.")),
        ),
    }
}
