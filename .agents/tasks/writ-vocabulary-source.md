# Which writ vocabulary is the source

> Cross-repository coordination: `bearings/initiatives/writ-vocabulary-source.tmt` (the table and the order live there; the work stays here).

`twrit` keeps its own `writ` vocabulary (`.tomet/vocabularies/writ.vocabulary.tmt` on `main`; moved to
`tmtroot/schema/vocabularies/` in the default workspace): a long Japanese `about:` that explains why a rule is an entry, what
`guard:` takes (`twrit`, `runs`, `test`, `none`), and why `none:` needs a reason.

The `vocabularies` repository has `writ@1.0.0` (English, one short paragraph) and sisho and sisho-desk take that one through
catasto (`catasto.ctmt`). So there are two texts for one vocabulary and nothing says which one wins.

- [ ] Decide: twrit is the source and `vocabularies/writ` is made from twrit's text (translate or keep Japanese), or `vocabularies`
  is the source and twrit takes it with catasto too. Twrit implements the rules, which argues for the first; the one-text-for-
  everyone rule of `vocabularies` argues for the second.
- [ ] If twrit takes it from `vocabularies`: add `catasto.ctmt` + `catasto.lock`, a `catasto` flake input with its `sisho` input cut
  (as sisho-desk does), `catasto check .` in `docs-check`, and move the fixtures copy
  (`tests/fixtures/.tomet/vocabularies/writ.vocabulary.tmt`) too, or leave it: fixtures are frozen.
- [ ] clippy prints 21 lines on `main` (not checked for what they are). Look.

Not mine, in the default workspace: the layout change `.tomet/` -> `tmtroot/schema/` (blueprints and vocabularies), `default.config.tmt`,
`nix/dev.nix`, `.agents/tasks/follow-tomet-main.md`.

## The `[patch]` section of `tmtroot/agents.tmt` (AGENTS.md is generated from it)

Found on 2026-10-24; `[patch]` is not used in any repository any more (it dropped `source` from `Cargo.lock` and broke the Nix build of
sisho-desk), and twrit has no `.cargo/config.toml` now, yet "The two repositories" still describes it.

- [ ] Rewrite the section: a change in tomet reaches twrit by being pushed there and taken with `cargo update -p <crate>`; no `[patch]`.
  Drop "which is why eight of them carry a `[[patch.unused]]` block in their lockfile" if the fixture lockfiles no longer have one
  (check `tests/fixtures/**/Cargo.lock`), and clean the fixtures if they do.
- [ ] Same section: "on a private Forgejo over SSH" and `net.git-fetch-with-cli` are stale (tomet is on GitHub over https);
  `../tomet/docs/design/ideas/documentation-layers.tmt` is not at that path (it is under `tmtroot/deprecated/`).
- [ ] The section says both dev-shell tools come from `sisho`'s flake and that the loop is cut with `inputs.sisho.follows = ""`;
  true, and now `catasto` joins them through sisho the same way.

## Acceptance (how this will be judged when it is done)

- `tmtroot/agents.tmt` says nothing about `[patch]`, `[[patch.unused]]`, Forgejo, SSH or `net.git-fetch-with-cli`; every path and command in
  "The two repositories" exists or runs; `AGENTS.md` is regenerated from it (`just docs`), not edited.
- If any fixture `Cargo.lock` still carries `[[patch.unused]]`, it is gone and `twrit-tests` still pass.
- The vocabulary question is answered in one place a reader finds (this file's successor, a writ entry, or the vocabularies repository's
  readme), with the reason, and the two texts are no longer able to drift apart silently (one is generated from, or taken from, the other).
- `just docs-check` and `cargo test --workspace --locked` pass inside `nix develop`; clippy's 21 lines are either fixed or each named.
- Each remaining leftover found on the way is written into `.agents/tasks/`, not only reported.
- Commits are separate for separate concerns (docs rewrite, vocabulary move, lint); no `Co-Authored-By` trailers; nothing pushed
  that was not asked for.

## Update 2026-10-24

`main` (`571665d5`, twrit 0.4.0) replaced both copies it keeps, `.tomet/vocabularies/writ.vocabulary.tmt` and
`tests/fixtures/.tomet/vocabularies/writ.vocabulary.tmt`, with the text of `tomet/tmtroot/schema/vocabularies/writ.vocabulary.tmt` (the long
Japanese `about:`, in the `::elements` map form), because the old `@element(rule)` form is no longer read and twrit then saw 0 rules (see
`error-on-unresolved-rules.md`). That text is not the short English `writ@1.0.0` of the `vocabularies` repository. The question above is
still open: now there are three places holding a `writ` vocabulary (tomet, twrit, vocabularies), two of them the same text.
