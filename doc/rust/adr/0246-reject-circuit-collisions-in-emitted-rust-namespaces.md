---
id: RUST-ADR-0246
alias: ADR-0246
source_sha256: c69023f25b215a497f53912adaafb01b82dbcf4f7ba5eae0e31b5af7ad23135d
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0246 — Reject circuit collisions in emitted Rust namespaces

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/370
```

## ADR-0246 — Reject circuit collisions in emitted Rust namespaces

### Problem statement

The emitter replaces `$` with `_` when constructing Rust identifiers, but raw-name uniqueness does not imply emitted-name uniqueness. Distinct declarations `a$b` and `a_b` can generate two functions with one Rust name. Public wrapper arguments already have hygiene handling; public circuit names lack a corresponding rejection boundary.

### Decision

Reproduce the failure, then preflight normalized circuit identifiers per emitted namespace. Reject collisions with a dedicated typed error carrying original names and emitted identifier, located at the later conflicting declaration. Keep existing raw duplicate diagnostics. Preserve legal names in different Rust namespaces and single normalized names. Do not silently mangle a public API or change Compact entry-point strings.

### Before / after examples

```rust
// Before, two distinct Compact names can emit an invalid crate:
pub fn a_b() { /* Compact a$b */ }
pub fn a_b() { /* Compact a_b */ }
// After, reject before publishing output with both Compact names and source:
// circuit names "a$b" and "a_b" both emit Rust identifier "a_b"
```

### Ownership / compatibility

The emitter owns valid target naming; ledger/runtime semantics and cryptographic entry points remain unchanged. A new RenderError variant is an additive public diagnostic surface and may require exhaustive-match consumers to update. Runtime ABI and IR schema do not change. This intentionally refuses previously invalid generated Rust while preserving valid output.

### Alternatives and verification

Last-write maps or relying on downstream rustc hide source provenance. A new mangling policy would change generated public names and requires separate design/compatibility work. Use red reproducers, source-located negative tests, successful normalization and cross-namespace controls, package tests and unchanged generated fixture output.

### Local delivery — 2026-10-07

Delivered at `c70dd63cf91ce59e2bda82a0f27f32073639c669`, signed/DCO, local only. Clean 179-package cohort: 781 tests passed, none failed/ignored. 176 generated libraries unchanged; strict owned-package Clippy and formatting passed. [Code quality assessment](references-0.3.0.md#note-142) and [ADR0244-0246 — Quality validation receipt](references-0.3.0.md#note-013) record scope, hashes, coverage and remaining limits.

Internal review found an additional raw-identifier equivalence (`foo` / `r#foo`, `a$b` / `r#a_b`); a red reproducer preceded the comparison-key fix. Same-namespace rejection and cross-namespace acceptance pass.
