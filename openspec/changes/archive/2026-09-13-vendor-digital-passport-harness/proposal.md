## Why

The Rust backend's correctness has been validated only against fixtures we author, so real upstream idioms escape the corpus. PR #70 discovered this the hard way: vendoring `midnight-verifiable-credential-digital-passport` exposed the ternary and mixed-width operand gaps, but it did so *after* the ternary code was written, so the upstream contract arrived as a dependent artifact and the two bugs were fixed by review whack-a-mole. The dogfood contract is the **oracle**, not a passenger: it should be vendored first, its current gaps recorded as executable expected-refusals, and its TypeScript reference behaviour captured while only the TS target is needed — so the fix changes flip known probes instead of discovering unknown shapes.

## What Changes

- Vendor the upstream contract's **Compact sources only** under `examples/dogfood/digital-passport-credential/`: the `*.compact` subset of the upstream package `src/` tree (6 files: the entry `digital-passport-credential.compact` and its five modules) at pinned rev `cdeb860b`, plus `core-compact-staging/` (15 `.compact` files) staged from `@midnight-ntwrk/credential-compact@0.1.0-rc3` `dist/`. **Upstream TypeScript material MUST NOT be vendored** (the 16 TS files — runtime codecs, contract wrapper, testing utils, vitest suites — are deliberately excluded; no gate compiles, executes, or reads them).
- `PROVENANCE.md` recording upstream URL, rev, license (Apache-2.0), npm core package+version, the no-pnpm staging procedure, and the explicit manual refresh policy.
- Add the `dogfood` `excluded_directories` entry to `header_config.json` so upstream Apache-2.0 headers stay verbatim and pass header validation.
- Author the committed TS reference captures (`tests-e2e-rust/fixtures/capture-digital-passport-credential.mjs` → JSON) for the civil-date helpers (every conditional-expression site and assert-fail path) plus one issuance/presentation/verification round-trip. The TS target already compiles, so the oracle is behaviour-complete before any Rust fix.
- Add the real contract's known Rust-codegen gap to `rejection_corpus` as **expected-refusal** probes (minimal extracts at each known ternary site and a mixed-width operand site), asserting the exact current diagnostics, plus a whole-entry expected-failure gate. These flip from REJECTION to ACCEPTION in the fix changes and the packaging change.

## Capabilities

### New Capabilities

- `dogfooding/digital-passport-source`: the repo vendors the upstream digital-passport contract's Compact sources at a pinned revision, records provenance and refresh, and pins its current Rust-codegen gap as an executable oracle.

### Modified Capabilities

(none)

## Impact

- New vendored tree `examples/dogfood/digital-passport-credential/` (`.compact` only; 22 files incl. `PROVENANCE.md`), `header_config.json`.
- `tests-e2e-rust/fixtures/` (capture script + JSON), `tests-e2e-rust/tests/rejection_corpus.rs` (oracle probes).
- No compiler change, no `compiler-version.ss` bump → request the maintainer-applied `skip-changelog` label.
- Depends on `compiler-backed-ci-gate` (the probes must run in CI to be enforced). Prerequisite for `type-directed-expression-coercion`, `fix-ternary-expression-codegen`, and `add-digital-passport-dogfood-fixture`.
