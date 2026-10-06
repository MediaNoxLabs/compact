---
id: RUST-ADR-0122
alias: ADR-0122
title: "Record discarded Field witnesses with typed arguments"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "witness", "private-transcript"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e6355f126d37ee3603d11dd87e86523608d19c12720f2adea35cdeeabd458983
---
# RUST-ADR-0122 — Record discarded Field witnesses with typed arguments

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted standalone discarded Field witnesses with exactly typed effect-free arguments while retaining their private state, metering and FAB outputs. Both target APIs are proved; nested witnesses and unsupported Unit-with-arguments shapes remain refused. The parity callback intentionally excludes an unrelated extra ledger read.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#225 closure](https://github.com/MediaNoxLabs/compact/issues/225#issuecomment-6017612760). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`c6071253`](https://github.com/MediaNoxLabs/compact/commit/c6071253dbf6e35a75ca549bf2f7c0dcd29f4fe1). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 122
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/225
```

## Historical decision and amendments

### Problem statement

On schema-12 IR/runtime ABI 37 at `c6071253`, `call_arg_declared_type.compact::witnessBare` and `witness_vector_action.compact::discardResult` are proof-required exports with native Rust methods but no recorded or observed-call API. Both first execute `StateAction::Expression(WitnessCall)` with `sumWitness(Vector<2, Field>): Field`, discard its returned Field, and then write a public Cell. The recorder only admitted a zero-argument Unit-returning witness in this action. Both reported `unsupported_action` at `actions[0]`. Discarding the Field value does not discard the witness's private-state update or FAB output.

The nearby `impureConst` pure call and persistent/transient hash Let gaps remain separate. The previous Unit witness path remains supported.

### Decision

Admit a standalone witness expression if its declaration returns `Field`, its argument count matches, and every argument can be lowered through the existing typed, effect-free `cell_source`. Bind each argument to a Rust temporary of the declared parameter type and invoke exactly one `RecordingFrame::try_witness_metered` call before subsequent public ledger actions. Keep the zero-argument Unit case; reject Unit calls with arguments, other result types, and nested witness calls or other unsupported argument expressions. A renderer test checks the nested-witness rejection at `actions[0].value.arguments[0]`.

The emitter owns signature checks and typed lowering. The generated `TryWitnesses` bridge and runtime `RecordingFrame` own witness execution, private state, metering and private transcript construction. Ledger slots own public writes. No new runtime primitive, IR schema change, ABI change or native-only fallback is added.

### Before and after generated Rust

Before, only the native method existed. It evaluated `sumWitness` and pushed its aligned Field output even though Compact discarded the return value. There was no generated `recorded::witnessBare`, `recorded::discardResult`, or typed observed call. After, the recorder emits the typed argument and preserves effect order (shown from `witnessBare`; `discardResult` has the same shape):

```rust
let __compact_recorded_witness_arg_0: runtime::FixedVector<runtime::Field, 2> = {
    let __compact_cast_source_0 =
        (runtime::Field::from(0u128), runtime::Field::from(1u128));
    let (__compact_cast_item_0_0, __compact_cast_item_0_1) = __compact_cast_source_0;
    runtime::FixedVector::new([__compact_cast_item_0_0, __compact_cast_item_0_1])
};
let (frame, _) = frame.try_witness_metered(|context, meter| {
    witnesses.sumWitness(
        context.witness_context_with(super::LedgerView {
            state: context.query.state.get_ref(),
            meter,
        }),
        __compact_recorded_witness_arg_0,
    )
})?;
let frame = crate::ledger_slots::fieldCell.record_write(frame, runtime::Field::from(7u128))?;
```

The `Field` witness result is intentionally ignored in the public expression; the metered frame still retains its private effects and aligned transcript output.

### Local evidence and acceptance

- Frozen parent compiler `${HISTORICAL_NIX_STORE}/59wsqxxa054qxakl5gnmzda0w01ngzjz-compactc` supplies the unchanged schema-12 frontend. Local AST renderer changes only the two proof-required APIs in a 935-declaration inventory: 251/316 to 253/316 proof-ready, 65 to 63 known gaps, 25 unassessed unchanged. Source identity is stable.
- Fresh TypeScript captures and generated native/recorded fixture tests compare serialized state, one witness invocation, private state 7→8, one aligned private FAB output, four execution gas dimensions, full ordered public VM and Verify replay. Both circuits write Field 7 and execute ordered `push(false), push(true), ins(cached=false,n=1)`. `witnessBare` gas is readTime 85000000 ps, computeTime 1233942932 ps, bytesWritten 364, bytesDeleted 364. `discardResult` gas has the same read/compute and 36 written/deleted bytes. Native and recorded match TypeScript for each. The `discardResult` parity witness deliberately avoids an extra ledger read absent from the TypeScript callback; a separate preexisting native test still covers its ledger-reading witness behavior.
- Pinned ZKIR 2.1.0 generates keys and binary ZKIR for both. Generated typed observed calls equal direct recorded calls. Each trace replays, proves, verifies, and applies through ledger-8 with expected Cell state and value. Proof-smoke selectors: `--witness-vector-bare` and `--witness-vector-discard`.
- `call-arg-declared-type` fixture tests 6/6, `witness-vector-action` tests 2/2, renderer tests 92/92 plus the new negative guard. Focused two-source gate 2/2 passed (16/19 recorded exports; three unrelated call/hash gaps remain) at `${LOCAL_EVIDENCE}/compact-focused-adr122-both-retry/receipt.json`. The 147-fixture sweep and signed delivery commit are pending final check. No remote CI or push.

### Tracking

- Issue: https://github.com/MediaNoxLabs/compact/issues/225, `rust-backend-v2`.
- Base: `c6071253`.
- Delivery: pending signed conventional GPG+DCO feature commit, to be recorded below.


### Delivery (2026-10-05)

Signed conventional GPG+DCO feature commit `c24c48b3b1fd8e0b8d5638b74dd61c2edbc1b7a9` is clean and ready for root cherry-pick from `c6071253`. The exact 147-fixture sweep completed with zero stale and zero failed outputs. Focused two-source compiler receipt `${LOCAL_EVIDENCE}/compact-focused-adr122-both-retry/receipt.json` passed 2/2 with 16/19 recorded exports. Both fixture suites passed (6/6 and 2/2), renderer 92/92 plus the focused negative guard, and `cargo fmt --all -- --check` is clean. Both pinned proof-smoke commands replayed, proved, verified and applied. No remote CI or push. Root integration and broad gate result are pending.
