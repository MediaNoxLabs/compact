---
id: RUST-ADR-0307
alias: ADR-0307
source_sha256: 62807d37ea1762ed97aba09def8ff9567f94ae549ab063d4607484303b180a97
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0307 — Test resource accounting across typed IR carriers

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered-locally. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: delivered-locally
date: 2026-10-07
parent: R030-07
milestone: "0.3.0"
```

## ADR0307 — Test resource accounting across typed IR carriers

### Problem

Fresh329bf1bc coverage passes623 tests but backend conservative changed-line coverage is92.51%, below the proposed95% floor. Coverage review identified unexecuted payload paths in resource_limits/traversal.rs: constructor assertion text, typed state returns, collection/history actions and crypto expression operands. Exhaustive enum matching alone does not prove that each payload contributes to the finite resource budget. Existing resource tests cover global counters/depth/calls but not these distinct carriers.

This is a missing-test finding, not a demonstrated resource-bypass defect. Preserve the conservative changed-line denominator, including mechanical moves. Do not manufacture impossible dispatcher states or remove mandatory primitive paths to improve a percentage.

### Before / after

```text
Before: a long message or nested crypto operand has generic budget coverage
        but some constructor/return/action carrier paths have no focused case.
After:  the same bounded payload is placed in each reviewed real carrier;
        exact-boundary input passes, the independently counted excess fails
        with the expected typed ResourceLimit and original diagnostic fields.
```

Add adjacent meaningful tests in resource_limits/tests.rs. Prefer independently counted short payloads and existing supported IR constructors. A UTF-8 assertion checks bytes rather than Unicode scalar count; a nested operand checks that moving it into a crypto/return/action carrier does not omit accounting. Use exact boundary and one-over controls, not merely positive counters or a loop over every implementation variant.

### Accepted scope

1. Constructor Assert message: short multibyte UTF-8 input with independently known byte length and exact/one-over string budget.
2. Representative nested crypto children: commitment value/opening, EC point/scalar and contract-address input to the existing owned-coin nonce primitive, only where constructible under existing typed IR. Isolate the nested operand responsible for the limit; keep a smaller valid control.
3. Representative StateReturn Set/Map and List/history action payloads: include declared field identity and expression payload in the expected finite budget. Reuse real existing source/IR shapes; do not invent supported source behavior from an internal enum alone.
4. Carry one genuine supported-shape resource failure through all three public render entry points; assert structured resource/limit/observed and the useful Display content. Existing semantic diagnostics retain ownership.

No production emitter/runtime, resource threshold, ABI, schema or dependency changes. If a test reveals an actual omitted payload, retain a minimal failing case and request a further root decision before repairing production. Do not change tests to bless the current result. Tests may use the private census to isolate accounting; any public rendering claim requires a valid supported positive shape.

### Validation and limits

Run the focused resource library cohort on ordinary workers, scoped formatting and strict library Clippy. Reuse current warm targets; broaden only for a concrete integration need. Later instrument the accepted tests with the same frozen methodology; no promised coverage increase before measurement. Constructor/prover/rustc bounds and branch/security assurance remain distinct from compiler-owned finite budgets. Avoid the stopped consumer/cross-instance security investigations.

Record exact source/gates and meaningful misses. Full R030-07 and resource/adversarial parent acceptance remain open. Work stays local; no push/remote CI. Root integrates and signs after review.

### Tracking

https://github.com/MediaNoxLabs/compact/issues/431; parent#351, resource policy#415.


### 2026-10-07 — ADR0307 resource carrier tests delivered

Signed/DCO commit `ec10f324124ccca16cc510a12ccf8c1398496488` delivers five meaningful resource-accounting tests. All23 focused resource tests pass, strict library/test Clippy and scoped formatting pass. Root reviewed the diff and verified the signature; no production code, thresholds, dependencies or ABI changed. No omitted payload defect was found. Optional owned-coin nonce input was unavailable in this IR and was not invented. Coverage measurement remains separate.

Evidence: [ADR0307 — Resource payload tests.zip](references-0.3.0.md#note-091), SHA256 `3252faa9a0923eab6b1cb64273235010e91edb70b855509550eb99667ed53b72`. Issue https://github.com/MediaNoxLabs/compact/issues/431 closes locally delivered; parent#351 stays open. Milestone acceptance remains8/20. A follow-up instrumented23-test cohort will report explicit mixed-cohort provenance rather than claim a fresh whole-tree run.
