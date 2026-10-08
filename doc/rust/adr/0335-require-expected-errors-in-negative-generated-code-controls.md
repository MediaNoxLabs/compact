---
id: RUST-ADR-0335
alias: ADR-0335
source_sha256: 6de54279083fce1635302d396b8f360f87464be8d13b9bec290ad6f23b33431a
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0335 — Require expected errors in negative generated-code controls

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** approved bounded test correction, 2026-10-07. Parent R03007 / #351. No emitter or runtime change. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0335 — Require the expected error in negative generated-code controls

Status: approved bounded test correction, 2026-10-07. Parent R03007 / #351. No emitter or runtime change.

### Problem
The chunked-cell regression calls `assert_active(false)` on an initialized true Cell, but only compares whether native and recorded calls are errors. Two incorrect successes would satisfy that equality. The Compact source explicitly requires `AssertionFailed("active mismatch")` for this input.

### Decision and before/after
Keep the positive TypeScript comparisons and replace this weak negative assertion with two independent checks of the exact error variant and message. Each invocation uses a fresh existing test context.

```rust
// Before: agreement alone also accepts two unexpected successes.
assert_eq!(native.is_err(), recorded.is_err());
// After: require the intended refusal independently on both paths.
assert!(matches!(native,
    Err(runtime::CompactError::AssertionFailed(message))
        if message == "active mismatch"));
assert!(matches!(recorded,
    Err(runtime::CompactError::AssertionFailed(message))
        if message == "active mismatch"));
```

### Ownership and verification
Only `tests-rust-backend/chunked-cell/tests/chunked_cell.rs` changes. Existing source, generated library, oracle capture, compiler, runtime and dependency graph remain unchanged by this correction. Run the existing focused package tests, strict scoped Clippy and formatting. No new proof or broad runtime gate is justified by this test-only change.

This is a demonstrated test weakness, not a demonstrated runtime bug. It closes one explicit negative-case assertion gap; it does not establish exhaustive branch coverage or close the larger milestone audit. The broader export/evidence matrix records the source and scenario mapping. No stopped investigation is included.


### Delivery — 2026-10-07

## Native effects and exact negative controls — 2026-10-07

### Delivered locally

- ADR0334 / [#458](https://github.com/MediaNoxLabs/compact/issues/458): signed DCO commit `827ac965219e8fc203ab0ed8cfb8416094d06644`. Four generated native integration tests execute18 separately captured TS scenarios: equal conditional arms retain callbacks; Set/Map membership and Map lookup use witnessed keys; missing cells, refusal and zero-gas precedence have exact errors. Results, serialized state, private outputs, effects and gas are asserted. Strict scoped Clippy, formatting, JS validation and one-source freshness pass. Maintained roots now198; full joined freshness awaits the separately tracked compiler fix.
- ADR0335 / [#459](https://github.com/MediaNoxLabs/compact/issues/459): signed DCO commit `6dd4e965973042ce67ee87458d39ab35f0181fba`. The chunked-cell negative control independently requires `AssertionFailed("active mismatch")` from native and recorded calls, preventing two successes from passing a Boolean equality. One comprehensive integration test, strict scoped Clippy and formatting pass.

Root reviewed assertions, source hashes and manifest/lock changes. The only dependency change is one local fixture package. No production runtime/emitter changes in these two commits. Native query exports retain unavailable recorded APIs; query program/event evidence belongs to TS, while Rust asserts callback count, gas and error precedence. No new proofs or network acceptance are claimed.

### Evidence

[ADR0334-0335 — Native effect and negative-control execution.zip](references-0.3.0.md#note-113): 92 hashed entries; SHA256 `083f72be3af05c0b5b7172d305e89fa0121c2d5d414bd0089b28d687a56c579c`. Contains TS capture outputs and failed calibration attempts, generated source, execution joins, exact receipts/logs and signed commit patches. Original paths `/tmp/rust030-adr334-delivery` and `/tmp/rust030-adr335` remain available.

