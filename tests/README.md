# twrit-tests

What each guard reports, one fixture per failure path.

A guard is not finished when it passes. `AGENTS.md` asks for it to be
made to fail before it is believed -- mutate the writ it reads, confirm
each path reports and exits non-zero, then restore. This package is that
ritual written down, so it can be re-run by someone who was not there.

## The expectation is written first

**Every expectation here was typed before the code was run.** Where the
two disagreed, what moved was the code.

One exception, recorded rather than glossed: the four cases pinning the
entry shape (`rule-no-guard`, `rule-none-no-reason`, `rule-unknown-guard`,
`rule-unknown-kind`) had their message and their expectation written
together, because the message did not exist yet to disagree with. What
stands in for the ritual there is the mutation: each was made to fail on
purpose -- `KINDS` widened, `is_rule` reverted to the old bare match --
and seen to report before being believed.

This is the opposite of the sibling `tomet` repository's `ref/`, and
deliberately. Its references pin HTML and Markdown output, where the
correct answer is not knowable in advance, so they are a regression net
generated from current behavior. A guard's wanted error *is* knowable in
advance -- it is the specification -- and a reference generated from the
code would assert only that the code does what the code does. That is the
difference between a rule and a guard, which is the whole subject of this
tool; the suite must not make the mistake it exists to catch.

So expectations are inline `assert_eq!`s next to the case name. There is
no `UPDATE_REF` here and nothing committed to bless. (The root
`.gitignore` still carries a `tests/store/` line from `tomet`'s
convention; if an expectation ever grows too long to read inline, that is
where the machinery goes.)

Three of the messages this suite pins were wrong when it was written:

- `crate-layering` printed the *dependency's* directory inside the
  *depending* package's parentheses -- `mid (layer 1, top)`.
- `layers`'s own error messages named it without backticks, while every
  message `purity.rs` writes uses them.
- `parser-purity` named a forbidden path by absolute path, so the same
  finding read differently per machine and per invocation.

None would have been found by generating a reference and reading it.

## Layout

| Path | Contents |
| --- | --- |
| `fixtures/` | One directory per case: a writ, and a workspace where the rule needs one |
| `fixtures/default.config.tmt` | The vault every fixture is in |
| `fixtures/.tomet/vocabularies/` | The writ vocabulary `@rule` resolves against |
| `src/lib.rs` | Harness: locating a fixture, running `check` over it |
| `src/guards.rs` | What each guard reports |
| `src/list.rs` | What `twrit list` prints |
| `src/writ.rs` | Finding and reading writs, before any rule looks at one |

## The fixtures share one vault

A fixture's writ declares `::kind(writ)`, and `@rule` means nothing until
the vocabulary declaring it is in scope. Config discovery walks upward, so
one `default.config.tmt` at `fixtures/` governs all of them rather than
each carrying a copy -- which is also what the config file's *position*
means in Tomet: it is where the vault begins.

That vocabulary is kept in step with `tomet`'s own by hand. Nothing
checks the two against each other; if `@rule` grows a parameter there and
not here, these tests keep passing against a shape the real repository no
longer writes.

## Fixtures are real workspaces

The guards read a workspace through `cargo metadata`, so a fixture is a
real Cargo workspace and not an approximation of one -- testing an
approximation tests the approximation. The root manifest excludes
`tests/fixtures`, and each fixture carries its own `[workspace]`.

Every dependency is a path dependency, so no fixture needs a registry or
a network. Where a case needs an *external* crate -- one the purity guard
should not consider its own -- the fixture excludes a `vendor/`
directory, and the crates under it are dependencies without being
members.

`parser-purity` resolves the full dependency graph, so its fixtures have
a committed `Cargo.lock`. `crate-layering` uses `--no-deps` and needs
none.

A fixture is a directory you can enter and run by hand, which is still
the check `AGENTS.md` asks for:

```bash
cargo run -- check tests/fixtures/upward-edge
```

## Known holes are tests too

Some cases assert what the tool does today rather than what it should do,
under a `KNOWN HOLE` comment naming the gap:

- `parser-purity`'s `forbid` half never reads `build.rs`, nor the sources
  of a workspace member it reaches through. Its `allow-external` half
  walks the whole closure, so one check trusts a crate the other never
  looks at.

They are written down because a hole nobody has recorded is
indistinguishable from one nobody has found, and because a guard that
quietly stopped checking looks exactly like a guard that passes. Each
fails when its hole is closed, which is how the closing gets noticed.

One already has. A bare `@layers` -- the declaration written without its
`{...}` -- used to be absorbed into the paragraph above it, never become
an element, and leave the guard reporting nothing. That was not
hypothetical: it is why `crate-layering` had never once run against the
sibling `tomet` repository, for want of one blank line.

It is closed by the entry shape rather than by a check. Parameters live
inside `@rule(id){ ... }`, so a declaration cannot come detached from the
rule that owns it, and a rule with no group is refused by name. The
fixture is still here as `layers-bare`, asserting the refusal.

## Running

```bash
cargo test -p twrit-tests
cargo test -p twrit-tests --test guards
cargo test -p twrit-tests --test list
```
