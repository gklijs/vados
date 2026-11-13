use dialoguer::Input;
use std::path::PathBuf;
use std::process::ExitCode;
use vados::feed::is_site_url;
use vados::init;
use vados::page_date::PageDate;

/// Prints what a change did, or why it couldn't be made.
pub fn report_change(result: Result<String, String>) -> ExitCode {
    match result {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(problem) => {
            eprintln!("{problem}");
            ExitCode::FAILURE
        }
    }
}

/// Asks for the site URL until an absolute http(s) one is given.
pub fn prompt_site_url() -> String {
    loop {
        let answer = prompt_required("Site URL, e.g. https://example.com");
        if is_site_url(&answer) {
            return answer;
        }
        println!("  That isn't an absolute http(s) URL; try again.");
    }
}

/// Asks for the site URL, which can be left blank to set later.
pub fn prompt_optional_site_url() -> Option<String> {
    loop {
        let answer = prompt_optional(
            "Site URL, e.g. https://example.com, needed for feeds [leave blank to set later]",
        )?;
        if is_site_url(&answer) {
            return Some(answer);
        }
        println!("  That isn't an absolute http(s) URL; give another, or leave it blank.");
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
pub fn require_flag(value: Option<String>, flag: &str, prompt: &str) -> Option<String> {
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
pub fn optional_flag(value: Option<String>, prompt: &str) -> Option<String> {
    if value.is_some() {
        return value;
    }
    if !dialoguer::console::user_attended() {
        return None;
    }
    prompt_optional(prompt)
}

/// Asks for a page date, offering today's; without a terminal, takes
/// today's outright.
pub fn prompt_date() -> PageDate {
    let today = PageDate::today();
    if !dialoguer::console::user_attended() {
        return today;
    }
    Input::new()
        .with_prompt("Date (YYYY-MM-DD)")
        .default(today)
        .interact_text()
        .unwrap_or(today)
}

/// Prompts until a non-blank answer is given. Used only where there truly is
/// no default -- a required flag the maintainer didn't supply.
pub fn prompt_required(prompt: &str) -> String {
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

/// Asks for a favicon until it gets one `init::check_favicon` accepts, or a
/// blank answer to skip it. Quotes around the path are dropped, since a
/// terminal adds them when a file is dragged onto it.
pub fn prompt_favicon() -> Option<PathBuf> {
    loop {
        let answer =
            prompt_optional("Favicon: path to an SVG, PNG or ICO file [leave blank to skip]")?;
        let path = PathBuf::from(answer.trim_matches(|c| c == '"' || c == '\''));
        match init::check_favicon(&path) {
            Ok(()) => return Some(path),
            Err(problem) => println!("  {problem}; give another path, or leave it blank to skip."),
        }
    }
}

/// Prompts for an answer the maintainer can skip. A blank answer means the
/// default applies, so it is returned as `None` rather than the default's
/// own text -- the default itself is resolved downstream.
pub fn prompt_optional(prompt: &str) -> Option<String> {
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
