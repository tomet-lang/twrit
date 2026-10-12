//! What the `python-door` guard reports.

#[test]
fn unauthorized_capability_is_reported() {
    assert_eq!(
        twrit_tests::check("python-door-unlisted"),
        "subprocess-door: other/worker.py is not a subprocess door, and references it\n\
         1 writ, 1 rule, subprocess-door: 1 violation (2 files)\n"
    );
}

#[test]
fn authorized_capability_passes() {
    assert_eq!(
        twrit_tests::check("python-door-clean"),
        "1 writ, 1 rule, subprocess-door: ok (2 files)\n"
    );
}
