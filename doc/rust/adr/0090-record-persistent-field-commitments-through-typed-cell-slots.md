---
id: RUST-ADR-0090
alias: ADR-0090
title: "Record persistent Field commitments through typed Cell slots"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "crypto", "typed-slots"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: fb61e4c69f89458e533897263c915caea3f481becac867411b7335fab93e6cfe
---
# RUST-ADR-0090 — Record persistent Field commitments through typed Cell slots

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted exact persistent commitments of Field literals using a recorded Bytes32 opening Cell read and typed Cell write. Three literal-range APIs and TS VM/gas/private comparisons were delivered; the documented follow-through proves commitSmall, not every admitted commitment API. Other commitment value types remain separate.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#191 closure](https://github.com/MediaNoxLabs/compact/issues/191#issuecomment-6017553846). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`0c4d0f11`](https://github.com/MediaNoxLabs/compact/commit/0c4d0f11b12d2892d0a44d106b46ff953e7c1bc8) · [`715c1ef7`](https://github.com/MediaNoxLabs/compact/commit/715c1ef77aa11d395bcd68bff60be5bc85748c58) · [`74f959c1`](https://github.com/MediaNoxLabs/compact/commit/74f959c1bacab10c21751cefb98ea320664c1565). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 90
status: proposed
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem and measured scope

The checked `call_arg_declared_type.compact` contract has 16 exported proof-required circuits without Rust recorded/observed-call APIs at integrated ABI37 HEAD `715c1ef7`. They have several unrelated first blockers. Three bounded candidates—`commitSmall`, `commitU128`, and `commitFieldOnly`—share one typed IR shape: `StateAction::Let` binds a `Bytes<32>` value from `Expr::PersistentCommit { value: FieldLiteral, opening: CellRead(opening,index0) }`, then writes it to `commitCell` index1. Their first reason is `unsupported_action` at `actions[0]`. The three literal values deliberately span small integer, u128 and Field-only ranges; using an inferred Rust integer would silently alter the commitment bytes. This ADR promises no gain for the other 13 circuits.

The native generated crate already calls the runtime's `persistent_commit<Field>` and writes the result, but the developer cannot obtain a typed observed call:

```rust
let native = contract.commitFieldOnly(context)?;
// contract.recording.commitFieldOnly_call(&confirmed, private_state) is absent.
```

After this slice, the intended developer surface is:

```rust
let call = contract.recording.commitFieldOnly_call(&confirmed, private_state)?;
let proven = prover.prove(call)?;
ledger.apply(proven)?;
```

### Decision and ownership

Lower only the checked `Bytes<32>` Let binding whose expression is `PersistentCommit(Field, opening CellRead)` with an exact declared `Cell<Bytes<32>>` opening path/index. Emit source-order typed operations: `ledger_slots::opening.record_read(frame)?`, `runtime::persistent_commit(field_value, opening)`, then `ledger_slots::commitCell.record_write(frame, digest)?`. Reuse `field_expression`/the declared Field type to render each literal as `runtime::Field`; never let Rust infer `i32`, `u128` or raw bytes. Retain typed unsupported reasons for other commit inputs, wrong ledger declarations and nested expressions. The emitter uses `syn`/`quote` syntax nodes; generated code contains no VM opcode stream.

The runtime already provides `CellSlot<FixedBytes<32>>::record_read/record_write` and `persistent_commit<T: Aligned + Into<Value>>`, which delegates to pinned midnight-ledger's persistent commitment and FAB representation. The ledger/zk packages own VM and proof semantics. No new runtime method, private IR variant, capability report field or ABI bump is planned (IR schema9, runtime ABI37, report schema3 at proposal time). If a new public runtime method becomes necessary, re-evaluate and bump ABI before delivery.

### Acceptance and limits

Use a frozen compiler and report exact before/after capability movement for the three names, with zero losses across the checked compiled corpus. Test native/recorded/replayed state, result, ordered public operations, private FAB output/alignment and all four gas dimensions against a pinned TypeScript capture for small, u128 and Field-only literals. The full Field-only value must commit to the Field encoding, not an inferred integer encoding. Include a wrong path/type rejection and generated standalone consumer. Generate the matching pinned ZKIR/keys, prove, verify, validate and apply at least the small and Field-only calls on ledger-8. Refresh 138 generated fixtures, run focused Rust 1.99 Clippy and strict capability checks, then integrate with the local factory receipt. Remote CI remains deferred by user direction. The other `call_arg_declared_type` blockers, full TypeScript corpus parity and release gates remain open.

### Tracking

- Compiler source: `examples/rust_backend/call_arg_declared_type.compact`, lines 65–74.
- Typed IR/reasons: `${LOCAL_EVIDENCE}/adr90-call-arg-probe/contract/compact-rust-ir.json` and `rust-capabilities.json`, frozen ABI37 compiler SHA-256 `28d158fae415ec47b2caf188f633526e53471bd423e466f3fd466af961c1e47e`.
- Focused issue: pending creation in `MediaNoxLabs/compact` rust-backend-v2 before code.


Focused issue created before code: [#191](https://github.com/MediaNoxLabs/compact/issues/191), assigned to `rust-backend-v2`. This ADR remains proposed; no emitted API, proof or parity gain is claimed.


### Local delivery checkpoint — 2026-10-05

Signed/DCO commit `74f959c1` adds exact typed recording for `persistentCommit(Field literal, Cell<Bytes<32>> opening)` in `StateAction::Let` and regenerates `call_arg_declared_type`. The three proof-required circuits `commitSmall`, `commitU128`, and `commitFieldOnly` now expose recorded and observed calls. `local_parity_gate.py` focused receipt `${LOCAL_EVIDENCE}/compact-local-focused-adr90-715c1ef7/receipt.json` passed fixture freshness and generated crate tests; 138/138 fixtures checked with one expected refresh. Inventory `${LOCAL_EVIDENCE}/adr90-inventory.json` measures 202/293 proof-required APIs available versus 199/293 at ADR-0087, zero membership drift, 91 known proof gaps, 123 unassessed exports. The inventory's `--require-full` intentionally exits 1 while those gaps remain. Source-to-proof-to-ledger validation is still pending in the separate focused harness.


### Proof and TypeScript parity follow-through — 2026-10-05

Signed/DCO `0c4d0f11` adds `commitSmall` pinned source-to-proof-to-ledger application and TypeScript VM/gas/private parity for all three commitments. Isolated proof smoke passed against `commitSmall` prover/verifier/ZKIR. Integrated generated crate tests passed 2/2 at root. ABI37/schema10 full local gate remains pending other in-flight slices. Issue #191 has the exact evidence comment.
