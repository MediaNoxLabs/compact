---
id: RUST-ADR-0005
alias: ADR-0005
title: "Use a typed runtime DSL instead of a body-wide macro"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 111f80d151ac21a4121fdcc0a81cb23d688b6a6dedbc3d337566c3e46c589346
---
# RUST-ADR-0005 — Use a typed runtime DSL instead of a body-wide macro

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed slots and consuming frames as the lightweight runtime vocabulary, with selective native CircuitFrame lowering rather than a body-wide macro. Later delivery and 2026-10-06 measurements resolve the bounded ergonomics/timing probe: nested output is 275 lines and total reduction89. No reliable compile-time speedup/regression or universal generated-body migration was established.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#110 closure](https://github.com/MediaNoxLabs/compact/issues/110#issuecomment-6017414744). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4868f001`](https://github.com/MediaNoxLabs/compact/commit/4868f0017333b074d276ae6275a7d1563ebe5609) · [`5ca0cb61`](https://github.com/MediaNoxLabs/compact/commit/5ca0cb612be8e6a5fe856ea626431fa721c7393e) · [`98d33ca7`](https://github.com/MediaNoxLabs/compact/commit/98d33ca7d8ee52ab129d861f0604dc0826d929ca) · [`9e784a90`](https://github.com/MediaNoxLabs/compact/commit/9e784a9057b3ef021e24d2e92c37f88b90e0cf36) · [`c3010bc9`](https://github.com/MediaNoxLabs/compact/commit/c3010bc97a0b3dd46cc656ba5f62126e1bd223c7) · [`fa97aa45`](https://github.com/MediaNoxLabs/compact/commit/fa97aa457c877282c119946d66104dd217e6a50c). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 0005
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 110
```

## Historical decision and amendments

### Problem

Generated native bodies repeat context transfer, cost addition, transcript pushes, and temporary variables. A procedural macro around the entire circuit could shorten source but would duplicate Compact control-flow semantics and make source mapping and effect order hard to inspect. The generated crate should read like Rust application code while keeping the typed Compact IR as the compiler authority.

### Before and direction after

```rust
// Before, in native generated code:
let step = context.write_cell(0, value)?;
let context = step.context;
total_cost += step.gas_cost;

// Delivered for a fully recorded slice:
let frame = crate::ledger_slots::flag.record_write(frame, value)?;

// Candidate native frame API, not implemented yet:
let mut frame = CircuitFrame::new(context);
frame.apply(|ctx| crate::ledger_slots::flag.write(ctx, value))?;
Ok(frame.finish(()))
```

### Decision and ownership

Use a small typed Rust library of field descriptors and frame operations as the lightweight DSL. The emitter continues to construct ordinary Rust `if`, `for`, `?`, typed calls, and bindings from its closed IR. Runtime frame methods own state/gas/transcript accumulation; individual ledger methods own VM operations. Avoid a body-wide macro unless measured evidence shows the typed library cannot express required semantics without harming diagnostics. Optional narrow derives remain governed by ADR-0004.

The current `RecordingFrame` and `ledger_slots` are partial implementation of this direction. A native `CircuitFrame` is still a proposal. Constructor sequencing needs its own parity design because `ConstructorResult` does not expose the same gas/transcript shape.

### Acceptance and trade-offs

Migrate one witnessed Cell body and one nested stateful call first. Compare result, private state, FAB output order, ledger state bytes, gas, Verify ops, short-circuit errors, and TypeScript oracle behavior. Then measure generated lines, compile time, and external call-site clarity for tiny, election, zerocash, and passport. Shorter code alone is not success; semantic parity and readable diagnostics are required. More runtime API surface is the principal maintenance cost.

### Tracking

- Parent: [M2 diagnostics and VM parity #104](https://github.com/MediaNoxLabs/compact/issues/104).
- Focused issue: [#110](https://github.com/MediaNoxLabs/compact/issues/110). Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Delivery: typed slots and recording methods local; native frame and measurements open.


### Decision history

- 2026-10-02: Created ADR-0005 from the generated-crate research probes with status `accepted-partial`; linked MediaNoxLabs/compact#110 in `rust-backend-v2`. Local delivery evidence is recorded above. Future changes should append a dated amendment or link a superseding ADR.


### Amendment — 2026-10-02: nested call expansion

**Problem and before.** A generated native `add_twice` called `add(context, amount)` twice, while recorded methods existed only for leaf circuits. A body-wide macro could hide nested call behavior but would make trace completeness and proof alignment harder to inspect.

**After.** Commit `9e784a90` expands a supported Unit-returning `CircuitCall` at the typed IR boundary into ordered `ledger_slots::count.record_increment(frame, amount)` steps. Witnessed nested calls use the same `frame.witness` and `CellSlot::record_write` primitives. `Contract::default().recording.add_twice(context, amount)` now returns a complete `RecordedCircuitResult`; `Contract::from(Secret).recording().write_nested_twice(context, seed)` keeps witness ownership explicit. The emitter owns expansion, temporary argument bindings, cycle rejection, and conservative method exposure; runtime DSL and VM construction remain unchanged.

**Evidence and tradeoff.** The generated nested Counter and witnessed Cell calls proved and applied with emitted ZKIR in the offline ledger-8 gate (toolchain `0.31.124`); 129 fixtures and the workspace check passed. Repeated callee bodies produce repeated generated slot calls. Measure generated lines and compile time on larger contracts before deciding whether private generated helpers should replace expansion. Track that design under [#110](https://github.com/MediaNoxLabs/compact/issues/110) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2); this commit is local and does not close the issue.



### Amendment — 2026-10-02: isolate recorded emission

**Problem and before.** `stateful.rs` contained both native body emission and roughly 500 lines of replayable trace emission. Adding expression effects there would make the ownership and completeness checks harder to review. A value-returning nested witness call had native code but no generated recorded method:

```rust
let step = inner_value(context)?;
let context = step.context;
let value = step.result;
```

**After.** Commit `5ca0cb61` moves recorded emission to `recorded.rs` and lowers typed Field expressions into explicit ordered frame operations. The consumer calls `Contract::from(Secret).recording().outerValue(context)?`; the generated body records the private FAB from `innerValue` before `ledger_slots::value.record_write(frame, value)`. This is ordinary generated Rust composed from typed IR and typed runtime methods, with no body-wide procedural macro.

**Ownership and evidence.** The emitter owns expression traversal, evaluation order, recursive-call rejection, and withholding recorded APIs for unsupported shapes. Runtime `RecordingFrame`, slot descriptors, VM construction, and transaction adapter are reused unchanged. The TypeScript oracle, negative Multiply exposure test, 129 fixture freshness check, full workspace check, and packaged ledger-8 proof/application gate passed at toolchain `0.31.125`. A separate `recorded.rs` makes review boundaries clearer but does not reduce generated source duplication; helper extraction, native `CircuitFrame`, larger-crate size and compile-time measurements remain [#110](https://github.com/MediaNoxLabs/compact/issues/110) work in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).

### Research amendment — 2026-10-02: measured native frame probe (no implementation)

The generated-code subagent inspected the current ABI-5 branch under [#110](https://github.com/MediaNoxLabs/compact/issues/110). Native witnessed Cell bodies still repeat `context`, `total_cost` and `private_transcript_outputs` plumbing. Recorded nested calls inline callee bodies, so a larger call graph may multiply generated source. A proposed consuming API is:

```rust
let (frame, secret) = CircuitFrame::new(context).witness(|ctx| witnesses.secret(ctx.witness_context_with(LedgerView { state: ctx.query.state.get_ref() }), seed));
let (frame, ()) = frame.apply(|ctx| ledger_slots::cell.write(ctx, secret))?;
Ok(frame.finish(()))
```

`CircuitFrame<P,D>` would own `CircuitContext<P,D>`, total `RunningCost` and private FAB outputs. `apply` would take the frame and a closure `CircuitContext -> Result<CircuitResult<_,T,_>, CompactError>`, return the successor frame and result, and propagate errors before `finish`. A native nested call could use `frame.apply(|ctx| write_inner(ctx, witnesses, seed))?`, preserving child gas/output order without manual transfer. Runtime owns those operations; the emitter owns ordinary Rust control flow and typed calls. Recorded nested calls require a different refactor: generate one private function that takes and returns the same `RecordingFrame` so a nested call does not create a second initial public trace. This is a proposal; no generated/runtime API has changed.

Measured generated source sizes at commit `fa97aa45`: tiny 512 lines, election 954, zerocash 622, passport 4,532. Isolated `cargo check --locked --offline --jobs 4` on tiny took 29.23 seconds cold including dependencies; warm package-only checks after `cargo clean -p` took 0.401/0.395/0.396 seconds for tiny, 0.463 election, 0.434 zerocash and 0.724 passport. These are local type-check samples, not full build times or comparative before/after results. There is no established compile-time win. The next review gate is an implementation probe on witnessed Cell and nested calls with result, private outputs, state bytes, gas, Verify order, failure and TypeScript parity before accepting this API.


### Delivery amendment — 2026-10-02: consuming native `CircuitFrame` probe

**Problem and before.** Generated witnessed Cell and nested native call bodies transfer `context`, add each `step.gas_cost`, and append private FAB outputs by hand. The repeated bookkeeping makes nested calls hard to audit for ordering. This first slice tests whether a small runtime type can own those transfers before changing the emitter.

```rust
// Before: native generated body, abbreviated.
let step = ledger_slots::cell.write(context, secret)?;
let context = step.context;
total_cost += step.gas_cost;
// After in a handwritten consumer probe; generated bodies have not migrated.
let (frame, secret) = CircuitFrame::new(context).witness(|ctx| witness_from(ctx, seed));
let (frame, ()) = frame.apply(|ctx| ledger_slots::cell.write(ctx, secret))?;
Ok(frame.finish(()))
```

A nested native call composes as `let (frame, ()) = frame.apply(|ctx| write_secret(ctx, witnesses, seed))?;`. `CircuitFrame<Private, D>` owns the successor `CircuitContext`, accumulated `RunningCost`, and ordered private `AlignedValue` outputs. `witness` updates private state and appends its FAB; `apply` consumes the frame and one `CircuitResult`, merges its gas/outputs and returns the child result; `finish` yields the existing `CircuitResult`. An error propagates through `?` before a finished result exists. Ledger VM programs stay in typed slots/context; the emitter and generated output are unchanged. The API is additive to runtime ABI 5, and private IR stays schema 8. Recorded calls still use `RecordingFrame` and require a separate frame-taking callee design before deduplicating nested recorded bodies.

Conventional GPG-signed/DCO local commit `c3010bc97a0b3dd46cc656ba5f62126e1bd223c7` advances [#110](https://github.com/MediaNoxLabs/compact/issues/110) in `rust-backend-v2`. The witnessed Cell fixture compares manual frame composition with current generated `write_secret` and `write_nested_twice`: private state, ledger state and effects, total gas, FAB output order and the existing TypeScript private/cell oracle match. A later-step closure remains uncalled after an injected assertion error. All six focused tests, the full runtime suite and workspace check pass. Formatting, headers (0 missing in 1,466 files), diff check, signature and DCO pass. Both package archives compile, and the saved clean-source manifest writes/verifies with commit `c3010bc9`, `dirty: false` (runtime archive 227 entries).

**Limits and review gate.** This validates runtime composition semantics; it does not yet make generated crates shorter or improve their public API, nor prove a compile-time difference. The emitter must migrate a witnessed Cell body and nested call, then compare exact output/state/gas/Verify order/failure with the old output and TypeScript. The recorded-body reuse proposal also remains unimplemented. No M2 branch push, remote CI, registry publication or wallet/node submission is claimed. Preserve the research-only measurements above as baseline, not as a speedup claim.


### Delivery amendment — 2026-10-02: generated native frame lowering

**Problem and before.** The runtime frame probe did not change generated code. A witnessed Cell write still manually replaced the context, added gas, and pushed private outputs; nested calls repeated that merge:

```rust
let (__compact_next_private, secret) = witnesses.secret(context.witness_context_with(view), seed);
context.private_state = __compact_next_private;
private_transcript_outputs.push(AlignedValue::from(secret.clone()));
let step = crate::ledger_slots::cell.write(context, secret)?;
let context = step.context;
total_cost += step.gas_cost;
```

**After.** Local conventional GPG-signed/DCO commit `4868f0017333b074d276ae6275a7d1563ebe5609` makes the typed Rust emitter construct `syn` statements for eligible unit-returning Field circuits. The generated crate uses ordinary Rust calls and `?`:

```rust
let frame = runtime::context::CircuitFrame::new(context);
let (frame, secret) = frame.witness(|context| witnesses.secret(
    context.witness_context_with(LedgerView { state: context.query.state.get_ref() }),
    seed,
));
let (frame, ()) = frame.apply(|context| crate::ledger_slots::cell.write(context, secret))?;
Ok(frame.finish(()))
```

A nested call becomes `let (frame, ()) = frame.apply(|context| self::write_inner(context, witnesses, seed))?;`. The `self::` path prevents a circuit named `frame` colliding with the generated local. The actual emitter uses hygienic `__compact_*` bindings.

**Ownership and eligibility.** New `native_frame.rs` reads closed schema-8 IR and emits `syn` AST; `prettyplease` formats it. Supported expressions are Field parameters, literals, addition and witness calls; supported actions are lexical `Let`, Field Cell writes, sequences and Unit nested calls. Every action must fit before selection. Unsupported shapes use the existing general emitter. The runtime `CircuitFrame` owns context, gas and private FAB order; ledger slots own VM construction. The outer `located(circuit.source)` diagnostic wrapper remains. Generated/runtime ABI stays 5 and private IR stays schema 8. No body-wide macro was introduced.

**Evidence.** All 53 renderer tests, the three affected generated crates' tests (2 nested, 6 Cell, 1 witness oracle), workspace check, 131/131 fixture freshness check, 37 pinned oracle sources, two rejection probes, external generated crate compile, and packaged compiler/consumer/proof gate pass. The gate replays, partitions, proves, verifies, validates and applies 45 offline ledger-8 call cases, including `write_twice` and `write_nested_twice`. Existing Cell tests compare private state, final Cell value and FAB atoms/alignment with a TypeScript capture; they compare generated native state/effects, gas and FAB order with the frame probe. Recorded calls check Verify count and prove/apply. An injected error stops a later frame step.

The three affected generated `lib.rs` files shrink 298→274, 434→380 and 164→152 lines (90 lines total). Tiny/election/zerocash/passport remain 512/954/622/4,532 lines. The new emitter module adds 286 lines, so this is a consumer output reduction, not an overall repository size reduction. No comparative compile-time gain is claimed.

**Open review gate.** Test exact TypeScript state bytes, four-dimensional gas and Verify sequence for newly lowered native calls; measure compile time and external call-site clarity where the lowering applies. Expand to other typed actions only with parity tests, and decide whether shared lowering can remove duplication between this selective path and the general emitter. Recorded callee helper reuse, remote CI, branch publication and release remain open under [#110](https://github.com/MediaNoxLabs/compact/issues/110) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


### Residual acceptance audit and measurement plan — 2026-10-06

Issue #110 remains open pending this evidence and final remote checks. The exact TypeScript state/FAB/Verify/summed-gas and error-order acceptance is covered by ADR0015 `98d33ca7` plus current `witness-cell-write` tests and ADR0016 nested-call tests; earlier open wording predates those deliveries. The selective frame fallback remains the accepted architecture, not a promise to migrate every native body. No broad native-frame optimization or timing improvement is established. ADR0210 measures separate RecordingFrame debug storage/stack behavior and must not be attributed to native CircuitFrame.

The missing comparison will use immutable parent `c3010bc97a0b3dd46cc656ba5f62126e1bd223c7` and frame-emitter commit `4868f0017333b074d276ae6275a7d1563ebe5609`. Their runtime, macro crate and Cargo.lock bytes are identical. Extract the historical source into a temporary experiment, retaining the unchanged full manifest/lock. Compare unchanged generated lib.rs blobs at the same package path with Rust1.99.0, CARGO_INCREMENTAL=0, --locked --offline and a single exclusive warm cache. Warm setup is logged and excluded; force only the selected source package check by rewriting its exact Git blob, alternating before/after and reverse-order pairs across three samples each. Keep raw wall times and median/range; no cold-build, release-performance or speedup claim. If historical dependencies cannot resolve without changing the lock, retain the refusal rather than rewrite ABI/dependency pins.

Seven fixtures: the three affected witnessed Cell/nested/witness oracle crates, plus unchanged tiny, election, zerocash and passport controls. Git SHA256 confirms the four controls are byte-identical across this delivery and contain no native CircuitFrame. Actual committed split-line counts are 434→380, 298→275, 164→152: **89 fewer lines**, correcting the earlier 274/90 statement while retaining its history. Later current output size changes include ABI/features and are not a frame comparison. Static evidence: `${LOCAL_EVIDENCE}/compact-adr5-static-comparison.json`.

External ergonomics review will retain actual native/recorded Contract facade and typed passport-call samples. Compare public signatures across the frame-only delivery, documenting whether callers need any new setup, imports, traits or steps. Byte-identical public call sites would show an internal maintenance change, not a demonstrated usability improvement. New compiled consumer snippets may be tested against both unedited historical generated outputs with the same runtime. Existing private-state/trace semantics and source diagnostics remain separate gates.

This is temporary experimental work under existing ADR5/#110; no new repository harness, production changes, broad gate, remote dispatch or speed claim. The warm asset-registry target is leased exclusively for this measurement.


### Completed bounded measurement — 2026-10-06


### Conclusion

The remaining measurement and external-call evidence for #110 is now recorded. Native frame selection removes explicit state/gas/private-output bookkeeping in its approved bounded cases and preserves public call sites. It does **not** improve the public API itself, and this experiment establishes **no compile-time speedup or regression**. The four named larger controls do not use CircuitFrame at this historical change. General native fallback remains accepted; migrating all native bodies was not an acceptance promise.

### Controlled method

Before: c3010bc97a0b3dd46cc656ba5f62126e1bd223c7. After:4868f0017333b074d276ae6275a7d1563ebe5609. Runtime, macros and full Cargo.lock bytes identical. Full historical source archived to a temporary directory; same package paths and exact unchanged generated Git blobs used for both conditions, no ABI substitution. Rust1.99.0 aarch64-darwin; CARGO_INCREMENTAL=0; exclusive pre-existing target/adr157 warm cache; cargo check --locked --offline --lib --jobs4. Three alternating pairs per fixture (middle pair reversed),42 samples. Each log confirms the selected package was actually rechecked. Full lock unchanged. Initial dependency/setup8.742s excluded. Report wall-clock medians and ranges; this is warm package typecheck, not cold build, release execution, statistical significance or product latency.

| Fixture | Lines before→after | Median before→after seconds | Before range | After range |
|---|---:|---:|---:|---:|
| witness-cell-write | 434→380 | 0.338→0.347 | 0.330–0.351 | 0.334–0.358 |
| nested-witness-call-oracle | 298→275 | 0.329→0.338 | 0.328–0.340 | 0.334–0.344 |
| witnesses-oracle | 164→152 | 0.325→0.322 | 0.324–0.326 | 0.321–0.328 |
| tiny-oracle | 512→512 | 0.378→0.372 | 0.368–0.385 | 0.359–0.403 |
| election-oracle | 954→954 | 0.387→0.383 | 0.387–0.391 | 0.381–0.388 |
| zerocash-oracle | 622→622 | 0.377→0.383 | 0.370–0.380 | 0.376–0.394 |
| passport-dogfood | 4532→4532 | 0.627→0.643 | 0.624–0.633 | 0.619–0.643 |

Exact source correction: nested output is275 lines, not274; total reduction is89, not90. Original history is retained. Four controls have identical bytes across the pair; current157 output sizes differ for unrelated later ABI/features, so they are not the causal comparison. No additional source reduction is claimed.

### External ergonomics

`external-call-sites.rs` is the identical external consumer source compiled against both variants. It depends only on five generated crates and imports primitive/context types through their runtime re-export. Examples call typed facades:

```rust
tiny::ledger_contract::Contract::default().get(context)
election::ledger_contract::Contract::from(witnesses).vote_commit(context, vote)
zerocash::ledger_contract::Contract::from(witnesses).spend(context, recipient, coin)
witnessed::ledger_contract::Contract::from(witnesses).write_nested_twice(context, seed)
passport::pure_circuits::assertValidDigitalPassportAgePredicate(
    credential, presentation, today, birthday, opening, current_date, birth_date,
)
```

Both unedited outputs compile these exact calls. A bool replacing the witnessed Field seed fails with Rust E0308 on both. All public function header sequences across the seven fixtures are identical. Consumers do not construct CircuitFrame, build VM instructions, add gas, append FAB or import a body macro. They still provide typed context and witnesses; passport still has seven typed arguments and application value construction. Those are explicit remaining API costs, not an unmeasured claim of usability improvement. The effect is internal generated-body maintenance and reuse.

Consumer setup initially used cargo generate-lockfile, which selected newer cached anyhow. A provenance guard refused that graph before comparative consumer checks. Retained rejected lock/receipt; replaced setup with cargo check preserving the copied historical lock pins, then verified every registry package version/source/checksum against the original. Both successful checks and both negative checks used the same resulting consumer lock; measured package samples always used the unchanged original full lock and are unaffected.

### Existing semantic evidence and remaining exit

ADR0015 /98d33ca7 and current witness-cell-write tests compare independent TypeScript serialized state, private FAB alignment/order/private state, four-dimensional summed query cost, complete recorded Verify sequence, replay and injected-error short circuit for single/repeated/nested calls. ADR0016's parameterized helper additionally pins ordered argument/callee witness outputs and exact TypeScript gas/Verify operands. Current full local gate executes these fixtures. ADR0210 private RecordingFrame boxing/default-worker measurements concern a separate type and are not a native-frame performance claim.

This closes the identified measurement/ergonomics evidence gap in the accepted selective design; #110 still requires parent review and final same-revision remote CI. Do not conflate this historical experiment with all current-source generated APIs or change the recorded gas policy. No repository source changed, no proof rerun, no push/dispatch. Raw logs, scripts, exact sources and receipts live beneath `${LOCAL_EVIDENCE}/compact-adr5-native-frame-measurement`.
