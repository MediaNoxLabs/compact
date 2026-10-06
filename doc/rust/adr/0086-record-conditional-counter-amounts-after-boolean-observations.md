---
id: RUST-ADR-0086
alias: ADR-0086
title: "Record conditional Counter amounts after Boolean observations"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "counter", "control-flow", "parity"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 4fd0a082a2129487a5e37daac63160e2f4923429bb76a19b8abc6b86dbea988c
---
# RUST-ADR-0086 — Record conditional Counter amounts after Boolean observations

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the bounded Uint16 conditional Counter amount path, preserving a single lazy selection after an observed Boolean and existing slot primitives. Three measured complete APIs were gained; the historical proof cases include both parameter branches but only the default-false ledger flag. Broader expressions and the true seeded flag proof were separate follow-ups, including ADR0232.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#187 closure](https://github.com/MediaNoxLabs/compact/issues/187#issuecomment-6017547274). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`00e2b78c`](https://github.com/MediaNoxLabs/compact/commit/00e2b78c5e96c20817a9fc2513913e0a4b540bba) · [`b6545561`](https://github.com/MediaNoxLabs/compact/commit/b6545561a2646b5974babc6c0587a28de40b6f4b). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 86
status: accepted-partial
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/187
```

## Historical decision and amendments

### Problem and measured rank

At local signed ADR-0084 commit `8aa72953` (ABI36/private IR8/report schema3), 137 Compact sources expose 333 exported circuits. Compiler proof metadata identifies 286 as `proof_required=true`; 191 have complete recorded and observed-call APIs and 95 remain unavailable. The ten `ternary_cond_oracle.stream*` Boolean CellRead first-blocker candidates from ADR-0084 gain no complete API. Boolean CellRead now lowers, and their next failures are concrete:

| Circuit | First downstream IR failure | Further path |
|---|---|---|
| `streamIncrement` | `StateAction::Let` at `actions[0].action.actions[0]`: `UnsignedCast<65535>(If(f,3,4))` | second `Let` casts `If(f,10,20)` to Uint<64> before `wideCell` write |
| `streamCompareEq` | `Let` at `actions[0].action`: `Equal(1,If(f,1,0))` | Field ternary write binding |
| `streamCallPure` | `Expr::Call` at `actions[0].action.bindings[0].value` | pure `idf` Field argument contains conditional Field cast |
| `streamVectorElement` | `Let` at `actions[0].action`: Vector<Field> with two conditional elements | composite typed value construction |
| `streamNativeArg` | `Let` at `actions[0].action`: hashToCurve of conditional Field | Jubjub projection |
| `streamStructMember` | `Let` at `actions[0].action`: Box struct with conditional Field | struct member projection |
| `streamCallWitness` | `Expr::WitnessCall` at `actions[0].action.bindings[0].value` | conditional Field witness argument |
| `streamConstAnnotated` | `Let` at `actions[0].action`: `UnsignedCast<65535>(If(f,1,2))` | Counter increment then constant Field Cell write |
| `streamAssertEq` | `StateAction::Assert` at `actions[0].action.actions[0]`: `If(f,Equal(1,1),Equal(2,2))` | later constant Field Cell write |
| `streamNestedIf` | `Let` at `actions[0].action`: nested Uint<4> `If` | Field cast/write and Counter increment |

Every row above is `proof_required=true` in authoritative `compiler/contract-info.json`, but only a complete recorded trace merits an API. The exact `UnsignedCast<65535>(If(Boolean,small literals))` subgraph is shared by `streamConstAnnotated`, `streamWrite` and `walkerInlineWrite`; the latter two are additional proof-true exported circuits. `streamIncrement` shares its first Counter amount but retains an independent Uint<64> write blocker. This is the smallest coherent next slice. No gain is claimed before running the full corpus and executable proof gates.

### Developer-facing change

Before, the generated crate exposes a native method for a conditional Counter amount, while `recorded::streamConstAnnotated`, `recorded::streamWrite` and their typed observed-call methods are absent:

```rust
let native = ledger_contract::streamWrite(context, false, Field::from(7_u64))?;
// No ledger_contract::recorded::streamWrite or typed streamWrite_call.
```

After a complete typed path is verified, the developer can record and prepare a call without manually assembling Counter VM ops:

```rust
let trace = ledger_contract::recorded::streamWrite(context, false, Field::from(7_u64))?;
let replay = trace.public.initial().query(
    trace.public.verify_ops(), None, &trace.execution.context.cost_model,
)?;
assert_eq!(replay.context.state.get_ref(), trace.execution.context.query.state.get_ref());
let call = contract.recording.streamWrite_call(&confirmed_state, private_state, false, Field::from(7_u64))?;
```

The emitter should evaluate the pure conditional once in a typed `u16` amount, then pass it to the existing `CounterSlot::record_increment` before subsequent writes:

```rust
let amount: u16 = if c { 1u16 } else { 2u16 };
let frame = crate::ledger_slots::ops.record_increment(frame, amount)?;
```

For `streamConstAnnotated`, the condition comes from ADR-0084's prior `CellSlot<bool>::record_read`, retaining source order. Untaken branches must stay lazy, and widening must enforce the Counter's `u16` limit. Do not turn this into general conditional expression lowering or call native code from the recorded path.

### Decision boundary and ownership

Extend only the `StateAction::Let` Counter amount source to recognize a typed `Expr::UnsignedCast` to max 65535 around a pure `Expr::If` with a Boolean parameter/local condition and representable small unsigned literal arms (including exact `Coerce` wrappers). Keep already supported direct Counter amounts byte-identical. Use the existing `CounterSlot::record_increment` and `RecordingFrame`/midnight-ledger counter program. There should be no runtime API, VM opcode, private IR schema, capability report schema or ABI change. Do not claim `streamIncrement` until its second Uint<64> binding is supported, and leave comparison, vector, native curve, struct, witness and Assert branches to later ADRs.

### Acceptance and risks

1. Run a fresh immutable compiler receipt across all 137 sources at the implementation base. Record exact gains/losses and new first failures; check `proof_required` joins, schema-3 reason invariant and strict diagnostics. Expected complete candidates are `streamConstAnnotated`, `streamWrite`, `walkerInlineWrite`; `streamIncrement` is expected to reach its second Uint<64> blocker, but these are hypotheses until measured.
2. Capture TypeScript true/false conditional branches and, for `streamConstAnnotated`, a Boolean Cell observation before Counter increment. Compare source-order Verify-op shape, private output, full ledger state and all four summed query-cost dimensions with Rust native, recorded and replay. Include zero and nonzero Counter state. Validate source-order read before increment/write and one Cell observation.
3. Compile a standalone generated consumer of a source containing only supported exports. Prove, verify, validate and apply at least one conditional Counter call with pinned ZKIR 2.1 and midnight-ledger 8.0.3. Full `ternary_cond_oracle` strict mode should still reject its remaining proof-required gaps truthfully.
4. Run focused renderer and consumer tests, all 137 fixture freshness checks, targeted Rust 1.99 Clippy and relevant local package/proof gates. Combined full Nix/CI stays with the parent integration; no push or remote CI now.

Risks: narrowing/widening mistakes across Compact Uint maxima, duplicated or eagerly evaluated ternary arms, reading a Cell twice, and treating a first-binder improvement as a complete proof trace. The generated TypeScript wrapper may report only its last query's gas; compare summed VM query costs as in ADR-0084.

### Tracking

- Predecessor: [ADR-0084 — Record Boolean ledger observations inside stateful bindings](0084-record-boolean-ledger-observations-inside-stateful-bindings.md).
- Delivery: accepted-partial at signed/DCO `84e7567b`; three proof-capable Counter call gains with deeper stream work remaining.


- Focused [#187](https://github.com/MediaNoxLabs/compact/issues/187) is in `rust-backend-v2`.


### Local delivery — 2026-10-05

Conventional GPG-verified/DCO local commit `84e7567ba8900cc79fcdc150952c452acde3885a` implements the narrow Uint16 conditional Counter amount decision on top of ADR-0084 `8aa72953`. Across the unchanged 137-source/333-export corpus, complete recorded and observed-call availability moves **191→194/333**; among 286 compiler proof-required exports, availability moves **191→194** and gaps fall **95→92**. The three exact gains are `ternary_cond_oracle.streamWrite`, `.walkerInlineWrite` and `.streamConstAnnotated`; no circuit loses an API. `streamIncrement` advances to the second `StateAction::Let` at `actions[0].action.actions[1]`, where `UnsignedCast<Uint64>(If(f,10,20))` remains unsupported. The other previously ranked stream failures remain separate.

Emitter code recognizes a `UnsignedCast<65535>` around a Boolean `If`, checked unsigned literal arms and exact `Coerce` wrappers; it emits one lazy typed Rust `u16` conditional passed to the existing `CounterSlot::record_increment`. It does not duplicate the preceding Cell read. Runtime ABI36, private IR8, capability schema3, ledger VM and ZKIR semantics are unchanged.

New TypeScript `capture-conditional-counter.mjs` captures six cases: `streamWrite` and `walkerInlineWrite` with false/true parameters, plus `streamConstAnnotated` with false/true ledger flag (the true flag is seeded by the same typed Cell write VM program used by the generated constructor). Native/recorded/replay state and effects match the captured full contract state and ordered Verify-op shape. Native and recorded gas match the sum of TypeScript VM query costs in all four dimensions; private transcript outputs are empty. Strict `compactc --rust-require-recording` accepts a checked-in minimal two-export source, and its standalone generated Cargo crate compiles with transaction support. Full ternary strict mode still rejects unrelated proof-required circuits with precise reasons.

Pinned ZKIR 2.1 on the original ternary source generates keys; local proof smoke replays, proves, validates and applies `streamWrite(false)`, `streamWrite(true)` and `streamConstAnnotated` with default false flag against midnight-ledger 8.0.3. The 68 renderer tests, focused ternary consumer tests, all 137 fixture freshness/reason checks, and targeted Rust 1.99 all-target Clippy with `-D warnings` pass. The combined full Nix/workspace checkpoint remains with parent integration; no push or remote CI.

Remaining in #187: Uint64 conditional binding for `streamIncrement`; broader Boolean/Field conditional, witness, vector, struct and Assert paths in the other stream circuits; a pinned proof for `streamConstAnnotated` with true ledger flag; and combined/same-revision CI. Issue stays open.


### Main integration checkpoint — 2026-10-05

Cherry-picked as GPG/DCO `b6545561` with no ABI change. The focused three-fixture gate at exact HEAD `00e2b78c` passed and records 7/33 raw APIs in those fixtures, including the three conditional Counter gains: `${LOCAL_EVIDENCE}/compact-local-focused-00e2b78c/receipt.json`. A frozen integrated compiler at `${LOCAL_EVIDENCE}/adr86-integrated-compactc` has SHA-256 `9c2d804ca8d3be5431d5d05497d1bc6fc47e8595775b00527f3e024571a87fa2`. The combined 190-source inventory `${LOCAL_EVIDENCE}/adr86-adtset-inventory.json` reports 198/293 proof-required APIs available, 95 gaps, 124 unassessed contract exports and zero compiler join mismatches. The isolated commit supplied TS true/false parity and three pinned proof applications; a full combined-head proof gate remains pending.
