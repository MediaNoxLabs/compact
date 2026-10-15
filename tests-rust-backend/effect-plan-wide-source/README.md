# Wide source proof fixture

This crate generates exactly the retained compiler IR for
`examples/rust_backend/effect_plan_wide.compact` during its build. It does not add
synthetic exports or use the original narrow contract as a substitute.

The default proof command is:

```sh
cargo run --offline -p compact-rust-proof-smoke -- \
  --effect-plan-wide-source /path/to/wide-source-artifacts
```

The artifact directory must contain fresh ledger8 keys/ZKIR and
`contract/compact-rust-ir.json` generated from that same Compact source. The
runner checks parsed IR equality before loading keys. This is a local input
identity check, not authenticated proof-key provenance; retain source, tool,
IR and key fingerprints with the execution receipt.

Both selected branches compare native/recorded results, private state, private
outputs, query state/effects and all execution gas dimensions. Direct and typed
observed-call preparation must agree. The shared official proof/ledger gate
then proves, verifies and applies each call using its existing **unbalanced**
smoke policy. Successful smoke runs do not establish funded network acceptance.

Run with the default feature set for default-backend evidence. The shared
`effectful_scenario` helper also serves the original `--effectful-return` route;
its adapter still selects the original fixture. Neither route substitutes one
generated contract for the other.
