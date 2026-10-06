---
id: RUST-ADR-0227
alias: ADR-0227
title: "Isolate remaining no-install CLI baselines"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["installer", "isolation", "baseline"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 003bdf61ce16390d5cb37bde51ba6e291ad5a7010f9354a9cdac8773a59538b1
---
# RUST-ADR-0227 — Isolate remaining no-install CLI baselines

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. Remaining compile/format/fixup/clean/self no-install tests use scoped private home/config/receipt while preserving explicit scenario continuity and the product endpoints. Synthetic cache baselines are not genuine archive installation evidence.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#332 closure](https://github.com/MediaNoxLabs/compact/issues/332#issuecomment-6017793019). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`b1ffd086`](https://github.com/MediaNoxLabs/compact/commit/b1ffd08621b913ed6a3d334c39c3d5820a9e2775). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: accepted for implementation, 2026-10-06. Depends on ADR226, integrated as b1ffd08621b913ed6a3d334c39c3d5820a9e2775.

### Problem
The compile, format, fixup, clean and self no-install tests inherit installed compiler and updater receipt state. Their exact error assertions can therefore depend on the developer machine. ADR226 binds the default executable to Cargo and provides synthetic release cache isolation; this slice extends that test-only boundary.

### Before and after
Before: `run_command(args, None, ...)` can inspect the user's compiler directories or updater receipt.
After: operational no-install cases hold a `ReadOnlyBaseline`, pass its per-child fresh environment, assert exact existing output/exit behavior and unchanged private state. Explicit `--directory` cases retain their arguments. Pure clap help/version/argument parsing remains read-only; help snapshots retain their existing parent-HOME presentation where necessary.

Private HOME, platform cache, XDG paths, COMPACT_DIRECTORY, RECEIPT_HOME and AXOUPDATER_CONFIG_PATH protect no-receipt self checks. If callers explicitly provide AXOUPDATER_CONFIG_PATH but no AXOUPDATER_CONFIG_WORKING_DIR, the common command helper removes an inherited working-directory override. An explicit caller working-directory override and explicit downloaded binary path remain authoritative.

### Scope and ownership
Five integration files contain 47 declared tests: compile 5, format 7, fixup 18, clean 8, self 9. Common test helper only; no production code, dependencies, compiler/runtime/emitter APIs, ABI/schema, output snapshots, or live update/download/scenario changes. ADR228 owns a separate archive fixture.

### Alternatives and limits
Do not update mutable latest-release expectations, skip installer scenarios, seed installed compilers, or introduce production endpoint overrides. Synthetic cache metadata proves deterministic no-install behavior, not published archive integration. Existing download and successful self-update scenarios remain separate residual acceptance work.

### Validation plan
Use the existing nondefault target/adr157, Rust1.99 and CARGO_INCREMENTAL=0. Run all 47 tests plus ADR226 controls, strict package Clippy and scoped formatting/diff checks. Run no-receipt self cases from a private polluted parent cwd/HOME/config with sentinel receipts; assert exact no-receipt error, no proxy connections, and unchanged sentinels. Preserve evidence and obtain independent review before GPG+DCO delivery.

### ADR227 signed delivery

Commit `622139bdc22a9acba0eb5a10a8b97653722d7287` (GPG verified, DCO) on `codex/adr227-no-install-baselines`, base `b1ffd086`. Seven test-only files; no production, dependency or expected-output fixture changes.

- All 47 declared cases pass: compile 5, format 7, fixup 18, clean 8, self 9. Plus 24 ADR226 controls: 71 total.
- 37 of these 47 cases now use private state. Ten clap help/usage cases retain their original read-only parent-HOME presentation because setting COMPACT_DIRECTORY changes the help environment display; they exit during argument parsing before installation or receipt operations.
- Explicit --directory arguments and exact expected stdout/stderr/exit assertions remain. Both default and explicit directories are checked for no installed state. Private receipt/config/data remain empty, cache bytes unchanged and refusal proxy counts zero connections.
- Actual polluted-parent probe runs both self check/update cases from a private cwd with inherited working-dir override and private HOME/XDG/config/receipt sentinels. Both return exact NoReceipt; four sentinels remain byte-identical. Direct CLI positive control retaining the override produces the different installation-information error, proving the flag would otherwise select the cwd receipt.
- Strict package all-target/all-feature Clippy, package fmt and committed diff checks pass. Independent read-only review found no remaining actionable issue.

Executable: `${COMPACT_SOURCE}/target/adr157/debug/compact`, SHA256 `33e3da1a4699c65fe9a8cc08d816123252db807bead92bdfca5d1bc2d57062e1`. Rust `rustc 1.99.0 (b940084d7 2026-09-28)`; existing target/adr157, CARGO_INCREMENTAL=0.

Exact signed-head receipt `${LOCAL_EVIDENCE}/compact-adr227-delivery-receipt.json`; logs `${LOCAL_EVIDENCE}/compact-adr227-focused-signed.log`, `${LOCAL_EVIDENCE}/compact-adr227-polluted-signed.log`, `${LOCAL_EVIDENCE}/compact-adr227-clippy-signed.log`. Probe source and sentinel receipt are retained in `${LOCAL_EVIDENCE}/compact-adr227-polluted.py` and `${LOCAL_EVIDENCE}/compact-adr227-polluted-receipt.json`.

Remaining boundary: actual release archive installation/update and successful self-update scenarios remain selected but outside this slice. No publication, remote CI or broad proof claim. Local proxy evidence is scoped to these private operational cases.
