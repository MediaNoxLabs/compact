# Owned witness result composition

This fixture renders the retained full schema20 IR for
`effect_plan_witness_domain.compact` using the current backend. Its public circuit
keeps the existing `Boolean -> Boolean` surface, and every witness has zero
parameters. The change under test is the admitted witness result domain.

`run` obtains an owned OpaqueBytes result after a ledger write, stores it, and
conditionally obtains a fixed vector, heterogeneous tuple, 160-bit unsigned value
and a named struct containing vector, tuple and wide integer members. Values are reused in equality expressions; the vector is also
hashed into ledger state. The callback observes the preceding ledger writes using
the generated metered ledger view.

Runtime tests compare generated native and recorded execution with an independent
recording frame built from official runtime operations. They check callback order,
once-only calls, private state and FAB outputs, dynamic payloads, final ledger
state/effects, gas, complete public transcript and replay. Each callback also has
a refusal control, including skipped branches and preceding successful callbacks.

This is runtime qualification only. There is no proof, network, broader witness
parameter, or broader public/stateful signature claim.

Run `cargo test -p compact-rust-effect-plan-witness-domain-source-fixture`.
