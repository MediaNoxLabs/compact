---
id: RUST-ADR-0016
alias: ADR-0016
title: "Reuse recorded callee bodies within one frame"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 57c2645799399b983713612ecee4028371cf618d387d50ce9e6c5bbd741f15ab
---
# RUST-ADR-0016 — Reuse recorded callee bodies within one frame

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept the later contract-wide, module-level same-frame helper design for its supported Unit and Field call slices. Preserve the earlier caller-local experiment at 9ebdea47 as explicitly not adopted. Argument evaluation, fallible witness ordering, cycle/unsupported-call refusal and one shared frame remain required; source-growth and timing limits are not optimization claims.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#117 closure](https://github.com/MediaNoxLabs/compact/issues/117#issuecomment-6017426633). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`551b7066`](https://github.com/MediaNoxLabs/compact/commit/551b70660a411256272ac9cf1c30cb16af6eae0f) · [`98d33ca7`](https://github.com/MediaNoxLabs/compact/commit/98d33ca7d8ee52ab129d861f0604dc0826d929ca) · [`d6576650`](https://github.com/MediaNoxLabs/compact/commit/d6576650ab8be8e47509f06d9476a15cb25dce9e) · [`ed05c0d8`](https://github.com/MediaNoxLabs/compact/commit/ed05c0d88d21ca6c9cb01663b4d7c30fa41b7a3b) · [`ee76fd1d`](https://github.com/MediaNoxLabs/compact/commit/ee76fd1da7d02a0c51a85cc61fcec0eb0b3e2747). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 0016
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 117
```

## Historical decision and amendments

### Problem

A nested recorded circuit is expanded at every call site. In `tests-rust-backend/stateful-circuit-call/lib.rs`, `add_twice` contains two copies of `count.record_increment`, and `tests-rust-backend/witness-cell-write/lib.rs` repeats the witness and Cell write sequence in `write_nested_twice`. Repeated expansion can grow generated crates and makes the relationship between a Compact callee and its Rust implementation less obvious. The current generated public recorded functions each construct a new `RecordingFrame`; calling one from another would incorrectly restart the public trace.

### Before

```rust
// Current generated recorded add_twice, abbreviated.
let frame = RecordingFrame::new(context);
let frame = ledger_slots::count.record_increment(frame, amount)?;
let frame = ledger_slots::count.record_increment(frame, amount)?;
Ok(frame.finish(()))
```

### Proposed decision and after

Generate one private body helper for a supported recorded callee. It consumes and returns the *same* `RecordingFrame`, plus the circuit result. The public function alone creates and finishes the frame. This is a design probe, not delivered code:

```rust
fn __compact_record_add<Private>(
    frame: RecordingFrame<Private>,
    amount: BoundedUint<65535>,
) -> Result<(RecordingFrame<Private>, ()), CompactError> {
    let frame = ledger_slots::count.record_increment(frame, amount.value() as u16)?;
    Ok((frame, ()))
}

pub fn add_twice<Private>(context: CircuitContext<Private>, amount: BoundedUint<65535>)
    -> Result<RecordedCircuitResult<Private, ()>, CompactError>
{
    let frame = RecordingFrame::new(context);
    let (frame, ()) = __compact_record_add(frame, amount)?;
    let (frame, ()) = __compact_record_add(frame, amount)?;
    Ok(frame.finish(()))
}
```

For witnessed callees the helper also borrows `&W`, with `W: Witnesses<Private>`; it passes the existing metered witness view through the same frame. For value-returning circuits, the tuple carries the result. The emitter must preserve left-to-right argument evaluation and short-circuit errors before any `finish` call. Public `Contract<W>::recording` methods and their result type stay unchanged.

### Alternatives and rationale

- Continue inline expansion: simple but duplicates bodies at each nested call and may scale poorly for larger graphs.
- Call the generated public recorded function: it creates a second initial trace and finishes a separate result, so merging context/gas/Verify operations becomes subtle.
- Add a macro around whole circuit bodies: hides the Compact call graph and diagnostic ownership. ADR-0005 already favors ordinary Rust from typed IR.

The helper boundary mirrors the Compact call graph while reusing the ledger-8 `RecordingFrame` and slot methods. Select it only for calls that pass the existing closed IR completeness check; unsupported shapes must retain the current safe exposure behavior.

### Emitter and runtime ownership

`tools/compact-rust-backend/src/recorded.rs` owns helper selection, generation with `syn`, typed argument/result lowering, recursive call rejection, source location wrappers, and public wrapper generation. `prettyplease` formats the AST. `runtime-rs/src/recording.rs` already owns context, query gas, ordered private FABs and one public Verify trace; this proposal needs no new runtime method or procedural macro. Expected compatibility: private helper is generated implementation detail, public crate API unchanged, runtime/generated ABI 6 and private IR schema 8 unchanged. Reassess the ABI assertion if implementation changes a public type or runtime surface.

### Verification and risks

Use `stateful-circuit-call` and `witness-cell-write` first. Compare before/after generated code and measure line counts and warm package-only `cargo check` for tiny, election, zerocash and passport where applicable. Require result, private state/FAB order, serialized ledger state, all four gas dimensions, exact normalized public Verify sequence, injected-error short circuit, and TypeScript parity. Run the existing fixture freshness, oracle, rejection, external consumer and packaged offline proof/application gates. A shared helper must not capture the wrong witness, shadow arguments, change evaluation order, accept an incomplete trace, or recurse indefinitely. Local timing alone is not a production compile-time claim.

### Tracking and delivery

- Parent design: [ADR-0005 — Use a typed runtime DSL instead of a body-wide macro](0005-use-a-typed-runtime-dsl-instead-of-a-body-wide-macro.md) and [#110](https://github.com/MediaNoxLabs/compact/issues/110).
- Related replay boundary: [ADR-0001 — Record circuit execution as a replayable ledger trace](0001-record-circuit-execution-as-a-replayable-ledger-trace.md) and [#107](https://github.com/MediaNoxLabs/compact/issues/107).
- Focused issue: [#117](https://github.com/MediaNoxLabs/compact/issues/117), assigned to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Local commits: none for this decision yet. Proposal only; no branch push or remote CI claim.

### Amendments

- 2026-10-02: Proposed from generated-code design probe and current ABI-6 emitter inspection. Append a dated implementation amendment with exact before/after output, ownership, commit and test evidence. Preserve this proposal when revising the choice.

- 2026-10-02: Focused [#117](https://github.com/MediaNoxLabs/compact/issues/117) created and assigned to `rust-backend-v2`; the proposal remains unimplemented.


### Baseline measurement — 2026-10-02, ABI 6

Current generated `lib.rs` line counts before the helper change: `stateful-circuit-call` 260, `witness-cell-write` 391, `nested-witness-call-oracle` 283, `tiny-oracle` 525, `election-oracle` 998, `zerocash-oracle` 648, and `passport-dogfood` 4,532. These are source-size baselines only. The earlier ADR-0005 warm `cargo check` samples were taken at a different commit and must not be treated as before/after timing for this proposal.


### Probe amendment — 2026-10-02: local caller helper, validation in progress

A local uncommitted implementation in `recorded.rs` tested a conservative Unit-returning recorded callee helper. The generated `bump_twice` and witnessed `write_nested_twice` functions each define one private local function that consumes and returns the same `RecordingFrame`, then call it twice; the public function alone calls `RecordingFrame::new` and `finish`. The emitter preserves a `BoundedUint<65535>` argument at the helper boundary rather than lowering it early to `u16`, and adds witness bounds only when the callee needs them. A callee that itself contains a stateful call stays on the earlier inline path. This is one helper per generated caller, not yet a module-level helper shared by every caller. Runtime, public generated API, ABI 6 and IR schema 8 are unchanged in this probe.

The measured generated source grew: `stateful-circuit-call` 260→274 lines and `witness-cell-write` 391→392; `nested-witness-call-oracle` remains 283 after the nested-chain gate. Eleven focused fixture tests (2 + 2 + 7), backend build and scoped diff/format checks passed. The full packaged proof/application gate is running, and no final code delivery or compile-time benefit is claimed. This result supports an explicit call-boundary semantics probe, but does not establish the ADR’s intended size or maintainability advantage. Module-level reuse or a measured eligibility threshold remains required before broader adoption; value-returning nested calls remain inline.


### Experimental branch result — 2026-10-02

Conventional GPG-signed/DCO commit `9ebdea4733217198f3d02c7cfecbc105fc712c63` preserves the direct Unit-callee probe on local branch `codex/recorded-helper-probe`. It is **not adopted** on the milestone implementation branch `codex/rust-backend-ast`, which remains at `98d33ca7` for this decision. The unrelated worktree edit in `doc/ledger-adt.mdx` was not included. The branch was not pushed.

The experiment’s backend build, 11 focused fixture tests, scoped diff/format checks, and Nix packaged compiler/consumer/proof gate passed. The packaged gate replayed, partitioned, proved, verified, validated and applied 45 offline ledger-8 calls, including `bump_twice`, `add_twice` and `write_nested_twice`. The changed generated crates still grew 260→274 and 391→392 lines, while `nested-witness-call-oracle` stayed at 283. No before/after compile-time improvement was measured. This validates frame continuity for these examples, not the proposed size or maintenance improvement.

Next decision gate: test a module-level callee helper reused across exported callers, or define an eligibility rule based on measured repeated body size and call count. Require a positive consumer-readability/size result and the ADR acceptance evidence before moving code onto the milestone branch. Value-returning calls and nested callee chains remain inline; issue #117 stays open.



### Contract-wide design revision — 2026-10-03, ABI 11

The generated-code research agent inspected the current renderer and experimental commit `9ebdea47`. Keep the caller-local experiment as rejected evidence. A module-level implementation needs one contract-wide call-graph and eligibility pass before rendering public wrappers. It must inspect `StateAction::CircuitCall` and `Expr::Call` in action bindings, Field/Boolean expressions, assertions, and returns, reject recursive strongly connected components, and preflight reachable callees for complete recording support. Stable private helper order follows the compiler IR declaration order. Emit one helper per supported referenced callee; the public wrappers alone open and finish a frame. Do not expose a partial public wrapper if a reachable callee is unsupported.

Current generated `add_twice` repeats the body at each call site:

```rust
let frame = RecordingFrame::new(context);
let frame = ledger_slots::count.record_increment(frame, amount.value() as u16)?;
let frame = ledger_slots::count.record_increment(frame, amount.value() as u16)?;
Ok(frame.finish(()))
```

A proposed *module-level* helper keeps the Compact argument type at its boundary and is shared by every caller:

```rust
fn __compact_body_add<Private>(
    frame: RecordingFrame<Private>,
    amount: BoundedUint<65535>,
) -> Result<(RecordingFrame<Private>, ()), CompactError> {
    let frame = ledger_slots::count.record_increment(frame, amount.value() as u16)?;
    Ok((frame, ()))
}

// Within a generated public wrapper, after ordered argument temporaries:
let (frame, ()) = __compact_body_add(frame, amount)?;
let (frame, ()) = __compact_body_add(frame, amount)?;
Ok(frame.finish(()))
```

For a witnessed callee, add `witnesses: &W` and `W: super::TryWitnesses<Private>`; the helper uses the same `try_witness_metered` path. For a value-returning callee, return `(frame, value)` and bind the value at the call site. Evaluate every argument into a typed temporary in Compact order before moving the frame; preserve `?` short circuit and do not clone or finish the frame inside helpers. The old proposal's `W: Witnesses<Private>` is superseded by the ABI-10 fallible trait.

**Ownership:** `recorded.rs` gains a contract-level plan and shared `syn` helper generation; `lib.rs` collects planned helpers and wrappers. The existing `runtime-rs/src/recording.rs` frame and `slots.rs` operations suffice, so no runtime or public generated API change is proposed. Current compatibility is generated/runtime ABI 11 and private IR schema 8. `StateAction`/`Expr` do not carry call-site spans; use the callee/caller declaration location for errors until nested source spans are implemented in ADR-0008/#104.

**Measurement gate:** at current `551b7066`, generated `lib.rs` line counts are stateful-circuit-call 263, witness-cell-write 412, nested-witness-call-oracle 298, tiny 540, election 1,123, zerocash 770, passport 4,532. These replace the earlier ABI-6 baselines; no timing comparison across revisions is valid. Estimated source saving is `(k - 1)B - H - kC` for `k` call sites, body size `B`, helper overhead `H`, and call overhead `C`. A tiny body can still grow. Measure the same-head before/after lines and warm package-only `cargo check` for affected generated crates; inspect caller readability; preserve all result/state/FAB/gas/transcript and injected-error parity plus the packaged proof/application gate. Do not claim optimization or adopt the experiment until this evidence is positive.

**Status:** researched design only. No emitter/runtime code was changed for this revision; focused [#117](https://github.com/MediaNoxLabs/compact/issues/117) remains open in rust-backend-v2.


### Module-level Unit helper delivery — 2026-10-03, ABI 12

Local conventional GPG-signed/DCO commit `d6576650ab8be8e47509f06d9476a15cb25dce9e` on `codex/rust-backend-ast` adopts the narrower **Unit `StateAction::CircuitCall`** part of the contract-wide design. The isolated source commit was `43102208`; the cherry-picked commit above is the milestone branch identity. This is a generated-code readability and semantic-reuse decision, **not** a measured size or compile-time optimization. It amends the earlier adoption gate accordingly. Value-returning `Expr::Call` remains inline; the full proposal and [#117](https://github.com/MediaNoxLabs/compact/issues/117) remain open.

Before, a generated `bump_twice` repeats the callee's ledger action and a public `bump` has a separate copy:

```rust
let frame = runtime::recording::RecordingFrame::new(context);
let frame = crate::ledger_slots::count.record_increment(frame, 1u16)?;
let frame = crate::ledger_slots::count.record_increment(frame, 1u16)?;
Ok(frame.finish(()))
```

After, one private module-level body is reused by the public wrapper and nested callers, with the frame opened and finished only at the public boundary:

```rust
fn __compact_recorded_body_bump<Private>(
    frame: runtime::recording::RecordingFrame<Private>,
) -> Result<(runtime::recording::RecordingFrame<Private>, ()), runtime::CompactError> {
    let frame = crate::ledger_slots::count.record_increment(frame, 1u16)?;
    Ok((frame, ()))
}
let (frame, _) = __compact_recorded_body_bump(frame)?;
let (frame, _) = __compact_recorded_body_bump(frame)?;
Ok(frame.finish(()))
```

The witnessed `inner` helper takes `&W` with `W: TryWitnesses<Private>`, uses `frame.try_witness_metered(...)`, computes its typed `Field`, and calls `ledger_slots::value.record_write`; `middle` and `outer` preserve the same frame, witness FAB order, four-dimensional gas and Verify order. Typed call arguments are evaluated into ordered temporaries before the helper call. The emitter uses a shallow Unit-call scan, exact recorded-lowering preflight, stable declaration-order helper output and collision-safe private names. A regression creates a Compact circuit named like a preferred helper and checks unique Rust functions. Recursive calls keep source-located errors; malformed names return `RenderError` rather than panic.

**Ownership and compatibility:** only `tools/compact-rust-backend/src/{recorded,lib}.rs`, its README/test, and three generated fixtures change. Existing `runtime-rs/src/recording.rs`, slots, ledger-8 VM, derives and macros are unchanged. Generated public method signatures and behavior are unchanged. Generated/runtime ABI stays 12 and private IR schema stays 8; no migration is required. This uses `syn` AST emission and ordinary Rust functions, without a new macro or a new low-level DSL.

**Evidence and tradeoffs:** 54 renderer tests, four CLI tests, 14 focused consumer tests, 132 fresh generated fixtures, and two compiler rejection probes pass. The branch-specific prover gate and the exact combined milestone checkout both pass all 52 packaged offline ledger-8 replay, partition, proof, verification, validation and application cases, including `bump_twice`, `add_twice`, `outer` and witnessed writes. Clean-source `target/rust-runtime-abi12-clean.json` writes and verifies for `d6576650`, tree `29850020`, `dirty=false`, with 8 macro and 229 runtime archive entries; the unrelated documentation edit was restored byte for byte. Same-head generated file sizes grow: stateful call 263→276 (+13 lines), nested witness 298→317 (+19), witnessed Cell 412→413 (+1). Twenty spawned debug CLI medians were 17.36→20.59 ms for the call-bearing stateful case and 351.92→352.46 ms for passport; host/process noise limits precision, but the preflight adds work for call-bearing contracts. The 0.72→0.44 s warm Cargo check sample is too small/noisy to support a speedup claim. No generated-size or compilation-performance benefit is asserted.

**Open review gates:** value-returning Field/Boolean stateful calls (`outerValue`→`innerValue`) still use inline lowering; recursive/unsupported forms remain closed. A cached `RecordedBody` plan may remove duplicate preflight lowering if measured cost warrants it. Broader transcript/error parity, branch publication, remote CI, release and wallet/node submission remain outstanding under milestone 2. This dated amendment preserves the earlier rejected caller-local experiment and proposal history.


### Field-returning witnessed helper proposal — 2026-10-03, ABI 15

The current `nested-witness-call-oracle` generated `outerValue` inlines its internal `innerValue()` witness body at the Cell write site. The same fixture already has Unit `inner`/`middle` helpers. This is an exact value-returning instance of the open #117 rule, with a TypeScript private-state/FAB/state oracle and a recorded replay test. Before:

```rust
let (frame, __compact_witness_0) = frame.try_witness_metered(|context, meter| {
    witnesses.secret(context.witness_context_with(LedgerView {
        state: context.query.state.get_ref(), meter,
    }))
})?;
let frame = ledger_slots::value.record_write(frame, __compact_witness_0)?;
```

Proposed generated body:

```rust
fn __compact_recorded_body_innerValue<Private, W: TryWitnesses<Private>>(
    frame: RecordingFrame<Private>, witnesses: &W,
) -> Result<(RecordingFrame<Private>, Field), CompactError> {
    let (frame, observed) = frame.try_witness_metered(/* same witness call */)?;
    Ok((frame, observed))
}
let (frame, value) = __compact_recorded_body_innerValue(frame, witnesses)?;
let frame = ledger_slots::value.record_write(frame, value)?;
```

Start with parameterless internal Field-returning calls used directly as a Cell write value. This avoids changing argument coercion or arbitrary expression order in the first slice. The helper owns the existing frame; only the public wrapper opens/finishes it. `recorded.rs` alone selects and emits the private helper via `syn`; runtime, macros, private IR schema 8, and public ABI 15 remain unchanged. Require exact existing TypeScript result/private FAB/state comparison, full Rust replay, four gas dimensions where captured, a separate generated consumer and packaged proof gate. Measure generated source growth; one call does not establish a size/speed optimization. Keep value calls with arguments and expression-nested calls open in #117.


### Parameterless Field-returning helper delivery — 2026-10-03, ABI 15

Local conventional GPG-signed/DCO commit `ee76fd1da7d02a0c51a85cc61fcec0eb0b3e2747` (`G` signature status) implements the preceding proposal on `codex/rust-backend-ast`. The ledger-8 IR places `innerValue()` in a `StateAction::Let` Field binding before a Cell write. `recorded.rs` now includes that call in the contract-wide plan, admits only parameterless Field-returning callees with no actions and a complete expression trace, and emits one private frame-taking helper reused by eligible callers. A two-caller renderer regression confirms one declaration and two references. The generated `outerValue` calls the helper once, receives its typed `Field`, then writes through `ledger_slots::value`. Only the public wrapper starts/finishes `RecordingFrame`; no hidden context reset occurs.

Actual generated code before:

```rust
let (frame, __compact_witness_0) = frame.try_witness_metered(|context, meter| {
    witnesses.secret(context.witness_context_with(LedgerView {
        state: context.query.state.get_ref(), meter,
    }))
})?;
let __compact_recorded_return_1: runtime::Field = __compact_witness_0;
let frame = ledger_slots::value.record_write(frame, __compact_recorded_return_1)?;
```

Actual generated structure after (line wrapping abbreviated):

```rust
fn __compact_recorded_body_innerValue<Private, W: TryWitnesses<Private>>(
    frame: RecordingFrame<Private>, witnesses: &W,
) -> Result<(RecordingFrame<Private>, Field), CompactError> {
    let (frame, observed) = frame.try_witness_metered(|context, meter| {
        witnesses.secret(context.witness_context_with(LedgerView {
            state: context.query.state.get_ref(), meter,
        }))
    })?;
    Ok((frame, observed))
}
let (frame, value) = __compact_recorded_body_innerValue(frame, witnesses)?;
let frame = ledger_slots::value.record_write(frame, value)?;
```

**Ownership and compatibility:** `tools/compact-rust-backend/src/recorded.rs` owns the planning and `syn` emission; one fixture, its tests, the renderer regression and the backend guide were updated. Runtime, VM builders, macro/derive crates and private schema 8 are unchanged; generated/runtime ABI remains 15 and no public method signature changes. The `midnight-ledger` Field and witness FAB path are reused rather than encoded again.

**Verification:** 55 renderer tests, 3 focused nested-witness fixture tests, 132 fixture outputs current, `cargo fmt --all --check`, `cargo check --workspace --all-targets`, and the packaged target/consumer/proof gate pass. The latter replayed, proved, verified, validated and applied all 55 offline ledger-8 calls, including `outerValue`. The fixture compares independent TypeScript private-state/FAB/serialized-state fields, native-vs-recorded four-dimensional gas and query effects, ledger replay effects, and an injected fallible-witness error propagated before a result is returned. Generated `nested-witness-call-oracle/lib.rs` grows 317→327 lines (+10). No compile-time or source-size optimization is claimed.

**Limits:** Value calls with parameters or nested inside larger expressions remain inline. This test does not compare `outerValue`'s complete serialized Verify operands or four-dimensional gas against a TypeScript gas capture; native parity and replay are narrower evidence. Warm package-only compile time and unchanged tiny/election/zerocash/passport baselines were not remeasured. #117 stays open for these, broader error/recursion probes, branch publication, clean remote CI, release and wallet/node submission. The branch remains local/unpushed; the user-owned `doc/ledger-adt.mdx` edit was not staged.


### Parameterized Field-value helper proposal — 2026-10-03, ABI 15

The delivered `innerValue()` helper has no arguments. A caller that supplies effectful Field arguments still inlines a value-returning callee, and helper reuse has not demonstrated left-to-right argument evaluation. Extend the same private `RecordingFrame` boundary only for eligible Field-returning callees whose own actions are empty and whose return expression has a complete trace. Keep the Compact parameter types at the helper signature, evaluate each call argument into an explicitly typed local in source order, then call the helper. The runtime frame retains all witness FABs and ledger Verify ops.

Before (inline caller, illustrative):

```rust
let (frame, first) = frame.try_witness_metered(/* ordered witness */)?;
let (frame, second) = frame.try_witness_metered(/* ordered witness */)?;
let (frame, inner) = frame.try_witness_metered(/* callee witness */)?;
let frame = ledger_slots::value.record_write(frame, inner + first + second)?;
```

After (proposed private helper):

```rust
let (frame, first) = frame.try_witness_metered(/* first argument */)?;
let first_arg: Field = first;
let (frame, second) = frame.try_witness_metered(/* second argument */)?;
let second_arg: Field = second;
let (frame, value) = __compact_recorded_body_innerValue2(
    frame, witnesses, first_arg, second_arg,
)?;
let frame = ledger_slots::value.record_write(frame, value)?;
```

`recorded.rs` owns selection and `syn` emission, with no public generated method, runtime, macro, schema or ABI change. Extend the existing nested witness oracle source with an `ordered()` witness that returns a value dependent on the current private state so the three private FABs reveal actual argument/callee order. Update the independent TypeScript capture, Rust native/recorded test, replay and packaged proof gate; retain the prior 55-call regression. Measure generated source growth and avoid claiming a speed/size gain without timing. #117 remains the focused rust-backend-v2 issue.


### Parameterized Field-value helper delivery — 2026-10-03, ABI 15

Local conventional GPG-signed/DCO `ed05c0d88d21ca6c9cb01663b4d7c30fa41b7a3b` extends the prior parameterless slice on `codex/rust-backend-ast`; signature status is `G`. The ledger-8 compiler accepts `outerValue2(): [] { value = innerValue2(disclose(ordered()), disclose(ordered())); }` with `innerValue2(first: Field, second: Field): Field { return disclose(ordered()) + first + second; }`. `ordered()` returns the current private counter as a Field and increments it, so observable private FAB outputs distinguish first argument, second argument and callee evaluation.

Before, the recorded caller inlined both argument witness effects and the callee witness/addition body, repeating that body at each call site. After, the actual generated `outerValue2` shape is:

```rust
let (frame, __compact_witness_0) = frame.try_witness_metered(/* ordered */)?;
let __compact_recorded_arg_1: runtime::Field = __compact_witness_0;
let (frame, __compact_witness_2) = frame.try_witness_metered(/* ordered */)?;
let __compact_recorded_arg_3: runtime::Field = __compact_witness_2;
let (frame, __compact_recorded_value_4) =
    __compact_recorded_body_innerValue2(
        frame, witnesses, __compact_recorded_arg_1, __compact_recorded_arg_3,
    )?;
let frame = crate::ledger_slots::value.record_write(frame, __compact_recorded_value_4)?;
```

The one private `__compact_recorded_body_innerValue2<Private, W: TryWitnesses<Private>>` helper takes the same frame and two `runtime::Field` parameters, calls `ordered()` once, sums its result with those parameters and returns `(frame, Field)`. A renderer regression renders two callers and confirms one helper definition. In `recorded.rs`, Unit and Field helper calls now share `shared_call_argument`, which lowers a Field or supported bounded/scalar argument through one typed, source-ordered path. `syn` emits syntax; `prettyplease` formats it. No text-appending Scheme Rust emitter, runtime primitive, macro, derive or second VM builder was added. Public generated signatures, private IR schema 8 and generated/runtime ABI 15 are unchanged.

The independent ledger-8 TypeScript capture is reproducible byte-for-byte from `runtime-rs/tests/fixtures/capture-nested-witness-call-oracle.mjs`. Called after the prior two fixture circuits, `outerValue2` produces private outputs `[9, 10, 11]`, private state `12`, one ledger query and three public Verify operations. The Rust fixture compares all private FAB atoms/alignment, exact serialized state, result/effects, four-dimensional query gas (`readTime=85000000`, `computeTime=1233942932`, `bytesWritten=36`, `bytesDeleted=36`), and full ordered Verify operands against that capture. Native/recorded gas and effects match; replay effects match. A fallible witness rejects on the third invocation inside the helper and returns an error before a recorded result exists.

**Validation:** 55 renderer tests, five focused nested-witness tests, `node --check` plus byte-for-byte recapture, 132 fresh fixtures, `cargo fmt --all --check`, `cargo check --workspace --all-targets`, and the compiler-backed target/consumer/proof gate pass. The gate replayed, partitioned, proved, verified, validated and applied all 56 offline ledger-8 calls, including `outerValue2`; its applied Cell value is exactly Field 24 from a fresh private state of 7. The first gate attempt stopped before proof on host disk exhaustion, with no source failure. Cleaning only the dedicated generated Cargo consumer cache with `cargo clean --target-dir target/compactc-consumer` restored space; the complete gate then passed twice, including after centralizing argument lowering.

**Size and limits:** The generated fixture changes 327→494 lines (+167), but it also adds one exported circuit, one internal callee and a witness, so this is not a same-source helper cost or optimization measurement. Warm package-only compile time was not measured. Direct Field action bindings are supported; value calls nested inside larger expressions remain inline. Broader negative/recursive probes, gas/Verify parity for other callees, branch publication, remote CI, registry release and wallet/node submission remain open under [#117](https://github.com/MediaNoxLabs/compact/issues/117) and milestone 2. The branch remains local/unpushed, and the user-owned `doc/ledger-adt.mdx` edit was not staged.
