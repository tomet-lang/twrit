//! Tests for the shared cargo metadata cache.
//!
//! Verifies:
//! 1. A workspace with all four guard kinds executes `cargo metadata` the minimum
//!    number of times (once per distinct flavour, 2 times total).
//! 2. A cached failure is reported consistently against every rule that needs it,
//!    without re-executing `cargo metadata`.

use std::fs;
use std::path::{Path, PathBuf};

struct TempTestDir(PathBuf);

impl TempTestDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("twrit-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(
                fixtures_dir.join("default.config.tmt"),
                dir.join("default.config.tmt"),
            );
            let _ = std::os::unix::fs::symlink(fixtures_dir.join(".tomet"), dir.join(".tomet"));
        }

        TempTestDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn all_four_guard_kinds_query_cargo_metadata_minimum_times() {
    let fixture_dir = twrit_tests::fixture("all-rust-guards");
    let manifest = fixture_dir.join("Cargo.toml");
    let guards = twrit_rust::rust_guards();

    let report = twrit_core::check(&fixture_dir, &guards)
        .unwrap_or_else(|e| panic!("fixture all-rust-guards failed to check: {e:#}"));

    // Check that all 4 rules were evaluated
    let rule_ids: Vec<_> = report.rules().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        rule_ids,
        vec!["rule-layers", "rule-door", "rule-pure", "rule-api"]
    );

    // 4 guards ran, but cargo metadata should have executed only 2 times
    // (1 for NoDeps: shared by layers & door, 1 for Deps: shared by pure & api).
    assert_eq!(
        twrit_rust::metadata::exec_count_for(&manifest),
        2,
        "expected exactly 2 cargo metadata calls (1 NoDeps, 1 Deps), but got {}",
        twrit_rust::metadata::exec_count_for(&manifest)
    );
}

#[test]
fn cached_failure_is_reported_against_every_rule() {
    let tmp = TempTestDir::new("broken-cargo");
    let fixture_dir = tmp.path();
    let manifest = fixture_dir.join("Cargo.toml");

    fs::write(&manifest, "invalid toml syntax [[[\n").unwrap();
    fs::write(
        fixture_dir.join(".writ.tmt"),
        r#"@kind(writ)

=[ Writ: broken-cargo fixture ]

==[ crate-layering ]

@rule(rule-layers){
  guard: { twrit: layers }

  layers: {
    0: list("core")
  }
}

==[ file-io ]

@rule(rule-door){
  guard: { twrit: door }

  door: {
    names: list("std::fs")
    allowed: list("core")
  }
}
"#,
    )
    .unwrap();

    let writs = twrit_core::writ::discover(fixture_dir).unwrap();
    let rules = writs[0].rules().unwrap();

    let rule_layers = rules.iter().find(|r| r.id == "rule-layers").unwrap();
    let rule_door = rules.iter().find(|r| r.id == "rule-door").unwrap();

    // First guard fails on broken Cargo.toml
    let res1 = twrit_rust::layering::check(rule_layers, fixture_dir);
    let err1 = match res1 {
        Err(e) => format!("{e:#}"),
        Ok(_) => panic!("expected layers guard to fail on broken Cargo.toml"),
    };
    assert!(
        err1.contains("failed to read the workspace with `cargo metadata`"),
        "unexpected error message: {err1}"
    );
    assert_eq!(twrit_rust::metadata::exec_count_for(&manifest), 1);

    // Second guard queries the same flavour and must fail with identical context/message,
    // without re-executing cargo metadata.
    let res2 = twrit_rust::door::check(rule_door, fixture_dir);
    let err2 = match res2 {
        Err(e) => format!("{e:#}"),
        Ok(_) => panic!("expected door guard to also fail on cached error"),
    };
    assert!(
        err2.contains("failed to read the workspace with `cargo metadata`"),
        "unexpected error message: {err2}"
    );
    assert_eq!(err1, err2, "error messages must be identical");
    assert_eq!(
        twrit_rust::metadata::exec_count_for(&manifest),
        1,
        "second guard must use cached failure without re-executing"
    );
}
