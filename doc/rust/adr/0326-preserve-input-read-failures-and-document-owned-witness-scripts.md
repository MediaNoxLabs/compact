---
id: RUST-ADR-0326
alias: ADR-0326
source_sha256: 19b6c826266fac340bb83024618e7e3710b3263ce41713c32bb6af2f53549937
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0326 — Preserve input read failures and document owned witness scripts

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** planned · 2026-10-07 · R03007/#351 and R03008/#352 · milestone0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0326 — Preserve input read failures and document owned witness scripts

Status: planned · 2026-10-07 · R03007/#351 and R03008/#352 · milestone0.3.0

### Problems
The finite compiler evidence review identified one untested I/O propagation path: a Read implementation may fail after returning a prefix. It must preserve the original error rather than treat partial input as a valid document or replace the error with a size diagnostic. Separately the developer guide requires an executable rustdoc example of the public WitnessScript API.

### Before / after
```text
Before: size/UTF8/parser-depth controls, no injected underlying read failure
After: immediate and after-prefix Read failures return original kind/payload
       through the shared byte reader in all three compiler binaries
```
```rust
// Owned private state can clone a script for independent scenario work.
let mut script = WitnessScript::new([(7u8, Ok(42u64))]);
let checkpoint = script.clone();
assert_eq!(script.answer(7)?, 42);
assert_eq!(checkpoint.remaining(), 1);
```
This example documents owned cloning, not rollback of external/shared effects. Include mismatch behavior with unchanged queue/journal, and explicit payload access only when testing a known failure.

### Ownership and validation
Test-only additions under existing bin_support/input.rs cfg(test), rustdoc-only addition in testkit-rs/src/witness.rs. No runtime implementation, ABI, emitter or output change. Verify original error identity and no partial success, test all three binary owners via focused cargo tests; run only the affected WitnessScript doctest and relevant strict Clippy/formatting. Reuse warmed target. Explicit before/after source and receipt hashes retained. No coverage percentage recalculation needed for an additive test/doc-only slice; old production-line mapping remains a historical measured cohort. Root review and signed conventional DCO commit. Keep stopped ADR0285/#409 untouched.

Delivery issue: https://github.com/MediaNoxLabs/compact/issues/451


### Delivery update — 2026-10-07
Delivered9012a86a; 12focused tests,1doctest and scopedlint/format pass. Issue#451 closes this bounded slice; parent#351/#352 remain open.


### 2026-10-07 — External tutorials validated; input errors and bounded CI delivered

Local disk capacity recovered to110GiB and build-heavy work resumed. Parent progress remains **10/19 required outcomes accepted**, with ACC removed from the original20.

ADR0324/#449: isolated witnessed ContractLab tutorial passes1test; unchanged DID tutorial passes2lifecycle tests. Both pass strictClippy/formatting. All477 staged SDK files match immutable5efa91c2;324 registry identities/checksums per example match reviewed source lock; each metadata graph has one runtime identity and no outside local package. Generated contracts remain unedited. Initial fresh-lock drift was rejected before compilation; corrected lock-seeding recipe and failure evidence retained. These are native/replay and finite DID TS-capture checks, not new proofs/network acceptance. Current-edition guides reflect actual results; parent#352 still needs remaining negative-control mapping, companion corrections and final publication.

ADR0326/#451 delivered in signed GPG/DCO9012a86abfa329b4b4c02896ab1e69ce52fba5f1. Two new injected Read failure cases pass in all three compiler binaries (12 focused tests including existing cases). Original error kind, display and typed payload survive before input/after a prefix; partial bytes do not become success. Public WitnessScript doctest passes and documents owned clone independence and mismatch preservation. Relevant strictClippy/formatting pass; production implementation unchanged. This fills the finite compiler-join I/O gap. Historical95.04% coverage remains its measured source cohort, not a new measurement.

ADR0325/#450 implementation committed as signed GPG/DCOfba6845f37fdbfbfbd49e3a73d896ecff7a70371. New bounded Rust workflow separates four-owner tests/lints/format from Rust1.88 backend/Jubjub/standalone checks; locked graphs, scoped caches and source/check receipts. actionlint1.7.12, ShellCheck, YAML, eight shell-script parses and ten receipt simulations pass. Actual core commands are undergoing local validation; no remote run or current remote green claim. Existing full compiler/proof gates preserved.

ADRs and historical source snapshots remain separate. StoppedADR0285 unchanged. User-owned doc/ledger-adt.mdx preserved. Saved goal stillusageLimited; active user-authorized execution continues.

Evidence archives:
- [ADR0324 — Validated external developer tutorials.zip](references-0.3.0.md#note-107) — SHA256 `3919b99d283f9417eee4267daa90c7c246bba9edc95e446eb446974fa1934203`
- [ADR0326 — Input failure and witness rustdoc checks.zip](references-0.3.0.md#note-110) — SHA256 `00516d3610e63c722e9f57dcebfbee78c496fa1411344f7432c86dc6bfc587aa`
- [ADR0325 — Bounded CI static validation.zip](references-0.3.0.md#note-108) — SHA256 `266d404226c5e117c3039efd0b457fc780d2ea799a190acbf1cc9e893d942eba`
