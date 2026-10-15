# Exact opaque hash operands and private outputs

This genuine default-ZKIRv2 fixture separates local execution, observed inputs,
private FAB and ledger preparation. It uses the existing opaque-byte witness
interface; no opaque-string witness support or language policy is added.

## Two small source paths

`witnessed(fallback, selected)` increments a Counter, chooses one byte witness or
its public fallback, hashes that same owned value through hash-to-curve,
transient hash and transient commitment, then requests permission. The result
contains both curve coordinates and both Field hashes. The byte witness runs
once, even though three operations consume its value. The permission callback
runs second and observes the same Counter value.

`nested(envelope)` hashes a named product containing another named product,
opaque string/bytes, a Field nonce and a two-element opaque-byte vector. This
checks recursive exact input conversion without changing ordinary ledger FAB.

Both roots were already admitted by `algebra.composite-values` before #583. This
fixture tests semantic correction rather than a newly admitted source shape.

## Sixteen independently captured TypeScript cases

| Surface | Cases | Expected boundary |
|---|---:|---|
| Canonical successful execution | 5 | Native/recorded/observed/manual/FAB/preparation agree |
| Noncanonical private byte witness | 2 | Exact execution and private FAB agree; preparation refuses |
| Noncanonical public fallback/product | 4 | Native/recorded exact hashes agree; observed input refuses |
| Witness provider or permission failure | 5 | Exact callback prefix and typed execution error |

The TS capture executes the original generated contract and independently checks
all three hashes using official runtime APIs. It separately attempts official
serialized-preimage conversion: five succeed, six successful executions refuse
because an aligned value contains trailing zero bytes. Those refusals are not
misrepresented as execution failures.

For private `[0]` and `[1, 0]` answers, native and recorded Rust must preserve those
exact private output atoms. The manual recorder uses
`RecordingFrame::try_witness_metered_exact`; ordinary Boolean permission output
uses the existing generic method. Observed execution succeeds, and preparation
returns `NonCanonicalPrivateOutput { output_index: 0, atom_index: 0 }`. No
normalization is used to manufacture a serializable witness result.

Noncanonical public inputs keep the existing `InvalidLedgerCell` refusal before
observed execution. All eleven native/recorded successes retain the original TS
hash values, including trailing-zero operands. The original observed state stays
immutable on provider, guard, input and preparation failures.

## What the tests compare

Three focused tests cover exact IDs and complete witness/input configuration;
source-order callbacks and private state; independent frozen hash answers;
private transcript atoms/alignment; state/effects; metered execution gas;
recorded versus manual verifier programs and replay; and original successful
prepared input/output FAB. Replay gas is compared to manual replay, separately
from execution gas. The manual reference does not call Rust hashing under test.

`tests/common/witness.rs` owns callback and recording semantics,
`values.rs` owns captured-result decoding and input construction, and `state.rs`
owns observed ledger setup. There is no alternate cryptographic implementation.

## Reproduce and limits

```sh
compactc --target rust --target ts --skip-zk \
  examples/rust_backend/effect_plan_exact_hash_input.compact /path/to/output
node tests-rust-backend/effect-plan-exact-hash-input-source/oracle/capture.mjs \
  examples/rust_backend/effect_plan_exact_hash_input.compact \
  /path/to/output/contract /path/to/official/compact-runtime \
  tests-rust-backend/effect-plan-exact-hash-input-source/oracle/typescript.json
cargo test --locked -p compact-rust-effect-plan-exact-hash-input-source-fixture \
  --features ledger-transaction
```

Source, full IR, generated JavaScript, capture script, runtime/lock and WASM hashes
are retained in the capture. Build-time Rust generation is unchanged output from
that IR. This fixture does not claim real proving, verification or funded ledger
application; those require separate evidence. Earlier #582 proof source and keys
remain unchanged.
