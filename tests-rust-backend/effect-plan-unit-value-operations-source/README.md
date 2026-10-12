# Local Unit value operations

Issue #586 / ADR0437. Genuine ledger8 Compact source generated with default ZKIR v2, retained whole schema20 IR, and a fresh same-source official TypeScript oracle.

The source retains Field subtraction, checked Jubjub construction and `default<Bytes<32>>` in its IR. `close` calls a stateful Unit helper; `evaluate` returns a named product after selected coordinate witnesses. Both increment `calls` first. Witness order is X, Y, permit; the unselected branch calls permit only.

The frozen #585 baseline records `close` through `selector.unit-composition`; its checked candidate refuses Subtract. `evaluate` is unavailable in that baseline and first refuses ConstructJubjubPoint. The build script requires both recorded and observed APIs after this slice. Current production ownership is separately measured by the admission exporter; this README does not infer ownership from the presence of an API.

## Behavioral coverage

Two tests use twelve uniquely identified TS scenarios:

- Five successful Rust/TS scenarios: identity, ordinary nonidentity subtraction, modular subtraction with a negative mathematical difference, lazy witness suppression, and the Unit helper.
- Six matching provider/assertion refusals: X, Y, permit, denied permit, wrong coordinate and wrong default bytes.
- One existing Rust safety boundary: coordinate pair `(1,1)` is rejected as `InvalidJubjubPoint` after X/Y callbacks, before permit. Official TS `constructJubjubPoint` stores the pair without curve validation, calls permit and serializes the proof preimage. That TS success remains unchanged in the oracle. It is not counted as Rust/TS execution parity or a successful proof.

Successful scenarios compare native, generated recorded, observed and a hand-recorded reference: output, exact private FAB, callback/private-state order, Counter state, effects, verifier program and preparation input/output. Replay gas is compared with independent manual replay gas. Execution gas uses the sum of TS circuit and callback ledger queries, not the TS wrapper's narrower reported cost. The original observed state remains unchanged.

The manual recorder uses frozen TS modular coordinates and explicit guard outcomes; it does not call the Rust point constructor or subtraction to produce the expected result. Random verifier keys here qualify preparation only, not proofs. A selected source-bound proof is a separate proof-smoke gate.

`Default` coverage here is exactly retained Bytes32, not the full generic Default domain. Raw compiler/runtime controls qualify other carriers separately.

## Reproduce

```sh
cargo test --locked -p compact-rust-effect-plan-unit-value-operations-source-fixture --features ledger-transaction
node oracle/capture.mjs /absolute/generated/contract /absolute/official-ts-runtime
```

The capture records source/fullIR/generated JS/script and official runtime file hashes. Historical first `(0,0)` exploration is retained only in external evidence, explicitly corrected: Rust treats `(0,0)` as an identity alias, so it must not be labeled invalid.
