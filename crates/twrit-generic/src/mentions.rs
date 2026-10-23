//! The `mentions` guard.
//!
//! Validates that `@rule(...)` references inside source code comments
//! (across Rust, Python, JavaScript, Java, Go, Shell, etc.) actually correspond
//! to rules declared in the repository's `.writ.tmt` files.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use tomet_ast::Value;
use tomet_extract::{Language, collect_comment_elements};
use twrit_core::writ::Rule;
use twrit_core::{Outcome, plural};

pub fn check(rule: &Rule, root: &Path) -> Result<Outcome> {
    // 1. Discover all declared rules across all writs in the repository
    let writs = twrit_core::writ::discover(root)?;
    let declared_rules: HashSet<String> = writs
        .iter()
        .map(|w| w.rules())
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .map(|r| r.id)
        .collect();

    // 2. Read optional configuration parameters (e.g. within, exclude)
    let mut within_patterns = Vec::new();
    if let Some(entries) = rule.map("mentions") {
        for (key, val) in entries {
            if key == "within" {
                if let Some(list) = twrit_core::writ::as_list(val) {
                    for item in list {
                        if let Value::String(s) = item {
                            within_patterns.push(s.clone());
                        }
                    }
                }
            }
        }
    }

    let mut violations = Vec::new();
    let mut checked_files = 0;

    // 3. Walk all source files in the repository
    for entry in ignore::WalkBuilder::new(root).hidden(false).build() {
        let entry = entry.context("failed to traverse repository")?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }

        let path = entry.path();

        // Skip non-code files or unsupported extensions
        if Language::from_path(path).is_none() {
            continue;
        }

        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        // Skip internal fixtures or test store
        if rel_path.starts_with("tests/fixtures/")
            || rel_path.starts_with("target/")
            || rel_path.starts_with(".git/")
        {
            continue;
        }

        if !within_patterns.is_empty()
            && !within_patterns.iter().any(|pattern| {
                if let Some(prefix) = pattern.strip_suffix('*') {
                    rel_path.starts_with(prefix)
                } else {
                    rel_path == *pattern
                }
            })
        {
            continue;
        }

        checked_files += 1;

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Extract all tomet elements in comments
        let elements = collect_comment_elements(&source, path);
        for (el, span, _) in elements {
            if el.sigil.name().is_some_and(|n| n.name == "rule") {
                if let Some(Value::String(rule_id)) = &el.args {
                    if !declared_rules.contains(rule_id) {
                        violations.push(format!(
                            "{rel_path}:{}: mentions `@rule({rule_id})`, which is not declared in any writ",
                            line_of(&source, span.start)
                        ));
                    }
                }
            }
        }
    }

    violations.sort();
    violations.dedup();

    Ok(Outcome::checked(plural(checked_files, "file"), violations))
}

/// The 1-based line of byte `offset` in `source`.
fn line_of(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())].matches('\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rule_in_comment() {
        let code = r#"
# normal line
# See @rule(subprocess-door) for policy.
def execute():
    pass
"#;
        let elements = collect_comment_elements(code, Path::new("test.py"));
        let rule_refs: Vec<String> = elements
            .into_iter()
            .filter(|(el, _, _)| el.sigil.name().is_some_and(|n| n.name == "rule"))
            .filter_map(|(el, _, _)| match el.args {
                Some(Value::String(s)) => Some(s),
                _ => None,
            })
            .collect();

        assert_eq!(rule_refs, vec!["subprocess-door"]);
    }
}
