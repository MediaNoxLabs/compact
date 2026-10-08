---
id: RUST-ADR-0267
alias: ADR-0267
source_sha256: 03b20d48c90a59cada1a37316a1fd2aa593235d8a1196a4efcfb692d2fa4e651
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0267 — Test runtime-source selection and repeated helper execution boundaries

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted. Parents R030-07/#351 and R030-17/#361. Source baseline `0b297501`. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0267 — Test runtime-source selection and repeated helper execution boundaries

Status: accepted. Parents R030-07/#351 and R030-17/#361. Source baseline `0b297501`.

### Problem and measurement

Fresh LLVM profiles from 89 library-only tests and 298 full backend tests identify two useful test gaps. Runtime preflight rejects alternate dependency sources even when versions match, but those refusal arms were untested. Composition caches declaration admission; the selected cohort did not establish that two actual calls still execute distinct arguments, witnesses and public effects.

Full backend production-line coverage is 201/216 (93.06%) for compatibility, 74/76 (97.37%) for naming, 269/284 (94.72%) for pure-call policies and 286/295 (96.95%) for composition. These are four modules, not whole-backend or generated-runtime coverage. Detailed scope and missing lines are retained in the linked measurement receipt.

### Decision

Add test-only coverage for:

1. Expected-version dependency with a changed macro sibling path, unexpected ledger path, git/registry/registry-index/package alias override; non-proc-macro package; invalid literal form where the validator promises a literal-only check. Assert a useful field diagnostic, no panic, no dependency execution and unchanged prior output in a representative public CLI case.
2. A small actual Compact contract that calls one Unit helper twice with different arguments and witness results, producing two ordered public effects. Compare unedited generated native/recorded execution, real VM replay, state/effects, private outputs, witness arguments/order and per-query gas against an independent TS capture. The admission cache must not cache execution. Keep a repeated pure helper as a positive control if it fits the same small source naturally.

Before: generic mismatch and single-helper tests establish earlier boundaries. After: specific same-version source overrides are refused, and repeated admitted declaration use proves both executions occur. No emitter/runtime behavior change is planned. Coordinate new fixture/manifests with DID and diagnostic owners; do not amend their production modules.

### Acceptance and limits

Focused tests and strict Clippy pass; actual generated fixture is fresh; exact source/compiler/capture identities retained. Rerun fresh instrumentation for affected components with a clearly scoped denominator. Do not bypass declaration preflight to hit unreachable name-collision branches or manufacture non-coordinate leaf calls for a percentage. The proposed 95% changed-line target is not a claim that every full-module line is reachable through the public API; exclusions/dispositions must remain explicit.

If a new semantic defect is reproduced, stop the test-only assumption and record its root cause and decision before changing production behavior. This ADR does not assert an existing execution-cache bug.

### 2026-10-07 — Test implementation delivered

Commit `82e1336d402f933be11ea21daa57d5afe20f1e31` is local, conventional, GPG-signed and DCO-signed. Fourteen compatibility and four generated-fixture tests, strict Clippy, deterministic TS recapture and freshness pass. [ADR0267 — Runtime source and repeated-helper tests](references-0.3.0.md#note-034) contains the exact owned hashes and commands. Issue #391 stays open pending the fresh scoped coverage amendment; no production bug or proof acceptance is claimed.

### 2026-10-07 — Scoped coverage accepted

Fresh profiles passed 312 backend tests; a separately instrumented compilation of the repeated-helper source exercises the cached declaration-admission return four times. Compatibility source is identical to the baseline and now covers 215/216 executable production lines (99.54%), versus 201/216 previously. The remaining fingerprint-read error continuation is not forced with a contrived filesystem race. Other scoped results: naming 74/76, pure-call policies 270/284, composition 338/357 after fixture compilation. Composition includes ADR0265 work and its denominator changed; this is not a comparable old/new percentage or whole-backend figure. New pure-Unit guard coverage refinements belong to ADR0265 and do not block acceptance of this test-only slice.

[ADR0267 — Scoped coverage amendment](references-0.3.0.md#note-035) retains source/profile/object hashes, fresh profile separation and exclusions. Issue #391 can close. Parents #351/#361 remain open.
