---
id: RUST-ADR-0130
alias: ADR-0130
title: "Record typed pair hashes before Cell writes"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "typed-hash", "cell"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e4f54b58ee72156ff2a4d630e6f14c666fda7f215ee289cc30219c400f2ef882
---
# RUST-ADR-0130 — Record typed pair hashes before Cell writes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted literal two-Field tuple persistent and transient hash bindings with distinct exact digest types before Cell writes. Both calls have full transcript parity and proofs; effectful or variable hash operands remain outside this slice. The published digest bytes are deterministic fixture outputs, not private key material.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#233 closure](https://github.com/MediaNoxLabs/compact/issues/233#issuecomment-6017626227). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1e9ed682`](https://github.com/MediaNoxLabs/compact/commit/1e9ed682d378aa8e4a5e346aa69aa7e2ea718424). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 130
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/233
```

## Historical decision and amendments

### Problem and typed evidence

At `1e9ed682` (IR schema 12/runtime ABI 37), `call_arg_declared_type.compact::hashPersistentVec` and `::hashTransientVec` are proof-required and native-runnable but lack recorded and observed-call APIs. Both lower to a `StateAction::Let` with one typed hash binding and a nested `CellWrite` that consumes that binding. Persistent is `Bytes<32> = PersistentHash(Tuple(Field 0, Field 1))`, then `hashCell.write`; transient is `Field = TransientHash(the same tuple)`, then `fieldCell.write`. The persistent gap was `unsupported_action`, `StateAction::Let`, `actions[0]`; transient was `unsupported_expression`, `Expr::TransientHash`, `actions[0].bindings[0].value`.

The native emitter already invokes `runtime::persistent_hash` and `runtime::transient_hash`. Those runtime functions encode typed FAB values and call ledger-8 `PersistentHashWriter` or transient hash. Re-implementing hashing or treating the two digest types alike would risk a different Cell value and proof input.

### Decision and ownership

Admit a hash binding only when its operand is an effect-free tuple of exactly two Field literals and the declared binding type matches the hash result: `Bytes<32>` for persistent, `Field` for transient. Emit a typed `(Field, Field)` local, invoke the existing runtime hash, bind the exact result type, and then lower the original nested action with its existing typed Cell slot. A Cell read, witness or call within the hash operand, a different tuple shape, a mismatched result type, or an unsupported nested action remains unavailable. A renderer guard checks the admitted order and rejects a Cell-read operand and a one-element tuple. This intentionally does not generalize hash recording beyond the proved source shape.

The AST emitter owns the structural admission and typed syntax. `midnight-compact-runtime` owns ledger-8 FAB/hash semantics and the existing `RecordingFrame` owns the public Cell-write VM, gas and replay. IR schema 12 and runtime ABI 37 remain unchanged; no new primitive, macro, or handwritten VM operation is introduced.

### Before and after generated Rust

Before, native `hashPersistentVec` computed `runtime::persistent_hash((Field::from(0), Field::from(1)))` and wrote `hashCell`; native `hashTransientVec` did the corresponding transient hash and wrote `fieldCell`. Neither had a generated `recorded::*` method or typed observed call.

After, the generated persistent recorder contains:

```rust
let __compact_recorded_hash_arg_0: (runtime::Field, runtime::Field) = (
    runtime::Field::from(0u128), runtime::Field::from(1u128),
);
let __compact_recorded_hash_1: runtime::FixedBytes<32> =
    runtime::persistent_hash(__compact_recorded_hash_arg_0);
let frame = crate::ledger_slots::hashCell
    .record_write(frame, __compact_recorded_hash_1)?;
```

The transient recorder uses `let __compact_recorded_hash_1: runtime::Field = runtime::transient_hash(__compact_recorded_hash_arg_0);` followed by `fieldCell.record_write`. The generated `Contract.hashPersistentVec_call` and `.hashTransientVec_call` retain Unit input and bind the recorded methods to observed state and the declared entry points.

### Local evidence

- Fresh TypeScript capture from the unchanged Compact source matches native and recorded Unit result, full serialized ledger state, and all four query gas dimensions for both: readTime `85000000` ps, computeTime `1233942932` ps, bytesWritten `430`, bytesDeleted `364`. The TypeScript capture keeps the reported final gas and query list. The fixture test checks these independently and checks native/recorded gas equality.
- Complete ordered TypeScript and recorded VM operations match, including operands: `push` path key (`02` persistent, `03` transient), `push` stored digest/Field, `ins`. Persistent stored bytes are `cb592844121d926f1ca3ad4e1d6fb9d8e260ed6e3216361f7732e975a0e8bbf6`; transient stored Field FAB bytes are `d69ee4b3d57bf5e75d629c9ec74d344a984f306b8ecb8f9b590b1e463ae0dc0a`. Both have zero private outputs. Rust tests also read each typed Cell and compare it with the runtime's ledger-8 hash of the exact tuple.
- Focused renderer positive/negative guard passes 1/1; generated call-argument fixture passes 8/8. Focused compiler gate passes 1/1, all 16 source exports recorded, receipt `${LOCAL_EVIDENCE}/compact-focused-adr130/receipt.json`. The source-built compiler snapshot is `${LOCAL_EVIDENCE}/compactc-adr130` with the pinned Scheme frontend. The full candidate inventory `${LOCAL_EVIDENCE}/compact-inventory-adr130.json` reports 267/316 proof-ready, 49 known gaps and 25 unassessed. Relative to the earlier ADR-0124 inventory, the only new exports attributable to this exact hash shape are these two; other changes are separate root work.
- Pinned `midnight-zkir 2.1.0` emitted binary ZKIR and prover/verifier keys under `${LOCAL_EVIDENCE}/compact-adr130-proof`. The `--pair-hash-cell` smoke replayed and partitioned both generated traces, compared each typed observed call with direct recording, then proved, verified, validated and applied each call through ledger-8. Applied states match recorded states and the exact typed digest/Field values.
- `cargo +1.99.0 fmt --all -- --check`, JavaScript syntax, Python syntax, and `git diff --check` pass. No remote CI or push.

### Limits and tracking

This slice covers literal two-Field tuple operands only. Effectful or variable hash operands still need ordered typed lowering and independent proof parity. A clean combined-head gate after cherry-pick remains root's integration check; the milestone's broader CI and production exit stay open.

Issue: https://github.com/MediaNoxLabs/compact/issues/233. Signed conventional GPG+DCO feature SHA: `8f3f4fbe64a6ad845489ea30dd69812ef7345e5c` on isolated `codex/adr130-vector-hash-recording`, based on `1e9ed682`. Root integration and exact combined-head package/receipt remain pending.
