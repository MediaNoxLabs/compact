---
id: RUST-ADR-0084
alias: ADR-0084
title: "Record Boolean ledger observations inside stateful bindings"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "collections", "gas", "parity"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b06352431b0fdf0289cd783740a816e126c5ba3d9377cb64f51e9bd3ffb4db13
---
# RUST-ADR-0084 — Record Boolean ledger observations inside stateful bindings

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted Boolean Set/Map emptiness and Cell observations in ordered bindings, with five measured complete APIs rather than the ten additional streaming candidates. Summed TS query costs are the comparison basis; last-query wrapper gas and replay gas are not interchangeable. Deeper streaming composition and independent asset-close evidence were later work, not delivered by this decision alone.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#185 closure](https://github.com/MediaNoxLabs/compact/issues/185#issuecomment-6017544055). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`00e2b78c`](https://github.com/MediaNoxLabs/compact/commit/00e2b78c5e96c20817a9fc2513913e0a4b540bba) · [`08e0c391`](https://github.com/MediaNoxLabs/compact/commit/08e0c3915398e6ef94e66bf24644e705fb92e694) · [`2895c986`](https://github.com/MediaNoxLabs/compact/commit/2895c98603b04d3a9fcb9a2a94ad0fba23a30ece) · [`97fb64be`](https://github.com/MediaNoxLabs/compact/commit/97fb64be9eadeac734958c49a856b52ef24c0aa9). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 84
status: accepted-partial
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/185
```

## Historical decision and amendments

### Problem and proof-aware measurement

At integrated local HEAD `08e0c3915398e6ef94e66bf24644e705fb92e694`, the immutable `${LOCAL_EVIDENCE}/adr81-82-compactc` reports capability schema 3 for the 137-source fixture corpus: 185/333 exported Rust circuits have recorded/observed APIs, 101/286 `proof_required=true` circuits still lack them, and 47 proof-false exported circuits are native-only. Forty-three of the 101 proof-required gaps first report `unsupported_action` at `StateAction::Let`. This proposal isolates Boolean ledger observations bound inside an action, where the native target already executes a ledger query but recorded lowering does not admit the binding expression.

Four exact first-blocker circuits bind Set/Map emptiness, then write the Boolean into a Cell; all four are `proof_required=true`:

| Source | Circuit | First binding | Next action |
|---|---|---|---|
| `nested_collection_query_write.compact` | `check_set_empty` | `Boolean = SetIsEmpty(seen)` | CellWrite `setEmptyFlag` |
| same | `check_map_empty` | `Boolean = MapIsEmpty(table)` | CellWrite `mapEmptyFlag` |
| `set_size_oracle.compact` | `check_set_empty` | `Boolean = SetIsEmpty(s)` | CellWrite `flag_set` |
| same | `check_map_empty` | `Boolean = MapIsEmpty(m)` | CellWrite `flag_map` |

Ten exact `proof_required=true` first-blocker circuits in `ternary_cond_oracle.compact` bind `Boolean = CellRead(flag)` before another operation: `streamIncrement`, `streamCompareEq`, `streamCallPure`, `streamVectorElement`, `streamNativeArg`, `streamStructMember`, `streamCallWitness`, `streamConstAnnotated`, `streamAssertEq`, and `streamNestedIf`. The first reported `Let` gate does **not** prove these ten become recorded after Boolean CellRead is added. Their later IR includes conditional Uint/Field expressions, vector construction, hash-to-curve, struct members, witness arguments and assertions. A fresh post-change schema-3 report must name the next blocker for each; no ten-circuit gain is promised.

Other first-bound Let clusters remain separate: six `asset_registry_oracle` OpaqueString parameters; four `call_arg_declared_type` persistentCommit/hash Bytes expressions; three Uint casts and two Uint conditionals; two `zerocash_oracle` struct witness results; two map-lambda vector transformations; and smaller struct/pure-call shapes. The zero-cash witness path has further Merkle, crypto and opaque operations, so broadening witness return types here would overstate this slice.

### Developer-facing before and after

Today the generated crate has native `check_set_empty` and `streamIncrement`, while recording/typed observed-call methods are missing:

```rust
let native = ledger_contract::check_set_empty(context)?;
let next = ledger_contract::streamIncrement(native.context)?;
// No recorded::check_set_empty, recorded::streamIncrement,
// check_set_empty_call or streamIncrement_call on the typed recording handle.
```

The intended Set/Map binding path exposes a complete recorded program without making the developer assemble VM operations:

```rust
let checked = ledger_contract::recorded::check_set_empty(context)?;
let replay = checked.public.initial().query(
    checked.public.verify_ops(), None, &checked.execution.context.cost_model,
)?;
assert_eq!(replay.context.state.get_ref(), checked.execution.context.query.state.get_ref());

let observed = contract.recording.check_set_empty_call(&confirmed_state, private_state)?;
```

For a streaming Cell read, a complete method is emitted only once its following expression and actions also lower truthfully:

```rust
let step = ledger_contract::recorded::streamIncrement(context)?;
// Its ordered program begins with flag.record_read(frame), followed by
// the branch-selected Counter/Cell updates when those shapes are supported.
```

Conceptually, the generated body should bind the observed value through a typed slot and then reuse it in the next action:

```rust
let (frame, empty): (_, bool) = crate::ledger_slots::seen.record_is_empty(frame)?;
let frame = crate::ledger_slots::setEmptyFlag.record_write(frame, empty)?;
// For flag.read(): let (frame, flag): (_, bool) = crate::ledger_slots::flag.record_read(frame)?;
```

### Decision and ownership

Extend the existing `boolean_expression` path invoked by `StateAction::Let` to recognize exactly `Expr::SetIsEmpty`, `Expr::MapIsEmpty`, and Boolean `Expr::CellRead`. Check the declaration kind and index before emission, and check `CellRead`'s declared type is Boolean. Emit the existing typed `SetSlot::record_is_empty`, `MapSlot::record_is_empty`, or `CellSlot<bool>::record_read` call at the source-order position; retain the returned `frame` and bind its observed Boolean for the nested action. Preserve the current first-definite-failure schema-3 reason, including a new path if the next expression is unsupported. Do not fall back to native execution or synthesize VM opcodes/string Rust.

The runtime already owns all three slot methods, and `RecordingFrame` delegates to pinned midnight-ledger query/program primitives. `read_cell` uses the actual ledger `GatherEvent::Read` FAB value for Verify operations, preserving alignment and transcript. Set/Map emptiness use the existing ledger `size == 0` observation path. The midnight-zk/ZKIR stack owns proof encoding, validation and application. No runtime API or ABI bump, private IR change, or capability report schema change is expected; rebase onto the integrated Merkle ABI if that independent slice lands first.

### Acceptance and risks

1. Save this proposed ADR and create its focused `MediaNoxLabs/compact` issue in `rust-backend-v2` before code. Re-run the immutable integrated schema-3 receipt at the implementation base; treat the four Set/Map and ten CellRead rows as candidates, not gains.
2. In renderer tests, assert declaration kind/index/type checks, source-order read before write, one observation per source read, and accurate reasons after a successfully lowered binding encounters a later unsupported expression. Avoid speculative frame steps leaking from rejected branches.
3. Run the TypeScript oracle captures `capture-nested-collection-query-write.mjs` and `capture-ternary-cond-oracle.mjs` from the same source/compiler revision, then compare native versus recorded state, result, ordered Verify ops, private transcript outputs and all four gas dimensions. Test empty and seeded nonempty Set/Map; test true and false Cell flags and both ternary branches. Existing checked JSON fixtures are a starting reference, not fresh per-call gas/transcript proof.
4. Refresh all 137 fixtures and schema-3 report invariants. Measure the exact four and ten rows individually; report any second blocker and zero-regression count. Require strict `--rust-require-recording` only for contracts whose entire proof-required export set is supported.
5. Compile a standalone generated crate and run pinned ZKIR source-to-proof-to-ledger validation/application for representative Set and Map emptiness calls; add a CellRead/conditional proof only after that full path is supported. Run targeted Rust 1.99 Clippy and local packaged compiler/consumer/proof gates. Remote CI remains deferred under the user's policy.

Main risks are duplicate reads, wrong query order, incorrect Boolean FAB/Verify alignment, changed gas metering, and incorrectly advertising a complete streaming trace after clearing only its first Let gate. Typed witness expression lowering, general conditional value lowering, OpaqueString ADT paths and persistentCommit/hash are separate decisions.

### Tracking

- Predecessors: [ADR-0079 — Explain missing Rust recording capabilities from typed lowering](0079-explain-missing-rust-recording-capabilities-from-typed-lowering.md), [ADR-0081 — Record composite Cell values through typed slots](0081-record-composite-cell-values-through-typed-slots.md), [ADR-0082 — Gate Rust recording by compiler proof applicability](0082-gate-rust-recording-by-compiler-proof-applicability.md).
- Focused issue: [#185](https://github.com/MediaNoxLabs/compact/issues/185), assigned to `rust-backend-v2`.
- Delivery: accepted-partial at local signed/DCO `8aa72953`; five proof-capable gains, with ten streaming candidates still blocked downstream.


### Local delivery — 2026-10-05

Conventional GPG-verified/DCO commit `8aa729530e79ba04392bf12959cce13dd440c369` implements the narrow decision on integrated ABI36/schema-3 base `2895c986`. It reuses `CellSlot<bool>::record_read`, `SetSlot::record_is_empty`, and `MapSlot::record_is_empty`; no runtime method, ABI 36, private IR schema 8, capability schema 3, or supported generated Rust body changes. Type and index gates remain explicit.

Across all 137 sources / 333 exported circuits, recorded plus observed availability rises **186→191**, with **zero losses**. Among 286 compiler proof-required circuits, available rises **186→191** and gaps fall **100→95**. The five exact gains are `set_size_oracle.check_set_empty`, `.check_map_empty`, `nested_collection_query_write.check_set_empty`, `.check_map_empty`, and `asset_registry_oracle.close` (Boolean CellRead assertion). All ten named `ternary_cond_oracle.stream*` first-blocker candidates remain unavailable; strict diagnostics now expose later Let/Call/WitnessCall/Assert failures. This is a truthful five-circuit gain, not a streaming implementation.

Fresh `capture-boolean-observation.mjs` TypeScript captures pin Set/Map empty and seeded nonempty ordered Verify-op shapes, query costs in all four gas dimensions, and zero private transcript outputs. Rust native and recorded costs match the sum of TypeScript VM query costs; recorded state/effects match native, full contract states match existing TypeScript captures, and replay reproduces state/effects. The generated TypeScript wrapper reports only the last query cost for these multi-query circuits, and one flat replay query has different metering from stepwise execution, so neither value is substituted for the summed native/recorded circuit gas. `asset_registry.close` additionally passes native/recorded/replay state/effects and true-then-false assertion behavior, but has no independent per-call TS transcript or proof in this slice.

Local gates: 68 renderer tests; focused Set/Map and asset fixture tests; all 137 generated fixtures and schema-3 reason invariants fresh; targeted Rust 1.99 all-target Clippy with `-D warnings`; strict `compactc --rust-require-recording` with pinned ZKIR 2.1 for `set_size_oracle`; standalone generated crate check with transaction feature; and independent `check_set_empty` plus `check_map_empty` source-to-proof-to-ledger validation/application. An unsupported ternary source still fails strict compilation with typed source/path reasons. Parent integration will run its combined Nix/full local gate. No push or remote CI.

Remaining in #185: deeper conditional/witness/native expression support for the ten streaming circuits, true/false CellRead with a complete proof-required streaming circuit, richer negative kind/index renderer cases, independent asset-close TS/proof evidence, and combined local plus eventual same-revision remote CI. The issue remains open.


### Main integration checkpoint — 2026-10-05

Cherry-picked into the main local branch as GPG/DCO `97fb64be`. The combined source inventory at HEAD `00e2b78c` and frozen compiler `${LOCAL_EVIDENCE}/adr86-integrated-compactc` retains the five ADR-0084 proof-required gains with no regression. Focused `set_size_oracle`, `asset_registry_oracle` and `ternary_cond_oracle` generated crate gate passed in `${LOCAL_EVIDENCE}/compact-local-focused-00e2b78c/receipt.json`. The later ADR-0086 and extra source cohort mean current total counts are 198/293 proof-required available and 95 gaps; do not attribute all of them to this ADR. The representative Set/Map source-to-proof-to-ledger check is in the isolated delivery evidence above. Full workspace/Nix acceptance for the combined head remains separate.
