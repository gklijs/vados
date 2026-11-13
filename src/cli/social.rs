use super::prompt::{optional_flag, require_flag};
use clap::{Subcommand, ValueEnum};
use std::process::ExitCode;
use vados::authoring::social;
use vados::init::RecognizedSocialProvider;

#[derive(Subcommand)]
pub enum SocialCommand {
    /// Adds a new social link. Name it either with `--provider`/`--handle`,
    /// or with `--url` (plus `--icon`/`--brand-color`/`--label` for a
    /// provider vados doesn't recognise automatically).
    Add {
        #[arg(long)]
        source: String,
        #[arg(long, value_enum)]
        provider: Option<SocialProviderArg>,
        #[arg(long)]
        handle: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        icon: Option<String>,
        #[arg(long = "brand-color")]
        brand_color: Option<String>,
        /// The link's accessible name -- required for an unrecognised
        /// provider, since it's rendered as an icon with no visible text.
        #[arg(long)]
        label: Option<String>,
    },
    /// Replaces an existing social link wholesale with a new one.
    Update {
        #[arg(long)]
        source: String,
        /// A substring of the existing link's url.
        #[arg(long = "match")]
        match_value: Option<String>,
        /// Disambiguates when more than one link matches; ranked among the
        /// matches, not the full list.
        #[arg(long)]
        index: Option<i64>,
        #[arg(long, value_enum)]
        provider: Option<SocialProviderArg>,
        #[arg(long)]
        handle: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        icon: Option<String>,
        #[arg(long = "brand-color")]
        brand_color: Option<String>,
        /// The link's accessible name -- required for an unrecognised
        /// provider, since it's rendered as an icon with no visible text.
        #[arg(long)]
        label: Option<String>,
    },
    /// Removes an existing social link.
    Remove {
        #[arg(long)]
        source: String,
        /// A substring of the existing link's url.
        #[arg(long = "match")]
        match_value: Option<String>,
        /// Disambiguates when more than one link matches; ranked among the
        /// matches, not the full list.
        #[arg(long)]
        index: Option<i64>,
    },
}

/// The CLI-facing spelling of `RecognizedSocialProvider`, kept as its own
/// type so `clap`'s `ValueEnum` derive can own its argument-parsing concerns
/// separately from the domain type it converts to.
#[derive(Clone, Copy, ValueEnum)]
pub enum SocialProviderArg {
    Github,
    Linkedin,
    Facebook,
    Youtube,
    Twitter,
    /// Not recognised from a url, so a Mastodon link is written with its
    /// icon, brand color and label; the handle is `user@instance`.
    Mastodon,
}

impl SocialProviderArg {
    fn recognized(self) -> Option<RecognizedSocialProvider> {
        match self {
            SocialProviderArg::Github => Some(RecognizedSocialProvider::Github),
            SocialProviderArg::Linkedin => Some(RecognizedSocialProvider::LinkedIn),
            SocialProviderArg::Facebook => Some(RecognizedSocialProvider::Facebook),
            SocialProviderArg::Youtube => Some(RecognizedSocialProvider::YouTube),
            SocialProviderArg::Twitter => Some(RecognizedSocialProvider::Twitter),
            SocialProviderArg::Mastodon => None,
        }
    }
}

pub fn run(command: SocialCommand) -> ExitCode {
    match command {
        SocialCommand::Add {
            source,
            provider,
            handle,
            url,
            icon,
            brand_color,
            label,
        } => run_social_add(source, provider, handle, url, icon, brand_color, label),
        SocialCommand::Update {
            source,
            match_value,
            index,
            provider,
            handle,
            url,
            icon,
            brand_color,
            label,
        } => run_social_update(
            source,
            match_value,
            index,
            provider,
            handle,
            url,
            icon,
            brand_color,
            label,
        ),
        SocialCommand::Remove {
            source,
            match_value,
            index,
        } => run_social_remove(source, match_value, index),
    }
}

// -- social add / update / remove ----------------------------------------

/// Resolves how the maintainer named a social link's target: `--provider`
/// with `--handle`, or `--url` alone. Falls back to asking for a url --
/// rather than walking through provider selection -- when neither was given
/// as a flag; a url alone is enough to classify the provider anyway.
#[allow(clippy::too_many_arguments)]
fn gather_given_social_link(
    provider: Option<SocialProviderArg>,
    handle: Option<String>,
    url: Option<String>,
    icon: Option<String>,
    brand_color: Option<String>,
    label: Option<String>,
) -> Result<social::GivenSocialLink, ExitCode> {
    match (provider, handle, url) {
        (Some(provider), Some(handle), None) => match provider.recognized() {
            Some(recognized) => Ok(social::GivenSocialLink {
                handle: Some((recognized, handle)),
                url: None,
                icon,
                brand_color,
                label,
            }),
            None => social::mastodon_link(&handle, icon, brand_color, label).ok_or_else(|| {
                eprintln!("A Mastodon handle looks like user@instance, e.g. gklijs@hachyderm.io.");
                ExitCode::FAILURE
            }),
        },
        (Some(_), _, Some(_)) => {
            eprintln!("Give either --provider/--handle, or --url, not both.");
            Err(ExitCode::FAILURE)
        }
        (Some(_), None, None) | (None, Some(_), None) => {
            eprintln!("Give both --provider and --handle together, or use --url instead.");
            Err(ExitCode::FAILURE)
        }
        (None, _, Some(url)) => Ok(finish_url_given_social_link(url, icon, brand_color, label)),
        (None, None, None) => {
            let url = match require_flag(None, "url", "Social profile url") {
                Some(u) => u,
                None => return Err(ExitCode::FAILURE),
            };
            Ok(finish_url_given_social_link(url, icon, brand_color, label))
        }
    }
}

fn finish_url_given_social_link(
    url: String,
    icon: Option<String>,
    brand_color: Option<String>,
    label: Option<String>,
) -> social::GivenSocialLink {
    let (icon, brand_color, label) =
        if social::classify_social_provider(&url) == social::SocialProviderKind::Other {
            (
                optional_flag(
                    icon,
                    "Icon (Material Design Icons name) for this custom provider",
                ),
                optional_flag(brand_color, "Brand color (hex) for this custom provider"),
                optional_flag(
                    label,
                    "Accessible name (e.g. \"Mastodon\") for this custom provider",
                ),
            )
        } else {
            (icon, brand_color, label)
        };
    social::GivenSocialLink {
        handle: None,
        url: Some(url),
        icon,
        brand_color,
        label,
    }
}

fn print_social_outcome(verb: &str, outcome: &social::SocialLinkChangedOutcome) {
    println!("\n{verb} social link:");
    println!("  url:      {}", outcome.url);
    if let Some(previous) = &outcome.previous_url {
        println!("  previous: {}", previous);
    }
    if let Some(icon) = &outcome.icon {
        println!("  icon:     {}", icon);
    }
    if let Some(color) = &outcome.brand_color {
        println!("  color:    {}", color);
    }
    if let Some(label) = &outcome.label {
        println!("  label:    {}", label);
    }
}

#[allow(clippy::too_many_arguments)]
fn run_social_add(
    source: String,
    provider: Option<SocialProviderArg>,
    handle: Option<String>,
    url: Option<String>,
    icon: Option<String>,
    brand_color: Option<String>,
    label: Option<String>,
) -> ExitCode {
    let given = match gather_given_social_link(provider, handle, url, icon, brand_color, label) {
        Ok(g) => g,
        Err(code) => return code,
    };
    match social::add_social_link(&source, given) {
        Ok(Ok(outcome)) => {
            print_social_outcome("Added", &outcome);
            ExitCode::SUCCESS
        }
        Ok(Err(reason)) => {
            eprintln!("social add rejected: {reason}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("social add failed while writing the project: {e}");
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_social_update(
    source: String,
    match_value: Option<String>,
    index: Option<i64>,
    provider: Option<SocialProviderArg>,
    handle: Option<String>,
    url: Option<String>,
    icon: Option<String>,
    brand_color: Option<String>,
    label: Option<String>,
) -> ExitCode {
    let match_value = match require_flag(
        match_value,
        "match",
        "Which social link to update (a substring of its url)",
    ) {
        Some(m) => m,
        None => return ExitCode::FAILURE,
    };
    let given = match gather_given_social_link(provider, handle, url, icon, brand_color, label) {
        Ok(g) => g,
        Err(code) => return code,
    };
    match social::update_social_link(&source, &match_value, index, given) {
        Ok(Ok(outcome)) => {
            print_social_outcome("Updated", &outcome);
            ExitCode::SUCCESS
        }
        Ok(Err(reason)) => {
            eprintln!("social update rejected: {reason}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("social update failed while writing the project: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_social_remove(source: String, match_value: Option<String>, index: Option<i64>) -> ExitCode {
    let match_value = match require_flag(
        match_value,
        "match",
        "Which social link to remove (a substring of its url)",
    ) {
        Some(m) => m,
        None => return ExitCode::FAILURE,
    };
    match social::remove_social_link(&source, &match_value, index) {
        Ok(Ok(url)) => {
            println!("Removed social link {url}");
            ExitCode::SUCCESS
        }
        Ok(Err(reason)) => {
            eprintln!("social remove rejected: {reason}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("social remove failed while writing the project: {e}");
            ExitCode::FAILURE
        }
    }
}
