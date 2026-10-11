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
docs:
    cargo build -p twrit-cli
    ./target/debug/twrit api .
    tomet export .
    tomet format -i .

# Check that all documents parse, format cleanly, and match export targets.
docs-check:
    cargo build -p twrit-cli
    ./target/debug/twrit check .
    tomet check .
    tomet format --check .
    tomet export --check .
