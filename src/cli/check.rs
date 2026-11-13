use std::process::ExitCode;
use vados::check::check;

pub fn run(source: String, img_source: String, output: Option<String>, external: bool) -> ExitCode {
    if !external_is_available(external) {
        return ExitCode::FAILURE;
    }
    let mut report = check(&source, &img_source);
    if let Some(output) = output {
        report
            .findings
            .extend(check_output_of(&source, &output, external));
    }
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

/// The link check of a published site, as the source's `main.json` describes
/// that site. A main.json that can't be read leaves it without a site URL;
/// `check` reports that problem itself.
pub fn check_output_of(source: &str, output: &str, external: bool) -> Vec<vados::check::Finding> {
    let site_url = vados::check::usable_site_url(source);
    let findings = vados::link_check::check_output(output, site_url.as_deref());
    #[cfg(feature = "external-links")]
    if external {
        let mut findings = findings;
        findings.extend(vados::link_check::check_external(
            output,
            site_url.as_deref(),
        ));
        return findings;
    }
    #[cfg(not(feature = "external-links"))]
    let _ = external;
    findings
}

/// Refuses `--external` up front when this build can't do it, rather than
/// generating a site and then quietly checking less than was asked for.
pub fn external_is_available(external: bool) -> bool {
    let available = cfg!(feature = "external-links");
    if external && !available {
        eprintln!("--external needs vados built with the `external-links` feature, which is on by default.");
    }
    available || !external
}
