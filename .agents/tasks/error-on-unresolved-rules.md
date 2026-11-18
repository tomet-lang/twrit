# `twrit` must not say `ok` over a writ it could not read

## What happened (2026-10-24)

A vault whose `writ` vocabulary is not in a form the `tomet` that `twrit` is built with can read makes `@rule` resolve to nothing.
`twrit` then found **0 rules**, printed `1 writ, 0 rules,` and exited 0. In sisho, tomet and twrit itself every guard
(`crate-layering`, `process-door`, `tools-by-subprocess`, `rust-api-docs`) was off, and nothing failed. In sisho it also let a generated API page go stale unseen. Two causes met:

- the vocabulary files were still in the old `@element(rule){ ... }` form, which `tomet` stopped reading without a message
  (see `tomet/.agents/tasks/vocabulary-map-form-followups.md`);
- `twrit 0.3.0` depended on a `tomet` older than the one that reads the `::elements` map form (fixed on `main` in 0.4.0: `3725724c`, released as `571665d5`).

The second is fixed. The first stays possible: any vault with a stale vocabulary gets the same silence. `Writ::rules` says in a comment that
"a rule half-understood is worse than a rule that is not there", and this is the case it did not cover: a rule not understood at all.

## Do

- [ ] In `crates/twrit-core/src/writ.rs`, `Writ::rules`: besides the elements that classify as `writ.rule`, count the elements spelled
      `rule` or `writ.rule` that do **not**. If there is at least one, return an error that names the file, the spelling, and the
      likely cause ("`@rule` does not resolve to `writ.rule`: the `writ` vocabulary is missing from `vocabularies` in the config, or is
      in a form `tomet` does not read; write it with `::elements{ rule: { ... } }`"). Do not guess at `@rule`'s absence from a writ that has none.
- [ ] A writ with no `@rule` at all is still fine (`1 writ, 0 rules,` is a true report there). The failing case is only "spelled like a
      rule, not resolved".
- [ ] Also look at `vault.vocabulary_errors`: `discover` prints them as `warning:` lines; check that a vocabulary that fails to load shows up
      there, and not only in a test.
- [ ] A fixture for it in `tests/fixtures/` (frozen corpus; add, do not edit): a `.writ.tmt` with `@rule` and a `writ.vocabulary.tmt` in the
      old `@element(rule)` form, and a test in `tests/src/` that `twrit check` of it is an error (exit 2 with the message, not `ok`).
- [ ] Update the doc comment of `Writ::rules`, the writ spec under `tmtroot/spec/features/`, and `AGENTS.md`'s source (`tmtroot/agents.tmt`) if
      they describe what a writ without resolved rules does. Run `just docs`.
- [ ] A changelog line / version: this changes what an existing vault sees (a stale one now fails). Bump to 0.5.0 or not is the author's call;
      `sisho` holds a minimum version for twrit (`crates/sisho-tools/src/tool.rs`), currently 0.2.0, and could raise it to the version
      that has this (and, at least, to 0.4.0, which is the first that reads the map form).

## Done on 2026-10-24, in the default workspace

- The default workspace was put onto `main`: the old `.tomet/vocabularies/writ.vocabulary.tmt` is gone, the text of `main` is at
  `tmtroot/schema/vocabularies/writ.vocabulary.tmt`, and `.tomet/` is gone.
- `tmtroot/schema/vocabularies/rust.vocabulary.tmt` is in the `::elements` map form (it was the old `@element(crate)` form and failed
  `tomet check`).

Acceptance: a vault with a stale `writ` vocabulary makes `twrit check` fail loudly; the fixture test proves it; `just docs-check` passes.
