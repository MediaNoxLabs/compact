---
id: RUST-ADR-0244
alias: ADR-0244
source_sha256: f2dc665c9bbc35f898176af46aadd90f830c9e35cbb50dac35f054922b3c5660
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0244 — Test owned semantic boundaries before claiming code quality

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/368
```

## ADR-0244 — Test owned semantic boundaries before claiming code quality

### Problem statement

The generated fixture suite proves valuable end-to-end behavior, but local error guards need direct tests. The macro crate has five unit tests; Merkle and CellValue expansion paths lack direct tests. Capability classification tests cover individual bad entries but not late-failure atomicity across exports. Runtime replay effects/state guards and local-helper policy fields need explicit negative controls. A raw test count or grep of unwrap calls is not a quality assessment.

### Decision

Add table-driven, behavior-oriented tests at the owning component boundary. Use private unit tests for implementation validation and runtime-facing tests where real VM state is the relevant oracle. Cover positive controls, malformed input, exact typed errors, failure atomicity and order. Keep existing generated/TS/proof integration evidence distinct. Measure owned handwritten source with LLVM coverage when available, reporting unit-only vs combined exercised scope and actual missing lines; exclude third-party/generated boilerplate from ownership totals. Do not promise 100% or count unreachable scaffolding as useful assurance.

### Before / after examples

```rust
// Before: one malformed export checks only a schema number.
assert!(report.apply_contract_info(&bad_metadata).is_err());
assert_eq!(report.schema_version, 2);
// After: a valid first export precedes a malformed later export.
let before = serde_json::to_value(&report).unwrap();
assert!(report.apply_contract_info(&late_failure).is_err());
assert_eq!(serde_json::to_value(&report).unwrap(), before);
```

### Emitter/runtime changes

This decision authorizes tests and local assessment, not semantic changes. Test current intended invariants without pinning incidental formatting. Separate reproducible bugs into focused issues/ADRs before correction. No ABI, private IR schema or generated API change is expected from tests.

### Alternatives and limits

Only adding happy-path fixtures hides branch gaps. Blanket snapshot tests pin syntax without proving semantics. A full dependency/world rebuild on each change wastes time. Use targeted packages and expand gates for concrete affected risks. Coverage instrumentation is a separate build and must be bounded by local disk/time; unavailable instrumentation must be reported, never inferred from counts.

### Assessment and follow-up

Review correctness/type safety, fail-closed behavior, domain ownership/coupling, readability, duplication, diagnostics, testability, resource use and supported portability. Distinguish observed defects, verified guarantees and unmeasured risks. Human/developer friendliness includes understandable test names and diagnostics. Store receipts and the quality assessment in the vault, then publish at closeout.

### Local delivery — 2026-10-07

Delivered at `c70dd63cf91ce59e2bda82a0f27f32073639c669`, signed/DCO, local only. Clean 179-package cohort: 781 tests passed, none failed/ignored. 176 generated libraries unchanged; strict owned-package Clippy and formatting passed. [Code quality assessment](references-0.3.0.md#note-142) and [ADR0244-0246 — Quality validation receipt](references-0.3.0.md#note-013) record scope, hashes, coverage and remaining limits.
