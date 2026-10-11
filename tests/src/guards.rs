//! What each guard reports, one fixture per failure path.
//!
//! Every expectation below was written before the code was run. Where the
//! two disagreed, the message that moved was the code's.

#[test]
fn upward_dependency_is_reported() {
    // `mid` (layer 1) depends on `top` (layer 2). Both sides name their
    // own directory, because the directory is the half of a crate's
    // identity that `layers` matches on -- a reader has to be able to
    // find both ends in the declaration they just violated.
    assert_eq!(
        twrit_tests::check("upward-edge"),
        "crate-layering: upward dependency: mid (layer 1, mid) -> top (layer 2, top)\n\
         1 writ, 1 rule, crate-layering: 1 violation (3 members)\n"
    );
}

#[test]
fn unclassified_member_is_reported() {
    // The half of the layering rule that catches a rule never written.
    // A member no pattern covers is not "unconstrained" -- it is a hole
    // in the declaration, and nothing else would ever point at it.
    assert_eq!(
        twrit_tests::check("unclassified"),
        "crate-layering: unclassified member: stray matches no layer in `layers`\n\
         1 writ, 1 rule, crate-layering: 1 violation (2 members)\n"
    );
}

#[test]
fn ambiguous_member_names_both_layers() {
    // Both patterns, not just the count: the fix is to change one of
    // them, and the reader needs to know which two are in play.
    assert_eq!(
        twrit_tests::check("ambiguous"),
        "crate-layering: ambiguous member: thing matches layer 1 (`thing*`) and layer 0 (`thing`)\n\
         1 writ, 1 rule, crate-layering: 1 violation (1 member)\n"
    );
}

#[test]
fn the_floor_may_not_depend_on_a_member() {
    // Sideways, not upward: both crates sit on layer 0, so the direction
    // check passes and this is the rule that has to catch it.
    assert_eq!(
        twrit_tests::check("floor-depends"),
        "crate-layering: layer 0 depends on a workspace member: base -> helper\n\
         1 writ, 1 rule, crate-layering: 1 violation (2 members)\n"
    );
}

#[test]
fn a_dev_dependency_may_reach_upward() {
    // `upward-edge` with the one edge moved to `[dev-dependencies]`.
    // The exemption is written into the writ, so the guard has to be
    // seen honouring it, not just seen catching things.
    assert_eq!(
        twrit_tests::check("dev-dependency"),
        "1 writ, 1 rule, crate-layering: ok (3 members)\n"
    );
}

// A writ this tool cannot read is an error, not a violation. These pin
// the message, because the message is the whole of what the author gets.

#[test]
fn a_layer_key_must_be_a_number() {
    assert_eq!(
        twrit_tests::check_err("layers-not-a-number"),
        "`layers` key `low` is not a layer number: invalid digit found in string"
    );
}

#[test]
fn a_rule_guarding_layers_without_the_parameter_is_refused() {
    // The guard says this tool holds the rule; the parameter it would
    // read is absent. Refusing is the only honest answer -- running with
    // no patterns would classify nothing and report clean.
    assert_eq!(
        twrit_tests::check_err("layers-no-group"),
        format!(
            "{}: `@rule(crate-layering)` guards `layers` but declares no `layers:` parameter",
            twrit_tests::fixture("layers-no-group")
                .join(".writ.tmt")
                .display()
        )
    );
}

#[test]
fn a_layer_must_hold_a_list() {
    assert_eq!(
        twrit_tests::check_err("layers-not-a-list"),
        "`layers`'s `1` is not a list"
    );
}

#[test]
fn a_pattern_must_be_a_string() {
    // Only the prefix. The tail is `tomet-ast`'s `Debug` for whatever was
    // found, which is that crate's business and not a promise this suite
    // makes.
    let msg = twrit_tests::check_err("layers-non-string");
    assert!(
        msg.starts_with("`layers`'s `1` holds a non-string entry: "),
        "unexpected message: {msg}"
    );
}

#[test]
fn an_empty_layers_declaration_is_refused() {
    // Not "everything is unclassified" -- that reports the symptom N
    // times and never the cause.
    assert_eq!(
        twrit_tests::check_err("layers-empty"),
        "`layers` declares no members"
    );
}

#[test]
fn two_layer_declarations_are_refused() {
    assert_eq!(
        twrit_tests::check_err("layers-twice"),
        "more than one rule guards `layers` (crate-layering, crate-layering); a layer order has to be one thing"
    );
}

// -- Known holes -------------------------------------------------------
//
// These assert what the tool does today, not what it should do. They are
// written down rather than left out because a guard that quietly stopped
// checking looks exactly like a guard that passes, and a hole nobody has
// written down is indistinguishable from one nobody has found. Each will
// fail when the hole is closed, which is the point.

#[test]
fn a_rule_with_no_group_is_refused() {
    // CLOSED HOLE. This fixture used to hold the worst failure this tool
    // had: `@layers` written without its `{...}` was absorbed into the
    // paragraph above, never became an element, and the guard reported
    // nothing -- indistinguishable from a repository with no layering
    // rule at all. One missing blank line could do the same.
    //
    // It is not detectable-now, it is impossible-now. Parameters live
    // inside the entry, so a declaration cannot come detached from the
    // rule that owns it, and a rule with no group is refused by name.
    assert_eq!(
        twrit_tests::check_err("layers-bare"),
        format!(
            "{}: `@rule(crate-layering)` has no `{{...}}` group, so it declares neither a \
             guard nor parameters",
            twrit_tests::fixture("layers-bare")
                .join(".writ.tmt")
                .display()
        )
    );
}

// -- parser-purity -----------------------------------------------------

#[test]
fn an_unlisted_external_dependency_is_reported() {
    assert_eq!(
        twrit_tests::check("pure-external"),
        "parser-purity: core reaches an external crate not in `allow-external`: core -> outside\n\
         1 writ, 1 rule, parser-purity: 1 violation (1 crate)\n"
    );
}

#[test]
fn an_external_reached_through_a_member_is_reported() {
    // The closure, not the direct list. `core`'s own manifest names one
    // internal dependency and reads like the whole story; `inner` is what
    // reaches outside. The edge reported is the real one, not `core`'s.
    assert_eq!(
        twrit_tests::check("pure-closure"),
        "parser-purity: core reaches an external crate not in `allow-external`: inner -> outside\n\
         1 writ, 1 rule, parser-purity: 1 violation (1 crate)\n"
    );
}

#[test]
fn a_forbidden_path_names_the_file_relative_to_the_root() {
    // Relative to the workspace root, not absolute: the message goes into
    // CI logs and diffs, and an absolute path makes it differ per machine
    // and per invocation -- `twrit check .` and `twrit check /abs` would
    // otherwise report the same finding two ways.
    assert_eq!(
        twrit_tests::check("pure-forbidden"),
        "parser-purity: core names a forbidden path: std::fs in core/src/lib.rs\n\
         1 writ, 1 rule, parser-purity: 1 violation (1 crate)\n"
    );
}

#[test]
fn a_pure_crate_that_does_not_exist_is_reported() {
    assert_eq!(
        twrit_tests::check("pure-not-a-member"),
        "parser-purity: `pure` names nope, which is not a workspace member\n\
         1 writ, 1 rule, parser-purity: 1 violation (1 crate)\n"
    );
}

#[test]
fn an_unknown_pure_key_is_refused() {
    // Not ignored. An unread key is a rule its author believes is being
    // enforced and is not.
    assert_eq!(
        twrit_tests::check_err("pure-unknown-key"),
        "`pure` does not know the key `colour`"
    );
}

#[test]
fn a_pure_declaration_naming_no_crate_is_refused() {
    assert_eq!(
        twrit_tests::check_err("pure-no-crates"),
        "`pure` names no crates"
    );
}

#[test]
fn a_forbidden_path_in_build_rs_is_missed() {
    // KNOWN HOLE. `grep_sources` walks `<crate>/src` only, so `build.rs`
    // is never read -- and a build script is the plainest way for a crate
    // that may not touch the filesystem to touch it anyway.
    assert_eq!(
        twrit_tests::check("pure-build-rs"),
        "1 writ, 1 rule, parser-purity: ok (1 crate)\n"
    );
}

#[test]
fn a_forbidden_path_in_an_internal_dependency_is_missed() {
    // KNOWN HOLE, and an asymmetry rather than an omission: the
    // `allow-external` half walks the whole closure, so `inner` is inside
    // the trust boundary, while the `forbid` half stops at `core/src`.
    // One check trusts the crate, the other never looks at it.
    assert_eq!(
        twrit_tests::check("pure-dep-source"),
        "1 writ, 1 rule, parser-purity: ok (1 crate)\n"
    );
}

// -- the entry shape ---------------------------------------------------

#[test]
fn a_namespaced_rule_resolves_the_same() {
    // The bug this whole shape came out of. `@rule` and `@writ.rule` are
    // two legal spellings of one element in a `@kind(writ)` document, and
    // this tool used to match a bare `Sigil::Named` -- finding the first
    // and silently ignoring the second. A rule silently ignored is a rule
    // that stopped being checked.
    //
    // The fixture is `upward-edge` with the namespaced spelling and
    // nothing else changed, so the expectation is that file's, verbatim.
    assert_eq!(
        twrit_tests::check("rule-namespaced"),
        "crate-layering: upward dependency: mid (layer 1, mid) -> top (layer 2, top)\n\
         1 writ, 1 rule, crate-layering: 1 violation (3 members)\n"
    );
}

#[test]
fn a_rule_without_a_guard_is_refused() {
    // Not "unguarded": unwritten. Counting it as unguarded would make an
    // oversight and a decision look alike, which is the disease this
    // field exists to cure.
    assert_eq!(
        twrit_tests::check_err("rule-no-guard"),
        format!(
            "{}: `@rule(crate-layering)` declares no `guard`; write `guard: {{ none: \"...\" }}` \
             when nothing holds it yet",
            twrit_tests::fixture("rule-no-guard")
                .join(".writ.tmt")
                .display()
        )
    );
}

#[test]
fn an_unguarded_rule_must_say_why() {
    // `none` is a decision and reads as one only with a reason attached.
    // Empty, it is the silence it was meant to replace.
    assert_eq!(
        twrit_tests::check_err("rule-none-no-reason"),
        format!(
            "{}: `@rule(blueprint-shape)`'s `guard: none` has no reason; an unguarded rule says \
             why, so it cannot be read as an oversight",
            twrit_tests::fixture("rule-none-no-reason")
                .join(".writ.tmt")
                .display()
        )
    );
}

#[test]
fn an_unknown_guard_holder_is_refused() {
    // The four holders are the whole vocabulary. A fifth spelling is a
    // guard field nobody reads, which is a rule nobody checks.
    assert_eq!(
        twrit_tests::check_err("rule-unknown-guard"),
        format!(
            "{}: `@rule(crate-layering)`'s guard holder `by` is not one of `twrit`, `runs`, \
             `test`, `none`",
            twrit_tests::fixture("rule-unknown-guard")
                .join(".writ.tmt")
                .display()
        )
    );
}

#[test]
fn a_twrit_kind_this_tool_does_not_have_is_refused() {
    // The registry check. A rule may say `twrit:` only about a kind that
    // exists here; otherwise the writ claims this tool holds it and
    // nothing does. Refused rather than skipped, and the message names
    // what is on offer so the author can see the typo.
    assert_eq!(
        twrit_tests::check_err("rule-unknown-kind"),
        format!(
            "{}: `@rule(import-cycles)` names `twrit: cycles`, which this tool does not \
              implement; it implements layers, pure, door, rust-api, placement",
            twrit_tests::fixture("rule-unknown-kind")
                .join(".writ.tmt")
                .display()
        )
    );
}

#[test]
fn a_guard_pointing_at_nothing_fails_the_check() {
    // The writ says a test holds this rule. The test is not there, so the
    // statement is false and the rule is held by nobody -- while still
    // reading, in the writ and in `list`, as guarded.
    //
    // Reported under the rule that made the claim, not under a rule kind:
    // nothing about `crate-layering` or `parser-purity` is wrong here.
    // What is wrong is one entry's own account of itself.
    assert_eq!(
        twrit_tests::check("guard-kinds"),
        "renamed-away: guard names tests/src/gone.rs, which does not exist\n\
         1 writ, 5 rules (1 unguarded), 1 dead guard, crate-layering: ok (1 member)\n"
    );
}

#[test]
fn a_dead_guard_is_counted_as_a_violation() {
    // Which is what makes the run exit non-zero. A finding only `list`
    // shows is a finding only a person who remembered to look will see,
    // and `check` is what CI runs.
    assert_eq!(twrit_tests::report("guard-kinds").violation_count(), 1);
}

#[test]
fn an_allow_external_entry_nothing_reaches_is_reported() {
    // The other direction of the allowlist. An entry outside the closure
    // is not permissive -- it is a claim about this workspace that has
    // stopped being true, and saying what is true is the entire reason
    // the list is worth more than "no external dependencies".
    assert_eq!(
        twrit_tests::check("pure-unused-allow"),
        "parser-purity: `allow-external` lists gone, which nothing under core reaches\n\
         1 writ, 1 rule, parser-purity: 1 violation (1 crate)\n"
    );
}
