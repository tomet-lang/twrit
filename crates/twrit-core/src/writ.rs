//! Finding and reading `.writ.tmt` files.
//!
//! A writ is an author-owned normative file, one per directory, scoped to
//! that directory. This module locates them, resolves their names, and
//! reads each entry's declaration; deciding what a given rule *means* is
//! each guard's own job.
//!
//! Names are resolved through `tomet-load`, not matched by hand. A writ
//! declares `@kind(writ)`, so `@rule` and `@writ.rule` are two legal
//! spellings of the same element and `Bindings::classify` maps both to
//! `writ.rule`. This module used to match a bare `Sigil::Named` instead,
//! which found the first spelling and silently ignored the second --
//! and "silently ignored" means the rule stops being checked, which is
//! the exact failure this tool exists to prevent.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tomet_ast::{Block, Document, Element, ElementValue, Entry, Value};
use tomet_load::Vault;
use tomet_semantics::{Bindings, ElementKind};

/// One parsed writ, kept with its path so a violation can name the file the
/// rule came from rather than just the rule.
pub struct Writ {
    pub path: PathBuf,
    pub doc: Document,
    bindings: Bindings,
}

/// Who guarantees a rule right now, and nothing else.
///
/// The writ holds the rule and the reason; whatever enforces it lives
/// outside and is only named here. A guard is a pointer, never a program:
/// an earlier draft put a shell line in the entry, which is a Makefile
/// wearing prose and would mean reading a `.writ.tmt` could run arbitrary
/// commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Guard {
    /// A rule kind this tool implements. Parameters come from the same
    /// group.
    Twrit(String),
    /// A runner the repository already ships. Recorded, never executed.
    Runs(String),
    /// A test in the repository being checked. What can be verified from
    /// here is that the path still exists -- a dead path means the rule
    /// quietly stopped being tested.
    Test(String),
    /// Nobody, said out loud, with why not.
    None(String),
}

impl Guard {
    /// What this guard points at, when the pointer no longer resolves.
    ///
    /// Only `test:` can be followed. `runs:` names a runner this tool does
    /// not execute, and verifying it would mean knowing every runner a
    /// repository might ship; `twrit:` is checked against `KINDS` before
    /// anything runs; `none:` points at nothing by construction.
    ///
    /// Existence is a weak proxy and says so: a file that is there may
    /// hold no test at all. It is one-sided on purpose -- a path that
    /// resolves proves nothing, and a path that does not proves the rule
    /// is held by nobody.
    pub fn dead_pointer(&self, root: &Path) -> Option<&str> {
        match self {
            // Paths are workspace-root relative, as in `layers`.
            Guard::Test(path) if !root.join(path).exists() => Some(path),
            _ => None,
        }
    }

    fn from_map(entries: &[(String, Value)], id: &str) -> Result<Self> {
        let [(key, value)] = entries else {
            anyhow::bail!(
                "`@rule({id})`'s `guard` holds {} entries; it names exactly one holder",
                entries.len()
            );
        };
        let text = match value {
            Value::String(s) => s.clone(),
            other => anyhow::bail!("`@rule({id})`'s `guard: {key}` is not a string: {other:?}"),
        };
        match key.as_str() {
            "twrit" => Ok(Guard::Twrit(text)),
            "runs" => Ok(Guard::Runs(text)),
            "test" => Ok(Guard::Test(text)),
            "none" if text.trim().is_empty() => anyhow::bail!(
                "`@rule({id})`'s `guard: none` has no reason; \
                 an unguarded rule says why, so it cannot be read as an oversight"
            ),
            "none" => Ok(Guard::None(text)),
            other => anyhow::bail!(
                "`@rule({id})`'s guard holder `{other}` is not one of \
                 `twrit`, `runs`, `test`, `none`"
            ),
        }
    }
}

/// One entry: its id, who holds it, and the parameters it declares.
pub struct Rule {
    pub id: String,
    pub guard: Guard,
    /// Which writ it came from, so a violation can name the file.
    pub path: PathBuf,
    group: Vec<Entry>,
}

impl Rule {
    /// A nested `key: { ... }` from this rule's own group.
    ///
    /// The parameters live inside the entry rather than beside it, which
    /// is what makes a dropped blank line a parse error instead of a
    /// declaration nobody owns.
    pub fn map(&self, key: &str) -> Option<&[(String, Value)]> {
        self.group.iter().find_map(|entry| match entry {
            Entry::Pair(k, Value::Map(entries)) if k == key => Some(entries.as_slice()),
            _ => None,
        })
    }

    /// A top-level entry value from this rule's own group.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.group.iter().find_map(|entry| match entry {
            Entry::Pair(k, v) if k == key => Some(v),
            _ => None,
        })
    }

    /// All entries declared in this rule's `{...}` group.
    pub fn group(&self) -> &[Entry] {
        &self.group
    }
}

/// Every `.writ.tmt` at or below `root`, in walk order.
///
/// Hidden files are explicitly included. `.writ.tmt` is the nameless form of
/// the `<name>.<kind>.tmt` convention, so the leading dot is a kind
/// separator rather than a request to be invisible -- but every directory
/// walker treats it as the latter by default, which is precisely how the
/// tomet repository's own `check-links` came to skip these files.
pub fn discover(root: &Path) -> Result<Vec<Writ>> {
    // One vault for the run. Vocabularies are declared by the config that
    // governs the tree, so they are read once here rather than per writ.
    let vault = Vault::discover(root);
    for problem in &vault.vocabulary_errors {
        eprintln!("warning: {problem}");
    }

    let mut found = Vec::new();

    for entry in ignore::WalkBuilder::new(root).hidden(false).build() {
        let entry = entry.context("failed to walk the tree")?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if entry.file_name() != ".writ.tmt" {
            continue;
        }

        let path = entry.path().to_path_buf();
        // `LoadError` already names the path, so this does not prepend it.
        let (doc, bindings) = vault.document(&path)?;

        found.push(Writ {
            path,
            doc,
            bindings,
        });
    }

    // Sorted by path, so a listing reads root-first the way the writs
    // themselves are read -- and so two runs over the same tree print the
    // same thing. `WalkBuilder`'s order is its own business.
    found.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(found)
}

impl Writ {
    /// Every `@rule` this writ declares, in document order.
    pub fn rules(&self) -> Result<Vec<Rule>> {
        let mut elements = Vec::new();
        collect_rule_elements(&self.doc.blocks, &|el| self.is_rule(el), &mut elements);
        elements.into_iter().map(|el| self.read_rule(el)).collect()
    }

    /// Whether `el` is a `@rule`, by resolution rather than by spelling.
    ///
    /// `@rule` and `@writ.rule` are both legal in a `@kind(writ)` document
    /// and mean the same thing; only `Bindings::classify` knows that.
    fn is_rule(&self, el: &Element) -> bool {
        el.sigil
            .name()
            .and_then(|name| self.bindings.classify(name).ok())
            .is_some_and(|kind| kind == ElementKind::Custom("writ.rule".into()))
    }

    fn read_rule(&self, el: &Element) -> Result<Rule> {
        let id = match &el.args {
            Some(Value::String(id)) => id.clone(),
            Some(other) => anyhow::bail!(
                "{}: `@rule`'s id is not a bare name: {other:?}",
                self.path.display()
            ),
            None => anyhow::bail!(
                "{}: a `@rule` carries no id; the heading is prose and cannot serve as one",
                self.path.display()
            ),
        };

        let Some(ElementValue::Group(group)) = &el.value else {
            anyhow::bail!(
                "{}: `@rule({id})` has no `{{...}}` group, so it declares neither a guard nor \
                 parameters",
                self.path.display()
            );
        };

        let guard = group
            .iter()
            .find_map(|entry| match entry {
                Entry::Pair(k, Value::Map(entries)) if k == "guard" => Some(entries.as_slice()),
                _ => None,
            })
            .with_context(|| {
                format!(
                    "{}: `@rule({id})` declares no `guard`; \
                     write `guard: {{ none: \"...\" }}` when nothing holds it yet",
                    self.path.display()
                )
            })?;

        Ok(Rule {
            id: id.clone(),
            guard: Guard::from_map(guard, &id)
                .with_context(|| format!("{}", self.path.display()))?,
            path: self.path.clone(),
            group: group.clone(),
        })
    }
}

/// Every rule across `writs` whose guard names `kind` as this tool's.
pub fn twrit_rules(writs: &[Writ], kind: &str) -> Result<Vec<Rule>> {
    let mut found = Vec::new();
    for writ in writs {
        for rule in writ.rules()? {
            if matches!(&rule.guard, Guard::Twrit(k) if k == kind) {
                found.push(rule);
            }
        }
    }
    Ok(found)
}

/// Returns the slice of elements if `value` is either a sequence or a `list(...)` call.
pub fn as_list(value: &Value) -> Option<&[Value]> {
    match value {
        Value::Seq(items) => Some(items.as_slice()),
        Value::Call(name, args) if name == "list" => Some(args.as_slice()),
        _ => None,
    }
}

/// Extracts a list of strings from either a `Value::Seq`, a `list(...)` call,
/// or bracketed `Value::Blocks`.
pub fn as_string_list(value: &Value) -> Option<Vec<String>> {
    if let Some(items) = as_list(value) {
        let mut strings = Vec::with_capacity(items.len());
        for item in items {
            if let Value::String(s) = item {
                strings.push(s.clone());
            } else {
                return None;
            }
        }
        return Some(strings);
    }

    if let Value::Blocks(blocks) = value {
        let mut strings = Vec::new();
        for b in blocks {
            if let Block::Paragraph(p) = b {
                for inline in &p.content {
                    if let tomet_ast::Inline::Text(t) = inline {
                        for part in t.value.split(',') {
                            let trimmed = part.trim().trim_matches('"').trim_matches('\'');
                            if !trimmed.is_empty() {
                                strings.push(trimmed.to_string());
                            }
                        }
                    }
                }
            }
        }
        if !strings.is_empty() {
            return Some(strings);
        }
    }

    None
}

/// Reads a rule parameter's `{...}` as `key: [string, ...]` pairs.
///
/// Returns an error rather than skipping anything it cannot read: a rule
/// half-understood is worse than a rule that refuses to run, because the
/// half it dropped is exactly where a violation would hide.
pub fn string_list_map(
    entries: &[(String, Value)],
    what: &str,
) -> Result<Vec<(String, Vec<String>)>> {
    entries
        .iter()
        .map(|(key, value)| {
            if let Some(items) = as_list(value) {
                let strings = items
                    .iter()
                    .map(|item| match item {
                        Value::String(s) => Ok(s.clone()),
                        other => {
                            anyhow::bail!("{what}'s `{key}` holds a non-string entry: {other:?}")
                        }
                    })
                    .collect::<Result<Vec<_>>>()?;
                return Ok((key.clone(), strings));
            }
            if let Some(strings) = as_string_list(value) {
                return Ok((key.clone(), strings));
            }
            anyhow::bail!("{what}'s `{key}` is not a list");
        })
        .collect()
}

fn collect_rule_elements<'a>(
    blocks: &'a [Block],
    is_rule: &impl Fn(&Element) -> bool,
    out: &mut Vec<&'a Element>,
) {
    for block in blocks {
        match block {
            Block::Element(el) => {
                if is_rule(el) {
                    out.push(el);
                }
                if let Some(content) = &el.content {
                    collect_rule_elements(content, is_rule, out);
                }
            }
            Block::Section(sec) => {
                collect_rule_elements(&sec.blocks, is_rule, out);
            }
            Block::List(list) => {
                collect_rule_list(list, is_rule, out);
            }
            Block::Paragraph(_) => {}
        }
    }
}

fn collect_rule_list<'a>(
    list: &'a tomet_ast::List,
    is_rule: &impl Fn(&Element) -> bool,
    out: &mut Vec<&'a Element>,
) {
    for item in &list.items {
        if is_rule(&item.element) {
            out.push(&item.element);
        }
        if let Some(content) = &item.element.content {
            collect_rule_elements(content, is_rule, out);
        }
        if let Some(sublist) = &item.sublist {
            collect_rule_list(sublist, is_rule, out);
        }
    }
}
