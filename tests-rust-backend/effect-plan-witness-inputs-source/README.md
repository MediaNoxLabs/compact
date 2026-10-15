# Owned witness arguments from complete Compact source (#557)

The current backend emits the retained complete schema-20 IR for
`examples/rust_backend/effect_plan_witness_inputs.compact`. The build requires
recorded/observed capability for the single public `run` circuit. Compiler tests
separately identify actual algebra ownership; no source or generated Rust is
rewritten to obtain runtime coverage.

`run` passes Bytes, String, tuple2, vector2 and a nested named product into one
Boolean-result witness. The first call evaluates two stateful argument helpers.
Its public Counter transcript has distinct increments 1, 2, 3; the witness sees
Counter6 and private state7. An optional second call reuses the owned arguments
and sees private state8. Witness implementations mutate their received copies
after recording exact inputs, so the next invocation must still receive the
source's original values.

## Tests

Four test functions cover:

- Seven raw text/bytes cases × both branches × two fresh-context runs on the
  same witness objects (28 executions per native/recorded/manual path): empty,
  Unicode/invalid-UTF8 Bytes, internal/leading zeros, long values and terminal
  zeros. Every callback argument form is compared to the original raw values.
- Eight refusal/lazy cases: callback1/2 returns false or Err, with selected and
  unselected branches. Exact errors and callback prefixes are compared.
- Six canonical observed/prepared cases: public input atoms match literal
  original bytes; only one/two Boolean private outputs are encoded.
- Three noncanonical public-input cases: generated observed guards refuse
  before callbacks, while the raw execution cases above demonstrate unchanged
  witness-input preservation.

Successful native/recorded/manual executions compare results, state, effects,
private state, gas, private outputs and complete recorded verify transcripts.
Public replay matches execution state/effects. Batched replay gas is compared
against manual batched replay, not the sum of separate execution queries.

The independent manual reference uses official RecordingFrame/CounterSlot APIs;
it verifies compiler wiring, order and ownership, not the underlying ledger VM
implementation. Distinct helper immediates expose argument order in the full
transcript rather than relying only on the commutative final Counter sum.

## Boundaries

Witness arguments are passed directly as typed owned Rust values. They are not
FAB-encoded separately and need no public/storage normal-form validation.
Callback **results** still use the existing private FAB conversion; this fixture
uses canonical Boolean results and does not change that result policy.
Generated observed public guards remain separate and deliberately reject raw
terminal-zero public arguments before execution.

This qualifies bounded source/runtime/preparation behavior, not proofs, funded
transactions, network access, zero-copy callbacks or dynamic payload bounds.
Tuple9 and larger-product compiler compatibility probes are separate evidence;
this source covers tuple2/vector2 and named product composition.

```sh
cargo test -p compact-rust-effect-plan-witness-inputs-source-fixture --all-features --tests
```
