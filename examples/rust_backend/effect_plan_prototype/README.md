# Compositional recording prototype — #485 / ADR0355

The default compiler now uses the checked effect plan inside the existing
legacy-admitted effectful-return family, with legacy fallback for unsupported
fragments. The **opt-in** feature additionally admits algebra compositions that
the legacy profile refuses. The two real Compact circuits here demonstrate that
broader admission; they still require the feature. Their explicit offline proof
and ledger-application gate retains the shared unbalanced test policy.

## Run the experiment

From the repository root:

```sh
cargo test -p compact-rust-backend --features experimental-recording-plan
cargo test -p compact-rust-recording-effect-plan-prototype-fixture \
  --features experimental-recording-plan,ledger-transaction
```

The fixture's build script generates Rust into Cargo's `OUT_DIR` from the
retained schema20 frontend IR, retaining its two exports and adding nine
backend-only test exports. No generated Rust copy needs manual maintenance.
With the feature disabled this experimental fixture is inert; the default
compiler retains the legacy admission boundary. The feature applies to the compiler build; it is not a runtime flag.

To compile the Compact source through a compatible ledger8 Scheme frontend:

```sh
export COMPACTC_SCHEME=/path/to/compatible/compactc-scheme
cargo build -p compact-rust-backend --bin compactc \
  --features experimental-recording-plan
target/debug/compactc --target rust --skip-zk --rust-require-recording \
  --rust-runtime-root "$PWD" \
  examples/rust_backend/effect_plan_prototype/compositions.compact \
  /tmp/effect-plan-prototype
```

Both exports must report `recorded: true` and `observed_call: true`. Without the
feature they remain native-valid but fail `--rust-require-recording`.
`proof_required: true` identifies a proof obligation; it does not report a proof
run. `--skip-zk` does not prove or verify the new compositions.

## What changed

Before: an effectful return containing a supported assertion or counter update
can fail a whole-circuit shape check.

After: ordered operations and typed values compose into a checked semantic plan:

```text
read(state) -> before
assert(next == 9)
increment(round, 1)
if selector == 0:
    write(state, next)
    return before
else:
    witness(mark) -> observed
    assert(observed == 77)
    ...
```

There is no circuit-name, branch-count, or operation-count admission rule in
this plan. Unsupported primitives still refuse with an IR path. Both branches
are statically checked; only the selected branch executes.

The implementation is in `tools/compact-rust-backend/src/recorded/effect_plan/`:

| Component | Responsibility |
| --- | --- |
| `model.rs` | Typed value identifiers, ordered bindings, structured branches |
| `lower.rs`, `lower/expressions.rs` | Resolve declarations and lexical names; lower IR in evaluation order |
| `validate.rs` | Check use-before-definition, value types, signatures and branch joins |
| `render.rs` | Convert a checked plan into `syn` syntax using existing runtime operations |
| `tests.rs` | Exercise invalid plans, source paths, scalar conversions and fixture lowering |
| `property_tests.rs`, `property_tests/corpus.rs` | Check finite composition laws, lexical scope and refusal/fallback boundaries |
| `resource_tests.rs` | Exercise selected depth and width limits in isolated processes |

An unsupported fragment may fall back to legacy recording. A violated plan
invariant or post-validation rendering failure is a hard compiler error.
Native validation and resource preflight run before recording. Declaration
resolution is trusted from the private lowerer; plan validation is not a second
ledger schema resolver.

The runtime ABI is unchanged. Reads/writes use existing typed slots; witnesses
use the existing metered frame operation; transaction preparation and official
ledger/ZK behavior retain their own checks. Syntax-independent plans are not
proofs of transaction readiness.

## Evidence and regeneration

`compositions.compact` has two exports, `guardedChoice` and
`orderedObservation`. The retained IR is
`tools/compact-rust-backend/tests/effect-plan-prototype-schema20-ir.json`.
After compiling the source with the matching frontend, compare that file with
`contract/compact-rust-ir.json`; refresh it only from actual compiler output.

The generated-crate acceptance tests cover eight successful scenarios, five
assertion failures, and skipped witness calls. They compare native and recorded
execution with an independent TS capture: return values, serialized ledger,
private state, witness observations, ordered transcripts, ledger effects,
all four execution/replay gas dimensions, and public trace replay. Two scenarios
start from a previously updated ledger state.

The capture script is
`runtime-rs/tests/fixtures/capture-effect-plan-prototype.mjs`. Compile the same
source with `--target ts --skip-zk`, make the matching
`@midnight-ntwrk/compact-runtime` package resolvable from the generated JS, then:

```sh
node runtime-rs/tests/fixtures/capture-effect-plan-prototype.mjs \
  /path/to/ts/contract/index.js /path/to/runtime/dist/index.js \
  > runtime-rs/tests/fixtures/effect-plan-prototype.json
```

## Proof and ledger-application gate

Put the matching ledger8 `zkir` 2.1.0 on `PATH`, then compile the same source
without `--skip-zk`. Use the experimental compiler built above:

```sh
target/debug/compactc --target rust --rust-require-recording \
  --rust-runtime-root "$PWD" \
  examples/rust_backend/effect_plan_prototype/compositions.compact \
  /tmp/effect-plan-proof
cargo run -p compact-rust-proof-smoke --features experimental-recording-plan -- \
  --effect-plan-prototype /tmp/effect-plan-proof
```

The runner checks that the artifact directory's IR matches the retained IR,
compares direct recording with typed observed-call preparation, then proves,
verifies and ledger-applies all eight successful cases. It uses the official
`LocalProvingProvider`, resolver and parameter interfaces. The shared test policy
**disables balancing**; real proof verification and the other default ledger
checks remain enabled. This is offline contract acceptance, not funded network
transaction qualification. IR equality is a local artifact sanity check, not
an authenticated build-provenance mechanism.

## Resource boundary checks

```sh
cargo test -p compact-rust-backend --features experimental-recording-plan \
  --lib recorded::effect_plan::resource_tests -- --nocapture --test-threads=1
```

These process-isolated tests request a 2 MiB worker stack, exercise direct
plan checking/rendering and the public compiler pipeline, and test the first
rejected value beyond each selected boundary. They cover syntax depth 48
(branches, arithmetic and lexical scopes), 4,092 local bindings and 3,275
sibling branches under the current conjunctive preflight limits. They do not
raise any resource ceiling. Measurements qualify these shapes on the tested
host/toolchain; arbitrary combinations and other platforms require evidence.

For Darwin ARM64 debug builds, `tools/compact-rust-backend/calibrate_effect_plan_frames.py`
can retain symbol prologues and binary/source hashes. Its static entry
reservations are not measurements of live stack usage. The validator currently
clones visible scope for each branch, giving quadratic aggregate copy work for
some wide inputs; the tested maximum is bounded, and a future optimization must
preserve branch scope and use-before-definition checks.

## Composition and migration controls

The deterministic compiler corpus checks 32 capture-avoiding alpha-renaming
pairs, 91 action-sequence split points with two regroupings each, shadowed
initializers and scope restoration, and unsupported children in unused bindings
and unselected branches. These are finite properties, not an exhaustive proof.
Public compiler tests run with the feature both enabled and disabled: malformed
native types/references remain errors; a valid wide unsigned literal outside the
experimental domain still records through legacy fallback. When both lowerers
refuse, the feature intentionally returns a more precise diagnostic IR path.

The generated fixture adds eight cumulative rewrites of the two original
circuits: alpha renaming, sequence regrouping with empty identities, identity
parameter shadow scopes, and negated conditions with swapped branches. Tests
compare 96 variant input cases with 24 base cases through native and recorded
execution. They check exact failures and witness prefixes, successful results,
ledger/private state, effects, private outputs, public programs and all four
execution/replay gas dimensions. These synthetic IR variants have no new
Compact source, independent TS capture or proof claim. The original two exports
remain unchanged and retain their separate TS and proof gates.

A ninth synthetic export selects a branch with no public operations. Native and
recorded execution both succeed; transaction preparation returns exactly
`PrepareCallError::EmptyTranscript`. Compiler controls show that the legacy
lowerer also admits a potentially empty selected branch. Thus `recorded` and
`observed_call` describe API availability, not guaranteed transaction
preparability for every input. This experiment does not change that boundary.

## Promotion gates

The feature attempts **every native-valid `StateReturn::Effectful` root**, not
just the two source examples. The supported algebra can therefore admit other
combinations. Unsupported helpers, collections, cryptographic operations and
Zswap fall back to legacy lowering. The scalar unsigned domain is bounded by
`u128`. Precise nested source spans remain separate work; diagnostics use IR
paths.

A local 198-contract comparison of the staged default compiler against the
pre-migration baseline found unchanged capabilities and IR for every contract;
only the existing effectful-return oracle's Rust syntax changed. Only two roots in that corpus use `StateReturn::Effectful`
(`choose` and coracle's `start`), so 198 is a regression count, not experimental
language coverage. The coracle fallback remained byte-identical.

## Staged default migration

Default integration first runs the complete legacy `lower_effectful` lowering,
including its read/write evidence checks. If admitted, it prefers the checked
plan; a genuine Unsupported result retains the legacy plan. An invalid plan is
a hard compiler error. When that family declines, the start-funding route and
generic default refusal remain unchanged. No new default admission is added.
The feature retains its experiment-first path for broader combinations.

This transition deliberately constructs the old plan before the new one. Both
can temporarily coexist; it does not remove recognition cost or promise a
compiler speedup. New default resource controls cover legacy-admitted depth 48
and a shape-specific maximum of 4,090 local bindings on requested 2 MiB workers,
plus neighboring preflight refusals. They qualify the tested macOS ARM64 debug
configuration, not arbitrary platforms or a universal live-stack bound.

The effectful-return fixture now compiles current default output at build time
and compares it with frozen legacy source. Its original TS controls remain,
plus 120 success inputs and 4 callback failure/skip inputs. Both original proof
branches use that fresh default fixture. See
[the migration oracle](../../../tests-rust-backend/effectful-return-oracle/README.md).

Generated public signatures, runtime ABI and IR schema remain unchanged.
`RenderError::RecordingPlanInvariant` is now also available in default compiler
builds; consumers exhaustively matching that public enum may need a new arm.

Broader default admission and retirement of legacy admission remain future work:
review scalar/arithmetic/error facets, transaction preparation policy and
mixed-shape/platform resource evidence first. The selected proof tests use the
established unbalanced policy. Do not infer full language parity, formal
equivalence, or code-size reduction. Decisions and execution receipts remain in
the Obsidian midnight vault until publication.
