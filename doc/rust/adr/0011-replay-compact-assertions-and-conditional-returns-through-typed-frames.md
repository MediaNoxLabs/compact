---
id: RUST-ADR-0011
alias: ADR-0011
title: "Replay Compact assertions and conditional returns through typed frames"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: cdaad660d884d057727a309845f31303f9d0cb2bf8573dff16a0e81d797b1b08
---
# RUST-ADR-0011 — Replay Compact assertions and conditional returns through typed frames

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed frame lowering of the supported assertion, enum/Bytes witness and conditional-return forms used by tiny. Preserve operand/witness/query order and short-circuit failure, with gas reported under ADR0010. The delivered complete recorded flow is a bounded control-flow slice rather than general admission of all conditional expressions.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#112 closure](https://github.com/MediaNoxLabs/compact/issues/112#issuecomment-6017418051). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`af62a9db`](https://github.com/MediaNoxLabs/compact/commit/af62a9dbd5f1731a096c6e4d808dead7a89f3d59). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 0011
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/112
```

## Historical decision and amendments

### Problem

`tiny_oracle.compact` compiled and its native Rust `clear`, `set`, and `get` matched TypeScript state and total per-query gas, but no complete `recording` methods existed. The recorder could write root Cells but could not represent `in_state`'s enum comparison and Cell read, witness-backed Bytes bindings, pure `public_key`, assertions, or the conditional `Maybe<Field>` return. A generated consumer therefore could not prepare a ledger-8 call from this representative contract. Recording only the final state would discard the ordered Gather reads and exact public transcript required by ZKIR.

### Before

```rust
// tests-rust-backend/tiny-oracle/lib.rs before this change:
let cleared = Contract::from(witness).clear(context)?;
let got = Contract::default().get(cleared.context)?;
// No recording.clear, recording.set, or recording.get method.
```

### Decision and after

Lower a supported complete circuit as ordinary typed Rust control flow over a `RecordingFrame`. An internal Boolean circuit call is expanded through its typed arguments and return expression; enum/Bytes equality records an actual Cell read before comparing values. `Assert` returns the same `CompactError::AssertionFailed` as native emission. Bytes witnesses use `frame.witness`, so the private output is retained; a pure Bytes call invokes the generated pure function after its arguments are evaluated. A conditional return evaluates its condition once and carries `(frame, result)` out of each branch, recording only the branch actually taken.

```rust
let recorded = Contract::from(witness).recording().clear(context)?;
let call = prepare_call(recorded, CallSpec::new("clear", verifier, (), rand))?;

// Representative generated body, shortened:
let (frame, state): (_, STATE) = ledger_slots::state.record_read(frame)?;
if !(state == STATE::set) {
    return Err(runtime::CompactError::AssertionFailed(message.to_owned()));
}
let (frame, sk) = frame.witness(|context| witnesses.private_secret_key(/* view */));
let apk: FixedBytes<32> = pure_circuits::public_key(sk.clone())?;
let (frame, authority): (_, FixedBytes<32>) = ledger_slots::authority.record_read(frame)?;
if !(apk == authority) { /* same Compact assertion error */ }
let frame = ledger_slots::authority.record_write(frame, FixedBytes::<32>::default())?;
```

For `get`, the generated `if` branch calls `ledger_slots::value.record_read(frame)` and `pure_circuits::some`; the other branch calls `pure_circuits::none` without a value read. Both return a typed `Maybe` with their frame. The generated contract remains a normal Rust crate; no body-wide macro or handwritten VM opcode stream is added.

### Alternatives and rationale

A final-state diff cannot reconstruct `Idx`/`Popeq` reads or prove against Compact's public transcript. Calling native `clear`/`get` and guessing its trace afterward is also insufficient. A specialized `tiny` emitter would pass one oracle while hiding the language rules. The chosen lowering follows the closed stateful IR and typed slots; unsupported whole-circuit shapes still receive no recorded method.

### Emitter and runtime ownership

`tools/compact-rust-backend/src/recorded.rs` lowers `StateAction::Assert`, witnessed Bytes and pure Bytes local bindings, internal Boolean `Expr::Call`, Cell-backed `Expr::Equal`, and `StateReturn::Expression` with an `If` whose branches call typed pure circuits. `lib.rs` supplies the validated pure-circuit declaration map to the return lowering. `runtime-rs` retains ownership of Cell VM programs, ordered Verify operations, Gather observations, private outputs, gas and transaction partitioning. This decision requires no runtime ABI, private IR schema, or proc-macro change. The new return lowering deliberately handles the validated pure-call branch shape; other conditions and expression forms remain gated out.

### Verification and risks

The local `tiny-oracle` generated-crate test executes clear/set/get in native and recorded forms, compares state and result to the pinned TypeScript oracle, matches total per-query gas in all four dimensions and private outputs, compares every normalized public Verify operation to the TypeScript public transcript, replays the trace against the initial state, and checks absent/present `Maybe` branches. It also compares native and recorded failures for overwrite, unauthorized clear, and clear-while-unset. The 51 renderer and four CLI tests pass. Offline proof/application and full fixture/workspace gates are pending in this proposal state. One-query replay gas differs from the compiler's sum of separate queries; state replay and per-query gas parity are tested separately, consistent with ADR-0010. Other unsupported control flow and nested ledger paths remain open.

### Tracking and delivery

- Focused issue: [#112](https://github.com/MediaNoxLabs/compact/issues/112); parent issues: [#104](https://github.com/MediaNoxLabs/compact/issues/104) for `tiny` acceptance/transcript, [#107](https://github.com/MediaNoxLabs/compact/issues/107) for complete recorded methods, [#105](https://github.com/MediaNoxLabs/compact/issues/105) for proof/application.
- Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2); all four issues are assigned.
- Local commit: pending validation and commit; branch `codex/rust-backend-ast` remains local.
- Delivery state: proposed; update this record with proof result and commit before claiming delivery.

### Amendments

Append dated evidence or a superseding decision here; retain the original rationale.



### Delivery amendment — 2026-10-02: `tiny` complete recorded flow

**Status and decision.** Accepted for the supported closed-IR shapes, with the broader language surface still partial. The proposal above is retained as the original rationale. Local conventional GPG-signed/DCO commit `af62a9dbd5f1731a096c6e4d808dead7a89f3d59` adds the generated `clear`, `set`, and `get` methods from the unchanged `tiny_oracle.compact` fixture; the generated crate exposes witnessed calls through `Contract::from(witness).recording()` and unwitnessed `get` through `Contract::default().recording`.

**Proof and acceptance evidence.** The focused generated-crate suite passes native/recorded state, result, total per-query gas in all four dimensions, private witness output, and exact normalized public Verify-operation comparison against the TypeScript oracle for `clear`, `set`, and present `get`. Unset `get` returns the native `Maybe::none` and records only the state read. Replay reproduces final state. Native and recorded assertions agree for overwrite, unauthorized clear, and clear while unset. The packaged `compactc --target rust --consumer --proof` gate emits buildable Cargo output, ZKIR and keys, then replays, partitions, proves, verifies, validates and applies four `tiny` calls: clear, set, present get, absent get. The full offline ledger-8 gate now covers 44 call shapes. All 130 fixture outputs are current (0 stale/failed); 51 renderer and four CLI tests, three focused `tiny` tests, full Cargo workspace check, formatting and diff checks pass. Compiler 0.31.133, private IR schema 8, runtime ABI 4; the runtime and macro crates are unchanged.

**Gas interpretation and limits.** `RecordedCircuitResult.execution.gas_cost` is the sum of the separate generated VM queries and matches the TypeScript per-query sum; replaying the concatenated Verify program as one query has a different gas cost, so the replay test checks state while the per-query comparison checks gas. This is the explicit ADR-0010 contract, not a mismatch hidden by normalization. The conditional return lowering is currently limited to validated pure-call branches; Bytes pure local bindings use typed coerced arguments. Other nested control-flow/expression forms remain gated out. Constructor query topology/gas comparison, other high-risk oracle flows, wallet/node submission, package release and remote CI remain M2 work. See focused [#112](https://github.com/MediaNoxLabs/compact/issues/112), parent [#104](https://github.com/MediaNoxLabs/compact/issues/104), [#107](https://github.com/MediaNoxLabs/compact/issues/107), and [#105](https://github.com/MediaNoxLabs/compact/issues/105). The branch remains local; no remote CI pass is claimed.
