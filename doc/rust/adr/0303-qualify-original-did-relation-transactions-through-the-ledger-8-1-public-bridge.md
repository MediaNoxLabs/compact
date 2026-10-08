---
id: RUST-ADR-0303
alias: ADR-0303
source_sha256: 82ccc9886236231d62702435c95965ef5f2419ca7a9c25464eb0ce402c0948e1
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0303 — Qualify original DID relation transactions through the ledger 8.1 public bridge

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted bounded compatibility slice, pending implementation. Parents R030-09/#353 and R030-16/#360. Original proposal: [R03016 — Original DID relation ledger8.1 bridge proposal — 2026-10-07](references-0.3.0.md#note-164). Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0303 — Qualify original DID relation transactions through the ledger 8.1 public bridge

Status: accepted bounded compatibility slice, pending implementation. Parents R030-09/#353 and R030-16/#360. Original proposal: [R03016 — Original DID relation ledger8.1 bridge proposal — 2026-10-07](references-0.3.0.md#note-164).

### Problem

ADR0295 supplies the final original DID export and strict native relation proofs, but its saved results lack serialized per-call public transactions and pre-ledgers. Those summaries cannot establish downstream ledger 8.1 compatibility. ADR0290/0300 already cover the other eleven exports with separate historical snapshots.

### Decision and engineer-facing before/after

Before:
```text
setVerificationMethodRelation -> native strict proof/apply receipt
ledger 8.1 consumer -> no concrete public transaction carrier for this export
```
After:
```text
original DID call -> proof -> exact public pre-ledger + transaction + expected post-ledger
ledger 8.1 consumer -> strict verify/apply -> full ledger byte equality
                     -> same-time replay refusal and unchanged state
```

Extend the existing optional public exporter with an exact reviewed allowlist for the original 16-call relation chain and three-call Schnorr signing chain. Select only eleven positive relation rows for ledger 8.1 qualification: all five insert/remove pairs and absent-generic-JWK Schnorr Authentication insertion. Eight setup rows remain separately labeled setup. Preserve existing digest immutability checks and old allowlist tests. No public export of private oracle state, seeds, witness responses or proof preimages.

If another final-source producer qualification is needed, attach capture to it. If emitted-byte identity makes such a rerun unnecessary, root explicitly authorizes one bounded 19-call producer run to create the missing concrete carriers. Its purpose is new public-wire evidence; do not pretend that historical transactions were reused. Reuse existing unchanged keys and pinned original TS source/oracle. Preserve real constructor data deployment and exact captured prestate guards; do not introduce synthetic oracle-prestate deployments or weaken state-rewrite checks. Constructor execution remains unproved.

### Emitter/runtime and component changes

No emitter/runtime semantics or native ledger pin changes. The proof-smoke public interchange component owns tuple admission and capture metadata. Reuse the existing coherent ledger 8.1 receiver, six JS carrier roundtrips and two malformed refusals per carrier. Coordinate source ownership with ADR0295; do not build against mutable candidate paths. Final receipts must hash accepted source, compiler/proof runner, keys, oracles and public carrier artifacts.

### Acceptance and limits

- Exact complete eleven-row receiver selection; duplicates, extras, missing rows and any producer/receiver/JS error fail aggregation.
- Eleven independent ledger 8.1 snapshots, full ledger-byte result equality, strict verification/apply, changed-binding rejection where supplied by producer, same-time replay rejection without mutation.
- 66 JS roundtrips and 132 malformed refusals. Record producer initial, transported pre-ledger and receiver default parameter identities without rewriting them.
- Retain native/recorded absent-JWK KeyAgreement refusal and witness-order/unchanged-state evidence. It has no prepared transaction and is not an 8.1 proof refusal.
- Test exporter absent-environment behavior, exact tuple refusal and compatibility with existing digest rows. Preserve selected/nonselected Set invariants from the unchanged TS after-state checks.
- Distinguish the two native sequential chains from independent 8.1 snapshots. Historical 14+2 bridge rows are not freshly rerun. No new keys, native dependency promotion, broad full-workspace gate, remote CI, push or publication.

This can finish bounded public-wire compatibility for the twelfth original DID export. Full release migration, audit, coverage, performance and final qualification remain separate obligations. Planning/evidence stay in midnight until milestone closeout.

Issue: https://github.com/MediaNoxLabs/compact/issues/427


### Signed joined delivery — 2026-10-07

Release migration and standalone lock: `b6fcb06cec7e8913e91faae167926805c4872ffb`. Relation compiler, maintained reducers and public exporter: `329bf1bc80441125d2800fc9f1dae8b570da5dac`. Both commits are conventional, GPG verified and DCO signed.

The joined source passes 427 backend tests, 43 original DID test methods (including the 42-row relation table), three reducer methods, nine exporter tests, 99 Python checks, strict Clippy, whole-workspace formatting and all 196 generated-fixture freshness checks. Actual Rust 1.88 passes both workspace and freshly isolated standalone backend checks. The unchanged original DID source/import hashes remain pinned to v0.7.0.

The maintained relation gate passes 19 original calls and six reducer calls under default ledger strictness, with changed-binding rejection and replay refusal. All 512 recorded source hashes still match the signed delivery; the receipt records the pre-commit HEAD and explicitly binds unchanged source bytes to the final commits. Existing keys were verified and reused. Original relation TS outcomes match 23 successes and 19 refusals, including ordered programs, per-query/total gas, witnesses, state and ContractLab rollback.

The separate ledger 8.1 receiver passes eleven independent relation snapshots with full-ledger byte equality and unchanged replay refusal. JavaScript passes 66 carrier roundtrips and 132 malformed-input refusals. Eight original setup calls are retained separately. Earlier 14+2 public snapshots are historical evidence, not a fresh 27-row run. Native pins remain ledger 8.0.3; constructor data is deployed but constructor execution is not proved. No live network acceptance is claimed.

[ADR0293-0295-0303-0304 — Signed joined delivery.zip](references-0.3.0.md#note-076) contains 3081 verified entries; SHA256 `9062cfe7afcf8c15f77cb8674ea75cfcb0865a37898cdc31f45e0fb8c6b7c748`. Tool executables, parameter files and reused keys retain original paths/hashes and are not duplicated into this archive. It includes joined source, proof receipts/logs, public carriers/receiver results, release migration evidence and standalone-lock validation. Failed intermediate attempts remain alongside corrected passing runs.

These four child deliveries are complete. Parent evidence reconciliation, final coverage/performance, audit and release qualification remain separate obligations. Accepted parents remain 6/20 pending that reconciliation. No push, registry publication or remote CI. The user-owned ledger document remains byte-identical.
