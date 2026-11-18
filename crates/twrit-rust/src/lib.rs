//! `twrit-rust` -- Rust workspace rules enforcement (layering, dependencies, door, purity).

pub mod api;
pub mod dependencies;
pub mod door;
pub mod layering;
pub mod metadata;
pub mod purity;

use twrit_core::Guard;

pub fn rust_guards() -> Vec<Guard> {
    vec![
        Guard {
            kind: "layers",
            singleton: true,
            check: layering::check,
        },
        Guard {
            kind: "dependencies",
            singleton: false,
            check: dependencies::check,
        },
        Guard {
            kind: "pure",
            singleton: false,
            check: purity::check,
        },
        Guard {
            kind: "door",
            singleton: false,
            check: door::check,
        },
        Guard {
            kind: "rust-api",
            singleton: false,
            check: api::check,
        },
    ]
}
