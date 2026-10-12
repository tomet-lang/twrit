//! Harness for the guard tests: locating a fixture and running `check`
//! over it.
//!
//! The expectation each test asserts against is written by hand, before
//! the code is run. A reference generated from current behavior asserts
//! only that the code does what the code does -- which is precisely the
//! difference between a rule and a guard that this tool exists to make.
//! So there is no `UPDATE_REF` here and no committed output to bless:
//! the wanted error is typed next to the case name, and the code is what
//! moves when the two disagree.
//!
//! Tests call the library rather than spawning the binary, because
//! `CARGO_BIN_EXE_*` is defined only for the package declaring the bin.
//! `Report::render` produces the exact text `main` prints, so what is
//! pinned is still the CLI's output.

use std::path::PathBuf;

fn all_guards() -> Vec<twrit_core::Guard> {
    let mut guards = twrit_rust::rust_guards();
    guards.extend(twrit_python::python_guards());
    guards.extend(twrit_generic::generic_guards());
    guards
}

/// Runs `check` over `tests/fixtures/<name>`, returning what the CLI
/// would have printed.
///
/// Panics on an error rather than returning it: a fixture that cannot be
/// read is a broken test, not a finding.
pub fn check(name: &str) -> String {
    report(name).render()
}

/// The same run, when a test needs the counts rather than the text.
pub fn report(name: &str) -> twrit_core::Report {
    let dir = fixture(name);
    let guards = all_guards();
    twrit_core::check(&dir, &guards)
        .unwrap_or_else(|e| panic!("fixture `{name}` failed to check: {e:#}"))
}

/// The path to one fixture workspace.
pub fn fixture(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);
    assert!(dir.is_dir(), "no fixture at {}", dir.display());
    dir
}

/// The message `main` would print after `twrit: `, for a fixture the tool
/// refuses to check at all.
///
/// A writ it cannot read is an error rather than a violation, and
/// deliberately so: a rule half-understood is worse than a rule that
/// refuses to run, because the half it dropped is exactly where a
/// violation would hide.
pub fn check_err(name: &str) -> String {
    let dir = fixture(name);
    let guards = all_guards();
    match twrit_core::check(&dir, &guards) {
        Ok(report) => panic!(
            "fixture `{name}` was expected to fail, but printed:\n{}",
            report.render()
        ),
        Err(e) => format!("{e:#}"),
    }
}

/// The listing `twrit list` would print for one fixture.
pub fn list(name: &str) -> String {
    let dir = fixture(name);
    let guards = all_guards();
    twrit_core::list(&dir, &guards)
        .unwrap_or_else(|e| panic!("fixture `{name}` failed to list: {e:#}"))
        .render()
}
