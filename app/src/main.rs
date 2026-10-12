mod cli;
mod commands;

use std::process::ExitCode;

use clap::Parser;
use cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.command) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("twrit: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> anyhow::Result<ExitCode> {
    match command {
        Command::Check { path, fix } => commands::check::check(&path, fix),
        Command::List { path } => commands::list::list(&path),
        Command::Api {
            path,
            out,
            check,
            private,
        } => commands::api::api(&path, out, check, private),
    }
}
