---
id: RUST-ADR-0290
alias: ADR-0290
source_sha256: d5924da0008b3dfbeb047f711fd470650dab92638cc36f8867dbf2b1940fa5d7
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0290 — Qualify the existing DID effect families across ledger8 profiles

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-bounded-bridge-expansion. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-bounded-bridge-expansion
date: 2026-10-07
milestone: "0.3.0"
parents: [R030-09, R030-16]
issue: https://github.com/MediaNoxLabs/compact/issues/414
```

## ADR-0290 — Qualify the existing DID effect families across ledger8 profiles

### Problem
ADR0289 proves one controller rotation crosses Rust ledger8.0.3→8.1 and JS8.1 wire boundaries. It does not yet qualify the existing recovery, Boolean lifecycle, String Set and three different Map product representations. The existing reviewed DID proof matrix already provides14cases across10exports; a new generic scenario suite is unnecessary.

### Decision
Expand the optional public exporter and isolated receiver to exactly those14unique `(scenario,case,operation)` rows. Retain realproof verification, changed-public-input refusal, default strict full transaction verification/application, byte-identical complete post-ledger, same-time replay refusal/unchanged ledger, and six JS8.1 public roundtrips with malformedbyte refusals per row. Missing, extra, duplicate or failed rows fail the aggregate gate. Original contracts, generated/runtime/compiler code, pins and profile metadata stay unchanged.

| Before | After |
|---|---|
| One root containing rotation public transaction/preledger/postledger/context | New case-scoped public directories for exactly14reviewedrows, plus per-row and aggregate evidence |

```rust
// Before: only rotateControllerKey selected internally.
capture_if_requested(&balanced, &state.ledger, &state.context(), name)?;
// After: fixed producer scenario/case identity accompanies the public carriers.
capture_if_requested(&balanced, &state.ledger, &state.context(), scenario, id, name)?;
```

The bounded expected matrix is points rotate/recover/deactivate; aliases insert-unicode/remove-unicode; services insert-unicode/update-empty-fields/remove-unicode; schnorr-methods insert-unicode/update-point/remove-unicode; jwk-methods insert-unicode/update-jwk/remove-unicode. Operations must equal the existing did_proof_gate.py reviewed table. No digest/relation expansion until their separately approved delivery and inventory.

### Public data and context boundary
Preserve concrete sealed/proven public transaction and public ledger/BlockContext types; no witness, preimage, seed, RNG or wallet private state export. Preserve create-new case directories, explicit absolute-root selection, absent-environment no-op, whitelist refusal, reference-ledger equality guard and actual original-source deployment semantics. Case strings come from the fixed reviewed matrix, never arbitrary contract inputs as paths.

Each receiver case is an independent transported snapshot. TestState post-block updates and any funding history between producer calls are not silently reconstructed as one continuous8.1chain. All checks apply to actual transported prestate parameters/context.

### Parameter diagnostic
Capture producer8.0.3 INITIAL_PARAMETERS, receiver8.1 INITIAL_PARAMETERS and each actual transported pre-ledger parameters. Compare serialized initial identities and report actual dynamic fee-price evolution separately. Upstream post_block_update updates fee_prices; divergence from initial values is not inherently incompatibility. Never replace transported parameters or rebalance/regenerate the transaction to claim acceptance. This does not promote a native8.1 default runtime profile.

### Verification and ownership
Reuse existing ten reviewed keys/ZKIR, ADR0289 exact receiver graph and warm targets; no broad compiler corpus, new keygen, CI or push. Producer ownership only did_public_interchange.rs and minimal lifecycle call-site identity, coordinated with ADR0288. Receiver/driver/JS remain isolated evidence until closeout. Add meaningful aggregate refusal tests for omitted/duplicate/failed rows. Freeze exact input hashes, final receipts and retained first-failure logs. Root reviews before commit and owns dashboard/journal.

This finite extension leaves DID#353/#360 open for remaining exports, final profile/migration/MSRV/consumer/drift/network decisions. No authorization redesign, stopped-scope retry or dependency upgrade is included.

### Local delivery
All14 reviewed rows across ten exports pass the real Rust8.1 proof/strict apply/complete-ledger/replay boundary and six JS8.1 public carrier checks each. Every row preserves transported state/parameters/context. Initial8.0.3 and8.1 parameters serialize identically; all fourteen observed snapshots differ only in fee prices, with three distinct observed parameter hashes. Diagnostic comparison uses a separate clone and never changes the applied ledger. Optional artifact format v2 adds scenario/case identity and initial parameters; runtime ABI and production profile unchanged. Four exporter unit tests, five aggregate refusal tests and producer/receiver strict Clippy pass. Shared source lease was frozen and released to ADR0288 after producer gate; final receipt binds exact source copies/patch and frozen binaries. [ADR0290 — Fourteen DID calls across ledger8 profiles](references-0.3.0.md#note-071). Root reviews before selective commit; digest/relation and parent qualification remain open.
