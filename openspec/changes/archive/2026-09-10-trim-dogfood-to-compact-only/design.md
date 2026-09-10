<!--
This file is part of Compact.
Copyright (C) 2026 Midnight Foundation
SPDX-License-Identifier: Apache-2.0
-->

# Design: trim-dogfood-to-compact-only

See `proposal.md` for motivation; see the delta spec for the normative
requirements. This covers the how.

## Context

The enclave (`examples/dogfood/digital-passport-credential/`) currently
vendors upstream's entire package `src/` tree (22 files: 6 `.compact`, 16 TS)
plus `core-compact-staging/` (15 `.compact`), whole-tree-verbatim per
`PROVENANCE.md`. Only the `.compact` files are compiler input; the TS is inert
(verified: no CI step, script, or test reads it — the capture script imports
the *compiler-generated* TS output from `/tmp/dpp-ts-driver/`, and its fixture
logic is a port of upstream `src/testing/*.ts` written inside the script).
Everything downstream keys on paths that must not move:
`src/digital-passport-credential.compact` (CI smoke, FIXTURES row source path)
and the relative include `../core-compact-staging/credentials` from `src/`.

## Goals / Non-Goals

**Goals**

- Remove the inert TS with zero change to any gate's inputs: same entry path,
  same staged core, same generated crate, same workflows.
- Keep the per-file verbatim guarantee (each vendored `.compact` byte-identical
  to upstream at the pinned rev) and make subset *completeness* checkable, so
  refresh cannot silently drop a newly added upstream `.compact` module.
- Record the divergence from whole-tree verbatim where the current documents
  demand it: `PROVENANCE.md` (neutralization/divergence note) and ADR 0003
  (follow-up note, the ADR-0001 precedent).

**Non-Goals**

- No re-pinning / refresh of the vendored bytes (still rev `cdeb860b`, still
  core `0.1.0-rc3`).
- No change to the fixture crate, `codegen_regression` FIXTURES table,
  workflows, `header_config.json` (the `dogfood` exclusion stays — kept
  `.compact` files keep upstream headers), or the capture/parity tests.
- No neutralization/rebadging pass (PROVENANCE's "neutralization note" stays
  a future option).

## Decisions

### D1: Delete TS in place; keep directory layout exactly

`git rm` the 16 `src/**.ts` files; keep `src/` and the module subdirectory
names so the entry path and intra-`src/` module includes resolve exactly as
today. Alternative considered — flattening `src/` to just the 6 files with
rewritten includes — rejected: it modifies vendored bytes, breaking per-file
verbatim, and would churn the FIXTURES row and CI paths for no benefit.
(Git drops the emptied `src/test/`, `src/internal/`, `src/testing/`
directories automatically.)

### D2: Manifest-based verification replaces the whole-tree diff

`PROVENANCE.md`'s verification section becomes three checks:

1. **Per-file identity** — `cmp` each of the 6 `src/**/*.compact` files
   against its counterpart in a checkout of the pinned rev (replacing the
   single `git diff --no-index` of whole `src/` trees).
2. **Completeness** — diff the *file lists*:
   `(cd upstream…/src && find . -name '*.compact' | sort)` vs the same over
   the vendored `src/`. A new upstream module appears as a list diff instead
   of silent absence — this is the one property the whole-tree diff gave for
   free that a subset must re-establish explicitly.
3. **Unchanged staging checks** — the 15 `core-compact-staging/` `cmp`s,
   the `.gitignore`-collision `git status --ignored` check, and the file
   count (`git ls-files examples/dogfood | wc -l` → **22**: 6 + 15 +
   `PROVENANCE.md`).

The refresh `rsync` gains `--include='*/' --include='*.compact'
--exclude='*'` so it copies only Compact sources (and `--delete` prunes
removed modules). Alternative considered — keeping whole-tree vendoring but
`.gitignore`-ing TS — rejected: the files must not be present at all, and
ignored files still land in working trees and confuse verification.

### D3: ADR amended by follow-up note, not rewritten

ADR 0003 keeps its Status/Date and gains a dated follow-up note (the
documented ADR-0001 pattern): the enclave's vendored set is redefined to the
`.compact` subset; the decision table's "verbatim `.compact` + staged core"
cell and the accepted-risk bullet ("verbatim is what keeps the fixture
byte-comparable") are re-weighed in the note. Alternative — rewriting ADR
0003 in place — rejected: the ADR records a decision at a point in time; the
trim is a new decision that references it.

### D4: Amend the 0.31.119 CHANGELOG entry in place

The enclave landed on this branch in the same unreleased cycle (0.31.119,
bumped in `2f028a1`); the entry's "vendored **verbatim** … `src/` tree"
wording is updated to the subset description so release notes describe the
final state. Alternative — a new versioned entry documenting a trim of files
that never shipped — rejected: it would document tree churn invisible to any
consumer.

### D5: Comment touch-ups only where text asserts whole-tree verbatim

- `tests-e2e-rust/tests/codegen_regression.rs` (FIXTURES-row comment:
  "vendored verbatim" → compact subset, verbatim per file).
- `tests-e2e-rust/fixtures/capture-digital-passport-credential.mjs` header:
  the port's reference ("upstream `src/testing/*.ts` at the pinned rev (see
  PROVENANCE.md)") gains that those originals live upstream, not in-tree.
  No logic change.
- `tests-e2e-rust/Cargo.toml` and `digital_passport_credential.rs` comments
  say "vendored … (see its PROVENANCE.md)" — still true, untouched.

## Risks / Trade-offs

- [Refresh verification is now 3 steps instead of 1 command] → Procedure is
  fully scripted in `PROVENANCE.md` (copy-pasteable, same as today); the
  completeness check re-establishes the silent-drop protection.
- [Capture-script port loses its in-repo reference] → The pinned rev in
  `PROVENANCE.md` names the upstream files; reviewers diff the port against
  upstream at that rev.
- [Future upstream adds a `.ts`-only build dependency for `.compact`
  compilation] → None exists today (compiler input is `.compact` only —
  verified by the current CI compiling with TS present but unread); if one
  appears, the refresh procedure's file-list diff surfaces it as a
  compile failure at smoke time, and the policy can be revisited via a new
  ADR note.

## Migration Plan

Single PR/commit series, no ordering hazards: trim tree → rewrite
`PROVENANCE.md` → amend ADR/AGENT/CHANGELOG/comments → run gates
(`rust_codegen_byte_parity`, header validation, file count) → archive the
change (syncs the main spec). Rollback is `git revert` of the trim commit;
nothing downstream depends on the absence of the TS files.

## Open Questions

(none)
