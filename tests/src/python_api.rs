use std::fs;

fn all_guards() -> Vec<twrit::Guard> {
    let mut guards = twrit_rust::rust_guards();
    guards.extend(twrit_python::python_guards());
    guards.push(twrit::placement_guard());
    guards
}

#[test]
fn python_api_guard_lifecycle() {
    let fixture_dir = twrit_tests::fixture("python-api");
    let out_dir = fixture_dir.join("docs/api");
    let _ = fs::remove_dir_all(&out_dir);

    let guards = all_guards();

    // 1. Initial check: target file does not exist yet -> should report violation
    let report_initial = twrit::check(&fixture_dir, &guards).unwrap();
    assert_eq!(report_initial.violation_count(), 1);
    assert!(report_initial.render().contains("does not exist"));

    // 2. Generate documentation
    let writs = twrit::writ::discover(&fixture_dir).unwrap();
    let api_rules = twrit::writ::twrit_rules(&writs, "python-api").unwrap();
    assert_eq!(api_rules.len(), 1);
    let count = twrit_python::api::generate(&api_rules[0], &fixture_dir).unwrap();
    assert_eq!(count, 1);

    let doc_file = out_dir.join("pkg.calc.tmt");
    assert!(doc_file.exists());
    let content = fs::read_to_string(&doc_file).unwrap();
    assert!(content.contains("=[pkg.calc]"));
    assert!(content.contains("Calculation utilities."));
    assert!(content.contains("def add(a, b=...)"));
    assert!(content.contains("Adds two numbers together."));

    // 3. Re-check after generation -> should pass with 0 violations
    let report_after_gen = twrit::check(&fixture_dir, &guards).unwrap();
    assert_eq!(report_after_gen.violation_count(), 0);

    // 4. Stale check: modify generated doc -> should report violation
    fs::write(&doc_file, "stale content").unwrap();
    let report_stale = twrit::check(&fixture_dir, &guards).unwrap();
    assert_eq!(report_stale.violation_count(), 1);
    assert!(report_stale.render().contains("differs from source"));

    // Clean up generated output
    let _ = fs::remove_dir_all(fixture_dir.join("docs"));
}
