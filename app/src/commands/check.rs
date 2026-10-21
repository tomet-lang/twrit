use std::path::Path;
use std::process::ExitCode;

use super::all_guards;

pub fn check(path: &Path, fix: bool) -> anyhow::Result<ExitCode> {
    let guards = all_guards();

    if fix {
        let writs = twrit_core::writ::discover(path)?;
        let api_rules = twrit_core::writ::twrit_rules(&writs, "rust-api")?;
        for rule in api_rules {
            let count = twrit_rust::api::generate(&rule, path)?;
            println!(
                "Generated API documentation for {count} crate(s) (rule: {})",
                rule.id
            );
        }

        let py_api_rules = twrit_core::writ::twrit_rules(&writs, "python-api")?;
        for rule in py_api_rules {
            let count = twrit_python::api::generate(&rule, path)?;
            println!(
                "Generated API documentation for {count} Python module(s) (rule: {})",
                rule.id
            );
        }
    }

    let report = twrit_core::check(path, &guards)?;
    let mut out = std::io::stdout().lock();
    use std::io::Write as _;
    out.write_all(report.render().as_bytes())?;
    if report.violation_count() == 0 {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}
