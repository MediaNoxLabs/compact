---
id: RUST-ADR-0249
alias: ADR-0249
source_sha256: 7cc89a2b6110f040acf5faef64f3522506869c2ac8ecc4a59e0fbb33b2503518
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0249 — Isolate generated struct representation from source names

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** locally-delivered. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: locally-delivered
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/374
```

## ADR-0249 — Isolate generated struct representation from source names

### Problem and evidence
Real Compact source structs named Option, Fr, MemWrite, runtime and FieldRepr pass the frontend but fail Rust typechecking. Identical Pair control passes. Unlike ADR0247, failures arise in upstream field representation derives and emitter support imports. See [0.3.0 — Generated source-name hygiene evidence — 2026-10-07](references-0.3.0.md#note-001).

### Proposed direction under review
Reuse upstream BinaryHashRepr, FieldRepr, FromFieldRepr, Fr and MemWrite. Generate only hygienic structural delegation in one owned CompactStructRepr derive, retaining field order and malformed decoding semantics. Remove support imports from the generated type namespace and use explicit runtime paths. Preserve source names and public types::Name paths. Any new macro required by emitted code must be reflected in the runtime compatibility guard.

### Before / after examples
```rust
// Before: caller Option shadows upstream expansion's standard Option.
#[derive(BinaryHashRepr, FieldRepr, FromFieldRepr)]
pub struct Option { pub amount: runtime::Field }
// Proposed: owned derive delegates each field through upstream traits.
#[derive(::midnight_compact_runtime::CompactStructRepr)]
pub struct Option { pub amount: ::midnight_compact_runtime::Field }
```

### Ownership, alternatives, verification
Macro owns fully qualified mechanical trait glue; upstream owns representation semantics. Emitter owns public namespace and type paths. Banning common identifiers is an unnecessary language restriction. Renaming source types damages developer APIs. Upstream fixes alone do not address the runtime alias collision and may require version alignment. Test real source generated consumers, ordinary/empty/nested structs, malformed lengths, and independent upstream codec/field vectors. This is a compile-time defect, not evidence of a cryptographic vulnerability. Design acceptance and implementation receipt follow separately.

### Accepted bounded implementation

Independent macro review supports one owned CompactStructRepr derive, strictly delegating to existing field traits. Support the emitter's named nongeneric structs (including empty) first; reject unsupported declaration shapes explicitly instead of inventing a general generic contract. Fully qualify macro-owned identifiers and use generated positional bindings to avoid collisions with source field names. Compare ordinary and composite representations with upstream trait implementations and test short/trailing input.

Extract type declarations into an emitter component with explicit support-name qualification. New generated code requires CompactStructRepr, so increment the runtime compatibility guard from ABI49 to ABI50 in emitter/runtime and regenerate fixture outputs. This does not change IR20, capability schema3, ledger pins or serialized primitive representations. Rust consumers must regenerate/update compiler and runtime together under the existing exact-ABI policy.

### Delivery

Signed/DCO local commit `ee768a60706607580d4ff8b3c949f22c882501b0`; [ADR0249 — Generated representation local receipt](references-0.3.0.md#note-017), [ADR0249 — Accepted regression log](references-0.3.0.md#note-016). 795 tests / 180 packages pass; 177 fixtures regenerated with zero failures. New CompactStructRepr reuses upstream trait delegation, supports named concrete structs including empty, rejects unsupported shapes. Generated compatibility guard is ABI50. IR20, capability schema3, ledger8.0.3 pins unchanged.
