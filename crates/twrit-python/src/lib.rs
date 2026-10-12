//! `twrit-python` -- Python workspace rules enforcement (door, python-api).

pub mod api;
pub mod door;

use twrit_core::Guard;

pub fn python_guards() -> Vec<Guard> {
    vec![
        Guard {
            kind: "python-door",
            singleton: false,
            check: door::check,
        },
        Guard {
            kind: "python-api",
            singleton: false,
            check: api::check,
        },
    ]
}
