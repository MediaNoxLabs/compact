---
id: RUST-ADR-0124
alias: ADR-0124
title: "Record a typed Field helper with an ordered Cell read"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "stateful-helpers", "cell", "typed-hash"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 3494b03d79061346a556eb20d7baddbf4a4590395d9197a4968a9305b30f9a8c
---
# RUST-ADR-0124 — Record a typed Field helper with an ordered Cell read

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted a closed actionless Field helper that evaluates one typed pair hash, performs one declared Field Cell read and adds its result before the caller write. Seeded nonzero-read behavior and a representative proof establish the ledger dependency; extra actions or other helper trees remain refused.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#229 closure](https://github.com/MediaNoxLabs/compact/issues/229#issuecomment-6017619992). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`15b6184d`](https://github.com/MediaNoxLabs/compact/commit/15b6184d06020705c69fb8485bf334b7a37cd6e0) · [`368b19c2`](https://github.com/MediaNoxLabs/compact/commit/368b19c23cc4aa659c8ccfbbd80fcc75fb9becd6) · [`67a0d53b`](https://github.com/MediaNoxLabs/compact/commit/67a0d53b76dcbc5caaea12f38b401320eaa3a1f1). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 124
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/229
```

## Historical decision and amendments

### Problem and typed evidence

On schema-12 IR/runtime ABI 37 at `15b6184d`, `call_arg_declared_type.compact::impureConst` is proof-required and natively runnable but has no recorded or observed-call API. Its first action is `StateAction::Let` binding `r: Field = Call sumVecPlusCell([0 Field, 1 Field] as Vector<2,Field>)`; the nested action writes `r` to `fieldCell`. The internal `sumVecPlusCell(v: Vector<2,Field>): Field` has no actions and returns `Add(Call sumVec(v), CellRead armCell)`. `sumVec` is a closed typed Field-pair transient hash. The recorder reports `unsupported_expression`, `Expr::Call`, at `actions[0].bindings[0].value`. Scanning the 24 sources with known proof gaps found no other currently missing export with this exact `Expr::Call` reason. The same source's persistent and transient hash gaps are separate.

The native path evaluates the typed vector argument, invokes `sumVec`, reads `armCell` through a ledger query, adds the two Fields, and writes `fieldCell`. A recorder that skips the read would lose the public VM read transcript and change gas. A recorder that treats the helper as a pure call would also miss its ledger dependency.

### Decision and ownership

Admit an internal Field helper only when it has exactly one declared pair argument, no actions, and an exact return tree `Add(Call closed_pair_hash(parameter), CellRead field)`. The pure callee must match the helper parameter type and pass the existing transitive closed Field-pair hash whitelist. The read must name a declared Field Cell at the exact IR index. Lower the caller argument through the typed, effect-free `cell_source`. Emit one typed argument binding, one generated pure method call, one `RecordingFrame` Cell read, one Field addition, and then the original nested action. Helpers with extra actions, unsupported pure bodies, type mismatches or other return trees remain unavailable. The renderer guard tests the positive order and rejects a side-effectful helper and a nonhash pure callee.

The emitter owns the closed helper shape and typed lowering. The generated pure method owns Field-pair hashing; the existing ledger slot and `RecordingFrame` own the read and VM/gas effects. The runtime, ledger-8 primitives, IR schema and ABI do not change.

### Before and after generated Rust

Before, native `impureConst` called `sumVecPlusCell(context, typed_vector)?` and wrote its returned Field, while no `recorded::impureConst` or typed observed call existed. After, the recorded method retains the same order and result:

```rust
let __compact_recorded_helper_arg_0: runtime::FixedVector<runtime::Field, 2> =
    runtime::FixedVector::new([runtime::Field::from(0u128), runtime::Field::from(1u128)]);
let __compact_recorded_helper_pure_1: runtime::Field =
    crate::pure_circuits::sumVec(__compact_recorded_helper_arg_0)?;
let (frame, __compact_recorded_helper_read_2): (_, runtime::Field) =
    crate::ledger_slots::armCell.record_read(frame)?;
let __compact_recorded_helper_result_3: runtime::Field =
    __compact_recorded_helper_pure_1 + __compact_recorded_helper_read_2;
let frame = crate::ledger_slots::fieldCell
    .record_write(frame, __compact_recorded_helper_result_3)?;
```

The generated argument expression also contains the compiler's typed tuple-to-vector coercion; the shortened example omits that local destructuring for readability. The observed API constructs `RecordedCall` from this method with Unit input.

### Local evidence

- Fresh TypeScript capture, native Rust and recorded Rust match full serialized state, four execution gas dimensions, exact ordered public VM `dup, idx, popeq, push, push, ins`, zero private outputs and Verify replay. The read query costs readTime 170000000 ps, computeTime 1249914853 ps, 0 bytes written/deleted. The write query costs readTime 85000000 ps, computeTime 1233942932 ps, 430 written/364 deleted bytes. TypeScript's reported final gas is the write query's gas, while the ordered query list retains both charges; the Rust trace assertion checks both.
- A separate Rust regression seeds `armCell=5` and checks native/recorded gas and state equality and exact `fieldCell = sumVec([0,1]) + 5`, so the returned value demonstrably depends on the ledger read.
- The generated typed observed call equals direct recording. Pinned ZKIR 2.1.0 generated prover/verifier keys and binary ZKIR; `--impure-field-helper` replayed, proved, verified and applied the call through ledger-8 with exact stored Field.
- `call-arg-declared-type` fixture tests pass 7/7; the focused renderer guard passes. Focused source gate passes 1/1 (14/16 recorded exports) with receipt `${LOCAL_EVIDENCE}/compact-focused-adr124/receipt.json`. The candidate inventory reports 257/316 proof-ready, 59 known gaps, 935 declarations and 25 unassessed. Exact integrated inventory delta will be recorded after root cherry-pick.
- The 15b base has one unrelated stale Welcome fixture, fixed on root at `67a0d53b`. Its renderer suite is 95/96, with the sole unrelated Welcome diagnostic expectation fixed on root at `368b19c2`. This slice does not change those files. Root will run the combined clean-head gate. No remote CI or push.

### Tracking

- Issue: https://github.com/MediaNoxLabs/compact/issues/229, `rust-backend-v2`.
- Base: `15b6184d06020705c69fb8485bf334b7a37cd6e0`.
- Signed conventional GPG+DCO delivery commit: pending.


### Delivery (2026-10-05)

Signed conventional GPG+DCO feature commit `182017d105a95e564701b0d03fb5236520e62870` is clean and ready for root cherry-pick from exact `15b6184d`. The one-source focused compiler gate passed 1/1 (14/16 recorded exports), receipt `${LOCAL_EVIDENCE}/compact-focused-adr124/receipt.json`; the other two are the distinct persistent/transient hash gaps. Call-argument fixture 7/7, focused positive/negative renderer guard, formatting and diff checks pass. Pinned ZKIR 2.1.0 proof, Verify replay and ledger-8 apply passed. The full 15b renderer and fixture sweeps have only the unrelated Welcome expectation/snapshot failures fixed on the root branch at `368b19c2` and `67a0d53b`. Root integration and exact combined inventory/gate are pending. No remote CI or push.
