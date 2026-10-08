---
id: RUST-ADR-0273
alias: ADR-0273
source_sha256: a52e5f75ceb511d355dccce8b305bb9635906b084b1faccbfda744ce4c90c489
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0273 — Explain script failure privacy and distinct replay costs

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for a bounded documentation/regression slice. Parents R030-08/#352, R030-07/#351 and R030-13/#357. External review F5/F6. No runtime/generated API change. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0273 — Explain script failure privacy and distinct replay costs

Status: accepted for a bounded documentation/regression slice. Parents R030-08/#352, R030-07/#351 and R030-13/#357. External review F5/F6. No runtime/generated API change.

### Problem and decision

WitnessScript uses the generated witness boundary's CompactError; after conversion, a contract assertion and a script failure can carry identical text. Default LabError diagnostics deliberately redact arbitrary payloads. Preserve that policy and the explicit execution_error() accessor. Do not infer a typed script origin from assertion text, add a side-channel witness registry, or introduce a new runtime error variant for test harness metadata. A future direct-script typed API is deferred until a concrete consumer requires it.

Before: direct script tests mostly say is_err(), and the adapter guide does not show deliberate error inspection. After: test exact exhausted/mismatch errors and unchanged queue/journal; test consumption of a scripted failure and lab rollback. Show explicit inspection of a test's expected error alongside the redacted public message. The same script-like text returned by an application remains generic Execution. Retain secret-payload redaction in Debug/Display/Error::source.

```rust
// Existing API; deliberate inspection does not attest the error's origin.
assert_eq!(error.to_string(), "circuit execution failed (details redacted)");
assert_eq!(error.execution_error(), Some(&expected_error));
```

Execution gas is the sum of separately executed queries, including metered witness reads. Replay gas is one execution of the sealed public Verify program. Six actual scratch cases refute the review's universal witness-only delta: two writes with no witness have distinct summed/replay write accounting. Neither RunningCost nor its difference is a universal scalar witness-cost measure.

Before: users could read the two report values as interchangeable or subtract them to estimate witnesses. After: accessor documentation and the vault testing guide explain both scopes, grouping effects and the absence of an aggregate execution-gas policy. Fixture-specific regressions compare each report to independently measured component/full-program queries, retain frozen TS sums, and demonstrate the no-witness two-write distinction.

### Owners and validation

Only source API comments and focused tests in testkit-rs/src/{witness,error,report}.rs and tests/{boundaries,generated_scenarios}.rs; no runtime or emitter semantic change. New engineering guides and research remain in Obsidian until milestone closeout. Ordinary existing APIs suffice; no new derive, builder, gas field or role classifier.

Run focused script/privacy/generated/rollback tests and strict testkit Clippy. Preserve exact six-case scratch gas/source/lock receipts and the before/after documentation. No cryptographic proof rerun is necessary because neither generated programs nor execution semantics change. External final review independently checks the disposition. This does not close the parent audit or claim all runtime costs are equal or ordered.

Issue: https://github.com/MediaNoxLabs/compact/issues/397

### 2026-10-07 — Delivered

Commit `dc745e709dcaa0ed621a1690f9a7db9ffe90fe15`, conventional, good GPG signature and DCO. Existing APIs now have precise comments and regression coverage for deliberate error inspection, private script rollback and distinct gas scopes. Script exhaustion/mismatch leaves queue/journal unchanged; matching invocation consumes its answer and journals arguments even on error. Six native/recorded late-failure cases preserve the lab checkpoint. Application assertions matching script error text remain generic/redacted, with no invented origin classification.

Execution gas is compared to separately executed component queries and frozen TS sums; replay is compared to one independent full Verify program. A no-witness two-write control proves grouping/write accounting alone can differ. No universal greater-than or witness-only delta claim is made.20 ContractLab integration tests and strict all-target/all-feature Clippy pass. No runtime/emitter/API or execution-policy change.

The testing guide is updated in Obsidian. Root corrected one draft phrase to say the script journals arguments, not answers; final guide hash is recorded separately. No new standalone repository guide publication; that remains closeout work. No proof rerun is needed for comments/tests only.

[ADR0273 — receipt.json](references-0.3.0.md#note-044).
