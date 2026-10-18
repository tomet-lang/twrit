//! `twrit` -- core engine and models for enforcing rules in `.writ.tmt` files.
//!
//! A rule nobody checks is a wish. The point of this tool is that a writ
//! entry naming a guard can be held to it, so the rule fails loudly rather
//! than drifting quietly out of true.

pub mod writ;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Result;
use writ::Rule;

/// What one rule did on this run.
pub struct Outcome {
    pub scope: String,
    pub violations: Vec<String>,
}

impl Outcome {
    pub fn checked(scope: String, violations: Vec<String>) -> Self {
        Outcome { scope, violations }
    }

    pub fn violations(&self) -> &[String] {
        &self.violations
    }
}

/// One rule's result, reported under the id its writ gave it.
pub struct RuleResult {
    pub id: String,
    pub outcome: Outcome,
}

/// A registered guard implementation.
#[derive(Clone, Copy)]
pub struct Guard {
    pub kind: &'static str,
    pub singleton: bool,
    pub check: fn(rule: &Rule, root: &Path) -> Result<Outcome>,
}

/// Everything one `check` run has to say.
pub struct Report {
    root: PathBuf,
    writ_count: usize,
    rule_count: usize,
    unguarded_count: usize,
    dead_guards: Vec<String>,
    rules: Vec<RuleResult>,
}

impl Report {
    pub fn violation_count(&self) -> usize {
        self.rules
            .iter()
            .map(|r| r.outcome.violations().len())
            .sum::<usize>()
            + self.dead_guards.len()
    }

    pub fn rules(&self) -> &[RuleResult] {
        &self.rules
    }

    pub fn writ_count(&self) -> usize {
        self.writ_count
    }

    pub fn rule_count(&self) -> usize {
        self.rule_count
    }

    pub fn unguarded_count(&self) -> usize {
        self.unguarded_count
    }

    pub fn render(&self) -> String {
        if self.writ_count == 0 {
            return format!("no .writ.tmt found under {}\n", self.root.display());
        }

        let mut out = String::with_capacity(512);
        for finding in &self.dead_guards {
            let _ = writeln!(out, "{finding}");
        }
        for rule in &self.rules {
            for violation in rule.outcome.violations() {
                let _ = writeln!(out, "{}: {violation}", rule.id);
            }
        }

        let summary: Vec<String> = self
            .rules
            .iter()
            .map(|rule| {
                let Outcome { scope, violations } = &rule.outcome;
                if violations.is_empty() {
                    format!("{}: ok ({scope})", rule.id)
                } else {
                    format!(
                        "{}: {} ({scope})",
                        rule.id,
                        plural(violations.len(), "violation")
                    )
                }
            })
            .collect();

        let mut census = plural(self.rule_count, "rule");
        if self.unguarded_count > 0 {
            let _ = write!(census, " ({} unguarded)", self.unguarded_count);
        }
        if !self.dead_guards.is_empty() {
            let _ = write!(census, ", {}", plural(self.dead_guards.len(), "dead guard"));
        }

        let _ = writeln!(
            out,
            "{}, {census}, {}",
            plural(self.writ_count, "writ"),
            summary.join(", ")
        );
        out
    }
}

pub fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("{n} {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

pub struct Listed {
    pub id: String,
    pub holder: &'static str,
    pub target: String,
    pub dead: bool,
}

pub struct Listing {
    root: PathBuf,
    writ_count: usize,
    rules: Vec<Listed>,
}

impl Listing {
    pub fn rules(&self) -> &[Listed] {
        &self.rules
    }

    pub fn dead_count(&self) -> usize {
        self.rules.iter().filter(|r| r.dead).count()
    }

    pub fn render(&self) -> String {
        if self.writ_count == 0 {
            return format!("no .writ.tmt found under {}\n", self.root.display());
        }

        let id_width = self.rules.iter().map(|r| r.id.len()).max().unwrap_or(0);
        let holder_width = self.rules.iter().map(|r| r.holder.len()).max().unwrap_or(0);

        let mut out = String::new();
        for rule in &self.rules {
            let dead = if rule.dead { " -- MISSING" } else { "" };
            out.push_str(&format!(
                "{:id_width$}  {:holder_width$}  {}{dead}\n",
                rule.id, rule.holder, rule.target
            ));
        }

        let unguarded = self.rules.iter().filter(|r| r.holder == "none").count();
        let mut summary = format!(
            "\n{}, {}",
            plural(self.writ_count, "writ"),
            plural(self.rules.len(), "rule")
        );
        if unguarded > 0 {
            summary.push_str(&format!(" ({unguarded} unguarded)"));
        }
        let dead = self.dead_count();
        if dead > 0 {
            summary.push_str(&format!(", {}", plural(dead, "dead guard")));
        }
        summary.push('\n');
        out.push_str(&summary);
        out
    }
}

pub fn list(root: &Path, guards: &[Guard]) -> Result<Listing> {
    let writs = writ::discover(root)?;
    if writs.is_empty() {
        return Ok(Listing {
            root: root.to_path_buf(),
            writ_count: 0,
            rules: Vec::new(),
        });
    }

    let known_kinds: Vec<&str> = guards.iter().map(|g| g.kind).collect();
    let mut rules = Vec::new();
    for w in &writs {
        for rule in w.rules()? {
            let (holder, target, dead) = match &rule.guard {
                writ::Guard::Twrit(kind) => {
                    ("twrit", kind.clone(), !known_kinds.contains(&kind.as_str()))
                }
                writ::Guard::Runs(cmd) => ("runs", cmd.clone(), false),
                writ::Guard::Test(path) => ("test", path.clone(), false),
                writ::Guard::None(why) => ("none", why.clone(), false),
            };
            rules.push(Listed {
                dead: dead || rule.guard.dead_pointer(root).is_some(),
                id: rule.id,
                holder,
                target,
            });
        }
    }

    Ok(Listing {
        root: root.to_path_buf(),
        writ_count: writs.len(),
        rules,
    })
}

pub fn check(root: &Path, guards: &[Guard]) -> Result<Report> {
    let writs = writ::discover(root)?;
    if writs.is_empty() {
        return Ok(Report {
            root: root.to_path_buf(),
            writ_count: 0,
            rule_count: 0,
            unguarded_count: 0,
            dead_guards: Vec::new(),
            rules: Vec::new(),
        });
    }

    let declared: Vec<Rule> = writs
        .iter()
        .map(|w| w.rules())
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    let known_kinds: Vec<&str> = guards.iter().map(|g| g.kind).collect();

    for rule in &declared {
        if let writ::Guard::Twrit(kind) = &rule.guard
            && !known_kinds.contains(&kind.as_str())
        {
            anyhow::bail!(
                "{}: `@rule({})` names `twrit: {kind}`, which this tool does not implement; \
                     it implements {}",
                rule.path.display(),
                rule.id,
                known_kinds.join(", ")
            );
        }
    }

    for guard in guards {
        if guard.singleton {
            let ids: Vec<&str> = declared
                .iter()
                .filter(|r| r.guard == writ::Guard::Twrit(guard.kind.to_string()))
                .map(|r| r.id.as_str())
                .collect();
            if ids.len() > 1 {
                anyhow::bail!(
                    "more than one rule guards `{}` ({}); a layer order has to be one thing",
                    guard.kind,
                    ids.join(", ")
                );
            }
        }
    }

    let mut rules = Vec::new();
    for rule in &declared {
        let writ::Guard::Twrit(kind) = &rule.guard else {
            continue;
        };
        let Some(guard) = guards.iter().find(|g| g.kind == kind.as_str()) else {
            unreachable!("checked against known_kinds above");
        };
        let outcome = (guard.check)(rule, root)?;
        rules.push(RuleResult {
            id: rule.id.clone(),
            outcome,
        });
    }

    Ok(Report {
        root: root.to_path_buf(),
        writ_count: writs.len(),
        dead_guards: declared
            .iter()
            .filter_map(|rule| {
                rule.guard.dead_pointer(root).map(|target| {
                    format!("{}: guard names {target}, which does not exist", rule.id)
                })
            })
            .collect(),
        rule_count: declared.len(),
        unguarded_count: declared
            .iter()
            .filter(|r| matches!(r.guard, writ::Guard::None(_)))
            .count(),
        rules,
    })
}
