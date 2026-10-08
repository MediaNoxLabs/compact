---
id: RUST-ADR-0243
alias: ADR-0243
source_sha256: 66ff1da2d7d7ce2bf6a52c72b5028c20723d9ebeefcc903adcde4a794f9d2502
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0243 — Evaluate generated abstractions with bounded probes

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-experiment-scope. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-experiment-scope
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/366
```

## ADR-0243 — Evaluate generated abstractions with bounded probes

### Problem statement

Generated native leaves repeat context/gas ownership plumbing, whereas other paths already use CircuitFrame. Positional consumer methods can obscure parameter intent. A single abstraction-level setting would conflate runtime operations, admission, generated representation and public convenience methods before their guarantees and costs have been established.

### Decision

Accept the scope of two bounded, independent experiments: native Cell/Counter frame reuse, then named argument adapters delegating to the existing microDAO call facade. Preserve a deterministic supported output meanwhile. This decision accepts research work, not a new public API, runtime behavior or compiler flag. Record negative results as useful outcomes. The probes do not gate DID/passport adoption.

### Before/after, ownership, alternatives and measurement

The following research is the original read-only evidence. Examples explicitly marked illustrative have not been compiled or measured. An implementation outcome needs a recorded follow-up decision and actual measurements before adoption.

## R030-02 / #346 — generated Rust abstraction-level probe

2026-10-07. Read-only inspection; no compilation, execution or benchmarking. Observed working HEAD advanced from `1f35ac47` to `4c5eb5de` during root integration; the generated examples inspected below are unchanged by the facade module extraction. This is research input for the Obsidian plan, not a new accepted API or implementation.

### What the actual output already does

- `tests-rust-backend/counter/lib.rs:77–97`: native increment explicitly builds a checked Uint literal, executes the typed Counter slot, adopts context and accumulates gas. The recorded form at `:133` is already just `RecordingFrame::new`, `round.record_increment` and `finish`.
- `tests-rust-backend/cell-boolean/lib.rs:77–99`: native Boolean write repeats context/result/gas plumbing; recorded form at `:104` uses the typed Cell slot. Neither form emits raw VM instructions for its consumer.
- `tests-rust-backend/witness-cell-write/lib.rs:107–141`: native `write_twice` already uses `CircuitFrame`, metered witnesses and `apply` for each Cell write. It gives the second witness the successor context; this is not boilerplate to collapse into parallel callback evaluation.
- `tests-rust-backend/test-center-micro-dao/lib.rs:3333+`: recorded `set_topic` has typed numbered temporaries, a short-circuit value/color check, then witness/authority and effectful work. These explicit boundaries are useful for auditing evaluation order.
- Same microDAO file `:4691–4725`: the generated `_call` facade packages the source parameters in exact order into FAB input, records execution against an observation, then constructs a `RecordedCall` with the original entry point.
- `runtime-rs/src/context.rs:300–390` already supplies ordinary native frame operations. `runtime-rs/src/recording.rs:833–856` preserves the **actual observed aligned ledger value** when building the Verify program; serializing the decoded result again is not an equivalent simplification.

### Probe A: use the existing ordinary runtime helper for a small native leaf

**Before: representative actual Boolean Cell native body, shortened only by dropping qualification.**

```rust
let mut total_cost = RunningCost::default();
let private_transcript_outputs = Vec::new();
let step = ledger_slots::flag.write(context, true)?;
let context = step.context;
total_cost += step.gas_cost;
Ok(CircuitResult {
    context,
    result: (),
    gas_cost: total_cost,
    private_transcript_outputs,
})
```

**After: illustrative emission using the existing CircuitFrame API; not compiled here.**

```rust
let frame = CircuitFrame::new(context);
let (frame, ()) = frame.apply(|context| ledger_slots::flag.write(context, true))?;
Ok(frame.finish(()))
```

Ownership stays clear: slot chooses declared ledger path/type, runtime owns query/gas/result adoption, emitter owns source sequencing. This is a small extension of already accepted selective frame use, not a new runtime DSL. A checked Counter literal and its validation should remain in the Counter variant rather than disappear as incidental cleanup.

The recorded leaf is already appropriately concise:

```rust
let frame = RecordingFrame::new(context);
let frame = ledger_slots::flag.record_write(frame, true)?;
Ok(frame.finish(()))
```

A `record! { flag = true }` macro would hide the same three steps without demonstrating an extra safety or usability benefit. Do not add it just for line reduction.

### Probe B: named argument records at the consumer boundary

**Before: current generated microDAO consumer shape.**

```rust
let call = contract.recording().set_topic_call(
    bound.observed(), private_state, topic, beneficiary, seed_coin,
)?;
let prepared = bound.prepare(call, verifier, commitment_rand)?;
```

**After: illustrative additive convenience method; not an accepted name or compiled API.**

```rust
let args = calls::SetTopicArgs {
    topic,
    beneficiary,
    seed_coin,
};
let call = contract.recording().set_topic_with_args(
    bound.observed(), private_state, args,
)?;
let prepared = bound.prepare(call, verifier, commitment_rand)?;
```

Prefer retaining existing positional `_call` for compatibility during a probe. A named record helps discovery and call-site documentation; it is not proof of authorization, valid funding or installed-verifier agreement. Different existing parameter types already prevent several positional swaps, so do not overstate its new guarantees.

The first ordinary-Rust implementation should destructure the record and call the existing `_call` facade. It then inherits that facade's entry-point/input packaging, cloning rules and observation lifetime. It need not invent a `CallArgs` trait or codec derive.

A later derive candidate is narrowly mechanical: synthesize a positional FAB encoding from fields in **declared Compact parameter order**, with an explicit entry-point descriptor. Compare that with a normal generated impl. Do not use struct-memory layout, alphabetical field order or serde serialization as a substitute for existing FAB concatenation. A source argument record is not automatically equivalent to an arbitrary Compact struct codec at the transaction boundary.

### Probe C: generated helper extraction for large bodies — later

microDAO's numbered bindings should not be globally inlined into nested Rust expressions just to look idiomatic. Witness effects, errors, cost prefixes and branch-local values must remain ordered. A later candidate can extract a repeated **checked pure** helper, or an already-audited same-frame helper, after the internal typed plan records its scope and effects. This depends on model/admission work, unlike A or B.

Default-worker stack, clone/allocation counts and diagnostic source mapping must be checked. ADR0210 already fixed a concrete aggregate-move issue with private boxed frame storage; neither helper extraction nor a macro should be assumed to improve that result.

### Is a global codegen abstraction-level option advisable now?

**No.** A low/medium/high switch would currently multiply public API, diagnostic, feature and compatibility combinations without a stable definition of what each level guarantees. Native frames, recorded frames, type derives and consumer facades solve different problems; they are not points on one ordered scale.

Instead, keep one deterministic supported output and run temporary probe branches with fixed inputs. If a real downstream need eventually requires alternate output, define a named, versioned capability with a precise compatibility contract. Do not introduce a permanent compiler switch merely to conduct the experiment.

### Smallest achievable measured experiment

Start **A alone**: enable existing CircuitFrame for the current simple native Boolean Cell write, then one Counter leaf if the first result is sound. Keep public signatures, recorded output and checked literal behavior unchanged. Treat B as an independent external-consumer ergonomics probe so its results cannot be confused with runtime changes.

1. Freeze before/after compiler/runtime/lock/toolchain/profile and source hashes. Use Boolean Cell and Counter as affected cases; witnessed Cell and original microDAO as unchanged controls.
2. Compare actual generated bytes and syn structure. Only native leaf plumbing should change. All recorded code, runtime source and ABI remain identical if this is solely emitter reuse of existing APIs.
3. Run existing Cell/Counter/witnessed-Cell native tests, renderer tests, gas-limit/error assertions and generated freshness. Verify result, full state/effects, private outputs and summed query gas. Since recorded code is unchanged, no new proof-family claim or routine full proof rerun is needed.
4. Build unedited generated external consumers with before/after outputs. Measure warm package-only builds through alternating paired runs on one prepared dependency cache; retain all observations and dispersion. Do not call a cached no-op `cargo check` a generated-code compile measurement—force the same package rebuild for each sample.
5. Record default debug-worker success and inspect stack/object size if the compiler emits materially different frames. Record source versus expanded code separately; no artificial goal for macro or line count.
6. Adopt only if readable result ownership improves and semantic gates pass without a material measured regression. A neutral or rejected outcome is valid research evidence.

For B, one additive handwritten consumer adapter around the unchanged generated microDAO crate is sufficient for the first usability experiment. Exercise missing/mistyped fields and method discovery, inspect resulting compiler diagnostics, then compare a generated ordinary impl against any derive only if repetition justifies it. Do not modify generated source or present that scratch adapter as delivered compiler support.

### Boundaries and decision record

Preserve exact source argument/witness order, state after each operation, short-circuit and assertion timing, lexical scope, source-owned entry-point/input binding, actual aligned Read values, per-query budget policy, native summed cost versus replay cost, private-output privacy and exact offer reconciliation. No blanket TS safety comparison, no changed cryptographic semantics and no replacement VM.

This note contains illustrative alternatives and a test plan only. No examples above were newly compiled, benchmarked or accepted. Root can persist it through the Obsidian CLI under the #346 research history, with implementation proposals receiving their own prior ADR and focused issue scope.

### Executed experiment — 2026-10-07

Both scratch probes passed at source `24fa6639`. Frame controls preserve state/effects/private/query-gas and failures. Named arguments preserve successful original set_topic FAB ordering, prepared transcripts and bound observation identity; missing/mistyped fields fail compilation. Five paired rebuild timings overlap; frame source is shorter but its debug rlib is 42,136 bytes larger. No speedup or runtime/stack improvement claimed. [ADR0243 — Executed frame and named argument probes](references-0.3.0.md#note-012) retains code, logs and measurement protocol. Experiment #366 is complete. Production frame promotion and named-argument naming/generic/lifetime policy require separate decisions; no global abstraction-level option.
