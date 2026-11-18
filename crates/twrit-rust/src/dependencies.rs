//! The `dependencies` guard: exact workspace dependency allowlisting per crate.
//!
//! Declares the target `@rust.crate(...)` and the exact set of workspace
//! members it is allowed to depend on. Checked in both directions:
//!
//! 1. Any normal dependency on a workspace member not listed in `allow` is a
//!    violation.
//! 2. Any entry in `allow` that the crate does not actually depend on is a
//!    violation, so the allowlist stays a true statement about the code.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};
use cargo_metadata::DependencyKind;

use crate::metadata::{self, Flavour};
use twrit_core::writ::Rule;
use twrit_core::{Outcome, plural};

/// One `dependencies` declaration.
struct Dependencies {
    target_crate: String,
    allowed: Vec<String>,
}

impl Dependencies {
    fn read(rule: &Rule) -> Result<Self> {
        let entries = rule.map("dependencies").with_context(|| {
            format!(
                "{}: `@rule({})` guards `dependencies` but declares no `dependencies:` parameter",
                rule.path.display(),
                rule.id
            )
        })?;

        let mut target_crate = None;
        let mut allowed = None;

        for (key, value) in entries.keyed() {
            match key.as_ref() {
                "crate" => target_crate = Some(rule.as_crate(value)?),
                "allow" => allowed = Some(rule.as_crate_list(value, "`dependencies`", "allow")?),
                other => anyhow::bail!("`dependencies` does not know the key `{other}`"),
            }
        }

        let target_crate = target_crate.with_context(|| {
            format!(
                "{}: `@rule({})`'s `dependencies` declares no `crate`",
                rule.path.display(),
                rule.id
            )
        })?;
        let allowed = allowed.with_context(|| {
            format!(
                "{}: `@rule({})`'s `dependencies` declares no `allow`",
                rule.path.display(),
                rule.id
            )
        })?;

        Ok(Dependencies {
            target_crate,
            allowed,
        })
    }
}

/// Runs the `dependencies` guard against `workspace_root/Cargo.toml`.
pub fn check(rule: &Rule, workspace_root: &Path) -> Result<Outcome> {
    let deps = Dependencies::read(rule)?;
    let metadata = metadata::get(&workspace_root.join("Cargo.toml"), Flavour::NoDeps)
        .context("failed to read the workspace with `cargo metadata`")?;

    let packages = metadata.workspace_packages();
    let Some(package) = packages
        .iter()
        .find(|p| p.name.as_str() == deps.target_crate)
    else {
        return Ok(Outcome::checked(
            plural(1, "crate"),
            vec![format!(
                "crate `{}` is not a workspace member",
                deps.target_crate
            )],
        ));
    };

    let members: BTreeSet<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    let actual: BTreeSet<&str> = package
        .dependencies
        .iter()
        .filter(|d| d.kind == DependencyKind::Normal && members.contains(d.name.as_str()))
        .map(|d| d.name.as_str())
        .collect();
    let allowed: BTreeSet<&str> = deps.allowed.iter().map(String::as_str).collect();

    let mut violations = Vec::new();
    for dep in &actual {
        if !allowed.contains(dep) {
            violations.push(format!(
                "`{}` depends on workspace member `{dep}`, but it is not listed in `allow`",
                deps.target_crate
            ));
        }
    }
    for dep in &allowed {
        if !actual.contains(dep) {
            violations.push(format!(
                "`{}` allows dependency on `{dep}`, but does not depend on it",
                deps.target_crate
            ));
        }
    }

    violations.sort();
    violations.dedup();
    Ok(Outcome::checked(plural(1, "crate"), violations))
}
