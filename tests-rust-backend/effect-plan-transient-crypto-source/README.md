# Transient crypto composition

The build script renders the retained full IR for
`examples/rust_backend/effect_plan_transient_crypto.compact` with the current Rust
backend. Tests execute generated native and recorded entry points against the
official ledger8 runtime crypto operations.

- `run` combines digest degradation, tuple hashing, conditional witness opening,
  commitment and digest upgrade with ordered ledger reads/writes. Tests compare
  results, private state, witness observations, effects, gas and replay with an
  independently assembled recording frame.
- `products` reuses an owned fixed vector for hashing and commitment. Tests check
  official encodings and native/recorded state and replay.
- Witness rejection is tested after the stored write, including branch skipping.

Run `cargo test -p compact-rust-effect-plan-transient-crypto-source-fixture`.
The proof-smoke switch `--effect-plan-transient-crypto ARTIFACT_ROOT` qualifies
both `run` branches against matching source-generated keys and checks the complete
retained IR before deploying. `products` is a runtime test, not a proof claim.

The algebra admits fixed FAB values only. Dynamic opaque values remain outside
this qualification. This fixture does not claim complete voting-family support.
