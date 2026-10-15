# Rich checked value source qualification

Source: `examples/rust_backend/effect_plan_rich_values.compact`. Build-time
fixture generation uses its exact retained schema-20 IR and the current default
backend. The public root still has Boolean/Field arguments and a Field result;
no profile priority or family retirement is requested by this fixture.

The source exercises fixed bytes, enum variants, nested nonempty structs,
construction, defaults, struct projection, a rich conditional value, tuple
construction/equality temporaries, rich Cell reads/writes, rich witness
arguments/result, rich pure/stateful helper boundaries, and repeated uses of
owned struct values after those boundaries. Tuple interfaces and TupleIndex are
outside this bounded slice.

`support.rs` is an independent source-semantics oracle using official slot and
recording-frame operations. Runtime tests and the separate proof adapter share
that readable oracle; it never calls a generated pure or stateful helper. Tests
compare native/recorded/manual results, state, effects, execution gas, witness
inputs/private outputs, exact operation order and independent replay gas. Both
rich branches and a refused witness after the first Cell write are covered. The
all-zero testkit identity is rollback-only and is not proof provenance.

`--effect-plan-rich-values <proof-output>` checks the exact retained IR before
keys. It uses the actual current generated fixture for both branches, official
preimage checks, proof, verification and ledger application under the existing
unbalanced smoke policy. This is finite source qualification, not a claim of
production transaction balancing or full language coverage.

The Bytes literal is the standard Compact `pad(3, "tag")`, which emits a literal
Bytes IR node. A separate heterogeneous tuple containing a raw tuple-to-Bytes
cast is currently rejected by the Rust frontend; that source normalization gap
is recorded separately and is not bypassed by a synthetic retained IR here.
