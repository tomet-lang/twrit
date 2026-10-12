use std::path::Path;
use std::process::ExitCode;

use super::all_guards;

pub fn list(path: &Path) -> anyhow::Result<ExitCode> {
    let guards = all_guards();
    print!("{}", twrit_core::list(path, &guards)?.render());
    Ok(ExitCode::SUCCESS)
}
