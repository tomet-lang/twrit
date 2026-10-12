//! `twrit-generic` -- generic workspace rules (placement, mentions).

pub mod mentions;
pub mod placement;

use twrit_core::Guard;

pub fn generic_guards() -> Vec<Guard> {
    vec![
        Guard {
            kind: "placement",
            singleton: false,
            check: placement::check,
        },
        Guard {
            kind: "mentions",
            singleton: false,
            check: mentions::check,
        },
    ]
}
