//! What `twrit list` prints: every rule, who holds it, and whether that
//! holder is still there.
//!
//! `check` answers "does the code obey the rules". This answers "is
//! anything holding them", which is a different question and the one this
//! tool was built to make askable. Expectations here were written before
//! the command existed.

#[test]
fn every_rule_is_listed_with_its_holder() {
    // Four spellings, and the column says what each one means rather than
    // repeating the key: `twrit` runs it here, `runs` is recorded and not
    // executed, `test` is a pointer this can at least follow, and a rule
    // held by nobody shows the reason instead of a holder.
    //
    // `renamed-away` points at a test that is gone. Nothing else in this
    // tool would ever have said so: the rule stays written, the guard
    // stays named, and the file it names stopped existing.
    assert_eq!(
        twrit_tests::list("guard-kinds"),
        "\
crate-layering           twrit  layers
generated-md-not-edited  runs   just docs-check
no-vocabulary            test   tests/src/present.rs
renamed-away             test   tests/src/gone.rs -- MISSING
blueprint-shape          none   no cheap check for the args shape yet

1 writ, 5 rules (1 unguarded), 1 dead guard
"
    );
}

#[test]
fn a_repository_with_no_writ_says_so() {
    assert_eq!(
        twrit_tests::list("no-writ"),
        format!(
            "no .writ.tmt found under {}\n",
            twrit_tests::fixture("no-writ").display()
        )
    );
}

#[test]
fn a_present_script_is_listed_with_script_holder() {
    assert_eq!(
        twrit_tests::list("script-pass"),
        "clean-script  script  scripts/check.sh\n\n1 writ, 1 rule\n"
    );
}

#[test]
fn a_missing_script_is_marked_missing_in_list() {
    assert_eq!(
        twrit_tests::list("script-missing"),
        "missing-script  script  scripts/missing.sh -- MISSING\n\n1 writ, 1 rule, 1 dead guard\n"
    );
}
