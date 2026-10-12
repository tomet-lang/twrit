//! Finding and reading writs, before any rule looks at one.

#[test]
fn a_writ_is_found_despite_its_leading_dot() {
    // `.writ.tmt` is the nameless form of the `<name>.<kind>.tmt`
    // convention, so the dot is a kind separator rather than a request to
    // be invisible -- and every directory walker treats it as the latter
    // by default, which is how `tomet`'s own `check-links` came to skip
    // these files.
    let writs = twrit_core::writ::discover(&twrit_tests::fixture("upward-edge"))
        .expect("the fixture should be readable");
    assert_eq!(writs.len(), 1);
}

#[test]
fn every_writ_below_the_root_is_found() {
    // Not just the one at the top. Rules are scoped to a directory and
    // inherited downward, so a rule can be declared anywhere under it.
    let writs = twrit_core::writ::discover(&twrit_tests::fixture("layers-twice"))
        .expect("the fixture should be readable");
    assert_eq!(writs.len(), 2);
}

#[test]
fn a_directory_with_no_writ_is_not_a_failure() {
    // A repository that has written no rules down has broken none. It is
    // worth saying out loud rather than printing nothing, because "no
    // writ here" and "every rule held" are different answers.
    let dir = twrit_tests::fixture("no-writ");
    assert_eq!(
        twrit_tests::check("no-writ"),
        format!("no .writ.tmt found under {}\n", dir.display())
    );
}

#[test]
fn a_parse_error_names_the_file_and_the_position() {
    // The path, because a repository has many writs and the walker found
    // this one; the position, because the author has to go and fix it.
    //
    // Both halves come from `tomet-load` now rather than from a parse
    // call here, so this also pins that the reason survives the error
    // chain exactly once -- `LoadError` says where and `source` says
    // what, and neither repeats the other.
    let dir = twrit_tests::fixture("unparseable");
    assert_eq!(
        twrit_tests::check_err("unparseable"),
        format!(
            "failed to parse {}/.writ.tmt: 19:1: expected '}}'",
            dir.display()
        )
    );
}
