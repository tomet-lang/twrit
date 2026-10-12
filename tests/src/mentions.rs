//! What the `mentions` guard reports.

#[test]
fn non_existent_rule_mention_is_reported() {
    assert_eq!(
        twrit_tests::check("mentions-invalid"),
        "valid-rules: src/main.rs:2: mentions `@rule(ghost-rule)`, which is not declared in any writ\n\
         1 writ, 2 rules (1 unguarded), valid-rules: 1 violation (1 file)\n"
    );
}

#[test]
fn valid_rule_mention_passes() {
    assert_eq!(
        twrit_tests::check("mentions-valid"),
        "1 writ, 2 rules (1 unguarded), valid-rules: ok (1 file)\n"
    );
}
