---
id: RUST-ADR-0257
alias: ADR-0257
source_sha256: 010537fb6362032b785b67b93a57fe7828a681880f908b40d044593094edada1
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0257 — Reuse runtime frame ownership for simple native leaves

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for implementation, 2026-10-07. Parents #346/#348/#349. ABI50/IR20 and runtime APIs unchanged. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR-0257 — Reuse runtime frame ownership for simple native leaves

Status: accepted for implementation, 2026-10-07. Parents #346/#348/#349. ABI50/IR20 and runtime APIs unchanged.

Accept only the two structural leaf shapes below. The implementation should preserve existing source validation and fallback diagnostics. This is a context/gas/result ownership improvement; no performance benefit is asserted. Named-argument emission and an abstraction-level option remain outside this decision.

## ADR-ready proposal: selective native leaf frame ownership

Read-only proposal following executed ADR0243/#366 probes. No production edit or promotion. Evidence: `/tmp/rust030-abstraction-probes/RESULTS.md`, successful frame controls and named-call preparation; this proposal promotes neither named-argument generation nor any performance claim.

### Decision requested

Use the existing ordinary `CircuitFrame` for **parameterless Unit-returning native leaf circuits with exactly one Boolean Cell literal write or one normalized checked Uint16-literal Counter increment**. Keep existing witnessed Field frame lowering unchanged. Unsupported shapes continue through the existing general native emitter, retaining its diagnostics. No runtime, ABI50, IR20, recorded admission, macro or compiler flag changes.

### Why this is an ownership improvement

Current leaf code manually adopts the returned context, accumulates query cost and constructs a result with an empty private-output vector. The runtime already owns exactly this result adoption in `CircuitFrame::apply`/`finish`. Reusing it makes the emitter choose the declared slot/operation/value while runtime owns context/gas/private-output result accumulation. This removes a second convention for assembling otherwise equivalent CircuitResults; it does not remove source-semantic statements or ledger operations. There is no demonstrated execution or build speedup. The scratch debug rlib was slightly larger, and timing ranges overlapped.

### Exact implementation boundary

- `tools/compact-rust-backend/src/native_frame.rs:221` `render_if_supported`: add a small private leaf matcher/emitter before the existing witnessed-Field profile, or invoke it from that existing entrypoint. Keep its current witnessed profile's predicates and body unchanged.
- `stateful.rs:1745` already calls this entrypoint after malformed effectful-return validation. No new global dispatcher/profile flags are needed.
- Leaf matching uses declared IR types and the actual slot id/index; it must not identify contract or circuit names. Generated slots retain the complete physical path, including module/chunk paths already validated by root assembly.
- Use an internal two-variant leaf plan carrying a validated slot and Boolean or checked Uint16 literal if that keeps the matcher readable. This is not a new expression evaluator or generic action plan.
- Source location wrapping remains at existing caller; on unsupported/malformed candidate return `None` so general lowering supplies its existing precise diagnostic. Do not replace errors with `expect`, erase checks, or silently accept an invalid local name.

#### Accepted shapes

Common: no parameters, `result == Unit`, `StateReturn::Unit`, exactly one outer action, and an existing declared slot whose index/type matches.

A. `CellWrite { value: Boolean { value }, .. }`, declared `Cell<Boolean>`. Both Boolean values are valid, not just true.

B. One `StateAction::Let` with exactly one binding, `ty == Unsigned(max=65535)`, value exactly `UnsignedLiteral(max=65535, value=<canonical in-range decimal>)`, and body exactly `CounterIncrement` whose amount is `Parameter` naming that binding. The declared slot is a Counter. Validate the binding identifier and use existing literal syntax validation (not unchecked decimal parsing) so malformed IR falls back with existing errors. Materialize the same checked BoundedUint literal before increment; retain `.value() as u16` and literal-check `expect` semantics.

The B shape is the actual frontend output for `round.increment(1)` and `.increment(0)`; do not pretend it is a bare CounterAmount::Literal action. Zero increments still execute the VM query and accrue gas: never optimize them away.

### Before / after

Actual Boolean leaf in `tests-rust-backend/cell-boolean/lib.rs:77`:

```rust
let mut total_cost = runtime::context::RunningCost::default();
let private_transcript_outputs = Vec::new();
let step = crate::ledger_slots::flag.write(context, true)?;
let context = step.context;
total_cost += step.gas_cost;
let result = ();
Ok(runtime::context::CircuitResult {
    context, result, gas_cost: total_cost, private_transcript_outputs,
})
```

Proposed emission, exercised in the scratch consumer:

```rust
let frame = runtime::context::CircuitFrame::new(context);
let (frame, ()) = frame.apply(|context| crate::ledger_slots::flag.write(context, true))?;
Ok(frame.finish(()))
```

Counter preserves its original checked literal before the analogous apply:

```rust
let amount = runtime::BoundedUint::<65535>::new(1u128)
    .expect("Compact Uint literal fits its maximum");
let frame = runtime::context::CircuitFrame::new(context);
let (frame, ()) = frame.apply(|context| {
    crate::ledger_slots::round.increment(context, amount.value() as u16)
})?;
Ok(frame.finish(()))
```

Existing generated signatures, generics, visibility, native facade wrappers, observed methods, recorded functions and capability reports must remain unchanged. Fixed emitter identifiers should use the same collision-safe conventions as current native lowering; a valid parameterless source name cannot introduce a local capture.

### Expected fixture footprint

A generated-body scan followed by fresh frozen-compiler IR inspection confirmed these **17 native methods** as exact candidates. Full freshness after implementation is the authoritative check that no additional output changed; this inventory is not a name-based admission list.

| Fixture | Method | Shape |
|---|---|---|
| assert-parity-oracle | ping | Boolean write |
| cell-boolean | set_flag | Boolean write |
| hash-to-curve-vector-constructor | ping | Boolean write |
| list-oracle | ping | Boolean write |
| nested-map-oracle | ping | Boolean write |
| pure-circuit-oracle | ping | Boolean write |
| sealed-ledger-constructor | ping | Boolean write, nonzero slot |
| tuple-destructure | ping | Boolean write |
| tuple-oracle | ping | Boolean write |
| witness-ledger-cell | set_flag | Boolean write; other witnessed methods unchanged |
| counter | increment | Counter literal 1 |
| fold-oracle | ping | Counter literal 0 |
| for-iter-oracle | ping | Counter literal 0, differently named frontend local |
| for-range-oracle | ping | Counter literal 0 |
| module-boolean-constructor | bump_inner | Counter literal 1, module-owned slot |
| stateful-circuit-call | bump | Counter literal 1; caller stays general emitter |
| witness-ledger-counter | increment_round | Counter literal 1; witness caller unchanged |

`candidate-ir.json` records actual full circuit and ledger declarations. Source files are in `examples/rust_backend`; exact paths are retained in that file. `inventory.py` and log record the investigation.

### Countercases explicitly excluded

- Counter decrement/reset/read, dynamic parameter increments, arithmetic or witness-generated amounts, multiple bindings/actions and widened/bounded types other than the exact frontend Uint16 literal.
- StateAction Sequence flattening, conditional branches, short-circuit reads, actionful calls, native built-ins or witness calls; including such forms would require ordering/branch tests beyond the leaf experiment.
- Boolean parameter writes, arbitrary Boolean expressions, Field/struct/opaque/coin Cells, state reads in the RHS.
- Non-Unit/effectful returns, parameterized circuits and constructors.
- Broadening the older witnessed Field profile or deleting the general native fallback.

These exclusions describe optimization selection, not source rejection. Existing support must continue to compile and behave identically through its current path.

### Acceptance gates

1. Backend unit/AST tests: both booleans; Uint16 zero/one/max; wrong slot id/index/kind; wrong binding/result type; wrong referenced local; invalid local spelling; noncanonical/out-of-range literal; extra unused binding/action; branch/call/witness forms. For malformed inputs compare baseline diagnostics, not merely `None`. Keep the old witnessed-frame integration regression.
2. Full package tests + strict Clippy. Build fresh compiler and compare all fixture outputs: only the native bodies above should change; serialized capability reports and all recorded submodules byte-identical. Keep before/after hashes and actual affected count.
3. Run existing Cell/Counter, witnessed Cell/Counter, stateful-circuit-call, module-boolean-constructor and zero-increment fixture tests; these check parents adopting the leaf result too. Add specific generated-fixture controls for full state/effects/private/query gas, zero per-query budget, malformed state, zero increment still querying, and maximum stored Counter plus increment rejection behavior (derive exact upstream result rather than assume overflow). Default test worker remains required.
4. Build unedited generated external consumer using public native facade. Retain checked literal validation and normal error source locations.
5. The scratch timing already shows no performance win. If production generated output measurement is repeated, use paired forced rebuilds, unchanged controls and report all samples; it is not an acceptance target to reduce milliseconds or line count. No proof rerun solely because native leaf plumbing changed while recorded/VM output is identical.

### Named arguments remains separate

The ordinary handwritten SetTopicArgs adapter is a successful experiment, including real FAB/observation-binding evidence. It is not part of this production slice. Generated names/collisions, generic parameters, lifetimes, feature gates and witness bounds still need their own accepted design before emitting a public record/API.

### Local delivery —2026-10-07

`012fbb44f8c4df3c3dc78ee97b209476e4541db8`. Exactly17 parameterless native leaves reuse existing CircuitFrame ownership.269backend+37fixturetests, strictClippy and external public-facade consumer pass.182fixtures fresh; every recorded byte and capability report unchanged. Ten malformed IR diagnostics identical. Runtime/ABI/schema unchanged; no performance claim. [ADR0257 — Selective native leaf frame local receipt](references-0.3.0.md#note-024).
