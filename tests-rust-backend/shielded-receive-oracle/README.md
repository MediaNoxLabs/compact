# Unchanged shielded receive oracle

Builds the current backend from the complete retained schema-20 IR for
`examples/rust_backend/shielded_receive_oracle.compact`, including both `accept`
and `accept_renamed`. The build requires both recorded/observed capabilities.
Source, retained TypeScript oracle, and existing runtime tests remain unchanged.

```sh
cargo test -p compact-rust-shielded-receive-oracle-fixture --all-features
```

The runtime oracle compares native/recorded behavior and replay; its separate
wide-value test retains ledger u128 behavior without claiming TypeScript intent
parity above the existing TS output-descriptor limit.

The proof-smoke receive, wallet-funded receive and observational receive entry
paths share one full retained-IR check before any deployment or key access.
Original zero/wide/funded/observation proof scenarios retain their separate scope:
the wide output-only call proof does not establish a balanced offer.
