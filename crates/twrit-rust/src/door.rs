//! A capability's allowed locations, and everyone else held to not having
//! it.
//!
//! The rule this exists for was written in prose and could not fail.
//! `tomet`'s `parser-purity` says file resolution is
//! "`tomet-semantics-resolver`'s job -- and the only place reading a
//! referenced file is allowed". Five crates touch `fs`. The sentence was
//! true when it was written, a second place appeared, and nothing
//! reported it, because a sentence has no way to notice.
//!
//! The difference from `pure` is the quantifier, and it is the whole
//! point. `pure` checks the crates it names: a crate that acquires an
//! ability without being named stays invisible to it. `door` names the
//! crates that *may* have the ability and checks every other member of
//! the workspace, so a new one arriving is what fails.

use std::path::Path;

use anyhow::{Context, Result};
use rayon::prelude::*;
use tomet_ast::Value;

use crate::metadata::{self, Flavour};
use crate::purity::grep_sources;
use twrit_core::writ::Rule;
use twrit_core::{Outcome, plural};

/// What the capability is in source, and who is allowed it.
struct Door {
    /// The API names that constitute the ability, matched as substrings
    /// of a crate's own sources -- `std::fs`, `std::process`.
    names: Vec<String>,
    /// Directories, relative to the workspace root, allowed to name them.
    /// The directory and not the package name, as in `@layers`: the
    /// directory is the half arranged by hand.
    allowed: Vec<String>,
    /// Which members the rule speaks about at all. Patterns, as in
    /// `@layers`: a trailing `*` matches by prefix, everything else is
    /// exact. Empty means every member.
    ///
    /// A door is usually about the libraries. A CLI reads files because
    /// that is what a CLI is, and listing every program under `allowed`
    /// would leave a list that permits almost everyone and therefore says
    /// almost nothing.
    within: Vec<String>,
}

impl Door {
    /// Whether this rule speaks about `dir` at all.
    fn covers(&self, dir: &str) -> bool {
        self.within.is_empty()
            || self.within.iter().any(|p| match p.strip_suffix('*') {
                Some(prefix) => dir.starts_with(prefix),
                None => dir == p,
            })
    }

    fn read(rule: &Rule) -> Result<Self> {
        let entries = rule.map("door").with_context(|| {
            format!(
                "{}: `@rule({})` guards `door` but declares no `door:` parameter",
                rule.path.display(),
                rule.id
            )
        })?;

        let mut names = Vec::new();
        let mut allowed = Vec::new();
        let mut within = Vec::new();
        for (key, value) in entries {
            match key.as_str() {
                "names" => names = strings(key, value)?,
                "allowed" => allowed = strings(key, value)?,
                "within" => within = strings(key, value)?,
                other => anyhow::bail!("`door` does not know the key `{other}`"),
            }
        }

        anyhow::ensure!(!names.is_empty(), "`door` names no capability");
        // An empty `allowed` is legal and says something: nobody may have
        // this. Not an error, unlike an empty `names`, which asks nothing.
        Ok(Door {
            names,
            allowed,
            within,
        })
    }
}

fn strings(key: &str, value: &Value) -> Result<Vec<String>> {
    let Some(items) = twrit_core::writ::as_list(value) else {
        anyhow::bail!("`door`'s `{key}` is not a list");
    };
    items
        .iter()
        .map(|item| match item {
            Value::String(s) => Ok(s.clone()),
            other => anyhow::bail!("`door`'s `{key}` holds a non-string entry: {other:?}"),
        })
        .collect()
}

pub fn check(rule: &Rule, workspace_root: &Path) -> Result<Outcome> {
    let door = Door::read(rule)?;
    let metadata = metadata::get(&workspace_root.join("Cargo.toml"), Flavour::NoDeps)
        .context("failed to read the workspace with `cargo metadata`")?;

    let root = metadata.workspace_root.as_std_path();
    let packages: Vec<_> = metadata.workspace_packages().into_iter().collect();

    let results: Vec<(Vec<String>, bool)> = packages
        .par_iter()
        .map(|package| {
            let _token = jobserver_gate::acquire();
            let Some(dir) = package
                .manifest_path
                .as_std_path()
                .parent()
                .and_then(|d| d.strip_prefix(root).ok())
                .map(|d| d.to_string_lossy().replace('\\', "/"))
            else {
                return (Vec::new(), false);
            };
            if !door.covers(&dir) {
                return (Vec::new(), false);
            }
            if door.allowed.contains(&dir) {
                return (Vec::new(), true);
            }
            let mut v = Vec::new();
            for name in &door.names {
                if let Ok(files) = grep_sources(workspace_root, &dir, name) {
                    for file in files {
                        v.push(format!(
                            "{} is not a {name} door, and names it in {file}",
                            package.name
                        ));
                    }
                }
            }
            (v, true)
        })
        .collect();

    let mut violations = Vec::new();
    let mut checked = 0;
    for (v, is_checked) in results {
        violations.extend(v);
        if is_checked {
            checked += 1;
        }
    }

    for dir in &door.allowed {
        if !metadata.workspace_packages().iter().any(|p| {
            p.manifest_path
                .as_std_path()
                .parent()
                .and_then(|d| d.strip_prefix(root).ok())
                .is_some_and(|d| d.to_string_lossy().replace('\\', "/") == *dir)
        }) {
            violations.push(format!(
                "`door` allows {dir}, which is not a workspace member"
            ));
        }
    }

    violations.sort();
    violations.dedup();
    Ok(Outcome::checked(plural(checked, "member"), violations))
}
