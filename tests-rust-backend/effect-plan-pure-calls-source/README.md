# Source-bound pure helper composition

Source: `examples/rust_backend/effect_plan_pure_calls.compact`.
The retained schema-20 IR is generated from that exact source; `build.rs` invokes
the current default Rust backend and requires the public `run` recording and
observed-call capabilities. No synthetic exports are added.

The scalar call boundary encloses rich pure internals: structs, default aggregate
values, aggregate conditionals/coercions, nested named calls, a vector fold with a
named callback, transient hashes and commitments. The stateful helper calls a
pure helper after a storage read. Three ordered witnesses feed the helpers, including two direct pure arguments,
and thread private state. A failing pure assertion is deliberate additional native
pure support; it is not claimed as part of the legacy terminal pure grammar.

`tests/calls.rs` compares native and recorded execution to an independent manual
recording program. The oracle uses official hash/commit primitives and the
aligned tuple representation of the source's two-field struct. It checks result,
public state, effects, execution gas, witness order/private outputs, exact public
operations and replay gas. Failure controls check witness prefixes and rollback
after the first successful update. The all-zero testkit artifact identity is
used only for rollback, never proof provenance.

The separate proof smoke route `--effect-plan-pure-calls <proof-output>` checks
exact retained IR before reading keys and runs both source branches through the
official preimage check, proof, verification and ledger application. Its policy
is the existing unbalanced smoke policy, not production transaction balancing.
Generate proof artifacts with the pinned compiler/SDK recorded in the local
slice receipt, using the exact source above.

Backend unit tests `effect_plan::tests::pure_inclusion` independently require
legacy terminal admission first for each old-domain representative. This finite
corpus supports migration evidence; it is not a proof of full grammar inclusion
or of the Compact specification. The terminal selector remains in service.
