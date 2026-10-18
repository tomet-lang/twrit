use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "twrit",
    version,
    about = "Check a repository against its own writs"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
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
