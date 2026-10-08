---
id: RUST-ADR-0338
alias: ADR-0338
source_sha256: da3735069c422efc95f547a41dc1ebc1f783bdfc9134aa5992dd541a1bec77ba
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0338 — Anchor generated-value tests to independent expected values

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for bounded test-only implementation. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0338 — Anchor generated-value tests to independent expected values

Date: 2026-10-07
Status: accepted for bounded test-only implementation
Parents: R03007 / #351; 0.3.0 meaningful generated-code coverage

### Problem
Read-only assertion review found two concrete test weaknesses, not demonstrated production bugs.

1. wide-field-literal compares Rust `read_large` with Rust `constant`, while separately asserting the captured decimal string. Both Rust results could agree on a wrong value without that comparison noticing.
2. observed-composite-keys compares native/recorded/replayed state and operation shape, but shape extraction drops pushed values. The chosen generated calls insert vector[3,5], tuple(42,true), their CompositeKey and sum1..12; tests do not independently assert those exact members in resulting Sets. Existing FAB encoding controls exercise runtime encoding separately and cannot replace generated-call checks.

### Before / after
```rust
assert_eq!(read.result, constant().unwrap());
assert_eq!(oracle["read"], EXPECTED_DECIMAL);
```
After, derive the expected Field through the ordinary upstream-supported parser/conversion from the pinned decimal and compare each actual Rust result to it, retaining TS/state comparisons.

```rust
assert_eq!(read.result, expected_field);
assert_eq!(constant().unwrap(), expected_field);
```
For each generated insertion, inspect its result state's typed ledger view and require membership of the supplied vector/tuple/struct or Field78. Prefer exact cardinality/unrelated-member checks when they add a distinct useful guarantee. Retain existing native/recorded/replay comparison; do not alter recording/observation trust policy.

### Ownership and validation
Only the two existing integration test files change. No Compact source, generated library, capture, emitter, runtime API, dependencies or proof work. Use established state-view APIs and upstream Field parsing, not handwritten byte codecs. Root review the assertions, run these two packages with applicable features plus strict scoped Clippy and formatting. Record exact source/lock/log identities; avoid repeating unrelated gates or regenerating proofs.

### Boundaries
This is ordinary generated argument/value/state semantics. It does not reopen ADR0285 consumer-safety/cross-instance or observation-trust investigation. Partial native-only, replay, proof and network claims remain separate. No new whole-codebase coverage percentage follows from these assertion additions.

### Evidence
Current findings: tests-rust-backend/wide-field-literal/tests/wide_field_literal.rs lines50–56; tests-rust-backend/observed-composite-keys/tests/observed_composite_keys.rs check_recorded lines148–171, shape normalization190, comparison208–210 and generated calls229–295. Source sum is examples/rust_backend/observed_composite_keys.compact:56. Full read-only semantic join report is separately retained.


### Reviewed default-state follow-up — 2026-10-07

The frozen pure-family report additionally finds3partial initializer checks: field-cast-uint128 overwrites `stored` before checking its default; nested-collection-query-write overwrites Boolean flags before checking defaults; observed-composite-keys has no independent initial-empty Set check. Extend this same expected-value slice to assert the ordinary source-defined defaults immediately after initialization, before mutation. Add only tests/field_cast_uint128.rs and tests/nested_collection_query_write.rs to the two existing owned test files; observed composite default checks share its already-owned file. No new fixture, generation, source or runtime code. Run only the four affected packages and lint/format. The initial two-file plan remains historical; this amendment is recorded before the additional edits.


#### Expected Field construction

Upstream Field exposes little-endian construction but no direct decimal parser available in this fixture. Reuse the existing approved num-bigint version as a dev-dependency (already in the qualified workspace lock), parsing the independent decimal string with BigUint::parse_bytes and passing its bytes to upstream Field construction. This adds one local fixture dependency edge, no new registry package/version, and avoids a handwritten decimal/field codec. The manifest/lock change is explicitly approved after the initial plan; coordinate the shared lock with the concurrent receipt gate before mutation.


## Independent value assertions and local gate receipts — 2026-10-07

Three reviewed conventional GPG/DCO commits:

| Slice | Commit | Verified outcome |
|---|---|---|
| ADR0338/#462 | c921622dac4a7bed9afc5ce44ebf8e514ba714ac | Nine concrete assertion weaknesses resolved: independent wide Field values, exact inserted composite keys/sum78 and cardinality1, source-defined initial defaults before writes.8tests across4packages, strictClippy/format pass. |
| ADR0340/#464 | c8406a4dd74d3bee1cfd77b9337c14891621f7ab | Nine chunked read outputs independently anchored;25helper calls supply typed expected values.3package tests, strictClippy/format pass. ExactADR0335negative guards and all previous TS/state/replay checks retained. |
| ADR0339/#463 | 22ef5f23e115c3ad070ea1c8cd060a1b4fcafee8 | Reusable gate observes tool/config/environment inputs, hashes logs/stdout on success/failure, and retains end snapshots/refuses lock/config drift.19focused Python tests and3real generated behavior tests pass. |

Root reviewed all diffs and source/receipt bindings. The only dependency change is one fixture dev-edge to already-locked num-bigint; existing registry packages and versions are unchanged. No Compact/generated/compiler Rust/runtime source changes. The pure-family report identified9weaker assertions, not9production failures; the collection review identified9read-return weaknesses, not missing execution. The initial ADR0339candidate lacked end-of-run identity checks; root required the amendment, preserving both candidates and logs. Its197→198fixture-count correction belongs to previously deliveredADR0334registration.

### Evidence and scope

[ADR0338-0340 — Expected values and reproducible local receipts.zip](references-0.3.0.md#note-116) contains 113 hashed entries; SHA256 `4dbcc810683ade8bcc6c27a4a80bda21910177b585bec1d94749434b36201bf0`. Includes frozen pure/collection reviews,17exception joins and4historical proof-route joins, both receipt implementations and local execution evidence, exact patches and commit bindings. Large binary paths/hashes remain separately identified.

The17exceptions have qualified historical behavior joins; no missing tests were demonstrated. Four proof-runner routes remain historical, with explicitly qualified source/runtime differences and one older log lacking contemporaneous digest. Pure167rowreview:156bounded joins,9weaknesses(nowfixed),2exceptionsresolvedseparately. Collection185rowreview: concrete invocation/assertion routes and9return-valueweaknesses(nowfixed). Remaining family review is in progress; these counts are not new test counts or full semantic assurance.

Latest fresh coverage remains the exact31a02ebe production cohort: changed8,715/9,163=95.11%;198fresh renders. Subsequent changes here are test/receipt code. Exact31a02ebe bounded Linux CI passed; later branch run is separate. No new proof/network or branch-coverage claim. StoppedADR0285remainsunaccepted.
