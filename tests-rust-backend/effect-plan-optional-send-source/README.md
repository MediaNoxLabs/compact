# Optional shielded send: aggregate admission versus runtime branch

Separate genuine source `examples/rust_backend/effect_plan_optional_send.compact` preserves the original send corpus. The full schema20 IR is rendered with the current backend during build. The public root conditionally invokes unchanged standard-library `sendShielded` or returns `default<ShieldedSendResult>`.

The complete old planner aggregates effect evidence over branches. The compiler's whole-old-first source test separately checks old admission and checked-plan support; runtime tests here exercise the resulting branch behavior. Actual selected owner is recorded by compiler qualification, not inferred from this fixture's public capability.

## Two tests

- Eighteen source rows: both selected values × user/self/foreign-contract recipients × amounts17/42/43 against input42. Fifteen succeed; three active overspends refuse. Native and recorded behavior compare with independently assembled runtime frames (official kernel/coin primitives, source nonce domain constants), including returned change/sent values, public state/effects, Zswap intent order, private Unit outputs, gas, full VM operations and replay. Retained same-source official TS rows check outputs, private outputs, transcript and measured query-cost sum.
- Nine inactive observed calls suppress the overspending path and have no intents/transcript; preparation refuses with exact EmptyTranscript. Nine active calls preserve the existing unbound observed allocation boundary: valid amounts yield ZswapAllocationLocked; overspend yields UnsignedUnderflow first. A direct generated recorded call on the identical observed context must return the same error. This fixture does not fabricate an offer or claim active observed proof success.

The false branch returns defaults, leaves state/effects unchanged, allocates no output and keeps the initial cursor. Kernel query/callback counts are not mistaken for logical branch coverage. Underflow Rust results expose the error, not an inspectable partially consumed context; ordering is additionally qualified through successful full transcripts and the observed allocation-vs-underflow boundary.

`oracle/typescript.json` is an executed 18-row source capture using official Compact runtime0.16.101 with the ledger8 16-byte coin value descriptor. It retains source/frontend/generatedTS/runtime hashes. No proofs were performed for this companion. Existing strict funded send and transient adapters provide separate proof gates.

Local gate: `cargo test --offline --all-features -p compact-rust-effect-plan-optional-send-source-fixture`.
