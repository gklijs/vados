use clap::{Parser, Subcommand, ValueEnum};
use dialoguer::Input;
use std::path::Path;
use std::process::ExitCode;
use vados::authoring::menu::{self as menu_authoring, MenuLinkKind};
use vados::authoring::page::{self, NotificationSide, PageImageRole};
use vados::authoring::{image_registry, social};
use vados::check::check;
use vados::generator::generate;
use vados::init::{self, ProjectBasics, RecognizedSocialProvider, SocialHandle};

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
}

#[derive(Subcommand)]
enum PageCommand {
    /// Creates a page.json for a path not yet in the source tree. Anything
    /// not given as a flag is asked for interactively.
    New {
        #[arg(long)]
        source: String,
        /// Only needed together with `--image`, to resolve it.
        #[arg(long = "img-source")]
        img_source: Option<String>,
        /// Where the new page goes, e.g. `/blog/my-post`.
        #[arg(long)]
        path: String,
        /// An already-declared image's reference key to use as this page's
        /// hero image.
        #[arg(long)]
        image: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long = "sub-title")]
        sub_title: Option<String>,
        #[arg(long)]
        icon: Option<String>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        order: Option<u32>,
        /// A `.md`/`.html` file, inline HTML, or omit for a bare heading.
        #[arg(long)]
        content: Option<String>,
    },
    /// Attaches an image -- freshly registered or already declared -- to an
    /// existing page, as its hero image or as a notification. The first
    /// image attached to a page becomes its hero; every one after becomes a
    /// notification, unless `--as` overrides that.
    AddImage {
        #[arg(long)]
        source: String,
        #[arg(long = "img-source")]
        img_source: String,
        /// The page to attach the image to, e.g. `/blog/my-post`.
        #[arg(long)]
        path: String,
        /// Together with `--file-name`: registers a new image from this
        /// directory (relative to `--img-source`; omit for the image root).
        #[arg(long)]
        dir: Option<String>,
        /// Together with `--dir`: the file to register as a new image.
        #[arg(long = "file-name")]
        file_name: Option<String>,
        /// An already-declared image's reference key, instead of
        /// registering a new one. Mutually exclusive with `--dir`/`--file-name`.
        #[arg(long = "existing-reference")]
        existing_reference: Option<String>,
        /// Overrides the automatic hero/notification choice.
        #[arg(long = "as", value_enum)]
        as_role: Option<PageImageRoleArg>,
        /// Required when registering a new image.
        #[arg(long = "alt-text")]
        alt_text: Option<String>,
        /// Only asked about when the attachment becomes a notification.
        #[arg(long)]
        caption: Option<String>,
        /// Which side a notification is added to. Defaults to `right`.
        #[arg(long, value_enum)]
        side: Option<NotificationSideArg>,
    },
}

#[derive(Subcommand)]
enum ImageCommand {
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

#[derive(Subcommand)]
enum SocialCommand {
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

#[derive(Subcommand)]
enum FooterCommand {
    /// Replaces main.json's footer content reference.
    Set {
        #[arg(long)]
        source: String,
        /// A `.md`/`.html` file, or inline HTML.
        #[arg(long)]
        content: Option<String>,
    },
}

#[derive(Subcommand)]
enum MenuCommand {
    /// Appends one entry to menu.json's main menu. There is no
    /// `remove-item`/`update-item` yet -- edit menu.json by hand for that.
    AddItem {
        #[arg(long)]
        source: String,
        /// A site-relative path (e.g. `/blog`) or an `https://` url.
        #[arg(long)]
        url: String,
        /// Required for an external (`https://`) url; optional otherwise,
        /// where it falls back to the target page's own title.
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        icon: Option<String>,
    },
}

/// The CLI-facing spelling of `RecognizedSocialProvider`, kept as its own
/// type so `clap`'s `ValueEnum` derive can own its argument-parsing concerns
/// separately from the domain type it converts to.
#[derive(Clone, Copy, ValueEnum)]
enum SocialProviderArg {
    Github,
    Linkedin,
    Facebook,
    Youtube,
    Twitter,
}

impl From<SocialProviderArg> for RecognizedSocialProvider {
    fn from(value: SocialProviderArg) -> Self {
        match value {
            SocialProviderArg::Github => RecognizedSocialProvider::Github,
            SocialProviderArg::Linkedin => RecognizedSocialProvider::LinkedIn,
            SocialProviderArg::Facebook => RecognizedSocialProvider::Facebook,
            SocialProviderArg::Youtube => RecognizedSocialProvider::YouTube,
            SocialProviderArg::Twitter => RecognizedSocialProvider::Twitter,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum PageImageRoleArg {
    Hero,
    Notification,
}

impl From<PageImageRoleArg> for PageImageRole {
    fn from(value: PageImageRoleArg) -> Self {
        match value {
            PageImageRoleArg::Hero => PageImageRole::Hero,
            PageImageRoleArg::Notification => PageImageRole::Notification,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum NotificationSideArg {
    Left,
    Right,
}

impl From<NotificationSideArg> for NotificationSide {
    fn from(value: NotificationSideArg) -> Self {
        match value {
            NotificationSideArg::Left => NotificationSide::Left,
            NotificationSideArg::Right => NotificationSide::Right,
        }
    }
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
        Command::Page { command } => match command {
            PageCommand::New {
                source,
                img_source,
                path,
                image,
                title,
                sub_title,
                icon,
                summary,
                order,
                content,
            } => run_page_new(
                source, img_source, path, image, title, sub_title, icon, summary, order, content,
            ),
            PageCommand::AddImage {
                source,
                img_source,
                path,
                dir,
                file_name,
                existing_reference,
                as_role,
                alt_text,
                caption,
                side,
            } => run_page_add_image(
                source,
                img_source,
                path,
                dir,
                file_name,
                existing_reference,
                as_role.map(Into::into),
                alt_text,
                caption,
                side.map(Into::into),
            ),
        },
        Command::Image { command } => match command {
            ImageCommand::Add {
                img_source,
                dir,
                file_name,
                title,
                alt_text,
            } => run_image_add(img_source, dir, file_name, title, alt_text),
        },
        Command::Social { command } => match command {
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
        },
        Command::Footer { command } => match command {
            FooterCommand::Set { source, content } => run_footer_set(source, content),
        },
        Command::Menu { command } => match command {
            MenuCommand::AddItem {
                source,
                url,
                title,
                icon,
            } => run_menu_add_item(source, url, title, icon),
        },
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
    let language = prompt_optional("Site language, as a BCP 47 tag [default: \"en\"]");

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

// -- page new / page add-image -----------------------------------------

#[allow(clippy::too_many_arguments)]
fn run_page_new(
    source: String,
    img_source: Option<String>,
    path: String,
    image: Option<String>,
    title: Option<String>,
    sub_title: Option<String>,
    icon: Option<String>,
    summary: Option<String>,
    order: Option<u32>,
    content: Option<String>,
) -> ExitCode {
    if image.is_some() && img_source.is_none() {
        eprintln!("`--image` needs `--img-source` to look it up against.");
        return ExitCode::FAILURE;
    }

    // Every blocker is checked in one pass, before the maintainer is asked
    // for a title or anything else; see vados.allium's
    // `DetectPageCreationBlockers`.
    let blocks = page::detect_page_creation_blockers(
        &source,
        img_source.as_deref(),
        &path,
        image.as_deref(),
    );
    if !blocks.is_empty() {
        println!("`page new` can't create {path} yet:\n");
        for b in &blocks {
            println!("  {:<32} {}", b.reason, b.detail);
        }
        return ExitCode::FAILURE;
    }

    let title = match require_flag(title, "title", "Title") {
        Some(t) => t,
        None => return ExitCode::FAILURE,
    };
    let sub_title = optional_flag(sub_title, "Subtitle");
    let icon = optional_flag(icon, "Icon (Material Design Icons name)");
    let summary = optional_flag(summary, "Summary");
    let content = optional_flag(
        content,
        "Content reference (a .md/.html file, inline HTML, or leave blank for a bare heading)",
    );

    match page::create_page(
        &source,
        &path,
        image,
        page::PageCreationDetails {
            title,
            sub_title,
            icon,
            summary,
            order,
            content,
        },
    ) {
        Ok(outcome) => {
            println!("\nCreated page {}:", outcome.path);
            println!("  title:   {}", outcome.title);
            if let Some(sub_title) = &outcome.sub_title {
                println!("  subtitle: {}", sub_title);
            }
            println!(
                "  content: {}{}",
                outcome.content,
                if outcome.content_was_defaulted {
                    " (defaulted)"
                } else {
                    ""
                }
            );
            if let Some(image) = &outcome.image_reference {
                println!("  image:   {}", image);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("page new failed while writing the project: {e}");
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_page_add_image(
    source: String,
    img_source: String,
    path: String,
    dir: Option<String>,
    file_name: Option<String>,
    existing_reference: Option<String>,
    given_as: Option<PageImageRole>,
    alt_text: Option<String>,
    caption: Option<String>,
    side: Option<NotificationSide>,
) -> ExitCode {
    if existing_reference.is_some() && (dir.is_some() || file_name.is_some()) {
        eprintln!(
            "Give either --existing-reference, or --dir/--file-name for a new image, not both."
        );
        return ExitCode::FAILURE;
    }
    // Alt text belongs to the image itself, not to one attachment of it, so
    // an already-declared image keeps the alt text it was registered with --
    // `--alt-text` has nothing to attach to here and would otherwise be
    // silently discarded rather than doing what it looks like it does.
    if existing_reference.is_some() && alt_text.is_some() {
        eprintln!(
            "`--alt-text` is not accepted with `--existing-reference`; the declared image's own alt text is used. Use `--caption` to override what's shown here."
        );
        return ExitCode::FAILURE;
    }

    let given_source = if let Some(reference) = existing_reference {
        page::GivenImageSource::Existing { reference }
    } else if let Some(file_name) = file_name {
        page::GivenImageSource::New {
            dir: dir.unwrap_or_default(),
            file_name,
        }
    } else {
        // Neither named as a flag: ask which way the maintainer wants to
        // name the image, the same "gather whatever wasn't a flag" pattern
        // as every other missing piece.
        match optional_flag(
            None,
            "Existing image reference (leave blank to register a new image from a file instead)",
        ) {
            Some(reference) => page::GivenImageSource::Existing { reference },
            None => {
                let file_name = match require_flag(
                    None,
                    "file-name",
                    "Image file name (relative to --img-source, or --dir)",
                ) {
                    Some(f) => f,
                    None => return ExitCode::FAILURE,
                };
                page::GivenImageSource::New {
                    dir: dir.unwrap_or_default(),
                    file_name,
                }
            }
        }
    };

    let request = page::PageImageAttachmentRequest {
        given_path: path.clone(),
        source: given_source,
        given_as,
    };

    // Every blocker is checked in one pass before the maintainer is asked
    // for anything further; see vados.allium's
    // `DetectPageImageAttachmentBlockers`.
    let blocks = page::detect_attachment_blockers(&source, &img_source, &request);
    if !blocks.is_empty() {
        println!("`page add-image` can't attach an image to {path} yet:\n");
        for b in &blocks {
            println!("  {:<32} {}", b.reason, b.detail);
        }
        return ExitCode::FAILURE;
    }

    // Exposed to the maintainer before asking for alt text/caption, the same
    // way vados.allium's `PageImageAttachmentDetails` surface exposes
    // `effective_role` before `PageImageAttachmentDetailsProvided`.
    let role_preview = page::effective_role(given_as, page::hero_available(&source, &path));

    let alt_text = match &request.source {
        page::GivenImageSource::New { .. } => {
            match require_flag(alt_text, "alt-text", "Alternative text for the new image") {
                Some(a) => Some(a),
                None => return ExitCode::FAILURE,
            }
        }
        page::GivenImageSource::Existing { .. } => None,
    };
    let caption = if role_preview == PageImageRole::Notification {
        optional_flag(caption, "Caption [default: the image's alt text]")
    } else {
        None
    };

    match page::attach_image(
        &source,
        &img_source,
        request,
        page::PageImageAttachmentDetails {
            alt_text,
            side,
            caption,
        },
    ) {
        Ok(outcome) => {
            println!("\nAttached image to {}:", outcome.path);
            println!("  reference: {}", outcome.reference_key);
            println!(
                "  role:      {}",
                match outcome.role {
                    PageImageRole::Hero => "hero",
                    PageImageRole::Notification => "notification",
                }
            );
            if let Some(side) = outcome.side {
                println!(
                    "  side:      {}",
                    match side {
                        NotificationSide::Left => "left",
                        NotificationSide::Right => "right",
                    }
                );
            }
            if let Some(content) = &outcome.content {
                println!("  caption:   {}", content);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("page add-image failed while writing the project: {e}");
            ExitCode::FAILURE
        }
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
        (Some(provider), Some(handle), None) => Ok(social::GivenSocialLink {
            handle: Some((provider.into(), handle)),
            url: None,
            icon,
            brand_color,
            label,
        }),
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

// -- menu add-item ---------------------------------------------------------

fn run_menu_add_item(
    source: String,
    url: String,
    title: Option<String>,
    icon: Option<String>,
) -> ExitCode {
    let title =
        if menu_authoring::menu_link_kind_of(&url) == MenuLinkKind::External && title.is_none() {
            // Genuinely required here -- `add_menu_link` rejects an external
            // link with no title outright -- so this loops/fails like every
            // other required-conditional-on-context field (`run_page_new`'s
            // `title`, `run_footer_set`'s `content`), rather than
            // `optional_flag`, which would accept a blank answer only for
            // the rejection to arrive right after.
            match require_flag(None, "title", "Title (required for an external link)") {
                Some(t) => Some(t),
                None => return ExitCode::FAILURE,
            }
        } else {
            title
        };

    match menu_authoring::add_menu_link(&source, url, title, icon) {
        Ok(Ok(outcome)) => {
            println!("\nAdded menu link {}:", outcome.url);
            println!(
                "  kind:          {}",
                match outcome.kind {
                    MenuLinkKind::Internal => "internal",
                    MenuLinkKind::External => "external",
                }
            );
            println!("  would resolve: {}", outcome.would_resolve);
            ExitCode::SUCCESS
        }
        Ok(Err(reason)) => {
            eprintln!("menu add-item rejected: {reason}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("menu add-item failed while writing the project: {e}");
            ExitCode::FAILURE
        }
    }
}

// -- shared flag-or-prompt helpers -----------------------------------------
//
// Every content-authoring command accepts every `given_*` field from
// vados.allium as a flag; whatever the maintainer didn't supply that way is
// asked for interactively instead, the same pattern `init` already uses for
// its own basics. See vados.allium's Excludes section on why the prompt
// sequence itself isn't domain.

/// Resolves a required field: the flag if given, otherwise a prompt -- or a
/// clear failure if no terminal is available to ask on.
fn require_flag(value: Option<String>, flag: &str, prompt: &str) -> Option<String> {
    if let Some(v) = value {
        return Some(v);
    }
    if !dialoguer::console::user_attended() {
        eprintln!("`--{flag}` is required; no interactive terminal is available to ask for it.");
        return None;
    }
    Some(prompt_required(prompt))
}

/// Resolves an optional field: the flag if given, otherwise a prompt the
/// maintainer can leave blank. Silently stays `None` without a terminal,
/// rather than failing the whole command over an optional field.
fn optional_flag(value: Option<String>, prompt: &str) -> Option<String> {
    if value.is_some() {
        return value;
    }
    if !dialoguer::console::user_attended() {
        return None;
    }
    prompt_optional(prompt)
}

/// Prompts until a non-blank answer is given. Used only where there truly is
/// no default -- a required flag the maintainer didn't supply.
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
/// own text -- the default itself is resolved downstream.
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
