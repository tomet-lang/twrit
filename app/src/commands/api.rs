use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn api(
    path: &Path,
    out: Option<PathBuf>,
    check: bool,
    private: bool,
) -> anyhow::Result<ExitCode> {
    let out_dir = match out {
        Some(dir) => {
            if dir.is_absolute() {
                dir
            } else {
                path.join(dir)
            }
        }
        None => {
            let writs = twrit_core::writ::discover(path)?;
            let api_rules = twrit_core::writ::twrit_rules(&writs, "rust-api")?;
            if let Some(rule) = api_rules.first() {
                twrit_rust::api::RustApiConfig::read(rule, path)?.out_dir
            } else {
                anyhow::bail!(
                    "no output directory specified and no `@rule` with `guard: {{ twrit: rust-api }}` found; pass `--out <DIR>`"
                );
            }
        }
    };

    if check {
        let crates = twrit_rust::api::load_workspace_crates(path)?;
        let options = twrit_rust::api::RustExtractOptions { private };
        let mut stale = Vec::new();

        for (name, source) in &crates {
            let doc = twrit_rust::api::extract_crate_doc(source, &options)?;
            let formatted = twrit_rust::api::format_document(&doc);
            let target_file = out_dir.join(format!("{name}.tmt"));

            match std::fs::read_to_string(&target_file) {
                Ok(existing) if existing == formatted => {}
                _ => stale.push(target_file),
            }
        }

        if stale.is_empty() {
            println!("All API documentation files are up to date.");
            Ok(ExitCode::SUCCESS)
        } else {
            for f in &stale {
                println!("Stale API documentation: {}", f.display());
            }
            eprintln!(
                "{} API documentation file(s) differ from source",
                stale.len()
            );
            Ok(ExitCode::FAILURE)
        }
    } else {
        let count = twrit_rust::api::generate_to(&out_dir, private, path)?;
        println!(
            "Generated API documentation for {count} crate(s) into '{}'",
            out_dir.display()
        );
        Ok(ExitCode::SUCCESS)
    }
}
