<!--
This file is part of Compact.
Copyright (C) 2026 Midnight Foundation
SPDX-License-Identifier: Apache-2.0
-->

# Tasks: trim-dogfood-to-compact-only

## 1. Trim the enclave tree

- [ ] 1.1 `git rm` the 16 TS files under `examples/dogfood/digital-passport-credential/src/` (list in `proposal.md` Impact; leaves `src/` with exactly the 6 `.compact` files and their directories). Verify: `find examples/dogfood -name '*.ts' | wc -l` → 0 and `git ls-files examples/dogfood | wc -l` → 22.
- [ ] 1.2 Confirm no empties or strays: `git status --porcelain examples/dogfood` clean after the rm; `src/test/`, `src/internal/`, `src/testing/` gone; the entry `src/digital-passport-credential.compact` and `core-compact-staging/` (15 files) untouched (`git diff --stat` shows deletions only).

## 2. Rewrite PROVENANCE.md (design D2)

- [ ] 2.1 Update "What is vendored": `src/` row becomes the 6-file `*.compact` subset (entry + 5 modules), staging row unchanged; record the TS exclusion as a deliberate divergence from whole-tree verbatim (cross-link ADR 0003 follow-up note).
- [ ] 2.2 Rewrite "Refresh procedure": `rsync --include='*/' --include='*.compact' --exclude='*' --delete` for `src/`; add the new "Verification" checks — per-file `cmp` of the 6 files, file-list completeness diff (`find … -name '*.compact' | sort` on both sides), unchanged staging `cmp`s and `.gitignore`-collision check, and `git ls-files examples/dogfood | wc -l` → 22.
- [ ] 2.3 Update the neutralization note to state the subset policy (verbatim per `.compact` file; TS never vendored) and that the capture script's ported fixture logic references upstream originals at the pinned rev (not in-tree).
- [ ] 2.4 Verify: every claim in the file is true of the trimmed tree (counts, file lists, commands runnable as written against a fresh upstream checkout).

## 3. Amend decision record and docs (design D3/D4)

- [ ] 3.1 Add dated follow-up note to ADR 0003: vendored set redefined to the `.compact` subset (decision-table cell + accepted-risk bullet re-weighed); rationale unchanged (real third-party compiled code), inert TS dropped, verification manifest-based. Verify: note is dated, references this change, and leaves the original decision text intact above it.
- [ ] 3.2 Update AGENT.md §1 enclave wording: "verbatim, pinned upstream production contracts" → compact-sources-only vendoring, verbatim per file (keep the de-branding-supersession framing and PROVENANCE/ADR links).
- [ ] 3.3 Amend the unreleased 0.31.119 CHANGELOG entry: enclave description becomes the compact-only subset (6 + 15 files, per-file verbatim, manifest verification); no new version header.
- [ ] 3.4 Touch up whole-tree-verbatim comments: `tests-e2e-rust/tests/codegen_regression.rs` FIXTURES-row comment and `tests-e2e-rust/fixtures/capture-digital-passport-credential.mjs` header note (port reference now lives upstream at the pinned rev). No logic changes. Verify: `grep -rn "verbatim" tests-e2e-rust/ examples/dogfood/` shows no comment still asserting whole-tree/TS vendoring.

## 4. Gates and change closure

- [ ] 4.1 Byte-parity gate unchanged-green: `cargo test -p tests-e2e-rust rust_codegen_byte_parity` passes with the trimmed source tree (regeneration over the dogfood entry is byte-identical to the committed crate — the trim must not change compiler input).
- [ ] 4.2 License-header validation passes: `python add_headers.py --validate` (the `dogfood` exclusion covers the kept upstream headers; no new un-headered file introduced).
- [ ] 4.3 `openspec validate trim-dogfood-to-compact-only` passes; archive the change (syncs the `dogfooding/digital-passport-fixture` main-spec requirement per this change's delta).
