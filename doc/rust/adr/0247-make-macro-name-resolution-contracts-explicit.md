---
id: RUST-ADR-0247
alias: ADR-0247
source_sha256: d030bc2ec707247045a0422efa9ef20f229b80b59d23ac0e1d3f61f7f717b482
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0247 — Make macro name-resolution contracts explicit

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** locally-delivered. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: locally-delivered
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/371
```

## ADR-0247 — Make macro name-resolution contracts explicit

### Problem statement and evidence

Current derive/attribute expansions emit unqualified names such as Result, Option, Vec and From. Consumer compilation with valid local declarations of those names fails while the canonical control succeeds. The canonical runtime crate path is also assumed; a renamed dependency needs an explicit canonical alias. Exact sources, diagnostics and metadata hashes are preserved in [0.3.0 — Macro consumer hygiene probes — 2026-10-07](references-0.3.0.md#note-002). These are confirmed compile-time integration limitations, not a runtime vulnerability.

### Proposed direction — not delivered

Qualify standard library types/traits/constructors in macro output, and define the runtime dependency-name contract explicitly. Compile consumer reproducers and controls as regression tests. Assess emitter-level user type naming independently: fixing a derive cannot fix every unqualified name in generated functions. Do not silently rename source types or introduce dependency-discovery machinery without a measured need.

### Before / after example

```rust
// Current expansion can resolve Result to a consumer's own type.
fn decode(...) -> Result<Self, CompactError> { ... }
// Proposed expansion binds the standard result type explicitly.
fn decode(...) -> ::core::result::Result<Self, CompactError> { ... }
```

Illustrative syntax only; remediation is still open. A canonical dependency alias is a proven workaround for the renamed-crate case.

### Ownership and compatibility

Macros own names they introduce; emitter owns generated module/type/function namespaces. Runtime/ledger cryptographic behavior is unaffected. Qualifying macro output should preserve supported consumers, but full generated/native/recorded compilation must prove that. No current ABI/schema/version change is proposed.

### Alternatives and other findings

Banning common source type names is restrictive and does not solve arbitrary trait shadowing. Adding dependencies just to discover crate aliases requires a separate compatibility/dependency decision. CompactCellValue discriminant acceptance and delayed Merkle shape diagnostics are related quality observations, not accepted semantic changes in this ADR.

### Acceptance

Compile previously failing shadow-name examples with exact source/diagnostic evidence, preserve canonical controls, test chosen dependency alias policy and full generated consumer context. Issue remains open; this tranche delivers the finding and reproducers only.

### Accepted implementation scope — 2026-10-07

Qualify macro-owned standard-library paths and constructors, including introduced primitive spellings, without changing user-provided types, codecs, ordinals or Merkle layout. Witness bridges must use the canonical runtime CompactError path rather than a caller-local runtime alias. The runtime dependency contract remains the canonical crate name; a renamed dependency must provide `extern crate runtime_alias as midnight_compact_runtime;`. No discovery dependency is introduced.

Add actual Cargo consumer regressions for derives and witness bridges with hostile local names, plus canonical/renamed dependency controls. Assess generated types independently: upstream field-representation derives also introduce unqualified identifiers, so macro qualification alone does not establish complete generated namespace hygiene. Any additional namespace findings must be reproduced and recorded before widening the implementation. New explanatory documentation remains in Obsidian until milestone closeout.

### Delivery

Local signed/DCO commit `77c9ee144f3c326f29cb44a69b7174c5d81b8945`; [ADR0247 — Macro hygiene local receipt](references-0.3.0.md#note-014). Confirmed macro-owned name defects corrected; canonical dependency alias policy retained. Additional source-generated struct namespace failures remain tracked separately under ADR0249 / #374. No blanket claim of all possible consumer namespace hygiene.
