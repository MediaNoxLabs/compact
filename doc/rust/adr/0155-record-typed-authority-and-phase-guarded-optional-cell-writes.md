---
id: RUST-ADR-0155
alias: ADR-0155
title: "Record typed authority and phase guarded optional Cell writes"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "authorization", "optional-cell", "original-contracts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e5f14dd440498704f114a51bda983d52d7388e076b5aa80e2ffa1489fae078dc
---
# RUST-ADR-0155 — Record typed authority and phase guarded optional Cell writes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted a name-independent closed authority witness/hash and phase guard before optional topic Cell writes. Empty, nonempty and Unicode behavior is captured and a Unicode proof is applied. Original Election has no authority initializer; the oracle constructor is test provisioning, not production authorization setup. Preserve the corrected actual generated facade syntax.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#259 closure](https://github.com/MediaNoxLabs/compact/issues/259#issuecomment-6017668815). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`f54f1292`](https://github.com/MediaNoxLabs/compact/commit/f54f12927f3ee1517b185958acb2243c9468cfb7). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
The original election set_topic has native Rust execution but no proof-ready recorded call. Authorization reads a Bytes32 witness, computes a domain-separated persistent hash, checks an authority Cell and enum phase Cell, then writes Maybe<OpaqueString>.

### Before
```rust
ledger_contract::set_topic(context, &witnesses, topic)?;
// No recorded::set_topic / observed call available.
```

### After (proposed)
```rust
recorded::set_topic(context, &witnesses, topic)?;
recorded::Contract::new(witnesses).set_topic_call(&observed, private, topic)?;
```

### Decision
Admit a closed structural shape using exact types, formal bindings, ledger declarations and indexes. Preserve witness/hash/read/assert/write order. Match no contract, circuit, field, enum or struct names. Reuse ledger8 Cell recording and existing witness/private transcript machinery; no IR schema or runtime ABI change.

### Emitter/runtime changes
Emitter inspects the pure hash body and typed guarded write. Runtime primitives remain unchanged unless independent TS evidence demonstrates a mismatch.

### Evidence required
Fresh original TypeScript empty, nonempty and Unicode topics; native/recorded state, ordered VM, all gas dimensions, private outputs, replay; wrong authority and phase failures. Pinned ZKIR proof verify and ledger apply. Negative renderer guards, fixture freshness, focused gate and Clippy.

### Limits
advance requires Maybe projection and enum successor; add_voter needs witness path checks and Merkle append; voting flows are separate. No broader implicit opaque support.


### Delivery — 2026-10-05
Issue: https://github.com/MediaNoxLabs/compact/issues/259 (rust-backend-v2). Signed local commit: `366bedf5a038293cf4e64fdbdf55d21214eb9ff2` (`G`, DCO), based on `f54f1292`.

The proposed shape is now implemented in `closed_authorized_optional_write_steps`. It matches the witness and pure helper signature/body, exact authority/phase/write declarations and indexes, lexical parameter provenance, enum variant, and Boolean/string struct layout. Renaming circuit, witness, helper, fields, enum, variants and struct members stays supported. Changed hash provenance, reordered assertions and incorrect slot indexes fail closed. No runtime, ABI, schema or Scheme change.

#### Delivered Rust API
```rust
let generated = ledger_contract::Contract::from(witnesses);
let recorded = generated.recording().set_topic(context, topic.clone())?;
let call = generated.recording().set_topic_call(&observed, private, topic)?;
```
This is the actual facade syntax (the earlier proposal used illustrative `recorded::Contract::new`).

#### Independent evidence
Fresh TS capture uses original `election_oracle.compact`; it differs from `examples/election.compact` only by authority-seeding constructor. Both sources now report set_topic recorded + observed-call; 1/5 each. Empty string, hello and Unicode 議題 🗳️ agree on native/recorded state, query sequence, all four gas dimensions, private transcript atoms/alignment, private state 10→11, and replay. Wrong authority and post-setup phase reject with the original TS messages.

Each call has 3 ledger queries / 9 VM operations and 1 private witness output. ReadTime=425000000, computeTime=3733773148, bytesDeleted=518; bytesWritten=518/528/546 respectively. TS exposes the final query gas on CircuitResults here; tests sum independent captured query costs and also compare concatenated replay gas. TS unknown popeq results are normalized only for comparison; ledger replay retains concrete recorded values.

Pinned ZKIR 2.1.0 compiled set_topic at k=13 / 4184 rows. The Unicode topic proof replayed/partitioned, verified, and deployed/applied through ledger8; typed observed-call preparation equals manual preparation and resulting Maybe topic equals native state. Artifact `${LOCAL_EVIDENCE}/compact-adr155-rust`; selector `--election-topic`. The focused proof generated keys only for set_topic from the unmodified original source output; the broad consumer/proof harness includes full election compilation and this selector.

151 generated fixtures fresh; 125 renderer tests pass; 2 election tests pass; targeted all-target/all-feature backend/election/proof Clippy passes.

#### Remaining boundary
No claim of all-election recording: advance, add_voter, vote$commit and vote$reveal remain unavailable. Original examples/election.compact has no authority initializer; the oracle constructor is test setup only. Production authorization still depends on application provisioning/witnesses.


Focused exact-head gate passed at `366bedf5`: `${LOCAL_EVIDENCE}/compact-focused-366bedf5-election/receipt.json`, one fixture / 1 of 5 recorded, Rust 1.99.0. No push or remote CI.
