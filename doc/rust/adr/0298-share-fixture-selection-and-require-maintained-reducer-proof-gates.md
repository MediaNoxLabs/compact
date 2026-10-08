---
id: RUST-ADR-0298
alias: ADR-0298
source_sha256: d7cb91a68e918edb4bf59a4a7deb5ad6720a0ec78a70d4bd5d05c110231b2239
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0298 — Share fixture selection and require maintained reducer proof gates

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for local implementation. Parent R030-19/#363. Evidence baseline40047fb8. No Rust emitter/runtime or contract semantics change. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0298 — Share fixture selection and require maintained reducer proof gates

Status: accepted for local implementation. Parent R030-19/#363. Evidence baseline40047fb8. No Rust emitter/runtime or contract semantics change.

### Problem and observed evidence

The freshness checker registers193 source/fixture pairs while local_parity_gate registers182. The difference is the pinned vc-passport consumer closure, the digest read reducer and nine maintained primitive reducers. The full gate runs workspace tests, so this does not mean those behavior tests never ran. It means its generated-source freshness inventory and focused selection omit11 registered fixtures. Dedicated digest and primitive reducer proof gates passed separately under ADR0288/0294 but are absent from the full orchestration.

### Before and after

Before:
```python
# Independently maintained maps in two tools.
local_parity_gate.EXTRA = {...}     #182 total
check_fixture_outputs.EXTRA_SOURCES = {...}  #193 total
output = directory / "compiled" / source.stem
```
After:
```python
# One source/fixture mapping, reused by both entry points.
selected = fixture_inventory.fixture_map()
# Preserve distinct paths for same-stem imported contract entrypoints.
output = directory / "compiled" / source.relative_to(ROOT).with_suffix("")
```

Use a small fixture_inventory module with root-relative special paths and the existing top-level source discovery. Retain compatibility aliases used by historical evidence tools where cheap. Validate source/fixture uniqueness and existence. Include registered sources in the declaration inventory; review baseline additions and require no old declaration removals. Preserve named CLI source selection; full relative paths disambiguate duplicate basenames. Do not silently broaden --only basename selection beyond its existing documented semantics.

### Full proof orchestration

After the original DID lifecycle gate, invoke the existing digest and primitive reducer standalone gates with the same frozen compiler/Scheme and reusable Cargo target. No --only reducer selector in full mode. Require each expected child receipt to exist, parse, have its exact format and passed status, then hash it into the parent receipt. Failure/nonzero/missing/invalid receipts fail the parent. A full gate does not gain a skip option. Focused source mode continues to avoid proofs. Keep the existing dedicated gate ownership and independent assertions.

### Component changes and tests

Change only fixture registration, declaration baseline, gate orchestration and their regression tests. No compiler/runtime/ABI/schema change. Test shared193 mapping, newly selectable passport/reducers, deterministic distinct output paths for duplicate source stems, registered source inclusion without declaration loss, and full orchestration success/failure/missing/failed receipts. Exercise focused real compiler/freshness for all11 previously omitted sources using current frozen compiler; existing Rust behavior/proof evidence remains separately referenced. Run Python harness suite locally. No redundant full proof repeat is needed to validate orchestration; final full milestone qualification will execute the mandatory children together and must not be claimed now.

### Constraints and delivery

Record baseline identity additions explicitly (an inventory is not semantic parity). Keep Obsidian as planning source of truth. Conventional GPG+DCO commit, no push or remoteCI. Parent remains open. ADR0295 may add relation reducers later; its owner must extend the shared registry and full proof obligations when qualified. This decision does not accept DID or the pipeline parent by itself.

Issue: https://github.com/MediaNoxLabs/compact/issues/422

### Local delivery

ADR0298 / #422 delivered at `1b6c5d02207c08a9426b65fdce84c1b14825ae73`, conventional GPG+DCO verified. One shared fixture registry replaces independently maintained maps. The local gate now includes all 193 fixtures; same-stem contract entrypoints have distinct artifact directories. The declaration inventory adds 48 identities (1097→1145), removes none, and contains 234 sources. This is inventory coverage, not a claim of complete semantic parity.

The full gate now requires all three existing standalone proof runners: original DID lifecycles, digest reducer and nine primitive reducers. Failed commands, absent/malformed/wrong-format/failed receipts and changed earlier receipts fail orchestration. Focused mode remains bounded. Actual fresh generation of all 11 previously omitted fixtures passed (21/22 proof-required recorded; the reducer remove control remains explicitly unavailable). All 81 Python harness methods passed, including required-child failures and artifact-path isolation. A second agent independently checked all 29 special mappings, 193 distinct output roots and the five new focused test methods.

The full parent orchestration uses simulated commands in these regression tests. Real proof evidence remains ADR0288/0294; no new proof run, broad full-gate pass, runtime change, push or remote CI is claimed. Final joined qualification will execute all mandatory proof runners. R030-19 remains open; accepted parents remain 6/20.

Receipt SHA256 `b9a2cad9bbb2de69f617d556494a6c8bc4a5ae2e2278fff2ee82537041a8d04f`. [ADR0298 — Shared fixture and proof gate evidence.zip](references-0.3.0.md#note-084) has 37 entries; SHA256 `7141b5471f02634e6986581c8875778f59918d41485e2bb1b8b4168083ffbfa3`. Signed source content and every retained artifact digest were checked independently by root.
