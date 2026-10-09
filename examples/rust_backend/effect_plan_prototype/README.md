# Compositional recording prototype — #485 / ADR0355

This experiment adds an **opt-in** recording lowerer. It demonstrates two real
Compact circuits whose primitive operations already exist in the runtime, but
whose combinations the legacy effectful-return profile refuses. It does not
replace the default recorder or establish proof/transaction qualification.

## Run the experiment

From the repository root:

```sh
cargo test -p compact-rust-backend --features experimental-recording-plan
cargo test -p compact-rust-recording-effect-plan-prototype-fixture \
  --features experimental-recording-plan
```

The fixture's build script generates Rust into Cargo's `OUT_DIR` from the
retained schema20 frontend IR. No generated Rust copy needs manual maintenance.
With the feature disabled the fixture is inert, and the existing backend remains
the default. The feature applies to the compiler build; it is not a runtime flag.

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

## Promotion gates

This experiment selects only `StateReturn::Effectful` at compiler integration.
It deliberately leaves general helpers, collections, cryptographic operations
and Zswap on the legacy path. Its scalar unsigned domain is bounded by `u128`.
Precise nested source spans remain separate work; diagnostic IR paths are used.

Before default promotion: qualify real proofs, verification and ledger application
under a stated policy; validate resource/stack assumptions for the new traversal;
expand negative/composition coverage; and review legacy fallback retirement.
Do not infer full language parity, formal equivalence, or code-size reduction
from these examples. Planning, decisions and execution receipts remain in the
Obsidian midnight vault until milestone publication.
