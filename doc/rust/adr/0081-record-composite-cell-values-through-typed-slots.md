---
id: RUST-ADR-0081
alias: ADR-0081
title: "Record composite Cell values through typed slots"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "typed-slots", "composite-values"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 828a1e6977ab560eafe437b7e5a46d91f816a0408de2c1674592fa6a92f0421d
---
# RUST-ADR-0081 — Record composite Cell values through typed slots

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted for recursive direct composite Cell recording through existing typed slots. Fifteen measured API gains and representative Pair proof/application were delivered; the historical record explicitly leaves separate tuple/vector/point/ContractAddress proof cases and broader TS dimensions outside that slice. Later milestone closure must not turn API availability into exhaustive proof coverage.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#181 closure](https://github.com/MediaNoxLabs/compact/issues/181#issuecomment-6017537153). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`08e0c391`](https://github.com/MediaNoxLabs/compact/commit/08e0c3915398e6ef94e66bf24644e705fb92e694) · [`4656cd31`](https://github.com/MediaNoxLabs/compact/commit/4656cd31cb411bc1ada7f808bdb996df476a7b9d) · [`706d251e`](https://github.com/MediaNoxLabs/compact/commit/706d251e7281ffc78a598be32b3287002ca1e753). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 81
status: accepted-partial
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/181
```

## Historical decision and amendments

### Problem and measured candidates

The native Rust target already reads and writes ledger Cells containing generated structs, tuples, fixed vectors and Jubjub points. The recorded emitter still limits direct `StateAction::CellWrite` and `StateReturn::CellRead` to a smaller type set. Consequently a consumer can execute these circuits natively but cannot obtain their replayable recorded or typed observed-call APIs. This is an emitter eligibility gap over existing runtime and ledger primitives, not a request to generate VM instructions in contract code.

The schema-2 ABI-35 reason inventory, measured **before** ADR-0080's unsigned Cell slice, identified six composite `unsupported_type` writes and nine composite direct Cell reads after excluding `wide_uint_oracle.readWide`. These **15 circuits are candidates, not promised gains**. Rerun the reason inventory at the integrated ADR-0079/ADR-0080 head before editing; a newly cleared first blocker may expose another missing expression, type or call operation. The 137-source/333-exported-circuit corpus is a curated subset, not full Compact language coverage.

| Exact source and exported circuit | Declared Cell type | First known gap |
|---|---|---|
| `cell_struct.set_record` | `Pair { amount: Field, active: Boolean }` | write type |
| `cell_struct.read_record` | same `Pair` | direct read |
| `constructor_tuple_coercion.read_pair` | `[Field, Field]` | direct read |
| `constructor_vector_coercion.read_values` | `Vector<2, Field>` | direct read |
| `default_vector.read_values` | `Vector<2, Field>` | direct read |
| `jubjub_cell.set_point` | `JubjubPoint` | write type and value-source eligibility |
| `jubjub_cell.read_point` | `JubjubPoint` | direct read |
| `jubjub_cell.set_box` | `PointBox { point: JubjubPoint, count: Field }` | write type |
| `jubjub_cell.read_box` | same `PointBox` | direct read |
| `kernel_self_oracle.readAddress` | `ContractAddress { bytes: Bytes<32> }` | direct read |
| `ternary_cond_oracle.walkerVectorElement` | `Vector<2, Field>` | write type; nested vector expression may reveal another blocker |
| `vector_tuple_cell.set_values` | `Vector<3, Field>` | write type |
| `vector_tuple_cell.read_values` | same `Vector<3, Field>` | direct read |
| `vector_tuple_cell.set_pair` | `[Field, Boolean]` | write type |
| `vector_tuple_cell.read_pair` | same `[Field, Boolean]` | direct read |

### Developer-facing before and after

Before, the generated `cell_struct` crate exposes native methods while the proving entry points are absent:

```rust
let native = crate::ledger_contract::set_record(context, pair.clone())?;
let pair = crate::ledger_contract::read_record(native.context)?;
// No ledger_contract::recorded::set_record/read_record or set_record_call/read_record_call.
```

After, the same declared type is accepted by the typed recorded surface:

```rust
let recorded_write = crate::ledger_contract::recorded::set_record(context, pair.clone())?;
let recorded_read = crate::ledger_contract::recorded::read_record(
    recorded_write.execution.context,
)?;
let call = contract.recording.set_record_call(&observed_state, private_state, pair)?;
```

The corresponding generated bodies should remain slot calls, conceptually:

```rust
let frame = runtime::recording::RecordingFrame::new(context);
let frame = crate::ledger_slots::record.record_write(frame, pair)?;
Ok(frame.finish(()))

let frame = runtime::recording::RecordingFrame::new(context);
let (frame, value): (_, crate::types::Pair) =
    crate::ledger_slots::record.record_read(frame)?;
Ok(frame.finish(value))
```

The examples show intended API shape; final signatures and ownership must be verified against emitted code and its separate consumer tests.

### Decision and ownership

Extend only the checked AST recorder eligibility for direct `CellWrite`/`CellRead` to those composite `Type` variants whose generated Rust type implements `CellValue`. Keep exact declaration type/index checks and the existing source-order action lowering. Extend typed `cell_source` to admit a `JubjubPoint` parameter when its declared type matches; retain the existing checked vector/tuple/struct source path and reject any nested expression it cannot lower. Use the schema-2 capability reason to expose the *next* blocker where eligibility is insufficient. Do not synthesize string-based Rust code, VM opcodes, or a generic fallback that silently claims a complete trace.

The runtime already owns `CellSlot<T: CellValue>::record_read/record_write` and `RecordingFrame::read_cell/write_cell`. `CellValue` covers `JubjubPoint`, fixed vectors, tuples and generated structs through `CompactCellValue`; values use `Aligned` and upstream FAB `Value` conversion. The pinned ledger-8 VM, FAB, state and midnight-zk proof stack own encoding and application. No new runtime primitive, derive macro, DSL or ABI bump is expected if these existing types compile and replay through the slot. Private IR schema remains 8; capability report schema remains 2 after ADR-0079 integration. Reassess ABI only if implementation adds a public runtime method.

### Acceptance and risk

1. Before code, attach the focused issue to `rust-backend-v2` and compare a fresh integrated ADR-0079/ADR-0080 reason receipt with the 15 candidates. Record actual per-circuit before/after capability movement; never infer gain from removing a guard alone.
2. Use focused renderer tests for `Pair`, tuple, `FixedVector<Fr, N>`, `JubjubPoint`, `PointBox` and `ContractAddress`; ensure wrong declaration type/index and unsupported nested expression stay rejected with truthful reasons. Verify emitted source compiles in a separate Cargo consumer without edits.
3. Compare native, recorded and replayed values, state, ordered public Verify operations, private outputs and all four gas dimensions with TypeScript oracle runs. Test multi-atom alignment and FAB length/order explicitly: tuples mix Field/Boolean atoms, vectors multiply element alignment, `JubjubPoint` uses point representation, derived structs combine fields, and `ContractAddress` wraps 32 bytes. Test malformed length/point decoding and state mismatch behavior.
4. Run source-to-proof-to-ledger application for representative struct, tuple/vector and point calls using the matching pinned ZKIR and keys. Do not claim proof admission from native state parity or fixture freshness alone. For the nested ternary vector writer, compare branch evaluation order and output before adding it to the accepted count.
5. Iterate with focused renderer/generated-crate tests, then run all 137 fixture freshness and report invariants, the packaged Nix compiler/consumer, exact Rust 1.99 Clippy and the local proof/application gate because new recorded methods change the proving surface. Keep conventional GPG/DCO commits local and defer remote CI under the user's current policy.

Boolean `Expr::CellRead` inside Let/Assert and the policy for action-free witness/crypto return expressions are separate follow-ups. This ADR does not imply that their combined missing-capability buckets are fixed by composite Cell eligibility.

### Tracking

- Accelerator: [Milestone 2 — Full TS parity delivery accelerator](references.md#private-note-10).
- Reason baseline: [ADR-0079 — Explain missing Rust recording capabilities from typed lowering](0079-explain-missing-rust-recording-capabilities-from-typed-lowering.md) / [#178](https://github.com/MediaNoxLabs/compact/issues/178).
- Unsigned predecessor: [ADR-0080 — Record unsigned Cell reads and writes through typed slots](0080-record-unsigned-cell-reads-and-writes-through-typed-slots.md) / [#180](https://github.com/MediaNoxLabs/compact/issues/180).
- Parent capability issue: [#152](https://github.com/MediaNoxLabs/compact/issues/152).
- Focused issue: [#181](https://github.com/MediaNoxLabs/compact/issues/181), kept open for remaining proof and parity checks.
- Delivery: local partial acceptance at signed/DCO `df8c0f66`; full integrated gate pending.


### Tracking amendment — 2026-10-05

Focused [#181](https://github.com/MediaNoxLabs/compact/issues/181) was created in `MediaNoxLabs/compact` and assigned to `rust-backend-v2` after this proposed ADR and before any ADR-0081 code. The 15 circuits above remain candidate gains pending a fresh integrated ADR-0079/ADR-0080 reason receipt and executable acceptance. Boolean CellRead and action-free witness/crypto expression policy remain separate.


### Local delivery checkpoint — 2026-10-05

Conventional GPG-verified/DCO commit `df8c0f66dffba5c5d123f599560ac8c0c8f410e5` on local branch `codex/composite-cell-recording` implements recursive checked eligibility for generated struct, tuple (arity 1–8), fixed vector, and Jubjub Cell values. Generated methods still call existing `CellSlot<T: CellValue>::record_write/record_read` and `RecordingFrame`; no runtime or VM implementation, public runtime ABI change, private IR schema change, or capability report schema change was required (ABI 35 / IR 8 / report 2). The branch was not pushed.

Fresh integrated pre-change schema-2 receipt at `4656cd31`: 170/333 recorded among 137 fixture sources, 163 unavailable (action 82, return 59, expression 16, type 6). The source-built post-change compiler reports 185/333 recorded and observed, 148 unavailable (action 82, return 50, expression 16), exactly the 15 named candidates above gained and zero regressions. Compiler `contract-info.json` independently marks all 15 `proof:true`; the actionable proof-capable gap moves from 116/286 to 101/286. Forty-seven proof-false native-only circuits remain in the raw denominator. All false recorded and observed rows retain typed reasons.

Focused evidence: 68 renderer tests; all eight changed generated fixture packages compile and pass their existing tests; new Pair, Vector/tuple, JubjubPoint and PointBox tests compare native, recorded and VM replay state/result/gas against the checked TypeScript state/coordinate captures where present; all 137 fixture outputs are fresh with the report invariant; targeted Rust 1.99 all-target/all-feature Clippy with `-D warnings` passes for backend, affected fixtures and proof smoke. A strict generated `cell_struct` contract builds as a standalone crate. Pinned ZKIR 2.1.0 compiles `set_record` and `read_record` keys; the focused proof smoke partitions the generated Pair traces, proves each call, validates it and applies it to ledger-8 contract state. The full local proof harness now invokes this Pair mode under `--proof`.

This is **partial acceptance** for the 15 API surfaces: the representative multi-atom Pair source-to-proof-to-ledger path passed, while separate tuple/vector, point/PointBox and ContractAddress proof/application cases, fresh TypeScript per-call gas/transcript capture, malformed composite decode checks, and the full Nix packaged compiler/proof/consumer gate remain for the integrated checkpoint or follow-up. The full proof harness invocation and remote CI were not run on this isolated branch. Keep #181 open until those remaining acceptance checks are accounted for. Boolean CellRead inside Let/Assert and action-free expression policy remain separate work.

Issue evidence: [#181 local delivery comment](https://github.com/MediaNoxLabs/compact/issues/181#issuecomment-5983371315).

### Integrated ABI36 checkpoint — 2026-10-05

Cherry-picked as signed/DCO `08e0c391` and retained all 15 direct composite Cell API gains with zero regressions. The ABI36 full local gate at `706d251e7281ffc78a598be32b3287002ca1e753` passed all 137 fixture freshness checks, workspace tests, exact Rust 1.99 Clippy, generated consumers and the pinned source-to-proof-to-ledger harness including Pair set/read. Receipt `${LOCAL_EVIDENCE}/compact-local-full-abi36-706d251e/receipt.json` binds compiler SHA-256 `f5c8e80e3b50a9b6aee8425b928e55fb98bdc55dba6479e5f53e7270da5b1db8`. The overall corpus at this checkpoint has 186/286 proof-required APIs available and 100 gaps; this includes the separate Merkle gain. Tuple/vector/point/ContractAddress individual proof paths and broader semantic TS parity remain open under #181.
