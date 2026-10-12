# Source-derived Jubjub coordinate projections

The retained schema20 IR is the exact full pinned frontend output for
`examples/rust_backend/effect_plan_jubjub_projections.compact`. Build generation
uses the current Rust backend and requires both exported methods to provide
recorded and observed-call APIs. Parent compiler controls separately establish
actual algebra ownership; capability flags alone do not establish it.

Three runtime tests cover four public points (identity, generator, scalar2,
scalar17), selected/unselected two-witness branches with different points,
identity in either witness position, operand reuse, and first/second callback
failures. Callbacks read the Counter after its increment, making order and gas
observable. Every successful case compares native, generated recording, observed
facade and an independently assembled frame: result, state, effects, private
state/output, sequential gas, exact verify operations and replay. Batched replay
gas is compared to manual batched replay, not assumed equal to sequential gas.

Expected coordinates come directly from official midnight-transient-crypto
EmbeddedGroupAffine accessors; the manual oracle never calls the runtime X/Y
functions under test. Compact identity coordinates and the official point FAB identity input are
both(0,1). The pinned Jubjub CircuitCurve accessor returns Some coordinates
for every point; its identity is not a point at infinity. Prepared public input/output and
private witness FAB values are checked. No fabricated or invalid curve values
are introduced. Failed callbacks check precise errors and prefixes; the consumed
partial failure frame is unavailable, so no partial-failure gas/state claim is
made. The observed input snapshot is checked unchanged.

Historical baseline generated native/TS successfully but refused recording at
JubjubPointX. Baseline generation is not a runtime or proof result. No new runtime
API, curve arithmetic, TS oracle capture, proof, network or universal curve
coverage is claimed by these tests. Current execution receipts are kept in the
milestone evidence outside this fixture.
