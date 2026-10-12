use std::fs;

fn all_guards() -> Vec<twrit_core::Guard> {
    let mut guards = twrit_rust::rust_guards();
    guards.extend(twrit_generic::generic_guards());
    guards
}

#[test]
fn rust_api_guard_lifecycle() {
    let fixture_dir = twrit_tests::fixture("rust-api");
    let out_dir = fixture_dir.join("docs/crates");
    let _ = fs::remove_dir_all(&out_dir);

    let guards = all_guards();

    // 1. Initial check: target file does not exist yet -> should report violation
    let report_initial = twrit_core::check(&fixture_dir, &guards).unwrap();
    assert_eq!(report_initial.violation_count(), 1);
    assert!(report_initial.render().contains("does not exist"));

    // 2. Generate documentation
    let writs = twrit_core::writ::discover(&fixture_dir).unwrap();
    let api_rules = twrit_core::writ::twrit_rules(&writs, "rust-api").unwrap();
    assert_eq!(api_rules.len(), 1);
    let count = twrit_rust::api::generate(&api_rules[0], &fixture_dir).unwrap();
    assert_eq!(count, 1);

    let doc_file = out_dir.join("fixture-api-test.tmt");
    assert!(doc_file.exists());
    let content = fs::read_to_string(&doc_file).unwrap();
    assert!(content.contains("=[fixture-api-test]"));
    assert!(content.contains("Top-level library documentation."));
    assert!(content.contains("fn greeting"));

    // 3. Re-check after generation -> should pass with 0 violations
    let report_after_gen = twrit_core::check(&fixture_dir, &guards).unwrap();
    assert_eq!(report_after_gen.violation_count(), 0);

    // 4. Stale check: modify generated doc -> should report violation
    fs::write(&doc_file, "stale content").unwrap();
    let report_stale = twrit_core::check(&fixture_dir, &guards).unwrap();
    assert_eq!(report_stale.violation_count(), 1);
    assert!(report_stale.render().contains("differs from source"));

    // Clean up generated output
    let _ = fs::remove_dir_all(fixture_dir.join("docs"));
}
