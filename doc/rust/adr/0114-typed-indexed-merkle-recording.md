---
id: RUST-ADR-0114
alias: ADR-0114
title: "Typed indexed Merkle recording"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merkle", "indexed-allocation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 66df10c8d99d875d90629ad3e7d7fb695d47cb0cc356bdf9ca43a75310bea314
---
# RUST-ADR-0114 — Typed indexed Merkle recording

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted typed indexed placement/default insertion through existing runtime slots and canonical ledger programs for declared plain and historic Merkle trees. Four original exports and repeated/default index cases are validated; conditional leaf/index expressions remain outside the admitted domain.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#217 closure](https://github.com/MediaNoxLabs/compact/issues/217#issuecomment-6017598724). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`a0b11849`](https://github.com/MediaNoxLabs/compact/commit/a0b11849f842dcdfbd79726738f8a36828c17970). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 114
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/217
```

## Historical decision and amendments

### Problem
Schema-11 MerkleInsertIndex and MerkleInsertIndexDefault are native in Rust but first fail at actions[0] in the recorder. Thus exported place and place_default lack proof-capable calls even though ledger-8 already implements their VM semantics.

### Before and after generated Rust
Before: `ledger_contract::place(context, value, index)?` works, while `ledger_contract::recorded::place(context, value, index)` and `Contract.place_call(&observed, (value, index))` are absent. The same is true of place_default.
After: `recorded::place` calls `ledger_slots::t.record_insert_index(frame, value, index)?`; `recorded::place_default` calls `ledger_slots::t.record_insert_index_default(frame, index)?`. Both gain typed observed calls. The generated crate invokes typed slots, with no VM instructions in emitted Rust.

### Decision and ownership
The emitter accepts only an exact declared MerkleTree or HistoricMerkleTree field at its declared top-level path, a leaf value of the declared type and a Uint<64> position. Indexed default insertion requires the default of the declared leaf type. Runtime MerkleSlot delegates to RecordingFrame, which obtains typed verification programs from the same midnight-ledger 8 indexed insertion program builder as native execution. Hashing and VM operation order remain in runtime and midnight-ledger primitives. Other Merkle operations retain precise unsupported reasons.

### Acceptance
Compare TS/native/recorded/replay result, full state, four gas dimensions, ordered public VM, private outputs and FAB for both place and place_default. Prove/verify/validate/apply with pinned schema-11 ZKIR 2.1.0 and ledger-8. Add negative typed guard, focused renderer/fixture/rejection/gate checks and signed GPG+DCO conventional commit. Work locally without push or remote CI.

### Tracking
MediaNoxLabs issue https://github.com/MediaNoxLabs/compact/issues/217 in rust-backend-v2. Branch codex/adr114-merkle-typed-ops, base a0b11849.


### Expanded validation scope
The same two indexed operation kinds occur in four exported circuits: plain place/place_default and historic place/add_default. The shared runtime path admits all four. No new VM instructions are emitted in generated Rust. TS/native/recorded/replay checks cover exact full state, result type, four gas dimensions and ordered public VM for each; historic default additionally covers indexes 0, 2 and repeat 0. All four generated typed observed calls replay, prove, verify, validate and apply with pinned ledger-8/ZKIR 2.1.0. The negative renderer guard rejects conditional leaf or position expressions while leaving the separate default circuit recordable. Full inventory is 242/306 available, 64 missing, versus 238/306 and 68 on base a0b11849; only the four named circuit rows changed.


### Local gate result
- Exact pinned package qm7cvdcz9v1hjzkbs6j5iamyrmf6jq7j-compactc generated all plain/historic indexed ZKIR and keys with ZKIR 2.1.0. Four generated typed observed calls matched manual replay and each proof verified, validated and applied on ledger-8.
- Three TS oracle captures match native/recorded/replayed state, empty private outputs, Unit results, four gas dimensions and ordered VM; historic default includes index 0, index 2 and repeat index 0.
- 88 renderer tests, all three Merkle generated fixture test suites, runtime Merkle VM order test, 146 fixture regeneration checks (0 stale), 4 source rejection/output/proof-capability checks, rustfmt, Clippy and Python syntax passed.
- The full inventory changed exactly four rows from base a0b11849: plain place/place_default and historic place/add_default. Coverage 238/306 to 242/306; 64 gaps remain. Other Merkle actions retain exact unsupported nodes. No push or remote CI.

Delivery commit: eda0a4aa0e29422692e6aa7440af51fbe2016121 (verified GPG signature and DCO). Local branch codex/adr114-merkle-typed-ops; ready for integration.
