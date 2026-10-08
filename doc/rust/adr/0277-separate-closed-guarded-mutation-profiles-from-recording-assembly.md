---
id: RUST-ADR-0277
alias: ADR-0277
source_sha256: 2621e310840cc8f6549b68f44ce99cfc0de0657573ae48aad4fe9e4fd8ab6853
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0277 — Separate closed guarded mutation profiles from recording assembly

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for an ownership-only refactor after ADR0269/0274 freeze. Parent R030-05/#349. The user explicitly requested smaller, human-friendly recorded.rs/stateful.rs/lib.rs components; recorded.rs remains8661 lines despite earlier separations. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0277 — Separate closed guarded mutation profiles from recording assembly

Status: accepted for an ownership-only refactor after ADR0269/0274 freeze. Parent R030-05/#349. The user explicitly requested smaller, human-friendly recorded.rs/stateful.rs/lib.rs components; recorded.rs remains8661 lines despite earlier separations.

### Problem

recorded.rs interleaves public diagnostics, recording assembly, generic lowering and four closed guarded mutation profiles. The organizer Set gate, authority-prefixed optional Cell write, authority-prefixed enum advance and witness-admitted Merkle insert occupy one contiguous approximately950-line block. Three share the same private AuthorizedContinuation prefix. Engineers changing recording assembly must navigate unrelated proof-supported shape checks; moving only part of that prefix would duplicate or expose ownership.

### Decision and before/after

Move the four complete profiles and their shared private prefix/continuation into recorded/guarded_mutations.rs. Export only the four existing entry functions with pub(super) visibility. Keep explicit imports, not a parent glob. Recording assembly imports those four entry functions and calls them in the same order. The continuation and prefix remain private to the new component. Keep every matching predicate, evaluation order, optional/error result and quote-generated statement unchanged.

```rust
// Before: closed profiles and shared prefix embedded in recorded.rs.
fn closed_authorized_optional_write_steps(/* existing inputs */) -> Result<Option<Vec<syn::Stmt>>, RenderError> { /* existing body */ }

// After: recorded/guarded_mutations.rs owns the same body and private prefix.
pub(super) fn closed_authorized_optional_write_steps(/* unchanged inputs */) -> Result<Option<Vec<syn::Stmt>>, RenderError> { /* identical body */ }
// recorded.rs imports that narrow entry point; ordered selection is unchanged.
```

This is source ownership, not new admission or a broad policy rewrite. Keep Option/Result contracts and profile precedence; do not mix in tri-state profile migration, rename generated variables, change public APIs, alter type domains or add helpers to the runtime. Existing shared pure-expression/type functions retain their owners.

### Acceptance

- Preserve an exact before/after moved-body comparison after removing only visibility; shared prefix/continuation are byte-identical.
- Existing full backend tests pass on their ordinary configured workers with RUST_MIN_STACK absent; no stack-limit adjustment.
- Immutable renderer comparison across the183-source release fixture inventory has identical generated Rust and capability reports. Include actual guard-family fixtures and their existing admission refusal tests; no new mirrored implementation test is needed for an exact move.
- Strict backend Clippy passes. No generated fixture refresh, ABI/schema/dependency change or proof rerun if generated output is identical.
- Update the vault ownership/navigation guide and retain before/after file sizes and four entry-point responsibilities. Approximately1000-line component is a cohesive intermediate improvement, not a claim that the full large-file decomposition outcome is complete.

Issue: https://github.com/MediaNoxLabs/compact/issues/401

### Delivered locally — 2026-10-07

Four closed guarded mutation profiles and their private shared authority continuation now live in recorded/guarded_mutations.rs. Four pub(super) entry points preserve routing and profile precedence. The957-line moved body is exact after removing only visibility. recorded.rs shrank8661→7708 lines; the new cohesive component has986 lines.326 default-worker backend tests and strict Clippy passed. Immutable renderers produced identical complete Rust and capability files for all183 source fixtures. No generated ABI, admission, proof or runtime behavior change; no proof rerun was needed for byte-identical output. This is an intermediate decomposition, not full R030-05 acceptance.

Good GPG signature and DCO commit `fca7577b8a56150a869c56d1e7979727cef33059`. [ADR0277 — delivery-receipt.json](references-0.3.0.md#note-048).

Build provenance: initial scratch renderer resolution lacked the frozen lock and is excluded from acceptance. Rebuilt with the ADR0269 frozen lock (only scratch package name changed), --locked --offline, before the full comparison. The rejected setup is retained in /tmp/compact-adr277/build.log and Cargo.unreviewed-resolution.lock.
