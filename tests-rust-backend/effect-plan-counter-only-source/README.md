# Source-bound Counter-only frontend qualification

The build script renders the exact retained schema-20 IR using the current
default backend. Its source is `examples/rust_backend/effect_plan_counter_only.compact`.
This is the original Counter-only frontend failure, with no artificial outcome
Cell and no synthetic exports.

The shared boundary suite executes 108 inputs against native code, recorded
code and an independently assembled official slot program. It compares success
and official overflow/underflow refusal, result, state, effects, per-query gas,
private state, exact public operations and independent batched replay gas. A
query-order control demonstrates a changed transcript despite equal final state.

The separate `--effect-plan-counter-only` proof-smoke route checks matching fresh
IR before reading keys, then proves/verifies/applies both branches under the
existing unbalanced smoke policy. Its expected source has two Counters; the
older `--effect-plan-counter` route retains its Counter + outcome Cell source.
Successful compilation, replay and proof/application are distinct receipts.

```sh
cargo +1.99.0 test -p compact-rust-effect-plan-counter-only-source-fixture --locked
compact-rust-proof-smoke --effect-plan-counter-only /path/to/matching-compiler-output
```
