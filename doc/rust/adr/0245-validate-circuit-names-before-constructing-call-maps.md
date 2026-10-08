---
id: RUST-ADR-0245
alias: ADR-0245
source_sha256: 23167fab539e211dfdb57027619b21d4c58e0b8339dbb744d660504ec7f5dfd2
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0245 — Validate circuit names before constructing call maps

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/369
```

## ADR-0245 — Validate circuit names before constructing call maps

### Problem statement

Pure/stateful call maps currently collect declarations before the later duplicate-name check. HashMap last-write behavior can select a conflicting callee while rendering an earlier caller, causing an unrelated type/lowering error before the duplicate declaration is reported. This is a malformed-IR diagnostic quality finding, not a claim of accepted invalid output.

### Decision

First reproduce it. If confirmed, validate identifiers and duplicate declarations over the complete pure/stateful declaration set before constructing callable maps or lowering bodies. Preserve source locations of the offending declarations and legal forward calls. Keep output for accepted programs unchanged.

### Before / after

```rust
// Before: duplicate key can select a callee before validation.
let callable = circuits.iter().map(|c| (c.name.clone(), c)).collect();
// ... later rendering detects duplicate declarations ...
// After: one checked declaration namespace precedes map construction.
validate_circuit_declarations(contract)?;
let callable = circuits.iter().map(|c| (c.name.clone(), c)).collect();
```

The helper name is illustrative. Actual implementation should retain existing error types and avoid an unnecessary public abstraction.

### Ownership and compatibility

Emitter admission owns declaration uniqueness; HashMap is an index, not a validator. Runtime/ledger/zk behavior, ABI and IR schema are unchanged. Error precedence for malformed duplicate declarations intentionally improves; valid generated output must remain unchanged. Scope all new tests to exact duplicate/identifier/location/forward-call obligations.

### Alternatives / verification

Relying on insertion order or selecting the first duplicate still makes malformed calls order-sensitive. Render all bodies and report duplicates last retains confusing diagnostics. Require a red reproducer before the fix, table tests across declaration kinds, local emitter regression suite and unchanged generated fixtures. Record actual results before marking accepted/delivered.

### Local delivery — 2026-10-07

Delivered at `c70dd63cf91ce59e2bda82a0f27f32073639c669`, signed/DCO, local only. Clean 179-package cohort: 781 tests passed, none failed/ignored. 176 generated libraries unchanged; strict owned-package Clippy and formatting passed. [Code quality assessment](references-0.3.0.md#note-142) and [ADR0244-0246 — Quality validation receipt](references-0.3.0.md#note-013) record scope, hashes, coverage and remaining limits.
