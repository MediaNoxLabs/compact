---
id: RUST-ADR-0133
alias: ADR-0133
title: "Record guarded arithmetic before a Counter increment"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "assertions", "counter", "pure-helpers"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 208a41d1480b494aab1273c89c7636daa65fdba8b44e18cfc7d2ebccd292ebf7
---
# RUST-ADR-0133 — Record guarded arithmetic before a Counter increment

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the existing bounded struct freshness pure guard followed by the exact literal-one Counter continuation. Successful and disabled-age cases match behavior, while future/expired guards fail before public effects; one successful proof/application establishes this slice.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#235 closure](https://github.com/MediaNoxLabs/compact/issues/235#issuecomment-6017630014). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`9f328f10`](https://github.com/MediaNoxLabs/compact/commit/9f328f1038ee71025aa04e4a0f2f5083e2c05b86). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 133
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/235
```

## Historical decision and amendments

### Problem and scope

`examples/rust_backend/guarded_assert_arith_oracle.compact::recordFreshEnough` is TypeScript-positive and proof-required but lacks a Rust recorded/observed API. Its schema12 actions are `StateAction::PureCall assertFreshEnough(policy: VerifierPolicy, attestation: Attestation, currentTime: Uint<64>)` followed by `StateAction::Let` binding Uint<16> literal one around `CounterIncrement accepted`. The pure Unit body has an unconditional creation-time Assert and a conditional age Assert with two-level struct projection and trapping subtraction. The existing native function runs this guard before the Counter. ADR-0129 records this same three-type pure guard only when followed by a zero-argument Unit helper.

### Before and after generated Rust

Before, callers have only the native API:

```rust
ledger_contract::recordFreshEnough(context, policy, attestation, current_time)?;
// no recorded::recordFreshEnough or observed call
```

After, recording uses the same generated pure function and the existing typed Counter slot:

```rust
crate::pure_circuits::assertFreshEnough(policy.clone(), attestation.clone(), current_time)?;
let frame = crate::ledger_slots::accepted.record_increment(frame, 1)?;
let recorded = ledger_contract::recorded::recordFreshEnough(context, policy, attestation, current_time)?;
let call = contract.recording().recordFreshEnough_call(&observed, private_state, policy, attestation, current_time)?;
```

The exact generated temporary names may differ. The invariant is source order: a failing pure assertion returns before any Counter VM operation. Success increments once.

### Decision and negative boundary

Extend only ADR-0129's `closed_guarded_struct_pure_steps` second-action predicate. It accepts either the existing zero-argument Unit helper or the exact compiler-emitted Uint<16> literal-one Let around CounterIncrement, validated by `closed_counter_one_continuation`. Retain the three ordered direct typed Coerce(Parameter) arguments (Struct, Struct, Uint64), pure Unit body shape Sequence [Assert, If(Assert, Unit)] and Unit terminal, and Unit stateful result/return. Generic `append_steps` already lowers the closed Let/Counter action. No IR schema, runtime API or new primitive. Extra actions, different argument source/type/order, changed pure-body shape, and arbitrary Counter amounts remain unavailable. Other pure-call or Let gaps are independent.

### Acceptance

Fresh TypeScript success, max-age-disabled success, future creation and expired-age failure; compare native/recorded result and serialized state, four gas dimensions, ordered public VM, private effects, Verify replay and assertion error. Failure must have no Counter operation. Pinned ZKIR 2.1.0 proof for generated observed call must verify and ledger-8 validate/apply, with accepted Counter increment. Add renderer negatives and exact checked inventory gain. Run focused local tests, rustfmt, Clippy. No push or remote CI.

### Local evidence

- Exact schema12 source-scoped inventory: four exported circuits (three pure), one proof-required API; `recordFreshEnough` moves from unavailable to recorded and observed, leaving zero missing in this source. The combined repository denominator awaits root packaging.
- Fresh TypeScript captures cover accepted age, max-age disabled, future creation and expired policy. Native and recorded Rust match `[]` result, constructor/final serialized state, four gas dimensions, ordered VM, empty private output/effects and Verify replay. Both rejected calls match assertion text and stop before queries or Counter effects.
- Pinned ZKIR 2.1.0 compiled `recordFreshEnough` at k=9, 493 rows. Generated observed call and manual prepared prototype match; the proof verifies, ledger-8 validates/applies, resulting state matches the recorded state, and `accepted` equals one.
- 104 renderer tests (including counter amount, extra action and argument-source negatives), all guarded fixture tests, 23 inventory tests, 37-source oracle acceptance, rustfmt and focused Clippy passed locally. No push or remote CI.

### Tracking

- Milestone issue: https://github.com/MediaNoxLabs/compact/issues/235.
- Delivery: signed local commit `4b77dc69b5823e4cc7bd3a8528dcebea8a07d833` on `codex/adr133-guarded-assert`, based on integrated root `9f328f1038ee71025aa04e4a0f2f5083e2c05b86`; root integration and combined packaged inventory pending.
