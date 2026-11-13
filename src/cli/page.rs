use super::prompt::{optional_flag, prompt_date, prompt_site_url, report_change, require_flag};
use clap::{Subcommand, ValueEnum};
use std::process::ExitCode;
use vados::authoring::feed as feed_authoring;
use vados::authoring::luma::{self as luma_authoring, LumaEventDisplay};
use vados::authoring::page::{self, ChildOrder, NotificationSide, PageImageRole};
use vados::authoring::site_features::{
    self, GivenAttendance, GivenEvent, GivenFeedSelection, GivenStatus,
};
use vados::page_date::PageDate;

#[derive(Subcommand)]
pub enum PageCommand {
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
        /// Publication date, `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`. Defaults to
        /// today when the parent page orders its children by date.
        #[arg(long)]
        date: Option<PageDate>,
        /// How the new page orders its own children: by `order` (the
        /// default), or by date, newest or oldest first -- e.g. for a blog.
        #[arg(long = "child-order")]
        child_order: Option<ChildOrderArg>,
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
    /// Links an event hosted on Luma to an existing page, shown in its main
    /// area after the page's own content.
    AddLumaEvent {
        #[arg(long)]
        source: String,
        /// The page to link the event from, e.g. `/events`.
        #[arg(long)]
        path: String,
        /// Luma's event ID, `evt-...` (found under the event's embed options).
        #[arg(long = "event-id")]
        event_id: Option<String>,
        /// What the event is called; names the embedded page for screen
        /// readers and is the text of the link to the event.
        #[arg(long)]
        title: Option<String>,
        /// How the event is shown. Defaults to `event-page`.
        #[arg(long, value_enum)]
        display: Option<LumaEventDisplayArg>,
        /// The register button's text. Defaults to "Register".
        #[arg(long = "button-label")]
        button_label: Option<String>,
        /// When it starts, with a UTC offset. Lists it in agendas and their
        /// calendars; Luma is never asked. Needs --time-zone.
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        /// The IANA zone it happens in, e.g. `Europe/Amsterdam`.
        #[arg(long = "time-zone")]
        time_zone: Option<String>,
    },
    /// Publishes an Atom feed of a page's dated children, e.g. a blog's
    /// posts, newest first.
    EnableFeed {
        #[arg(long)]
        source: String,
        /// The page whose children the feed lists, e.g. `/blog`.
        #[arg(long)]
        path: String,
        /// The feed's title. Defaults to the site title plus the page's.
        #[arg(long)]
        title: Option<String>,
        /// How many of the newest pages the feed holds. Defaults to 20.
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
        limit: Option<u32>,
        /// Where the site is published, e.g. `https://example.com`; written to
        /// main.json. Asked for when main.json has none yet.
        #[arg(long = "site-url")]
        site_url: Option<String>,
        /// List pages below this one instead of the page's own children; give
        /// it more than once for several.
        #[arg(long)]
        from: Vec<String>,
        /// How many levels below `--from` to look. Without `--from` the feed
        /// looks below its own page, one level deep unless this says more.
        #[arg(long)]
        depth: Option<u32>,
        /// Also list pages with `"listed": false`.
        #[arg(long = "include-unlisted")]
        include_unlisted: bool,
        /// Entries carry their summary instead of their full content.
        #[arg(long = "summaries-only")]
        summaries_only: bool,
    },
    /// Makes a page an event, replacing any event it has. Details left out are
    /// shown as "not confirmed". Changing an event raises its sequence.
    SetEvent {
        #[arg(long)]
        source: String,
        #[arg(long)]
        path: String,
        /// With a UTC offset, e.g. `2026-11-05T14:00:00-05:00`.
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        /// The IANA zone the event happens in, e.g. `America/New_York`.
        #[arg(long = "time-zone")]
        time_zone: Option<String>,
        #[arg(long)]
        attendance: Option<AttendanceArg>,
        #[arg(long)]
        status: Option<EventStatusArg>,
        #[arg(long)]
        organiser: Option<String>,
        #[arg(long)]
        venue: Option<String>,
        #[arg(long)]
        address: Option<String>,
        #[arg(long)]
        cost: Option<String>,
        #[arg(long = "registration-url")]
        registration_url: Option<String>,
        #[arg(long = "virtual-url")]
        virtual_url: Option<String>,
        /// Overrides the sequence calendars use to spot a changed event.
        #[arg(long)]
        sequence: Option<u32>,
    },
    /// Lists events on a page, soonest first, with those that ended hidden.
    EnableAgenda {
        #[arg(long)]
        source: String,
        #[arg(long)]
        path: String,
        /// List events below this page instead of the page itself; give it
        /// more than once for several.
        #[arg(long)]
        from: Vec<String>,
        #[arg(long)]
        depth: Option<u32>,
        /// Also list events on pages with `"listed": false`.
        #[arg(long = "include-unlisted")]
        include_unlisted: bool,
    },
    /// Keeps a page out of search results and the sitemap.
    SetNoindex {
        #[arg(long)]
        source: String,
        #[arg(long)]
        path: String,
        /// Lets search engines index the page again.
        #[arg(long)]
        off: bool,
    },
}

#[derive(Copy, Clone, ValueEnum)]
pub enum AttendanceArg {
    Online,
    InPerson,
    Mixed,
}

#[derive(Copy, Clone, ValueEnum)]
pub enum EventStatusArg {
    Scheduled,
    Cancelled,
    Postponed,
    Rescheduled,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum PageImageRoleArg {
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
pub enum ChildOrderArg {
    Order,
    DateDesc,
    DateAsc,
}

impl From<ChildOrderArg> for ChildOrder {
    fn from(value: ChildOrderArg) -> Self {
        match value {
            ChildOrderArg::Order => ChildOrder::Order,
            ChildOrderArg::DateDesc => ChildOrder::DateDesc,
            ChildOrderArg::DateAsc => ChildOrder::DateAsc,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum LumaEventDisplayArg {
    EventPage,
    RegisterButton,
    Both,
}

impl From<LumaEventDisplayArg> for LumaEventDisplay {
    fn from(value: LumaEventDisplayArg) -> Self {
        match value {
            LumaEventDisplayArg::EventPage => LumaEventDisplay::EventPage,
            LumaEventDisplayArg::RegisterButton => LumaEventDisplay::RegisterButton,
            LumaEventDisplayArg::Both => LumaEventDisplay::Both,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum NotificationSideArg {
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

pub fn run(command: PageCommand) -> ExitCode {
    match command {
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
            date,
            child_order,
            content,
        } => run_page_new(
            source,
            img_source,
            path,
            image,
            title,
            sub_title,
            icon,
            summary,
            order,
            date,
            child_order.map(ChildOrder::from),
            content,
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
        PageCommand::AddLumaEvent {
            source,
            path,
            event_id,
            title,
            display,
            button_label,
            start,
            end,
            time_zone,
        } => run_page_add_luma_event(
            source,
            path,
            event_id,
            title,
            display.map(Into::into),
            button_label,
            (start, end, time_zone),
        ),
        PageCommand::EnableFeed {
            source,
            path,
            title,
            limit,
            site_url,
            from,
            depth,
            include_unlisted,
            summaries_only,
        } => run_page_enable_feed(
            source,
            path,
            title,
            limit,
            site_url,
            GivenFeedSelection {
                from,
                depth,
                include_unlisted,
                summaries_only,
            },
        ),
        PageCommand::SetEvent {
            source,
            path,
            start,
            end,
            time_zone,
            attendance,
            status,
            organiser,
            venue,
            address,
            cost,
            registration_url,
            virtual_url,
            sequence,
        } => {
            let Some(start) = require_flag(
                start,
                "start",
                "Start, with a UTC offset (2026-11-05T14:00:00-05:00)",
            ) else {
                return ExitCode::FAILURE;
            };
            let Some(time_zone) =
                require_flag(time_zone, "time-zone", "IANA time zone (Europe/Amsterdam)")
            else {
                return ExitCode::FAILURE;
            };
            let given = GivenEvent {
                start,
                end,
                time_zone,
                attendance: attendance.map(|a| match a {
                    AttendanceArg::Online => GivenAttendance::Online,
                    AttendanceArg::InPerson => GivenAttendance::InPerson,
                    AttendanceArg::Mixed => GivenAttendance::Mixed,
                }),
                status: status.map(|s| match s {
                    EventStatusArg::Scheduled => GivenStatus::Scheduled,
                    EventStatusArg::Cancelled => GivenStatus::Cancelled,
                    EventStatusArg::Postponed => GivenStatus::Postponed,
                    EventStatusArg::Rescheduled => GivenStatus::Rescheduled,
                }),
                organiser,
                venue,
                address,
                cost,
                registration_url,
                virtual_url,
                sequence,
            };
            report_change(
                site_features::set_event(&source, &path, &given).map(|sequence| {
                    format!("{path} is now an event (calendar sequence {sequence}).")
                }),
            )
        }
        PageCommand::EnableAgenda {
            source,
            path,
            from,
            depth,
            include_unlisted,
        } => report_change(
            site_features::enable_agenda(&source, &path, from, depth, include_unlisted)
                .map(|_| format!("{path} now shows an agenda.")),
        ),
        PageCommand::SetNoindex { source, path, off } => {
            report_change(site_features::set_noindex(&source, &path, !off).map(|_| {
                if off {
                    format!("{path} can be indexed again.")
                } else {
                    format!("{path} is kept out of search results and the sitemap.")
                }
            }))
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
    date: Option<PageDate>,
    child_order: Option<ChildOrder>,
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
    // A page in a section ordered by date needs one to be placed at all, so
    // it gets today's unless told otherwise; elsewhere a date stays opt-in.
    let date = match date {
        Some(d) => Some(d),
        None if page::parent_orders_by_date(&source, &path) => Some(prompt_date()),
        None => None,
    };
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
            date,
            child_order,
            content,
        },
    ) {
        Ok(outcome) => {
            println!("\nCreated page {}:", outcome.path);
            println!("  title:   {}", outcome.title);
            if let Some(sub_title) = &outcome.sub_title {
                println!("  subtitle: {}", sub_title);
            }
            if let Some(date) = &outcome.date {
                println!("  date:    {}", date);
            }
            if let Some(child_order) = &outcome.child_order {
                println!(
                    "  children: {}",
                    match child_order {
                        ChildOrder::Order => "by order",
                        ChildOrder::DateDesc => "by date, newest first",
                        ChildOrder::DateAsc => "by date, oldest first",
                    }
                );
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

fn run_page_add_luma_event(
    source: String,
    path: String,
    event_id: Option<String>,
    title: Option<String>,
    display: Option<LumaEventDisplay>,
    button_label: Option<String>,
    (start, end, time_zone): (Option<String>, Option<String>, Option<String>),
) -> ExitCode {
    let problems = luma_authoring::schedule_problems(start.clone(), end.clone(), time_zone.clone());
    if !problems.is_empty() {
        eprintln!("{}", problems.join("; "));
        return ExitCode::FAILURE;
    }
    let event_id = match require_flag(event_id, "event-id", "Luma event ID (evt-...)") {
        Some(id) => id.trim().to_string(),
        None => return ExitCode::FAILURE,
    };

    // Every blocker is checked in one pass before the maintainer is asked
    // for a title; see vados.allium's `DetectLumaEventAttachmentBlockers`.
    let blocks = luma_authoring::detect_luma_event_blockers(&source, &path, &event_id);
    if !blocks.is_empty() {
        println!("`page add-luma-event` can't link a Luma event to {path} yet:\n");
        for b in &blocks {
            println!("  {:<40} {}", b.reason, b.detail);
        }
        return ExitCode::FAILURE;
    }

    let title = match require_flag(title, "title", "Event title") {
        Some(t) => t,
        None => return ExitCode::FAILURE,
    };

    match luma_authoring::attach_luma_event(
        &source,
        &path,
        &event_id,
        luma_authoring::LumaEventAttachmentDetails {
            title,
            display,
            button_label,
            start,
            end,
            time_zone,
        },
    ) {
        Ok(outcome) => {
            println!("\nLinked Luma event to {}:", outcome.path);
            println!("  event:   {}", outcome.event_id);
            println!("  title:   {}", outcome.title);
            println!("  display: {}", outcome.display);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("page add-luma-event failed while writing the project: {e}");
            ExitCode::FAILURE
        }
    }
}

// -- page enable-feed --------------------------------------------------------

fn run_page_enable_feed(
    source: String,
    path: String,
    title: Option<String>,
    limit: Option<u32>,
    site_url: Option<String>,
    selection: GivenFeedSelection,
) -> ExitCode {
    // Every blocker is checked in one pass before anything is asked; see
    // vados.allium's `DetectFeedEnablementBlockers`.
    let blocks =
        feed_authoring::detect_feed_enablement_blockers(&source, &path, site_url.as_deref());
    if !blocks.is_empty() {
        println!("`page enable-feed` can't publish a feed for {path} yet:\n");
        for b in &blocks {
            println!("  {:<40} {}", b.reason, b.detail);
        }
        return ExitCode::FAILURE;
    }

    // A feed can't give its links in full without the site URL, so it's
    // required exactly when main.json doesn't have a usable one yet.
    let site_url = match site_url {
        Some(url) => Some(url),
        None if feed_authoring::usable_site_url(&source).is_none() => {
            if !dialoguer::console::user_attended() {
                eprintln!(
                    "main.json has no siteUrl yet, so `--site-url` is required; no interactive terminal is available to ask for it."
                );
                return ExitCode::FAILURE;
            }
            println!("A feed needs the address the site is published at.");
            Some(prompt_site_url())
        }
        None => None,
    };

    match feed_authoring::enable_feed(
        &source,
        &path,
        feed_authoring::FeedEnablementDetails {
            title,
            limit,
            site_url,
            selection,
        },
    ) {
        Ok(outcome) => {
            println!("\nEnabled the feed of {}:", outcome.path);
            if let Some(url) = &outcome.feed_url {
                println!("  feed:     {}", url);
            }
            if let Some(title) = &outcome.title {
                println!("  title:    {}", title);
            }
            println!("  holds:    the {} newest dated pages", outcome.limit);
            if let Some(url) = &outcome.site_url_written {
                println!("  site URL: {} (written to main.json)", url);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("page enable-feed failed while writing the project: {e}");
            ExitCode::FAILURE
        }
    }
}
