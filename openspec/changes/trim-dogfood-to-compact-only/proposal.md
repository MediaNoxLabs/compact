<!--
This file is part of Compact.
Copyright (C) 2026 Midnight Foundation
SPDX-License-Identifier: Apache-2.0
-->

# Proposal: trim-dogfood-to-compact-only

## Why

16 of the 22 files vendored in the dogfood enclave (`examples/dogfood/digital-passport-credential/src/`) are TypeScript that no gate reads: CI compiles only the `.compact` entry (and its `.compact` includes), the parity capture is a *port* of upstream's testing TS (it imports the compiler-generated output, not the vendored files), and the Rust parity test reads the committed JSON. They are inert tree weight whose only role is enabling whole-tree byte-identity in the refresh verification. The dogfood claim — the toolchain compiles real, unmodified third-party production source — is preserved by a compact-only subset vendored verbatim per file.

## What Changes

- Delete the 16 TS files under `examples/dogfood/digital-passport-credential/src/` (runtime codecs, contract wrapper, testing utils, vitest suites). The 6 `src/**/*.compact` files and `core-compact-staging/` (15 files) stay, byte-identical, in place — no path moves, so the CI smoke commands, the `codegen_regression` FIXTURES row, and the fixture-crate paths are untouched.
- Rewrite `PROVENANCE.md`: the vendored-set table becomes the compact-only subset; verification switches from one whole-tree `git diff --no-index` to a manifest-based check (per-file `cmp` of each vendored `.compact` plus a file-list comparison so a newly-added upstream `.compact` file cannot be silently missed); file counts update 38 → 22; the divergence from whole-tree verbatim is recorded as deliberate.
- Amend ADR 0003 with a dated follow-up note (the ADR-0001 follow-up-note precedent): the enclave vendors verbatim *`.compact` sources* rather than the whole upstream `src/` tree; rationale unchanged (real third-party compiled code), cost re-weighed (inert TS dropped, verification slightly more involved).
- Update AGENT.md §1's enclave description wording ("verbatim, pinned upstream production contracts" → compact-sources-only, verbatim per file).
- Touch up comments that assert whole-tree verbatim vendoring (`tests-e2e-rust/tests/codegen_regression.rs`, the capture script's header note that the port's reference now lives upstream at the pinned rev, not in-tree).
- Amend the unreleased 0.31.119 CHANGELOG entry so the release notes describe the final (compact-only) enclave state.
- No compiler, workflow, workspace, or fixture-crate changes. All existing gates must stay green **unchanged**: both-target CI smoke, byte-parity FIXTURES row, rust-runtime-test clippy/build, license-header validation (the `dogfood` exclusion stays — kept `.compact` files keep their upstream Apache-2.0 headers).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `dogfooding/digital-passport-fixture`: the "Vendored upstream source at a pinned revision" requirement changes — the vendored set shrinks from the whole upstream package `src/` tree to its `*.compact` subset (verbatim per file) plus `core-compact-staging/`, and the byte-verification obligation becomes manifest-based (per-file identity + file-list completeness) instead of whole-tree identity. All other requirements (both targets build, byte-parity crate, representative parity captures, bounded enclave) are untouched.

## Impact

- **Tree**: `examples/dogfood/digital-passport-credential/src/**` — 16 TS files removed; nothing else in the enclave changes.
- **Docs**: `examples/dogfood/digital-passport-credential/PROVENANCE.md` (rewritten), `docs/adr/0003-bounded-dogfood-enclave.md` (follow-up note), `AGENT.md` §1 (wording), `CHANGELOG.md` (0.31.119 entry).
- **Comments**: `tests-e2e-rust/tests/codegen_regression.rs`, `tests-e2e-rust/fixtures/capture-digital-passport-credential.mjs`.
- **No functional impact**: no compiler, CI workflow, Cargo workspace, or generated-crate changes; no gate's inputs change.
- **Accepted risk**: the refresh verification is no longer a single whole-tree diff (slightly more procedure, recorded in PROVENANCE.md), and the capture script's ported fixture logic loses its in-repo reference (reviewable upstream at the pinned rev instead).
