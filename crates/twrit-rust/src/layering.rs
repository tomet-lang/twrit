//! The `crate-layering` guard.
//!
//! Two checks over one `layers` declaration:
//!
//! 1. **Exhaustiveness** -- every workspace member matches exactly one
//!    layer. This is the half that catches a rule that was never written,
//!    rather than one that was broken, and it is why the layering rule is
//!    worth guarding before any other.
//! 2. **Direction** -- a member's `[dependencies]` may only resolve to its
//!    own layer or below.
//!
//! `[dev-dependencies]` are exempt, because a crate reaching up to the
//! parser to test itself says nothing about how the shipped code is
//! arranged.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand};

use twrit_core::writ::{Rule, string_list_map};
use twrit_core::{Outcome, plural};

/// The layer a pattern list was declared under, lowest number first.
struct Layers {
    /// `(layer, pattern)`, kept in declaration order so a member matching
    /// two layers can name both.
    patterns: Vec<(u32, String)>,
    /// The lowest layer declared, which may depend on nothing.
    floor: u32,
}

impl Layers {
    fn read(rule: &Rule) -> Result<Self> {
        let declared = rule.map("layers").with_context(|| {
            format!(
                "{}: `@rule({})` guards `layers` but declares no `layers:` parameter",
                rule.path.display(),
                rule.id
            )
        })?;

        let mut patterns = Vec::new();
        for (key, globs) in string_list_map(declared, "`layers`")? {
            let layer: u32 = key
                .parse()
                .with_context(|| format!("`layers` key `{key}` is not a layer number"))?;
            for glob in globs {
                patterns.push((layer, glob));
            }
        }

        anyhow::ensure!(!patterns.is_empty(), "`layers` declares no members");
        let floor = patterns.iter().map(|(l, _)| *l).min().expect("non-empty");

        Ok(Layers { patterns, floor })
    }

    /// Every layer whose pattern list matches `dir`.
    ///
    /// A pattern ending in `*` matches by prefix; every other pattern is
    /// exact. Returns all matches rather than the first, so that a member
    /// covered twice is reported as ambiguous instead of silently taking
    /// whichever layer happened to be declared first.
    fn matches(&self, dir: &str) -> Vec<(u32, &str)> {
        self.patterns
            .iter()
            .filter(|(_, pattern)| match pattern.strip_suffix('*') {
                Some(prefix) => dir.starts_with(prefix),
                None => dir == pattern,
            })
            .map(|(layer, pattern)| (*layer, pattern.as_str()))
            .collect()
    }
}

/// Runs the guard against the workspace whose manifest is at
/// `workspace_root/Cargo.toml`.
///
/// `Outcome::NotDeclared` when no rule guards `layers` -- not an empty
/// violation list, which would be indistinguishable from a workspace
/// whose every edge runs the right way.
pub fn check(rule: &Rule, workspace_root: &Path) -> Result<Outcome> {
    let layers = Layers::read(rule)?;
    let metadata = MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .no_deps()
        .exec()
        .context("failed to read the workspace with `cargo metadata`")?;

    let mut violations = Vec::new();
    let assigned = assign(&metadata, &layers, &mut violations);

    for package in metadata.workspace_packages() {
        let Some(&(own_layer, own_dir)) = assigned.get(package.name.as_str()) else {
            continue; // unclassified; already reported by `assign`
        };

        for dep in &package.dependencies {
            if dep.kind != DependencyKind::Normal {
                continue;
            }
            let Some(&(dep_layer, dep_dir)) = assigned.get(dep.name.as_str()) else {
                continue; // not a workspace member, so not ours to judge
            };

            if dep_layer > own_layer {
                // Both ends carry their own directory. `layers` matches on
                // the directory rather than the package name, and the two
                // differ often enough to matter -- `crates/tomet-syntax-parser`
                // publishes `tomet-parser`. A reader just told about an edge
                // has to be able to find both of its ends in the declaration
                // it violated.
                violations.push(format!(
                    "upward dependency: {} (layer {own_layer}, {own_dir}) -> {} (layer {dep_layer}, {dep_dir})",
                    package.name, dep.name
                ));
            }
        }

        // The floor may be depended on freely -- `tests` and the TUI both
        // reach for the tree-sitter grammar. What it may not do is depend
        // on anything, since its whole value is being arrived at
        // independently of the parser it approximates.
        if own_layer == layers.floor {
            for dep in &package.dependencies {
                if dep.kind == DependencyKind::Normal && assigned.contains_key(dep.name.as_str()) {
                    violations.push(format!(
                        "layer {} depends on a workspace member: {} -> {}",
                        layers.floor, package.name, dep.name
                    ));
                }
            }
        }
    }

    violations.sort();
    violations.dedup();
    Ok(Outcome::checked(
        plural(metadata.workspace_packages().len(), "member"),
        violations,
    ))
}

/// Maps each workspace member's package name to its layer and directory,
/// pushing a violation for every member that matches no layer or more than
/// one.
fn assign<'a>(
    metadata: &'a Metadata,
    layers: &'a Layers,
    violations: &mut Vec<String>,
) -> BTreeMap<&'a str, (u32, &'a str)> {
    let root = metadata.workspace_root.as_std_path();
    let mut assigned = BTreeMap::new();

    for package in metadata.workspace_packages() {
        let dir = package
            .manifest_path
            .as_std_path()
            .parent()
            .and_then(|d| d.strip_prefix(root).ok())
            .map(|d| d.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        // Leaked so the map can borrow it for the caller's lifetime; there
        // is one per workspace member and the process is short-lived.
        let dir: &'a str = Box::leak(dir.into_boxed_str());

        match layers.matches(dir).as_slice() {
            [] => violations.push(format!(
                "unclassified member: {dir} matches no layer in `layers`"
            )),
            [(layer, _)] => {
                assigned.insert(package.name.as_str(), (*layer, dir));
            }
            many => violations.push(format!(
                "ambiguous member: {dir} matches {}",
                many.iter()
                    .map(|(l, p)| format!("layer {l} (`{p}`)"))
                    .collect::<Vec<_>>()
                    .join(" and ")
            )),
        }
    }

    assigned
}
