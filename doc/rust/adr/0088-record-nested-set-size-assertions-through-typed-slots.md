---
id: RUST-ADR-0088
alias: ADR-0088
title: "Record nested Set size assertions through typed slots"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler", "ir", "recording", "collections", "compatibility"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: d0b4f62b3e0bed1833da5f57749e51deae361d85547f3e15c82e11096cb83715
---
# RUST-ADR-0088 — Record nested Set size assertions through typed slots

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted nested typed Set.size observations and bounded literal comparisons, plus the measured Boolean and Field-local forms needed by set_field. The final integration assigns private schema10, replacing the proposal's no-bump expectation. It delivered one recorded API and compile progress for other sources; its local record explicitly does not claim a set_field proof.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#190 closure](https://github.com/MediaNoxLabs/compact/issues/190#issuecomment-6017552137). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`251075dc`](https://github.com/MediaNoxLabs/compact/commit/251075dc6c5f57c748d405a332dbe7df95445554) · [`2ee29239`](https://github.com/MediaNoxLabs/compact/commit/2ee292399b5dc20b0ac682ba659db20c921d3a98) · [`e1ba6987`](https://github.com/MediaNoxLabs/compact/commit/e1ba69877e1db576b25dd402f6c5c4e241f6152a). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 88
status: proposed
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/190
```

## Historical decision and amendments

### Problem and measured scope

The checked ADT Set source cohort in [#188](https://github.com/MediaNoxLabs/compact/issues/188) contains five TypeScript-positive sources. On immutable integrated `${LOCAL_EVIDENCE}/adr84-issue184-compactc` (SHA-256 `cb2d1d45442ac80035f89a43b49218e5b2f78a0fa031bccfaa6e9583456b50e9`), `set_struct.compact` Rust-compiles but exports no circuit. The other four Rust targets reject at `assert(c.size() == 0)` following `c.resetToDefault()`, with `Rust backend does not yet support this nested ledger query`. TypeScript compiler `contract-info.json` marks their five exported circuits `pure:false, proof:true`: `set_enum.test`, `set_field.test`, `set_qualified_coin_info.test_QualifiedShieldedCoinInfo`, `.test_ShieldedCoinInfo`, and `set_vector.test`. These are compiler acceptance and proof-applicability facts, not existing Rust recorded API rows.

The shared first failure is in `compiler/rust-ir-passes.ss` `stateful-expression-ir`: `public-ledger Set.member` and `Set.isEmpty` are modeled, while `Set.size` is modeled only in top-level `stateful-return-ir`. `Expr` lacks a typed nested `SetSize` case, though `StateReturn::SetSize`, `SetSlot::size`, and `SetSlot::record_size` already exist. A first-match compiler fix alone could expose later recording blockers: `recorded.rs::boolean_expression` does not currently lower direct `SetIsEmpty` or equality involving `SetSize`, and the four contracts include Set member checks and typed Set keys. No four-circuit capability gain is promised from syntax acceptance alone.

### Developer-facing before and after

Today `compactc --target rust examples/adt/tests/set_field.compact OUT` stops at the nested query, so a Rust consumer cannot call this same source:

```rust
// No generated set_field crate for the TypeScript-positive source.
// let native = set_field::test(context)?;
// let recorded = set_field::recorded::test(context)?;
```

The intended generated crate exposes ordinary typed contract APIs once the whole circuit lowers:

```rust
let native = set_field::test(context)?;
let recorded = set_field::recorded::test(context)?;
let replay = recorded.public.initial().query(
    recorded.public.verify_ops(), None, &recorded.execution.context.cost_model,
)?;
assert_eq!(replay.context.state.get_ref(), recorded.execution.context.query.state.get_ref());
let observed = contract.recording.test_call(&confirmed_state, private_state)?;
```

The generated implementation threads `context` or `frame` through existing typed Set slots in source order. A nested size assertion conceptually becomes:

```rust
let step = crate::ledger_slots::c.size(context)?;
context = step.context;
total_cost += step.gas_cost;
let size = runtime::BoundedUint::<18446744073709551615>::new(step.result as u128)?;
if !(size == runtime::BoundedUint::<18446744073709551615>::new(0)?) {
    return Err(runtime::CompactError::AssertionFailed("Size should be 0".into()));
}
// Recorded path: let (frame, size): (_, u64) = c.record_size(frame)?;
```

The exact generated syntax may use the renderer's existing bounded-Uint constructor and local names. The invariant is one metered Set size observation per source call, before the assertion, with identical ordered VM query and failure behavior. No handwritten VM sequence belongs in the crate.

### Decision and ownership

1. Add a typed `Expr::SetSize { field, index }` in private schema-8 IR and lower `public-ledger Set.size()` with zero arguments from `stateful-expression-ir`. The existing typed field/index checks and source span diagnostics remain. The native renderer emits `SetSlot::size(context)`, threads its returned context and gas once, and converts `u64` to Compact `Uint<64>` with the same semantics as top-level `StateReturn::SetSize`.
2. Extend the recording Boolean assertion path for exactly the shapes observed after fresh compiler lowering: direct `SetIsEmpty` and `SetSize == Uint<64> literal` (either operand order), using `SetSlot::record_is_empty` and `SetSlot::record_size`. Preserve read/query order and use the observed value in the assertion. Keep unrelated expressions unavailable with the first definite schema-3 reason and no native-only fallback. Do not advertise a recorded API until every action and return of the exported circuit lowers.
3. Reuse `runtime-rs/src/slots.rs` and pinned midnight-ledger Set query/program primitives. No runtime API or ABI bump is planned. Any broader key encoding or expression form that appears in the enum/vector/qualified-coin sources needs a separate measured extension, not a guessed mapping. Keep compiler proof metadata authoritative; no private IR schema bump is planned for this paired compiler/renderer change unless compatibility checks require it.

### Acceptance and limits

Start with `set_field.compact` as the smallest full Set mutation/query/assertion case. Compare fresh TypeScript and Rust native results, state, ordered VM operations, private output and all four gas dimensions at empty and populated Set states, including an assertion failure. Then require recorded replay and pinned source-to-proof-to-ledger validation/application for at least that contract before calling the slice proof-ready. Inspect all four source compile outcomes and schema-3 capabilities after each change; classify new blockers truthfully. Refresh generated fixtures and run focused Rust 1.99 tests/Clippy plus the relevant local compiler/consumer gate using an isolated `CARGO_TARGET_DIR` and immutable compiler snapshot. The full 190-source inventory, 124 unassessed contract exports, other ADT query forms and remote CI remain separate.

Risks: duplicating a metered size query while comparing, losing query-before-assert order, converting `u64` to the wrong Uint bound, recording an observed Boolean without its ledger Verify operation, and claiming proof support after compiler acceptance but before replay/proof. The prior `set_size_oracle` and `set_boolean` fixtures provide typed slot and transcript precedents, but their top-level result shapes do not cover this nested assertion automatically.

### Tracking

- Predecessors: [ADR-0079 — Explain missing Rust recording capabilities from typed lowering](0079-explain-missing-rust-recording-capabilities-from-typed-lowering.md), [ADR-0084 — Record Boolean ledger observations inside stateful bindings](0084-record-boolean-ledger-observations-inside-stateful-bindings.md), [ADR-0086 — Record conditional Counter amounts after Boolean observations](0086-record-conditional-counter-amounts-after-boolean-observations.md).
- Source coverage: [#188](https://github.com/MediaNoxLabs/compact/issues/188).
- Focused implementation issue: pending.
- Delivery: proposed only; no code, proof or gate result claimed yet.


Focused implementation issue: [#190](https://github.com/MediaNoxLabs/compact/issues/190), assigned to `rust-backend-v2`.

### Local implementation evidence (2026-10-05)

The isolated ADR-0088 branch adds typed nested `Expr::SetSize`, native `SetSlot::size`, and recorded `SetSlot::record_size` for Uint<64> literal comparisons. It also records Boolean literal comparisons around Set membership/emptiness and effect-free Field literal local bindings. No runtime change was needed. The branch still uses private IR schema 8 / ABI 36; ADR-0085 landed in main with schema 9 / ABI 37 concurrently, so integration must assign the next private schema version (likely 10) and refresh generated fixtures against the combined compiler. This supersedes the earlier no-schema-bump expectation.

Using isolated Scheme `${HISTORICAL_NIX_STORE}/4aykpg4h700q97z4risdyv62y2zs14xi-compactc/bin/compactc-scheme` and local Rust CLI, the checked ADT Set cohort is 5/5 TypeScript acceptance, 4/5 Rust acceptance, 1/5 Rust rejection. `set_field.test` is `proof_required:true`, `recorded:true`, and `observed_call:true`; its generated crate checks and a temporary source consumer passes native execution, recorded execution and exact ledger replay with equal final state. `set_enum.test` and `set_vector.test` now compile but remain recorded unavailable at `StateAction::Let`; `set_qualified_coin_info.compact` rejects at line 58 on an additional ledger operation. This is a one-circuit recording gain, not five. Renderer tests pass 70/70 and the checked source receipt passes. No pinned proving key or end-to-end ZK proof was generated for `set_field` in this isolated slice; full proof validation remains an integration gate before claiming proof-proven parity.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/190.

### Root integration — schema 10, 2026-10-05

Root signed/DCO `2ee29239` integrates this slice on ABI37 and assigns private IR schema10. The Nix `compactc` package built successfully at `${HISTORICAL_NIX_STORE}/g7igz5r3rvdmabz28387fcw7h5y5k6cl-compactc`; direct `set_field` compile emits schema10 and records `test`. Renderer 72/72 passed. Signed/DCO `251075dc` refreshes two fixture libraries with only internal binding renames; all 138 fixture outputs match, and the 3-crate focused gate passed at `${LOCAL_EVIDENCE}/compact-local-focused-schema10-e1ba6987/receipt.json`. Inventory `${LOCAL_EVIDENCE}/adr88-schema10-inventory.json` now has 190 sources, 929 declarations, 159 Rust compiled roots, 203/296 proof-required APIs available, 93 known proof gaps and 120 unassessed contract exports. Compared with ADR90's 202/293, this adds three assessed ADT Set circuits, one available recorded call and two new measured gaps; it does not regress existing capability rows. Pinned ZK proof for `set_field.test` remains unverified.
