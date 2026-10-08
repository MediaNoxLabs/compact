---
id: RUST-ADR-0328
alias: ADR-0328
source_sha256: b9bf5ac1c999bcc7832313c62eebe220bc345f61eba740ba37a06c5d7118689e
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0328 — Exercise regression fixture helpers through public Rust APIs

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** planned bounded test slice; 2026-10-07; R03007/#351, milestone rust-backend-v0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0328 — Exercise regression fixture helpers through their public Rust APIs

Status: planned bounded test slice; 2026-10-07; R03007/#351, milestone rust-backend-v0.3.0.

### Problem
The complete197fixture render census exposed a distinction between owning a test package and directly invoking every public helper. Existing selected adoption/oracle rows cover53source roots; other maintained regression roots also need explicit roles and execution evidence. Preliminary review identifies small exported helpers/constructors used only indirectly, or not directly exercised, despite their generated API being public.

### Decision
Add focused typed calls for confirmed small regression-fixture gaps. Assert actual source-defined results/errors and relevant state/private-output transitions; reuse existing independent TS captures where they cover the same claim. Distinguish direct source-semantic controls from independent TS parity, VM replay and proof acceptance. Do not change generated code, sources, compiler/runtime behavior or ABI. Keep any historical passport-dogfood scope decision separate; current adopted75exports cannot silently qualify a different old source closure.

### Before / after
```rust
// Before: only an outer exported circuit exercises a helper transitively.
let result = save(context, value)?;
// After: retain the outer scenario and check the helper's own public API.
assert!(pure_circuits::require_positive(Field::from(7_u64)).is_ok());
assert!(matches!(pure_circuits::require_positive(Field::from(0_u64)),
                Err(CompactError::AssertionFailed(_))));
```
Actual expected errors and state values must be grounded in the maintained Compact source/captures. Avoid cloning implementation expressions into tests as their own oracle. Names such as require_positive are not stronger than the actual source condition.

### Ownership and gates
Cohesive tests in the already-owned fixture test files or separate direct_exports integration modules. Parallel agents may edit disjoint test packages; root serializes the shared warm-target focused test invocation. Preserve user-owned doc/ledger-adt.mdx. Record the exact resolved export list, source/generated/test hashes, test result and evidence dimension. Run focused package/integration targets, scoped strict Clippy and formatting; expand only for a concrete failure. No broad proof rerun for test-only additions. Root independent review and conventional signed GPG/DCO commit before closure.

This slice does not resolve every required semantic/security obligation or stopped ADR0285, and does not alone close#351.


### 2026-10-07 — Sixteen regression export gaps closed (ADR0328/#453)

Signed GPG/DCO `a162144f7ee253e5f3cd270f4460e3e37526beb2` adds15test methods covering16direct public APIs across11fixtures. Controls cover sourceguard, counter/Cell/read operations, close success/refusal/witness ordering, actual crypto constructor, public keys/color predicates, Merklepath perturbations, alive predicate and denomination constant. Retained TS states are reused where precise; source-semantic and changed-input checks are labeled separately.

Root11package default-feature test gate passes43tests (96s); strict all-target/all-feature Clippy and formatting pass. Independent review found no blocking findings;1811frozen tracked/test identities unchanged during checks. Generated source, Compact source, compiler/runtime/manifests/ABI unchanged. #453 closes only this slice. The48oldpassport scope decision, broader execution/semantic joins and stoppedboundary remain distinct. No freshproof/TS-generation/network claim.

Archive [ADR0328 — Direct regression export delivery.zip](references-0.3.0.md#note-112) SHA256 `689909fbc864183b01b9f64ef643dc52a2f75cf72f188435e83489643f41f0e5`. CurrentlocalHEADa162144f; remotebf7c719b remains its separately qualified boundedLinuxCIcheckpoint. Parentcount11/19unchanged. Userledgerdocpreserved.
