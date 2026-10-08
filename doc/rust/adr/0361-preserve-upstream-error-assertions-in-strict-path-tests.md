---
id: RUST-ADR-0361
alias: ADR-0361
source_sha256: 014013649b633c1cc34619f5b6fc4c4ff331b725b178de128b63d2a01f5f4a72
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0361 — Preserve upstream error assertions in strict path tests

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted, implementation in progress. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0361 — Preserve upstream error assertions in strict path tests

Date: 2026-10-08
Status: accepted, implementation in progress
Parent: #364 final qualification

### Problem

The bounded Linux run37739786445 at17433b40 passes tests and Rust1.88 checks but fails strict Clippy. Root reproduces 25 result_large_err diagnostics in two test functions in runtime-rs/src/ledger/program_path_tests.rs. The closures intentionally retain upstream TranscriptRejected<DefaultDB> (at least136bytes) so tests match the exact BoundsExceeded and InvalidArgs variants. Earlier targeted checks did not lint this entire lib-test target.

### Decision and before/after

Add #[expect(clippy::result_large_err, reason = ...)] only on the two affected test functions. Keep strict -D warnings globally, upstream return types, exact assertions and every test case unchanged. Expectation becomes a diagnostic if it is no longer fulfilled. Do not box or stringify official errors just to satisfy a test-only style warning, or suppress it across the production crate.

```rust
// Before: closure test table directly carries upstream error variants.
#[test]
fn mutation_operands_preserve_exact_encoding_boundaries() { /* existing cases */ }
```

```rust
// After: document the deliberate test-only upstream error representation.
#[test]
#[expect(clippy::result_large_err, reason = "Test builders preserve upstream errors for exact variant assertions")]
fn mutation_operands_preserve_exact_encoding_boundaries() { /* same cases */ }
```

### Runtime / emitter / domain impact

No production or generated-code changes, dependency changes, ABI changes or changed error semantics. This is a narrow qualification fix. The original17433b40 remote failed result stays recorded; qualify the signed successor and never relabel the old result green.

### Validation

Run all four path-boundary unit tests and the exact all-target/all-feature four-owner strict Clippy command. Preserve the failing log, successful local results, signed commit and successor CI result. Then freeze the successor for full applicable candidate qualification. The documentation package remains a historical17433b40 staging artifact until refreshed at publication.

### Local delivery

Issue [#498](https://github.com/MediaNoxLabs/compact/issues/498). Exact CI lint command reproduced25warnings; two scoped expectations retain all assertions. Four path tests and full four-owner all-target/all-feature Clippy pass. Signed/DCO successor `2c798d2dc5f94ad13e89610068035ed70d94f469` verifies G. Initial commit20b2b1b8had a bad signature and was replaced with the same tree/message under an exact remote lease; it is not an accepted candidate. CI37742764841qualifies the successor and is pending. No production changes.

### Bounded remote acceptance

CI37742764841passes both jobs at2c798d2d; artifactsource/lock/log hashes verified. [ADR0361 — Strict Clippy and bounded candidate CI](references-0.3.0.md#note-138) retains exact scope. #498 delivered; broader #364 remains open.
