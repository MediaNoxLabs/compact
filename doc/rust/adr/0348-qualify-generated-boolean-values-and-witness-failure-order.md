---
id: RUST-ADR-0348
alias: ADR-0348
source_sha256: 2256e32497c631a9a6633bbc09eb170306e950e25a61bdbbd1ebd4a4cd26e747
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0348 — Qualify generated Boolean values and witness failure order

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered; further conformance work deferred by owner. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0348 — Qualify generated Boolean values and witness failure order

Date: 2026-10-08
Status: delivered; further conformance work deferred by owner
Parents: #351 / #361; follows ADR0347 specification seed B02/B03/B04/B08

### Problem

The conformance map identifies only two of four pure AND/OR pairs in the historical oracle test. Selected witness success cases do not establish skipped right-hand failures, left-hand refusal or both NOT witness values. These are finite, source-defined requirements, independent of fixture totals.

### Before and after

Before: selected TS capture comparisons establish four binary witness cases and one NOT case.
After: an explicit four-row truth table exercises generated pure and witnessed functions; an instrumented fallible callback records input and private-state order, can refuse on a chosen call, and proves selected versus skipped evaluation. Expected Boolean results and transcript bytes are literals, not computed through the runtime under test.

```rust
// Before: selected captured case
assert_case(witnessed_both(context(), &Echo, false, true).unwrap(), &oracle["andSkip"]);
// After: skipped RHS configured to fail if wrongly called
let witness = TracedEcho::new(Some(2));
let result = witnessed_both(context(), &witness, false, true).unwrap();
assert_eq!(result.result, false);
assert_eq!(*witness.calls.borrow(), vec![(7, false)]);
```

### Ownership and scope

Add a separate human-readable test module to the existing generated boolean-logic fixture crate. Preserve generated Compact/Rust and historical capture/test files. Exercise public native generated APIs and the existing fallible witness bridge. Update the specification evidence map only after exact test execution. No emitter/runtime/API/ABI/dependency change. Preserve native-only versus recorded/proof qualification and existing historical attribution. No arbitrary callback rollback claim: on failure the test observes callback invocations and the exact returned error, not an inaccessible post-failure circuit context.

Non-Boolean source typing, ternary/statement conditionals, arbitrary nesting, fresh TS captures, recording, replay and proofs retain separate obligations. This slice implements the first bounded Boolean behavior vertical; no universal language/formal guarantee. Stopped ADR0285/#409 work is excluded.

### Acceptance

- All four pure AND/OR pairs and both NOT inputs have independently stated expected results.
- All four witnessed AND/OR pairs and both witnessed NOT inputs check result, callback order/arguments/private-state threading, final private state and canonical Boolean transcript atoms/alignment.
- A second-call refusal is skipped for false AND / true OR, returned unchanged for true AND / false OR; first-call refusal halts before RHS. NOT forwards operand refusal.
- Focused Rust tests, strict scoped Clippy and formatting pass using locked/offline warm target; baseline checker validates the revised evidence map.
- Independent review checks expectations, meaningful failure assertions and evidence scope. Record exact tool/source/log hashes in midnight. Conventional signed/DCO commit; no remote CI campaign.

Parent acceptance remains 12/19. User-owned doc/ledger-adt.mdx is preserved.

### ADR0348/#476 delivered; further conformance deferred — 2026-10-08

Signed GPG/DCO [cb687ddc](https://github.com/MediaNoxLabs/compact/commit/cb687ddc42ab6d5d7af17971b324db7c3689b226) adds five native Boolean conformance tests to the existing generated fixture. Complete literal AND/OR/NOT values, callback order and private-state threading, canonical transcript atoms/alignment, selected/skipped/first-call refusal and inverted witness-answer controls are tested. Historical comparison remains intact. Six fixture tests, strict scoped Clippy, formatting, 29 mapping tests and baseline check pass. Independent source review found no remaining issue. No emitter/runtime/generated/lock change, fresh TS/compiler/proof run or formal guarantee.

**Owner direction:** further specification conformance belongs in the next milestone. Preserve the map and define follow-ups without additional expansion now. Full-language coverage/formal repairs do not become new 0.3.0 release gates. Existing approved coverage, audit and release obligations remain required. Focus remaining work on documentation, audit and candidate closure; the recorded ADR0285/#409 stop remains unresolved. Parent acceptance stays 12/19.

Evidence [ADR0348 — Boolean values and witness ordering.zip](references-0.3.0.md#note-125), SHA256 `965a724823cb4dc9d0df8e848fc72f2d14566cf479de7f362249648a35dc497a`, 17 verified entries. 459 gate inputs captured after execution and rechecked unchanged; reviewer attribution and exact hashes retained. User-owned ledger-adt document preserved.
