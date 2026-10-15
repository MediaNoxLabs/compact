# Stateful helper call composition

Generated from the exact retained IR of
`examples/rust_backend/effect_plan_stateful_calls.compact` using the current
backend. The root has an effectful return and calls stateful helpers through
`Expr::Call`. This qualifies helper infrastructure; it does not retire the
ordinary terminal selector or add pure-helper support.

The source combines left-to-right witness arguments, repeated and nested helper
calls, caller/callee names that overlap, two branches, Cell reads/writes and an
assertion after a helper write. Three integration tests cover 18 successful
input/branch combinations, 12 witness failure prefixes and two assertion
rollback cases. Native and recorded execution are compared against an independent
program of official typed slots, witness transitions and arithmetic, including
result, public/private state, effects, witness order/private transcript, exact
public operations, execution gas and independent batched replay gas.

`compact-rust-proof-smoke --effect-plan-stateful-calls <matching compiler output>`
checks exact retained IR before loading keys, then proves both branches for
first=3, second=5, private state=7 and permit=true. Explicit six-witness seed and
output oracles accompany native/recorded and direct/observed preparation checks.
Proofs use the existing unbalanced smoke policy. Failure and rollback tests do
not claim proofs of failing calls; finite cases are not full language coverage.
