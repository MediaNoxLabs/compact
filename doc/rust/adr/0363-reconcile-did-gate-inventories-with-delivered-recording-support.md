---
id: RUST-ADR-0363
alias: ADR-0363
source_sha256: 6b47f2a07ff081d5bfffa11695ac16261fdcfe618af34b1051417f887796419c
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0363 — Reconcile DID gate inventories with delivered recording support

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted, implementation in progress. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.

**Current bounded disposition:** the dated local-delivery amendment below records signed delivery at `57fab7755097380b90619672be5150746f9d4a49`. The original opening status remains historical. This corrects qualification-harness/source-anchor expectations; it does not establish successful full candidate qualification or broaden conformance/proof acceptance. Parent #364 remains open.

## Original decision and amendments

## ADR0363 — Reconcile DID gate inventories with delivered recording support

Date: 2026-10-08
Status: accepted, implementation in progress
Parent: #364

### Problem

Final qualification at679be285 stops on two stale recording-gap expectations in the typed Unit source manifest. The original DID digest recording was delivered at8a52a010 (ADR0288/#413) and relation recording at329bf1bc (ADR0295/#427). The current generated fixture and capability report expose all12 original recorded exports, but the older manifest still expects10 and two unsupported diagnostics. An adjacent proof gate compares the total recording inventory to its11-key proof subset, conflating capability availability with proof execution scope.

### Decision

Require the delivered12-entry recording inventory in the source gate. Remove exactly the two superseded unsupported-gap expectations and add explicit positive recording assertions. Keep source hashes, fixture byte equality, TS compilation, compiler proof flags and Cargo checks. In the DID proof gate, assert the full recording inventory separately from the retained11-key proof cohort; keep actual key generation, witness cases, proof/apply/replay operations and strict refusal behavior unchanged. Relation proof coverage has a separate accepted gate and remains separately reported.

### Before / after

```python
# Before: historical capability gaps outlive their implementation.
expected_recording_gaps = {
    "verifySchnorrJubjubDigestSignature": old_digest_gap,
    "setVerificationMethodRelation": old_relation_gap,
}
assert recorded_exports == set(KEYS)  # KEYS is a proof subset
```

```python
# After: independent assertions for availability and exercised proofs.
expected_recording_gaps = {}
assert recorded_exports == EXPECTED_RECORDED_EXPORTS  # twelve
assert set(KEYS) <= recorded_exports                  # bounded proof subset
```

### Emitter/runtime/domain changes

None. This corrects stale qualification metadata and separates recording capability from finite proof evidence. It does not broaden the emitter profile or turn inventory counts into specification conformance. DID v0.7.0 source and native ledger8.0.3 remain pinned; public8.1 interchange is a separate receipt. TS remains the semantic comparison oracle on the documented finite cases.

### Evidence and verification

The stopped679be285 full gate is retained. Compare current generated DID bytes to the maintained fixture, require all12 capability exports, preserve compiler pure/proof flags, and rerun the bounded source manifest check. Inspect other full-gate source manifests together before another candidate, to avoid repeated whole-gate prefixes. Test the capability/proof cohort distinction without expensive proof generation; final full qualification still must run the actual maintained proof gates.

The retained joined relation receipt4d2730fd reports19 original plus6 reducer proof/apply/replay cases; it is historical evidence supporting the inventory correction, not a fresh candidate proof run. No milestone acceptance follows from this correction alone.

Tracking: [#500](https://github.com/MediaNoxLabs/compact/issues/500), rust-backend-v0.3.0.

### Local delivery

Signed/DCO57fab7755097380b90619672be5150746f9d4a49 verifies Good GPG and is pushed. [ADR0363-0364 — Candidate gate baseline receipt](references-0.3.0.md#note-139) retains successful scoped checks, original failures and historical/current source separation. Parent #364 remains open for full successor qualification.
