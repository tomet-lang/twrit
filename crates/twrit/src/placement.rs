//! The `placement` guard.
//!
//! Enforces directory layout and file placement rules. A directory tree
//! declares which files are allowed to exist under it, and anything else
//! is reported as stray.
//!
//! This replaces ad-hoc shell checks (`find ... | grep -v ...`) with a
//! declarative, mechanical guard that fails loudly when an unlisted file
//! appears.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use tomet_ast::Value;

use crate::writ::Rule;
use crate::{Outcome, plural};

/// Configuration for the placement guard.
struct Placement {
    /// Directory to scan.
    within: PathBuf,
    /// List of allowed glob patterns.
    allow: Vec<String>,
}

impl Placement {
    fn read(rule: &Rule, workspace_root: &Path) -> Result<Self> {
        let entries = rule.map("placement").with_context(|| {
            format!(
                "{}: `@rule({})` guards `placement` but declares no `placement:` parameter",
                rule.path.display(),
                rule.id
            )
        })?;

        let rule_dir = rule.path.parent().unwrap_or(workspace_root);
        let mut within = rule_dir.to_path_buf();
        let mut allow = Vec::new();

        for (key, value) in entries {
            match key.as_str() {
                "within" => {
                    let Value::String(s) = value else {
                        anyhow::bail!("`placement`'s `within` is not a string");
                    };
                    if s == "." {
                        within = rule_dir.to_path_buf();
                    } else {
                        let p = Path::new(s);
                        within = if p.is_absolute() {
                            p.to_path_buf()
                        } else if rule_dir.join(p).exists() {
                            rule_dir.join(p)
                        } else {
                            workspace_root.join(p)
                        };
                    }
                }
                "allow" => {
                    allow = strings(key, value)?;
                }
                other => anyhow::bail!("`placement` does not know the key `{other}`"),
            }
        }

        anyhow::ensure!(!allow.is_empty(), "`placement` names no allowed patterns");
        Ok(Placement { within, allow })
    }
}

fn strings(key: &str, value: &Value) -> Result<Vec<String>> {
    if let Some(items) = crate::writ::as_list(value) {
        return items
            .iter()
            .map(|item| match item {
                Value::String(s) => Ok(s.clone()),
                other => anyhow::bail!("`placement`'s `{key}` holds a non-string entry: {other:?}"),
            })
            .collect();
    }
    if let Some(items) = crate::writ::as_string_list(value) {
        return Ok(items);
    }
    anyhow::bail!("`placement`'s `{key}` is not a list");
}

/// Runs the placement guard.
pub fn check(rule: &Rule, workspace_root: &Path) -> Result<Outcome> {
    let placement = Placement::read(rule, workspace_root)?;

    if !placement.within.exists() {
        return Ok(Outcome::checked(
            "0 files".to_string(),
            vec![format!(
                "directory `{}` does not exist",
                placement.within.display()
            )],
        ));
    }

    let mut overrides = OverrideBuilder::new(&placement.within);
    for pattern in &placement.allow {
        overrides
            .add(pattern)
            .with_context(|| format!("invalid glob pattern `{pattern}` in `placement`"))?;
    }
    let matcher = overrides.build()?;

    let mut violations = Vec::new();
    let mut scanned_files = 0usize;

    for entry in WalkBuilder::new(&placement.within).hidden(false).build() {
        let entry = entry.context("failed to walk directory")?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }

        let path = entry.path();
        // Skip anything inside .git
        if path.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }

        let Ok(rel_within) = path.strip_prefix(&placement.within) else {
            continue;
        };

        scanned_files += 1;

        match matcher.matched(rel_within, false) {
            ignore::Match::Whitelist(_) => {
                // Matched an allowed pattern!
            }
            _ => {
                let display_path = path
                    .strip_prefix(workspace_root)
                    .unwrap_or(rel_within)
                    .display();
                violations.push(format!(
                    "{display_path}: unlisted file matches no allowed pattern"
                ));
            }
        }
    }

    // Sort violations so output is deterministic
    violations.sort();

    Ok(Outcome::checked(plural(scanned_files, "file"), violations))
}
