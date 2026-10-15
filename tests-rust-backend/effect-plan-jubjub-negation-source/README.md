# Composable Jubjub negation

This fixture recompiles the unchanged, genuine default-ZKIR-v2 source
`effect_plan_jubjub_negation.compact`. `close(expected)` calls a witnessed Unit
helper which checks both negated coordinates before its final permit witness.
`coordinates(fallback, selected)` lazily chooses the point witness or fallback,
then returns a named coordinate product. Counter increments before all callbacks.

At the frozen #584 baseline, `close` was already recorded/observed through the
Unit selector, while `coordinates` was unavailable. Both algebra candidates
refused precisely EcNeg. The build now requires both generated APIs to be
recorded/observed; the containing compiler admission report records actual owner
migration separately.

Two tests bind ten distinct same-source official TS cases: five execution and
preparation successes, five provider/assertion refusals. Cases cover identity and
nonidentity points, both output coordinates, suppressed unselected provider,
point-provider refusal, permit-provider refusal, false permit, and a wrong-x
assertion in the Unit helper which stops before permit. The wrong expected value
is a valid identity point. No invalid-coordinate constructor or separate wrong-y
failure claim is made.

Native, recorded, observed and manual execution agree on values, exact aligned
private outputs, callback order and private state, Counter state/effects and gas.
Recorded verifier replay is compared with independent manual replay, including
gas; replay gas is not equated with execution gas. Successful prepared input and
output atoms/alignment match TS. Supplied observed state remains unchanged.

The oracle captures official ecNeg answers plus official involution/additive
inverse reference laws. Expected points are decoded from frozen TS coordinates;
the manual recorder never calls Rust ec_neg. The maintained capture adds explicit
`negated` coordinates to the original ten unchanged case rows for the independent
wrong-x assertion control. This is an additive oracle field, not a source or
scenario change.

```sh
cargo test --offline --locked -p compact-rust-effect-plan-jubjub-negation-source-fixture --all-features
```

To recapture, first regenerate both targets from the unchanged source using the
pinned frontend. Then run `node oracle/capture.mjs GENERATED_CONTRACT_DIR
OFFICIAL_TS_RUNTIME_DIR`. The capture hashes source/full IR/generated JS, its
script and official runtime lock/dist/WASM. It claims execution and serialized
preimage agreement; a selected strict proof has separate source-bound evidence.
