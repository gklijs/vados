use super::check::{check_output_of, external_is_available};
use std::process::ExitCode;
use vados::generator::generate;

pub fn run(
    source: String,
    img_source: String,
    destination: String,
    check_links: bool,
    external: bool,
    jobs: Option<std::num::NonZeroUsize>,
) -> ExitCode {
    if !external_is_available(external) {
        return ExitCode::FAILURE;
    }
    let result = match jobs {
        Some(jobs) => {
            vados::generator::generate_with_jobs(&source, &img_source, &destination, jobs)
        }
        None => generate(&source, &img_source, &destination),
    };
    match result {
        Ok(report) => {
            for warning in &report.warnings {
                println!("{warning}");
            }
            println!("generation passed: {} warning(s)", report.warnings.len());
        }
        Err(error) => {
            eprintln!("{error}");
            eprintln!("generation failed: {} problem(s)", error.findings.len());
            return ExitCode::FAILURE;
        }
    }
    if !check_links {
        return ExitCode::SUCCESS;
    }
    let findings = check_output_of(&source, &destination, external);
    for finding in &findings {
        println!("{}", finding);
    }
    let errors = findings
        .iter()
        .filter(|f| f.severity() == vados::check::Severity::Error)
        .count();
    if errors == 0 {
        println!("link check passed");
        ExitCode::SUCCESS
    } else {
        println!("link check failed: {errors} problem(s)");
        ExitCode::FAILURE
    }
}
