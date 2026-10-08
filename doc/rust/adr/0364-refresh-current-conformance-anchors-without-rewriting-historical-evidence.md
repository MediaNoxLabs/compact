---
id: RUST-ADR-0364
alias: ADR-0364
source_sha256: 666556c8be7757e709a61299d1809ad1b1952df717280551ce6d8dfac2ea9778
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0364 — Refresh current conformance anchors without rewriting historical evidence

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted, implementation in progress. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.

**Current bounded disposition:** the dated local-delivery amendment below records signed delivery at `57fab7755097380b90619672be5150746f9d4a49`. The original opening status remains historical. This corrects qualification-harness/source-anchor expectations; it does not establish successful full candidate qualification or broaden conformance/proof acceptance. Parent #364 remains open.

## Original decision and amendments

## ADR0364 — Refresh current conformance anchors without rewriting historical evidence

Date: 2026-10-08
Status: accepted, implementation in progress
Parent: #364

### Problem statement

The broader Python harness runs149tests and fails one repository-baseline check. Workspace and frozen679be285 each report49 identical hash/anchor errors over five files: the current lock, rejection harness and three backend source files. Reviewed accepted slices added wide-bound diagnostics (ADR0352), optional ProofLab dependency edges (ADR0360) and the output-recovery harness fix (ADR0362). Grammar, classification, version, reference headings, formal inputs and semantic support values have not drifted. Unrelated working-tree documentation is outside this baseline.

The registry currently uses live repository anchors inside historical execution evidence. Blindly refreshing those hashes would falsely associate old logs with new sources/locks.

### Decision

Refresh only current source identities and corresponding anchors in equal source blocks. Preserve original executed source identities using the existing historical artifact-location representation, and retain current source guards separately. Record the anchor checkpoint separately from the original grammar extraction and executed evidence checkpoints. Preserve old logs, evidence strength, support statuses, formal gaps and requirement mappings exactly. Keep the checker and its drift refusals unchanged.

### Before / after

```json
{"path": "Cargo.lock", "sha256": "OLD_EXECUTED_HASH", "line": 1}
```

```json
{
  "artifact_location": "git:ORIGINAL_COMMIT:Cargo.lock",
  "sha256": "OLD_EXECUTED_HASH",
  "qualification": "Historical execution input, not the current lock"
}
```

The current top-level source guard still binds Cargo.lock to its actual new hash. Current requirement/test anchors retain path/line/hash and therefore remain subject to drift checks. Historical provenance is explicit rather than exempting current sources.

### Emitter/runtime/domain impact

No production change, dependency resolution change, schema change or conformance-status promotion. This is registry maintenance after accepted source changes. The historical Boolean native/source cases retain their actual execution qualifications; source continuity is not a new execution or formal theorem.

### Verification and timebox

Use the existing reviewed five-file delta and44equal-block anchor relocations. Assert before/after equality of grammar inventory/classifications, semantic support statuses, requirements/evidence identities and formal artifacts/gaps. Run existing baseline mutation/refusal tests and all Python harness tests. A focused regression preserves historical artifact hashes while current anchors still detect drift. No Agda work, new requirement research, fresh broad extraction or conformance campaign; maintenance is bounded to this observed failure.

### Bounded current execution follow-up

Tracking [#501](https://github.com/MediaNoxLabs/compact/issues/501). Five existing Boolean native conformance tests pass on the current locked graph (4.91s compile,0.02s tests). This new execution receives separate provenance; old ADR0348/0349 evidence hashes remain associated with their original sources. No new cases, extraction, TS rerun, recording, proof or formal claim. Current source-anchor maintenance and Python negative tests are in progress.

### Local delivery

Signed/DCO57fab7755097380b90619672be5150746f9d4a49 verifies Good GPG and is pushed. [ADR0363-0364 — Candidate gate baseline receipt](references-0.3.0.md#note-139) retains successful scoped checks, original failures and historical/current source separation. Parent #364 remains open for full successor qualification.
