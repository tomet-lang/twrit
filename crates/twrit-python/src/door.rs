//! Python capability access control guard (`python-door`).
//!
//! Enforces that specific capabilities (e.g. `subprocess`, `os`, `sys`, `requests`)
//! are only accessed from authorized directories/modules.

use std::path::Path;

use anyhow::{Context, Result};
use rustpython_ast::{Mod, Stmt};
use rustpython_parser::{Mode, parse};
use tomet_ast::Value;
use twrit::writ::Rule;
use twrit::{Outcome, plural};

/// What capability is restricted, and who is allowed it.
pub struct Door {
    /// Capability names/modules (e.g. `subprocess`, `os.system`).
    pub names: Vec<String>,
    /// Directories relative to the repository root allowed to use them.
    pub allowed: Vec<String>,
    /// Scope of paths covered by this rule. Prefix match if ends with `*`.
    pub within: Vec<String>,
}

impl Door {
    pub fn covers(&self, path: &str) -> bool {
        if self.within.is_empty() {
            return true;
        }
        self.within.iter().any(|pattern| {
            if let Some(prefix) = pattern.strip_suffix('*') {
                path.starts_with(prefix)
            } else {
                path == pattern
            }
        })
    }

    pub fn read(rule: &Rule) -> Result<Self> {
        let entries = rule.map("door").with_context(|| {
            format!(
                "{}: `@rule({})` guards `python-door` but declares no `door:` parameter",
                rule.path.display(),
                rule.id
            )
        })?;

        let mut names = Vec::new();
        let mut allowed = Vec::new();
        let mut within = Vec::new();

        for (key, value) in entries {
            match key.as_str() {
                "names" => names = parse_string_list(key, value)?,
                "allowed" => allowed = parse_string_list(key, value)?,
                "within" => within = parse_string_list(key, value)?,
                other => anyhow::bail!("`door` does not know the key `{other}`"),
            }
        }

        anyhow::ensure!(!names.is_empty(), "`door` names no capability");
        Ok(Door {
            names,
            allowed,
            within,
        })
    }
}

fn parse_string_list(key: &str, value: &Value) -> Result<Vec<String>> {
    let Some(items) = twrit::writ::as_list(value) else {
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

/// Checks whether a Python file imports or references any of `needle_names`.
pub fn file_references_capability(content: &str, needle_names: &[String]) -> Vec<String> {
    let mut hits = Vec::new();

    // 1. AST check for imports
    if let Ok(ast) = parse(content, Mode::Module, "<source>") {
        if let Mod::Module(m) = ast {
            for stmt in m.body {
                match stmt {
                    Stmt::Import(imp) => {
                        for alias in imp.names {
                            let name = alias.name.as_str();
                            for needle in needle_names {
                                if name == needle || name.starts_with(&format!("{needle}.")) {
                                    hits.push(needle.clone());
                                }
                            }
                        }
                    }
                    Stmt::ImportFrom(imp) => {
                        if let Some(module) = &imp.module {
                            let mod_str = module.as_str();
                            for needle in needle_names {
                                if mod_str == needle || mod_str.starts_with(&format!("{needle}.")) {
                                    hits.push(needle.clone());
                                }
                                for alias in &imp.names {
                                    let full = format!("{mod_str}.{}", alias.name.as_str());
                                    if full == *needle {
                                        hits.push(needle.clone());
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // 2. Also check direct literal/call mentions (e.g. `subprocess.run`, `os.system`)
    for needle in needle_names {
        if needle.contains('.') && content.contains(needle) && !hits.contains(needle) {
            hits.push(needle.clone());
        }
    }

    hits.sort();
    hits.dedup();
    hits
}

pub fn check(rule: &Rule, root: &Path) -> Result<Outcome> {
    let door = Door::read(rule)?;

    let mut violations = Vec::new();
    let mut checked_files = 0;

    // Walk all .py files in the workspace
    for entry in ignore::WalkBuilder::new(root).hidden(false).build() {
        let entry = entry.context("failed to walk repository")?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if entry.path().extension().is_none_or(|e| e != "py") {
            continue;
        }

        let rel_path = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");

        if !door.covers(&rel_path) {
            continue;
        }

        checked_files += 1;

        // Check if file is in an allowed directory
        let is_allowed = door.allowed.iter().any(|allowed_dir| {
            let allowed_clean = allowed_dir.trim_end_matches('/');
            rel_path.starts_with(&format!("{allowed_clean}/")) || rel_path == allowed_clean
        });

        if is_allowed {
            continue;
        }

        let content = std::fs::read_to_string(entry.path())
            .with_context(|| format!("failed to read {}", entry.path().display()))?;

        let hits = file_references_capability(&content, &door.names);
        for hit in hits {
            violations.push(format!("{rel_path} is not a {hit} door, and references it"));
        }
    }

    // Two-way allowlist check: ensure allowed directories actually exist
    for allowed_dir in &door.allowed {
        let target = root.join(allowed_dir);
        if !target.exists() {
            violations.push(format!("`door` allows {allowed_dir}, which does not exist"));
        }
    }

    violations.sort();
    violations.dedup();

    Ok(Outcome::checked(plural(checked_files, "file"), violations))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_detection() {
        let code = r#"
import subprocess
import os.path
from sys import argv
"#;
        let needles = vec![
            "subprocess".to_string(),
            "os".to_string(),
            "sys".to_string(),
            "requests".to_string(),
        ];
        let hits = file_references_capability(code, &needles);
        assert_eq!(hits, vec!["os", "subprocess", "sys"]);
    }

    #[test]
    fn test_import_from_member() {
        let code = r#"
from os import system
"#;
        let needles = vec!["os.system".to_string(), "subprocess".to_string()];
        let hits = file_references_capability(code, &needles);
        assert_eq!(hits, vec!["os.system"]);
    }

    #[test]
    fn test_direct_member_usage() {
        let code = r#"
def do_something():
    subprocess.run(["echo", "hi"])
"#;
        let needles = vec!["subprocess.run".to_string()];
        let hits = file_references_capability(code, &needles);
        assert_eq!(hits, vec!["subprocess.run"]);
    }
}
