---
id: RUST-ADR-0271
alias: ADR-0271
source_sha256: 7ccf1890fec3fc328c417c56be301359f0b063838c6f9b1a09754f36af1a57b0
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0271 — Classify and test every local-helper context boundary

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for implementation. Parents R030-03/#347, R030-05/#349, R030-07/#351, R030-13/#357. Independent external review F2/H1. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0271 — Classify and test every local-helper context boundary

Status: accepted for implementation. Parents R030-03/#347, R030-05/#349, R030-07/#351, R030-13/#357. Independent external review F2/H1.

### Problem and evidence

RecordingFrame::call_local already refuses public state/effects, call-context, native intent, wallet and execution-policy changes. Existing tests exercise state/policy/call fields and the wallet frontier but do not directly exercise every wallet collection or an effects-only native kernel mutation. No bypass is reproduced: this is a test gap around correct existing checks.

The review also questioned future upstream fields. Root inspected pinned midnight-onchain-runtime3.0.0: QueryContext has four fields, CallContext eight and BlockContext four. Current comparisons account for them. Wallet State has five public fields. There is no confirmed omitted current field; future additions could compile unnoticed when comparisons use only selected field reads.

### Before and after

Before: field-by-field comparisons such as `before_call.own_address == after_call.own_address` leave future upstream additions implicit. After: a small, exhaustive destructuring of each policy-bearing upstream record makes its classified fields explicit before the same comparison. Do not use `..`. Prefer private ordinary functions or local patterns; avoid new public trait hierarchies or a generic serialization comparison.

Before: the effects and wallet collection guards lack direct cases. After: an effects-only kernel operation (state unchanged) and independent coins/pending-spends/pending-outputs/Merkle mutations are each rejected through the public call_local boundary. Positive private-state/witness/gas controls remain accepted.

### Decision and ownership

Add meaningful tests in runtime-rs/tests/recording_local_boundary.rs. Introduce only the minimum private exhaustive field classification in runtime recording/context and the existing testkit environment/wallet comparisons. Verify the pinned upstream structs first; reject unsupported representation assumptions rather than rewriting upstream types. Keep user-owned runtime context fields explicitly classified; no changes to gas, recorded ops, private output order or trusted callback limits.

Do not infer transaction authorization or source-level admission guarantees from these runtime checks. No public API, ABI, schema or dependency change is planned.

### Acceptance

Each independent effects/wallet mutation fails for the expected boundary reason, with positive controls passing. Existing runtime/helper and ContractLab tests plus strict Clippy pass. A bounded source mutation probe should demonstrate that removing the effects guard makes the new effects-only regression fail; run only in scratch, preserve the baseline and changed source hashes, and restore no production mutation. Exhaustive patterns compile against the current pinned types; a code review maps all fields to policy. Final independent review checks the remediated scope.

Issue: https://github.com/MediaNoxLabs/compact/issues/395

### 2026-10-07 — Delivered locally

Commit `095b67c531b37f82a62e0daedb8dde30a013145f`. Private exhaustive destructuring now classifies all pinned QueryContext4, CallContext8 and wallet5 fields; testkit also classifies BlockContext4 and its Environment6. The current guards and equality policy are preserved. Future upstream field additions require an explicit decision at compile time.

New tests independently refuse effects-only kernel mutation and wallet coins, pending spends, pending outputs and Merkle changes; each wallet case proves the other fields/frontier remain unchanged. An actual metered witness positive preserves private state, output and upstream read gas. 48 focused runtime tests and17 ContractLab integration tests pass with strict Clippy. A scratch copy passes the effects test before removing only the effects equality; that mutant then fails the expected refusal assertion. Production bytes remain untouched by the probe.

No existing bypass was alleged or reproduced. No public API/ABI/schema/dependency or semantic change. Arbitrary trusted callback transient behavior remains outside these final-context checks.

[ADR0271 — Local delivery receipt](references-0.3.0.md#note-042). Independent final retest remains a parent audit obligation.
