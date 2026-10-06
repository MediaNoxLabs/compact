---
id: RUST-ADR-0160
alias: ADR-0160
title: "Record witness admitted authorized Merkle insertion"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "authorization", "witness", "merkle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 05b2d610f7a97c2ce0927864d55ca5897709f5cf07e2c103a2a001d7dabcddfd
---
# RUST-ADR-0160 — Record witness admitted authorized Merkle insertion

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the closed optional-path witness admission, authority/phase guards and typed Merkle insertion sequence. Both empty and occupied tree insertions are proved; duplicates stop after the actual path witness. TS inspects a real pre-call tree snapshot because ledger fields are unexported, while Rust uses its local readonly view; no witness gas is normalized away.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#264 closure](https://github.com/MediaNoxLabs/compact/issues/264#issuecomment-6017676817). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
Original election add_voter has native execution only. It checks an optional Merkle-path witness to reject duplicate voters, then uses the authority witness/hash and enum phase guards before inserting a Bytes32 voter into the eligible tree. Both witness outputs and readonly witness context must stay ordered.

### Before / after
```rust
// Before: native only
ledger_contract::add_voter(context, &witnesses, pk)?;
// After: proposed proof-ready facade
let generated = ledger_contract::Contract::from(witnesses);
generated.recording().add_voter(context, pk)?;
generated.recording().add_voter_call(&observed, private, pk)?;
```

### Decision
Reuse the checked authority prefix from ADR0158. Match the closed typed witness optional-path projection/negation, exact public argument provenance, enum phase guard and plain Merkle Bytes32 insertion. Preserve witness admission → authority witness/hash/read/assert → phase read/assert → insertion order. Names are data. Reuse existing ledger8 Merkle and witness recording primitives, no schema/ABI changes.

### Evidence required
Independent TS/native/recorded state, gas, ordered queries and private transcript/replay for empty and occupied tree, using actual readonly ledger lookup in the path witness. Duplicate, wrong authority and wrong phase reject. Pinned proof verified and ledger applied, renderer mutation guards, fixture freshness and Clippy.

### Limits
Voting flows and arbitrary witness/guard shapes remain separate. A caller-provided witness is part of the contract API; admission correctness follows source witness behavior.


### Oracle API boundary
The unmodified election source does not export its ledger fields, so its generated TypeScript WitnessContext.ledger is empty. The TS witness adapter reads the actual pre-call StateValue snapshot using ledger8 StateBoundedMerkleTree.findPathForLeaf; it does not return fixture membership answers. Rust uses the actual WitnessContext.ledger.eligible_voters().find_path_for_leaf. The metered Rust view dereferences to the local MerkleTreeView lookup, which does not call WitnessReadMeter or execute a VM query. Both are local readonly tree inspections. Unmodified aggregate/query/replay gas is compared; no witness cost is normalized away. This is an API visibility difference documented separately from execution equivalence.

### Evidence in progress
Fresh TS/native/recorded/replay cases insert Bytes32 [8;32] into the empty tree, then [9;32] into the occupied tree. Both emit 3 public queries / 16 VM ops and 2 ordered private outputs (optional absent path, authority secret), private state increments twice. Duplicate admission consults the real populated tree and stops after the path witness; authority/phase rejection calls path then secret. State, all four gas dimensions, transcript atoms/alignment, query program and replay effects agree.

Insertion1 readTime=1870000000, computeTime=5730467922, bytesWritten=1520, bytesDeleted=518; insertion2 readTime=1870000000, computeTime=5730467922, bytesWritten=1580, bytesDeleted=1520. No gas normalization.

Both proof cases compiled with pinned ZKIR2.1.0 (k13/6751 rows), replayed/partitioned, verified and ledger-applied. Typed observed-call preparation equals manual preparation; applied tree contains the exact inserted key at the expected next index. Artifacts `${LOCAL_EVIDENCE}/compact-adr160-rust`, selector `--election-add-voter`.


### Local delivery — 2026-10-05
Issue #264: https://github.com/MediaNoxLabs/compact/issues/264 (rust-backend-v2). Signed commit `d3a3a12541f4aab70f828a0a21c75bfe1e094889`, GPG G + DCO, parent ADR0158 `cfb60ee6`.

Emitter `closed_witness_admitted_merkle_insert_steps` checks optional path struct, Bytes32 leaf and public argument, path depth matching the declared plain tree, witness/formal types, negative presence guard and exact phase/insert action order. Shared authority prefix now accepts a borrowed action slice, avoiding synthetic circuit copies. Existing ledger8 witness and Merkle recording reused; runtime/ABI/schema unchanged. Contract/circuit/field/optional/enum name-renaming guard stays available, argument substitution/reordered guard/depth mismatch fail closed.

152 fixtures fresh, 129 renderer tests, 4 election tests, targeted all-target/all-feature backend/election/proof Clippy passed. Original and oracle election report add_voter + advance + set_topic, 3/5 recorded. Root will change the original-source cohort manifest gap to a positive requirement at integration; isolated manifest left unchanged to avoid a concurrent format conflict.

Both voting calls remain unavailable. No push or remote CI.


Exact-head focused gate passed at `d3a3a125`: `${LOCAL_EVIDENCE}/compact-focused-d3a3a125-election/receipt.json`, 3/5 recorded, Rust1.99.0.
