# Twrit development tasks.

default:
    @just --list

build:
    cargo build --workspace

test:
    cargo test --workspace

test-suite:
    cargo test -p twrit-tests

# Generate and update all derived documentation across the workspace.
#
# `sisho` runs the `twrit` this checkout just built, not the one in the dev shell.
docs:
    cargo build -p twrit-cli
    ./target/debug/twrit api .
    PATH="{{ justfile_directory() }}/target/debug:$PATH" sisho fix .

# Check that all documents parse, format cleanly, and match export targets.
docs-check:
    cargo build -p twrit-cli
    PATH="{{ justfile_directory() }}/target/debug:$PATH" sisho check .
