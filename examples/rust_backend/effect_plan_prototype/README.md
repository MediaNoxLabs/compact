# Compositional recording prototype — #485 / ADR0355

The default compiler now admits supported `StateReturn::Effectful` compositions
through the checked effect plan (#511 / ADR0371). The old effectful shape
recognizer is test-only; independent start-funding and ordinary-root profiles
remain. Both real Compact circuits here work with the default backend. Their
explicit offline proof/application gate uses the shared unbalanced test policy.

The compatibility feature name `experimental-recording-plan` remains for the
prototype test/proof cohort and extra compiler property/resource tests. It no
longer changes production Effectful admission.
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
With the feature disabled this particular test fixture is inert. Enabling its
cohort does not enable a different backend admission mode; its build dependency
uses the default compiler. The feature is not a generated runtime flag.

To compile the Compact source through a compatible ledger8 Scheme frontend:

```sh
export COMPACTC_SCHEME=/path/to/compatible/compactc-scheme
cargo build -p compact-rust-backend --bin compactc
target/debug/compactc --target rust --skip-zk --rust-require-recording \
  --rust-runtime-root "$PWD" \
  examples/rust_backend/effect_plan_prototype/compositions.compact \
  /tmp/effect-plan-prototype
```

Both exports must report `recorded: true` and `observed_call: true` with the
default compiler and satisfy `--rust-require-recording`.
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

An unsupported fragment may try independent start-funding admission. A violated plan
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
native types/references remain errors. The former wide-literal fallback control
now lowers through the checked plan (#510), and production admission is now
independent (#511). When algebra and start-funding both refuse, the default
compiler returns the precise algebra diagnostic IR path.

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

## Current support boundary

The default recorder attempts **every native-valid `StateReturn::Effectful` root**, not
just the two source examples. The supported algebra can therefore admit other
combinations. Unsupported helpers, collections, cryptographic operations and
Zswap can still reach independently admitted legacy paths. Unsigned arithmetic,
casts and ABI surfaces remain bounded by `u128`. Local wide literals, references,
equality and coercion to Field use the existing canonical unsigned domain below
`2^248`; this does not add wide arithmetic or explicit wide `FieldCast` support.
Precise nested source spans remain separate work; diagnostics use IR paths.

A local 198-contract comparison of the staged default compiler against the
pre-migration baseline found unchanged capabilities and IR for every contract;
only the existing effectful-return oracle's Rust syntax changed. Only two roots in that corpus use `StateReturn::Effectful`
(`choose` and coracle's `start`), so 198 is a regression count, not experimental
language coverage. The coracle fallback remained byte-identical.

## Historical staged migration — before #511

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

## First production family retirement — #511

Default Effectful admission now uses the algebra directly. The old five
recognizer/entry functions live behind `cfg(test)` as an entry-level regression
reference with shared legacy helpers. Finite inclusion controls require old
admission first for every generated case, then checked support and matching
selected output. This is a reviewed rule mapping plus finite tests, not a formal
inclusion proof. Independent start-funding and ordinary-root selection remain.

The baseline admission inventory is preserved. Apply its reviewed transition:

```sh
python3 -B tools/compact-rust-backend/recording_admission_inventory.py \
  --transition tools/compact-rust-backend/recording_admission_inventory.transition-511.json
```

The gate verifies removal of the production call/definitions, test-only ownership
and reference fingerprints. Its 84 source boundaries and one retired route are
source-accounting metrics, not a language coverage percentage. Source changes
outside exact anchors/selector checks still need review.

More recording APIs can now be generated by default; Unsupported diagnostics use
the actual algebra node/path. Empty-public-trace preparation policy is unchanged.
The [wide-source proof fixture](../../../tests-rust-backend/effect-plan-wide-source/README.md)
uses its own exact generated contract and matching source artifacts. Broader
recognizer retirement, ordinary-route attribution, exhaustive IR classification
and mixed-platform qualification remain part of epic #508. Decisions and progress
remain in the Obsidian midnight vault until publication.
