//! The `pure` guard: crates that may not reach the outside world.
//!
//! Two halves, because there are two ways to acquire the ability:
//!
//! 1. **`allow-external`** -- every external crate in the dependency
//!    closure is named there. Something new cannot arrive without a
//!    decision, which is the property; "no external crates at all" would be
//!    a nicer sentence and a false one.
//! 2. **`forbid`** -- none of the named paths appears in the crate's
//!    source, so it cannot be done bare-handed either.
//!
//! Neither name nor path is written here. Which crates, and which paths,
//! are `pure` data in the writ -- see `AGENTS.md`'s "What belongs in this
//! tool".

use std::collections::{BTreeSet, VecDeque};
use std::path::Path;

use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Metadata};
use tomet_ast::Value;
use tomet_scalar::Scalar;

use crate::metadata::{self, Flavour};

use twrit_core::writ::{Rule, string_list};
use twrit_core::{Outcome, plural};

enum TargetCrate {
    /// Directory path, relative to the workspace root.
    Dir(String),
    /// Package name, from `@rust.crate(name)`.
    Name(String),
}

/// One `pure` declaration.
struct Pure {
    crates: Vec<TargetCrate>,
    allow_external: Vec<String>,
    forbid: Vec<String>,
}

impl Pure {
    fn read(rule: &Rule) -> Result<Self> {
        let entries = rule.map("pure").with_context(|| {
            format!(
                "{}: `@rule({})` guards `pure` but declares no `pure:` parameter",
                rule.path.display(),
                rule.id
            )
        })?;

        let mut crates = Vec::new();
        let mut allow_external = Vec::new();
        let mut forbid = Vec::new();

        for (key, value) in entries.keyed() {
            match key.as_ref() {
                "crates" => crates = read_target_crates(rule, value)?,
                "forbid" => forbid = string_list(value, "`pure`", &key)?,
                "allow-external" => allow_external = string_list(value, "`pure`", &key)?,
                other => anyhow::bail!("`pure` does not know the key `{other}`"),
            }
        }

        anyhow::ensure!(!crates.is_empty(), "`pure` names no crates");
        Ok(Pure {
            crates,
            allow_external,
            forbid,
        })
    }
}

fn read_target_crates(rule: &Rule, value: &Value) -> Result<Vec<TargetCrate>> {
    let Value::Tuple(tuple) = value else {
        anyhow::bail!("`pure`'s `crates` is not a list");
    };
    if !tuple.has_no_named() {
        anyhow::bail!("`pure`'s `crates` is not a list");
    }
    tuple
        .values()
        .map(|item| match item {
            Value::Scalar(Scalar::Text(dir)) => Ok(TargetCrate::Dir(dir.clone())),
            Value::Element(_) => Ok(TargetCrate::Name(rule.as_crate(item)?)),
            other => anyhow::bail!("`pure`'s `crates` holds an invalid entry: {other:?}"),
        })
        .collect()
}

/// Runs every `pure` declared across `writs`.
///
/// `Outcome::NotDeclared` when no writ carries one, for the same reason
/// `crate-layering` does: an empty violation list is what a guard that
/// checked everything returns, and what one that checked nothing
/// returns, and they are not the same answer.
pub fn check(rule: &Rule, workspace_root: &Path) -> Result<Outcome> {
    let metadata = metadata::get(&workspace_root.join("Cargo.toml"), Flavour::Deps)
        .context("failed to read the workspace with `cargo metadata`")?;

    let mut violations = Vec::new();
    let mut checked = 0;
    {
        let pure = Pure::read(rule)?;
        for target in &pure.crates {
            check_one(&metadata, workspace_root, target, &pure, &mut violations)?;
            checked += 1;
        }
    }

    violations.sort();
    violations.dedup();
    Ok(Outcome::checked(plural(checked, "crate"), violations))
}

fn check_one(
    metadata: &Metadata,
    workspace_root: &Path,
    target: &TargetCrate,
    pure: &Pure,
    violations: &mut Vec<String>,
) -> Result<()> {
    let mut reached: std::collections::BTreeSet<String> = BTreeSet::new();
    let root = metadata.workspace_root.as_std_path();
    let (package, label, dir) = match target {
        TargetCrate::Dir(dir) => {
            let pkg = metadata.workspace_packages().into_iter().find(|p| {
                p.manifest_path
                    .as_std_path()
                    .parent()
                    .and_then(|d| d.strip_prefix(root).ok())
                    .is_some_and(|d| d.to_string_lossy().replace('\\', "/") == *dir)
            });
            (pkg, dir.as_str(), dir.clone())
        }
        TargetCrate::Name(name) => {
            let pkg = metadata
                .workspace_packages()
                .into_iter()
                .find(|p| p.name.as_str() == *name);
            let dir = pkg
                .and_then(|p| {
                    p.manifest_path
                        .as_std_path()
                        .parent()
                        .and_then(|d| d.strip_prefix(root).ok())
                        .map(|d| d.to_string_lossy().replace('\\', "/"))
                })
                .unwrap_or_default();
            (pkg, name.as_str(), dir)
        }
    };
    let Some(package) = package else {
        violations.push(format!(
            "`pure` names {label}, which is not a workspace member"
        ));
        return Ok(());
    };

    {
        let members: BTreeSet<&str> = metadata
            .workspace_packages()
            .iter()
            .map(|p| p.name.as_str())
            .collect();

        // The *resolved* closure, and through externals as well as
        // members. Two ways to be wrong here, and this walk had both in
        // turn.
        //
        // Stopping at the first external made it one level deep on that
        // side: `serde` was seen and `serde_derive`, which `serde`
        // brings, was not -- so an ability arriving two hops out went
        // unreported, which is the whole thing the rule is for.
        //
        // Walking `Package::dependencies` instead has the opposite
        // fault. Those are *declared* dependencies, optional and
        // target-specific ones included, so the closure grows a
        // wasm-only `bumpalo` that nothing here compiles. A crate that
        // is never built cannot give the parser anything.
        //
        // `Resolve` is the graph cargo actually built, features
        // resolved across the workspace. That last part matters and is
        // not an over-approximation: `cargo tree -p tomet-parser` shows
        // `hashbrown` without `ahash`, and `cargo tree --workspace`
        // shows it with, because feature unification is per build and
        // this repository builds with `--workspace`.
        let resolve = metadata
            .resolve
            .as_ref()
            .context("`cargo metadata` returned no resolved dependency graph")?;
        let name_of = |id: &cargo_metadata::PackageId| {
            metadata
                .packages
                .iter()
                .find(|p| &p.id == id)
                .map(|p| p.name.to_string())
        };

        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::from([package.id.clone()]);
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id.clone()) {
                continue;
            }
            let Some(node) = resolve.nodes.iter().find(|n| n.id == id) else {
                continue;
            };
            let from = name_of(&id).unwrap_or_default();
            for dep in &node.deps {
                if !dep
                    .dep_kinds
                    .iter()
                    .any(|k| k.kind == DependencyKind::Normal)
                {
                    continue;
                }
                let Some(dep_name) = name_of(&dep.pkg) else {
                    continue;
                };
                queue.push_back(dep.pkg.clone());
                if members.contains(dep_name.as_str()) {
                    continue;
                }
                if pure.allow_external.contains(&dep_name) {
                    reached.insert(dep_name);
                } else {
                    violations.push(format!(
                        "{label} reaches an external crate not in `allow-external`: {from} -> {dep_name}"
                    ));
                }
            }
        }
    }

    // The other direction. An entry nothing reaches is not a permission:
    // it is a claim about this workspace that has stopped being true, and
    // what makes an allowlist worth more than "no external dependencies"
    // is precisely that it says what is true.
    for entry in &pure.allow_external {
        if !reached.contains(entry.as_str()) {
            violations.push(format!(
                "`allow-external` lists {entry}, which nothing under {label} reaches"
            ));
        }
    }

    for path in &pure.forbid {
        for hit in grep_sources(workspace_root, &dir, path)? {
            violations.push(format!("{label} names a forbidden path: {path} in {hit}"));
        }
    }

    Ok(())
}

/// Files under `<dir>/src` mentioning `needle`, named relative to
/// `workspace_root`.
///
/// Relative, because the hit goes straight into a violation line that is
/// read in a CI log or a diff: an absolute path differs per machine, and
/// makes `twrit check .` and `twrit check /somewhere` report the same
/// finding two different ways.
///
/// Text search, deliberately. Anything cleverer needs to parse Rust, and a
/// crate that has no business touching the filesystem has no business
/// writing `std::fs` in a comment either.
pub(crate) fn grep_sources(workspace_root: &Path, dir: &str, needle: &str) -> Result<Vec<String>> {
    let mut hits = Vec::new();
    let src = workspace_root.join(dir).join("src");
    if !src.is_dir() {
        return Ok(hits);
    }
    for entry in ignore::WalkBuilder::new(&src).hidden(false).build() {
        let entry = entry.context("failed to walk the crate's sources")?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if entry.path().extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(entry.path())
            .with_context(|| format!("failed to read {}", entry.path().display()))?;
        if text.contains(needle) {
            let rel = entry
                .path()
                .strip_prefix(workspace_root)
                .unwrap_or(entry.path());
            hits.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    hits.sort();
    Ok(hits)
}
