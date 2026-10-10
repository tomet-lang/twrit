<!-- Generated from tmtroot/agents.tmt. Edit that, then `tomet export .`. -->

# Twrit

## Core Philosophy & Principles

### Precedence of Instructions

The rules in this document serve as the project baseline and defaults. Explicit instructions given by the user in a conversation always take precedence over these defaults.

### User-First Design over Developer Convenience

Always prioritize the end-user experience, syntax ergonomics, and API design.
If there is tension between developer or implementation convenience and user simplicity, user simplicity must always win. Never compromise user experience just because an implementation is easier.

### Resist Path-of-Least-Resistance & Legacy Bias

The existing implementation is never a justification or reason for new design choices.

- Do not justify decisions with "it is already implemented this way, so let's stick with it."
- Prioritize architectural beauty, elegance, and long-term correctness over the path of least resistance.
- Understand the existing context thoroughly, but never take shortcuts or settle for lazy patches.

### Transparent Execution

Always state what you intend to do before doing it. Never execute non-trivial actions or commands silently.

## Single Source of Truth & Documentation

### Never edit `.md` directly

All Markdown files (`README.md`, `AGENTS.md`, `CLAUDE.md`) are generated build artifacts, not source files.

- Never edit a `.md` file directly.
- Always edit the corresponding `.tmt` file under `tmtroot/`.
- Run `just docs` (or `tomet export .`) to regenerate the Markdown artifacts, and verify with `just docs-check`.
- Direct edits to generated `.md` files will be overwritten and will fail CI / lint checks.

## What this is

`twrit` enforces the rules a repository writes down in its `.writ.tmt`
files. A rule nobody checks is a wish; this tool is what makes a written
rule fail loudly instead of drifting quietly out of true.

A *writ* is an author-owned normative file, one per directory, scoped to
that directory and inherited downward the way `.gitignore` is. Entries carry
**id / rule / why / guard**, and no status field -- a withdrawn rule is
deleted and git holds the history. Only the author edits a writ; an agent
that finds a rule in its way proposes an edit and stops, rather than working
around the rule or relaxing it to fit.

The sibling `../tomet` repository is both the language this reads and the
first consumer of this tool. Its `.writ.tmt` files are the working examples;
read them before changing how writs are parsed.

Read `CONTRIBUTING.md` for the contribution workflow, commit conventions, and verification steps.

## The two repositories

`.cargo/config.toml` carries a `[patch]` pointing the tomet git dependency
at the sibling checkout, so a change there is visible here without a round
trip through Forgejo. It applies to every cargo invocation under this
directory, including `cargo metadata` on the fixture workspaces -- which is
why eight of them carry a `[[patch.unused]]` block in their lockfile.
Inert, and stable across runs.

Neither builds the other. `tomet`'s flake takes `twrit` as a dev-shell tool
for project management; this flake takes `tomet` as a dev-shell tool for
formatting `.tmt` files. `flake.lock` pins both, so the mutual inputs
resolve over commits rather than looping.

At the Cargo level the direction is one way: `twrit-cli` depends on
`tomet-load`, `tomet-parser`, `tomet-ast`, `tomet-tree` and
`tomet-semantics` as git dependencies, because reading a `.writ.tmt` means
parsing Tomet *and resolving* what it parsed. Nothing in `tomet` depends on
this crate.

`.cargo/config.toml` sets `net.git-fetch-with-cli`: the tomet repository is
on a private Forgejo over SSH and cargo's built-in libgit2 fetcher cannot
authenticate against it.

## Reading a writ

An entry is one element:

```tmt
##[ any wording the author likes ]

@rule(crate-layering){
  guard: { twrit: layers }

  layers: { 6: [ "apps/*" ], 5: [ ... ] }
}

Every workspace member belongs to exactly one layer ...

---[ Why ]---
...
```

The heading is prose and will be reworded; `@rule(id)` is the name a tool
is held to. Parameters live *inside* the group, which is the point: a
declaration cannot come detached from the rule that owns it, and a rule
with no group is a refusal rather than a silence. It was not always so --
`@layers` used to float at the top level owned by nothing, and one missing
blank line before it deleted `tomet`'s layering rule for as long as that
rule had existed, without anything reporting it.

`guard:` is required and names exactly one holder:

| Spelling | Means | What this tool does |
| --- | --- | --- |
| `{ twrit: kind }` | a kind implemented here | runs it; parameters come from the same group |
| `{ runs: "just docs-check" }` | a runner the repository ships | records it, never executes it |
| `{ test: "tests/src/x.rs" }` | a test over there | can check the path still exists |
| `{ none: "why not" }` | nobody, said out loud | counts it |

`none:` must carry a reason, and a rule with no `guard:` at all is an
error. "Nobody holds this" and "somebody forgot" must not look alike --
that is the same disease as a summary printing `ok` for a declaration it
never read.

A guard is a pointer, never a program. It answers only *who guarantees
this right now*. Putting a shell line in the entry was considered and
rejected: it is a Makefile wearing prose, and it would mean reading a
`.writ.tmt` could run arbitrary commands.

**Names are resolved, not matched.** A writ declares `@kind(writ)`, so
`@rule` and `@writ.rule` are two legal spellings of one element and only
`Bindings::classify` knows that. Reading goes through `tomet_load::Vault`,
which assembles config discovery, vocabulary loading, parsing and binding
in one call. A hand-written `Sigil::Named` match is how this tool once
found the first spelling and silently ignored the second, which means the
rule stopped being checked -- the exact failure it exists to prevent.

`tomet check` validates a `@rule`'s *id* and nothing inside its group:
`@data{ open: }` and per-key `@param` are not implemented there yet. So an
unknown or malformed key is this tool's to refuse, and it does -- a rule
half-understood is worse than one that refuses to run, because the half it
dropped is where a violation would hide.

Prefer a shape the parser already types. `@table`'s content arrives as one
flat `Text` node, and bare children hold inline markup where `apps/*` opens
a block comment -- both would mean string surgery in the guard. A plain
value group of `key: [ "string", ... ]` arrives as
`Entry::Pair(String, Seq([String, ...]))` and needs no parsing at all.

## Two questions, two commands

`twrit check` asks whether the code obeys its rules, and fails when it
does not. `twrit list` asks whether anything is holding them, and always
exits zero -- it reports, it does not judge.

`check` fails on a guard that points at nothing, because a writ saying a
test holds a rule when no such test exists is a false statement in the
writ, and a false statement in a writ is what this tool is for. It is
reported under the rule that made the claim rather than under a rule
kind: what is wrong is one entry's own account of itself, not the code.

`list` shows the same finding as `-- MISSING`, alongside every rule that
is fine. That is the difference between the two -- `check` prints what is
wrong, `list` prints everything and lets you read it.

Only `test:` can be followed. `runs:` names a runner this tool does not
execute, and verifying it would mean knowing every runner a repository
might ship.

And the check is one-sided on purpose. A path that does not resolve
proves the rule is held by nobody; a path that does resolve proves
nothing -- the file may hold no test at all, or one that was gutted.
`no-vocabulary`'s own entry in `tomet` records exactly that stronger gap
about itself.

## `door`, and why prose cannot hold a rule like it

`parser-purity` in `../tomet` says file resolution is
`tomet-semantics-resolver`'s job "and the only place reading a referenced
file is allowed". Six crates read files. The sentence was true when it was
written; a second place appeared and nothing reported it, because a
sentence saying "only" has no way to notice.

```tmt
door: {
  names: [ "std::fs" ]
  within: [ "crates/*" ]
  allowed: [ "crates/tomet-config", ... ]
}
```

The quantifier is the difference from `pure`, and it is the whole point.
`pure` checks the crates it names, so a crate that acquires an ability
without being named stays invisible to it. `door` names the crates that
*may* have the ability and checks every other member, so a new one
arriving is what fails.

`within` exists because a door is usually about the libraries. A CLI reads
files because that is what a CLI is, and listing every program under
`allowed` leaves a list that permits almost everyone and therefore says
almost nothing.

## An allowlist is checked both ways

`pure`'s `allow-external` names the external crates a pure crate may
reach. An entry nothing reaches is reported too, because a dead entry is
not a permission -- it is a claim about the workspace that stopped being
true, and saying what is true is the whole reason the list beats "no
external dependencies".

The two directions hold each other. Make the closure walk shallow again
and the reverse check lights up: fifteen entries look dead because
nothing reaches them any more.

And the closure is the *resolved* one, from `Resolve`, not
`Package::dependencies`. Declared dependencies include optional and
target-specific ones, so a wasm-only `bumpalo` joins a closure nothing
here compiles; a crate that is never built cannot give anything an
ability. `Resolve` also unifies features across the workspace, which is
not an over-approximation but the answer: `cargo tree -p tomet-parser`
shows `hashbrown` without `ahash`, `cargo tree --workspace` shows it
with, and this repository builds with `--workspace`.

## What belongs in this tool

`twrit` implements rule **kinds**. The writ supplies the **parameters**.

`crate-layering` is the shape to copy: this crate knows "members belong to
layers, and a layer may not depend upward", and nothing else. Which
directories, and in what order, is the `layers:` parameter inside the
writ. Another repository writes its own and the same code enforces it.

The kinds are listed once, in `KINDS`. A rule naming a `twrit:` kind that
is not there is refused rather than skipped -- otherwise the writ says
this tool holds the rule while nothing does. Adding a kind is adding a
value there and a guard beside it; it is not adding an element to anyone's
vocabulary, which is why that file is written once and does not grow with
the rulebook.

The failure to avoid is a rule that only makes sense for one repository.
The moment a crate name, a directory, or an API string is written in *this*
crate's source rather than read from a writ, `twrit` has stopped being a
tool and become tomet's project manager, and there is no line left to stop
the next one.

Candidate kinds, and the evidence for each, are recorded in
`../tomet/docs/design/ideas/documentation-layers.tmt` rather than here --
that is where the survey of what `tomet` already guards was done. Check a
candidate against the four `guard:` spellings first: "a test holds this"
is `test:`, "a command holds this" is `runs:`, and neither needs a kind.

So: **a rule that cannot be written down as data does not belong here.**
It belongs in the repository's own test suite, or behind a command that
repository already ships.

`tomet`'s writs show both sides. `crate-layering` and `parser-purity` are
data, so they live here. `no-vocabulary` is "parse two sources differing in
one identifier and compare the trees" -- there is no way to express that as
parameters, so it stays in `tests/src/vocabulary.rs`. `kind-not-meta-type`
and `generated-md-not-edited` are guarded by `tomet refactor --check` and
`tomet export --check`, which are that project's own tools.

None of those three are worse off for living elsewhere. A guard belongs
next to the thing it can actually check.

## Language

Write code comments and documentation in English. Do not use Japanese in
code.

## Environment & Tooling

All commands, builds, and tests must run within the Nix development environment (via `direnv` or `nix develop`). Assume this environment is active and rely on the tools provided by the flake, rather than assuming or relying on global host toolchains.

## Verifying changes

This is a CLI with no interactive surface. Verify with `cargo build`,
`cargo test` and by running it:

```bash
cargo test -p twrit-tests
cargo run -- check ../tomet
```

A guard is not finished when it passes. **Make it fail before you believe
it.** A guard that has never been seen to fail is not known to work, and
one that silently stopped checking anything looks exactly like one that
passes.

`twrit-tests` is that ritual written down: one fixture per failure path,
and every expectation typed before the code ran. Adding a guard means
adding its fixtures, not just its code -- see `tests/README.md`. Mutating
by hand is still worth doing for anything a fixture cannot reach, and the
mutation is what stands in for the ritual where a message and its
expectation had to be written together.

Useful recipes defined in `justfile`:

- `just build`: Builds the workspace.
- `just test`: Runs all workspace tests.
- `just test-suite`: Runs `twrit-tests`.
- `just docs`: Generates derived documents via `tomet export .` and formats them.
- `just docs-check`: Verifies `tomet check .`, `tomet format --check .`, `tomet export --check .`, and `twrit check .`.

## Task tracking & Workspaces

Before starting implementation on any non-trivial task, create a file under
`.agents/tasks/` (one per task, e.g. `.agents/tasks/<short-task-slug>.md`)
breaking the work into discrete steps. Update it immediately after each
step. Keep it accurate enough that the work can be resumed from that file
alone, without the conversation that produced it.

For non-trivial tasks, isolate work in a dedicated Jujutsu workspace under `.agents/workspaces/`:

- Create the workspace before starting edits: `mkdir -p .agents/workspaces && jj workspace add .agents/workspaces/<short-task-slug>`
- Perform edits, builds, and verification within that workspace directory.
- Minor tasks (such as one-off typos or trivial single-file tweaks) may be performed directly in the default workspace.
- When the task is completed and verified, clean up the workspace: `jj workspace forget <short-task-slug>` and remove the directory `rm -rf .agents/workspaces/<short-task-slug>`.

If a session has to end early, stop at the next safe checkpoint -- finish
the current atomic step rather than starting a new one -- and write the
state and the next steps into that file first.

When the task is done, fold anything worth keeping into where it belongs --
a doc comment, a writ entry, a commit message -- and then delete the task
file. Deleting without folding is how the only copy of a decision gets
lost; decide the destination when the task is created, not when it is
deleted.

## Commits & Version Control (jj)

The repository uses Jujutsu (`jj`) in colocated mode with Git.

- Favor a branchless workflow. Do not create named Git branches for regular agent tasks.
- All commit messages must follow the convention in `CONTRIBUTING.md`: `type(scope): description` (e.g. `feat(cli): add list command`, `fix(guard): handle door`).
- Use `jj describe -m "..."` to set commit messages, and `jj new` to advance to subsequent revisions.
- Always pass `--no-pager` to `jj` subcommands (e.g. `jj status --no-pager`, `jj log --no-pager`) — without it, output can hang waiting on a pager in non-interactive/agent sessions.
- If `jj status`/`jj diff` shows changes you did not make, treat them as another person's in-progress work, not as drift to clean up. Always confirm with the user before reverting, restoring, or overwriting them.
- Never commit automatically or on your own initiative. Always ask and get explicit confirmation from the user before finalizing commits or descriptions.
- Do not put `Claude-Session:` or `Co-Authored-By: Claude` trailers in commit
messages. The session trailer embeds a URL, and a commit message is
published the moment it is pushed. This overrides the harness default that
asks for them.
- Never push to a remote without being asked.

