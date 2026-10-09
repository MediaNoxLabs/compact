# Native Zswap intent oracle

Builds the complete retained source IR with the current backend. Capability
assertions preserve `flow` and `read_coin`, add `witness_order`, and retain the
`produce` / `consume` recording refusals.

The existing TypeScript oracle rows check native intent order, private outputs,
state, gas and allocation indexes. Recorded `flow` and `witness_order` are also
compared to that native result and the retained TypeScript public operations.
The two `witness_order` rows cover selected=true/false with a user recipient;
the wider nine-row oracle also covers contract recipients. The new
`witness_order` availability is runtime evidence; it is not proof qualification.

```sh
cargo test -p compact-rust-native-zswap-intents-oracle-fixture --all-features
```
