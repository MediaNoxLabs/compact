---
id: RUST-ADR-0340
alias: ADR-0340
source_sha256: c038e4b8a98294b76c830596456dc5e7278f2750413b2861b4d3b750254a3d6f
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0340 — Assert source-defined returns in chunked ledger tests

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for test-only implementation. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0340 — Assert source-defined returns in chunked ledger tests

Date: 2026-10-07
Status: accepted for test-only implementation
Parent: R03007 / #351

### Problem
Nine chunked Cell/List/Map read exports execute in existing native/recorded/replay tests, but their return values only compare two Rust paths. TS state, gas and VM-shape comparisons remain meaningful. An independent expected return is missing; this is a test-strength finding, not a demonstrated production defect.

### Expected values

| Fixture | Export | Source-defined value for selected input |
|---|---|---|
| chunked-cell | `get_active` | `true` |
| chunked-cell | `get_amount` | `Field::from(3_u64)` |
| chunked-list | `item_count` | `BoundedUint::<{ u64::MAX as u128 }>::new(1).unwrap()` |
| chunked-list | `items_empty` | `false` |
| chunked-list | `first_item` | `types::Maybe { is_some: true, value: Field::from(1_u64) }` |
| chunked-map | `has` | `true` |
| chunked-map | `get` | `Field::from(1_u64)` |
| chunked-map | `table_size` | `BoundedUint::<{ u64::MAX as u128 }>::new(1).unwrap()` |
| chunked-map | `table_is_empty` | `false` |

### Before / after
```rust
assert_eq!(recorded.execution.result, native.result);
```
Keep this comparison and add an independently supplied typed expected result for each invocation. Either pass the expected value through the existing helper (including Unit for existing mutation cases), or assert at the selected call sites before their results are consumed. Prefer the simplest readable structure. Maybe<Field> must assert both presence and Field1; the old TS capture's `[object Object]` text is not an independent typed value. The source and selected constructor state establish that expectation.

### Ownership and tests
Only existing tests under chunked-cell, chunked-list and chunked-map. No Compact/generated/runtime/capture changes; retain ADR0335 exact assertion failures. Run these three packages with applicable features and strict scoped Clippy/format. Coordinate target/lock leases with adjacent slices. Preserve exactsource/log hashes and named invocation/assertion joins. No unrelated proof reruns or new coverage percentage.

### Boundary
Ordinary returned-value semantics only; no observation trust/cross-instance investigation or admission-policy changes. Keep prior read-only evidence frozen and add delivery amendments. Parent semantic/security completeness remains separately reviewed.

### Evidence
/tmp/rust030-collection-assertion-joins/result-anchor-gaps.json joins each selected source declaration, existing test line and capture case. The full read-only collection report is separate.


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
