---
id: RUST-ADR-0325
alias: ADR-0325
source_sha256: 8db6881d7d32621c3acc731dff332c2f8c58c03138134da364d21428d87c13e6
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0325 — Stabilize a bounded Rust CI lane at the midpoint

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** proposed implementation · 2026-10-07 · R03019/#363 · milestone0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0325 — Stabilize a bounded Rust CI lane at the midpoint

Status: proposed implementation · 2026-10-07 · R03019/#363 · milestone0.3.0

### Problem
The tenth original parent is now locally accepted. Current Compiler Build performs Nix/Scheme, broad Rust/workspace checks, Counter proof/wallet handoff and packaging in a long job. Recent green remote runs are at03c03a39, and remote branch HEAD939b7aaa differs from local5efa91c2. Those runs do not qualify current changes. Existing CI has no explicit current1.88 standalone lane or adoption-specific strict proof gate invocation.

### Decision
Begin with a dedicated bounded Rust checks lane, independent of the expensive full compiler job: existing four-owner tests/Clippy/formatting and actual1.88 backend+representative generated consumer compilation, with source/lock/toolchain/features recorded. Preserve full compiler/proof/package gates for candidate qualification. Final adoption-specific proof receipts must bind exact candidate sources; Counter smoke alone cannot imply all DID/Jubjub proof support.

### Before / after
```text
Before: long Compiler Build owns both quick Rust feedback and full compiler work
After:  bounded Rust checks -> fast type/test feedback
        full compiler/adoption/proof/release lanes -> separate qualified evidence
```
Use pinned existing Actions, locked Cargo, no secret-dependent routine Rust unit tests. Cache keys distinguish OS/architecture/toolchain/lock and features/profile. Preserve ordinary worker debug assertions; do not globally increase thread stack to hide recursion regressions. Workflow dispatch/current branch publication and actual remote results must be recorded, not inferred from local success.

### Current inventory and next work
Reviewed .github/workflows/build-compiler.yml, compact-test.yml and Rust gate scripts. build-compiler already runs Python harness tests, real ledger fixture acquisition, cargo workspace tests and all-feature Clippy; preserve these while splitting feedback. Installer compact-test is a separate3host compact-tool gate, not backendMSRV qualification. No remote rerun or publication has occurred in this checkpoint. Prepare/review the bounded workflow and validate its exact commands before dispatch. Existing local volume is below2GiB, so build-heavy local verification waits for owner-provided space; no cache/evidence deletion authorized here. FinalRC full remote gating remains required.

Delivery issue: https://github.com/MediaNoxLabs/compact/issues/450


### Delivery update — 2026-10-07
Implementedfba6845f; static gates pass, actual local core command validation in progress. Issue#450 remains open pending execution review; no remote evidence.


### 2026-10-07 — External tutorials validated; input errors and bounded CI delivered

Local disk capacity recovered to110GiB and build-heavy work resumed. Parent progress remains **10/19 required outcomes accepted**, with ACC removed from the original20.

ADR0324/#449: isolated witnessed ContractLab tutorial passes1test; unchanged DID tutorial passes2lifecycle tests. Both pass strictClippy/formatting. All477 staged SDK files match immutable5efa91c2;324 registry identities/checksums per example match reviewed source lock; each metadata graph has one runtime identity and no outside local package. Generated contracts remain unedited. Initial fresh-lock drift was rejected before compilation; corrected lock-seeding recipe and failure evidence retained. These are native/replay and finite DID TS-capture checks, not new proofs/network acceptance. Current-edition guides reflect actual results; parent#352 still needs remaining negative-control mapping, companion corrections and final publication.

ADR0326/#451 delivered in signed GPG/DCO9012a86abfa329b4b4c02896ab1e69ce52fba5f1. Two new injected Read failure cases pass in all three compiler binaries (12 focused tests including existing cases). Original error kind, display and typed payload survive before input/after a prefix; partial bytes do not become success. Public WitnessScript doctest passes and documents owned clone independence and mismatch preservation. Relevant strictClippy/formatting pass; production implementation unchanged. This fills the finite compiler-join I/O gap. Historical95.04% coverage remains its measured source cohort, not a new measurement.

ADR0325/#450 implementation committed as signed GPG/DCOfba6845f37fdbfbfbd49e3a73d896ecff7a70371. New bounded Rust workflow separates four-owner tests/lints/format from Rust1.88 backend/Jubjub/standalone checks; locked graphs, scoped caches and source/check receipts. actionlint1.7.12, ShellCheck, YAML, eight shell-script parses and ten receipt simulations pass. Actual core commands are undergoing local validation; no remote run or current remote green claim. Existing full compiler/proof gates preserved.

ADRs and historical source snapshots remain separate. StoppedADR0285 unchanged. User-owned doc/ledger-adt.mdx preserved. Saved goal stillusageLimited; active user-authorized execution continues.

Evidence archives:
- [ADR0324 — Validated external developer tutorials.zip](references-0.3.0.md#note-107) — SHA256 `3919b99d283f9417eee4267daa90c7c246bba9edc95e446eb446974fa1934203`
- [ADR0326 — Input failure and witness rustdoc checks.zip](references-0.3.0.md#note-110) — SHA256 `00516d3610e63c722e9f57dcebfbee78c496fa1411344f7432c86dc6bfc587aa`
- [ADR0325 — Bounded CI static validation.zip](references-0.3.0.md#note-108) — SHA256 `266d404226c5e117c3039efd0b457fc780d2ea799a190acbf1cc9e893d942eba`


### Branch-local first-run correction
Midpoint source checkpoint fba6845f was pushed successfully after all 67 pending commits passed signature/DCO/conventional-title audit. No user document changes were included.

Remote dispatch returned HTTP 404 because the new workflow is absent from the repository default branch. Add a narrowly scoped push trigger for codex/rust-backend-ast with the same Rust path filters, so branch-local workflow registration and first execution work without changing the default branch or creating a large unrelated PR. Keep workflow_dispatch and pull_request triggers. Validate with the existing actionlint gate, then push the trigger fix. This is CI stabilization, not final release qualification.


### 2026-10-07 — Bounded Linux CI accepted (ADR0325/#450)

[Run37601602223](https://github.com/MediaNoxLabs/compact/actions/runs/37601602223) succeeds at exact signed bf7c719beea14f7532feaf0d57577245f42c98ca. Linuxx86_64 Rust1.99 core passes679tests, zero ignored/filtered/failures, strict all-target/all-feature Clippy and formatting (job1m58s). Actual Rust1.88 backend/Jubjub workspace and isolated standalone checks pass (job1m21s). Both source/commit/tree/workflow/lock identities and12 artifact checksums verified. No reruns required. This run qualifies the bounded lane, not full compiler regeneration/adoption proofs/package/network/release.

Midpoint CI stabilization child#450 closes. Parent#363 final-candidate requirements remain open; accepted original parents unchanged10/19. Archive [ADR0325 — First bounded Linux CI bf7c719b.zip](references-0.3.0.md#note-109) SHA256 `572b9ef72e8365b55d9d4b0ffa24d6fa064517ff017b48770395806a3e51afb0`. Remote checkpoint includes all67 audited prior commits plus signed conventional trigger correction. Upcoming ubuntu-latest image migration is a runner notice, not failure.
