# Field-to-Bytes32 source oracle

`build.rs` emits the current compiler output from the complete retained IR of
`examples/rust_backend/field_to_bytes32_oracle.compact`. It requires the sole
stateful export `snapshot` to remain recorded and observed. The original source,
public pure `encode` export, runtime tests and pinned TS oracle are unchanged.

This refresh qualifies `snapshot` moving from the legacy return recorder to the
checked context-query route. Public recording capabilities are unchanged; no
proof or new source-language support is claimed by this fixture update.

The retained `field-to-bytes32-schema20-ir.json` was captured using the pinned
ledger-8 frontend. Recompile the complete source with `--target rust --skip-zk`
and compare `contract/compact-rust-ir.json` before updating that IR. Run:

```sh
cargo test --offline --locked -p compact-rust-field-to-bytes32-oracle-fixture --all-features
```
