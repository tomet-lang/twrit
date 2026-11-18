# `cargo clippy -D warnings` fails on `main`: collapsible `if` (Rust edition 2024)

Found 2026-11-06 while following tomet Phase 1 in twrit. Not started. Unrelated to tomet: the same 15 errors appear on a clean `git archive main` build with the dev-shell toolchain (12 `this if statement can be collapsed`, 2 `this if let can be collapsed into the outer if let`, 1 `this if can be collapsed into the outer match`). CONTRIBUTING.md does not list clippy as a check, so nothing is red today.

Files (lines as of `main` 21283917): `crates/twrit-generic/src/mentions.rs` (32, 97), `crates/twrit-python/src/api.rs` (101), `crates/twrit-python/src/door.rs` (88), `crates/twrit-rust/src/api.rs` (207, 426, 450, 564, 575).

- [ ] Collapse them with let-chains (`if a && let Some(x) = b`), one commit `refactor: ...` with no tomet change in it. The Phase 1 follow-up commit already rewrites the two `el.sigil` hunks in `mentions.rs` in that form, so rebase on top of it.
- [ ] Decide whether `cargo clippy --workspace --all-targets -- -D warnings` becomes part of `just docs-check` / CONTRIBUTING.md "Verification" so this does not come back.

Delete this file when done.
