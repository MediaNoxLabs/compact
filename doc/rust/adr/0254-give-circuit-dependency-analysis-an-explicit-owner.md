---
id: RUST-ADR-0254
alias: ADR-0254
source_sha256: a6482921eeb409781298c5c845cadd62c8b54a197f5fd74026b55628546f8e0a
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0254 — Give circuit dependency analysis an explicit owner

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for implementation,2026-10-07. Parent R030-01/#345 and R030-05/#349. No ABI/schema/public API change. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR-0254 — Give circuit dependency analysis an explicit owner

Status: accepted for implementation,2026-10-07. Parent R030-01/#345 and R030-05/#349. No ABI/schema/public API change.

### Problem

Witness-interface dependency, private-output dependency and call traversal currently live inside stateful.rs although constructors, native and recorded emission and public facades all depend on them. This obscures the distinction between user witnesses and native private transcript outputs and leaves their recursive analysis without a focused component test suite.

### Decision

Extract the existing IR-only traversal functions unchanged into private circuit_analysis.rs. Keep explicit internal reexports to avoid conflicting with the active DID composition slice. Do not change graph algorithms, call admission or diagnostic precedence during extraction. No new semantic evaluator, wildcard utility imports or all-purpose context object.

### Before and after

```rust
// Before: ownership coupled to native stateful emission.
use crate::stateful::{circuit_uses_witness, expression_requires_witness};
// After: shared IR analysis has its own component.
use crate::circuit_analysis::{circuit_uses_witness, expression_requires_witness};
```

Generated consumer Rust remains byte-for-byte identical. Runtime unchanged. ownPublicKey can produce private output without requiring the user's witness trait; the two facts remain separate. Unknown-callee declaration validation remains at its existing owner.

### Tests and acceptance

Mechanical extraction plus unit tests for pure/direct witness/native private output, nested argument and unused binding, both conditional arms, ReturnPlan and commitment opening, transitive chain/diamond/unrelated declarations, direct/mutual recursion and repeated acyclic calls. Backend tests and strict Clippy pass. Regenerated Rust and capability reports unchanged against an immutable compiler handoff after the DID slice stabilizes. No proof rerun solely for identical output.

### Alternatives and consequences

Leaving analysis in native lowering preserves behavior but hides its shared semantic role. A wholesale combined graph rewrite would mix optimization and ownership changes; defer until its diagnostic/performance evidence exists. Extraction adds a private module and temporary explicit reexports, reducing ownership coupling without claiming a new checked model or all-milestone completion.

### Research

[Next emitter components and developer API probes —2026-10-07](references-0.3.0.md#note-157) retains exact ranges, dependencies and later alternatives. ADR0243 remains the separate owner for bounded named-argument/frame probes.

### Delivery

`24fa6639441495858b2d3c5e132182febcb2626d`; exact extraction and eight semantic tests accepted. Generated Rust and capability reports unchanged for all 182 sources. Strict Clippy passed. [ADR0254 — Circuit analysis local receipt](references-0.3.0.md#note-021). Child #379 closed; broader domain/decomposition parents remain open.
