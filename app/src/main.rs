use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "twrit", about = "Check a repository against its own writs")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check every rule this tool knows how to enforce.
    Check {
        /// Repository root. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Automatically fix violations where supported (e.g. regenerate API docs).
        #[arg(long)]
        fix: bool,
    },
    /// List every rule the writs declare, and who holds it.
    List {
        /// Repository root. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Extract and generate Tomet API documentation from Rust crates.
    Api {
        /// Repository root. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Override output destination directory.
        #[arg(long)]
        out: Option<PathBuf>,

        /// Only check whether generated documentation is up to date.
        #[arg(long)]
        check: bool,

        /// Include private items as well as public items.
        #[arg(long)]
        private: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("twrit: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn all_guards() -> Vec<twrit::Guard> {
    let mut guards = twrit_rust::rust_guards();
    guards.extend(twrit_python::python_guards());
    guards.push(twrit::placement_guard());
    guards
}

fn run(command: Command) -> anyhow::Result<ExitCode> {
    let guards = all_guards();
    Ok(match command {
        Command::Check { path, fix } => {
            if fix {
                let writs = twrit::writ::discover(&path)?;
                let api_rules = twrit::writ::twrit_rules(&writs, "rust-api")?;
                for rule in api_rules {
                    let count = twrit_rust::api::generate(&rule, &path)?;
                    println!(
                        "Generated API documentation for {count} crate(s) (rule: {})",
                        rule.id
                    );
                }

                let py_api_rules = twrit::writ::twrit_rules(&writs, "python-api")?;
                for rule in py_api_rules {
                    let count = twrit_python::api::generate(&rule, &path)?;
                    println!(
                        "Generated API documentation for {count} Python module(s) (rule: {})",
                        rule.id
                    );
                }
            }

            let report = twrit::check(&path, &guards)?;
            print!("{}", report.render());
            if report.violation_count() == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        // A listing reports; it does not judge. A dead guard is a finding
        // and `check` is where a finding fails a run, so this exits zero
        // whatever it prints.
        Command::List { path } => {
            print!("{}", twrit::list(&path, &guards)?.render());
            ExitCode::SUCCESS
        }
        Command::Api {
            path,
            out,
            check,
            private,
        } => {
            let out_dir = match out {
                Some(dir) => {
                    if dir.is_absolute() {
                        dir
                    } else {
                        path.join(dir)
                    }
                }
                None => {
                    // Try to resolve from .writ.tmt
                    let writs = twrit::writ::discover(&path)?;
                    let api_rules = twrit::writ::twrit_rules(&writs, "rust-api")?;
                    if let Some(rule) = api_rules.first() {
                        twrit_rust::api::RustApiConfig::read(rule, &path)?.out_dir
                    } else {
                        anyhow::bail!(
                            "no output directory specified and no `@rule` with `guard: {{ twrit: rust-api }}` found; pass `--out <DIR>`"
                        );
                    }
                }
            };

            if check {
                let crates = twrit_rust::api::load_workspace_crates(&path)?;
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
                    ExitCode::SUCCESS
                } else {
                    for f in &stale {
                        println!("Stale API documentation: {}", f.display());
                    }
                    eprintln!(
                        "{} API documentation file(s) differ from source",
                        stale.len()
                    );
                    ExitCode::FAILURE
                }
            } else {
                let count = twrit_rust::api::generate_to(&out_dir, private, &path)?;
                println!(
                    "Generated API documentation for {count} crate(s) into '{}'",
                    out_dir.display()
                );
                ExitCode::SUCCESS
            }
        }
    })
}
