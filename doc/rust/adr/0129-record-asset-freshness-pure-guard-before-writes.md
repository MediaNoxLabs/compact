---
id: RUST-ADR-0129
alias: ADR-0129
title: "Record asset freshness pure guard before writes"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "authorization", "pure-helpers", "asset-registry"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 8628a9d7c7505fdfcc4abf87c728fc5bbf06bda119ad678087b9155057d465eb
---
# RUST-ADR-0129 — Record asset freshness pure guard before writes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the exact typed asset freshness pure guard before the existing ordered recordWrite helper. Nonempty input notes are validated without claiming opaque ledger-value decoding; future/expired failures precede all queries and witnesses, and one successful observed call is proved.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#232 closure](https://github.com/MediaNoxLabs/compact/issues/232#issuecomment-6017624730). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`1e9ed682`](https://github.com/MediaNoxLabs/compact/commit/1e9ed682d378aa8e4a5e346aa69aa7e2ea718424). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 129
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/232
```

## Historical decision and amendments

### Problem and source boundary

`examples/rust_backend/asset_registry_oracle.compact` has seven proof-required exports without recording. Six first stop at nested `StateAction::Let` and need separate Map/collection and composite decode work. `acceptIfFresh` first stops at `StateAction::PureCall actions[0]`. Its typed actions are a Unit pure `assertRecordFreshEnough(policy: FreshnessPolicy, record: AssetRecord, currentTime: Uint<64>)` call followed by zero-argument `recordWrite`. The latter already records in `tag`: two Counter increments, metered `currentTimestamp` witness and Cell write. The pure function has one unconditional registration-time assertion and one conditional max-age assertion.

### Before and after generated Rust

Before, only native `acceptIfFresh` exists. After, the generated recorded body calls the existing generated pure function before recording ledger/witness effects, then expands the existing recorded `recordWrite` helper:

```rust
crate::pure_circuits::assertRecordFreshEnough(policy.clone(), record.clone(), current_time)?;
let frame = record_write_recorded(frame, witnesses)?;
```

Names can differ. Source order, assertion messages, gas, VM and private transcript are the contract. The generated crate also exposes `acceptIfFresh_call` for successful proof-backed observed calls.

### Decision and limits

Admit only a typed immediate Unit PureCall with the three source parameter types and direct argument references, a compiler-declared pure Unit body with first assertion, conditional assertion and Unit result, followed by exactly one existing zero-argument `recordWrite` stateful call. Emit `syn` statements that call the generated pure Rust function before entering the existing helper recording path. No IR schema or runtime API change. Extra actions, changed argument source/order, changed pure-body shape and the six Let-blocked asset exports remain unavailable. The `AssetRecord.note` OpaqueString is an input, not a Map value decoded from ledger; known variable-length composite decode limitations remain.

### Acceptance

Capture TypeScript valid and nonempty-note inputs, plus future-registration and expired-policy failures. Compare native/recorded result/state, four gas dimensions, ordered public VM, private witness output/effects and Verify replay; failure must precede recordWrite. Pinned ZKIR 2.1.0 compiles the source circuit at k=11/1052 rows; prove, verify and ledger-8 validate/apply a generated observed call with nonempty note and inspect counters/updatedAt. Add renderer negative guards and checked source inventory only for fully assessed call. Focused local tests, rustfmt, Clippy, signed GPG+DCO commit. No push or remote CI.

### Local evidence

- Schema12 source-scoped inventory: 14 exports total, 10 proof-required, 4 recorded/observed and 6 unavailable. Before this slice the same source had 3 recorded/observed and 7 unavailable. The six remaining first blockers are typed `StateAction::Let` calls; `acceptIfFresh` is the only new admitted proof API. The combined exact packaged denominator is a root integration check, not a branch claim.
- TypeScript captures `fresh` and `unchecked_age` with a nonempty OpaqueString note; native and recorded Rust match result `[]`, constructor and final state serialization, four summed gas dimensions, ordered public VM, private transcript, witness order and Verify replay/effects. `future` and `expired` produce the same assertion errors before any query or witness.
- Pinned ZKIR 2.1.0 key compilation: k=11, 1052 rows. The generated observed `acceptIfFresh_call` equals the manual prepared prototype and proves, verifies, validates and applies on ledger-8 with the nonempty note. Resulting `revision` and `writeCount` each equal 1 and `updatedAt` matches the timestamp witness.
- Focused local gates: 101 renderer tests, all asset-registry fixture tests, 37-source oracle acceptance checker and Clippy for emitter, fixture and proof smoke passed. Remote CI was not run.

### Tracking

- Milestone issue: https://github.com/MediaNoxLabs/compact/issues/232.
- Delivery: signed local commit `5a2d8a0a765f20c1bdceb65386b2c40a92d525f8` on `codex/adr129-asset-registry`, based on root `1e9ed682d378aa8e4a5e346aa69aa7e2ea718424`; root integration and exact packaged inventory pending.
