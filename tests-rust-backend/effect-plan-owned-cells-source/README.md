# Declared owned Cell values

This fixture recompiles the full retained IR of
`examples/rust_backend/effect_plan_owned_cells.compact` with the current backend.
The build checks exactly `run` and `inspectText` are recorded and observed.

The source declares OpaqueString, OpaqueBytes, and a nested named Struct Cell
containing an optional-string Struct. Pure/default producers and copy helpers
feed a branch, storage writes, readback, projections and repeated non-Copy use.
The only root input is Boolean; results are an existing supported Receipt. No
opaque root or witness ABI is introduced. The 16 actual source declarations
place the string at physical path `[0,0]`, with other fields in the second chunk.

Three execution tests cover four populated/default-state × selected/unselected runs,
two isolated valid reads, source constructor/layout consistency, and one
malformed UTF-8 read. Nonempty strings/bytes are supplied as ledger-compatible seeded
ledger values, not Compact literals. A separate characterization test records existing constructor normalization of
noncanonical trailing-zero atoms. Pinned TS ledger boundaries reject those inputs;
Rust currently normalizes them before execution. This known runtime-policy
difference is recorded in CoPS-011 and [#539](https://github.com/MediaNoxLabs/compact/issues/539);
it is not newly introduced by recording. The false branch produces empty owned
values using source pure/default helpers.

Native and recorded executions are compared to a manually assembled program using existing runtime
Cell slots backed by the official ledger for results, ledger state/effects, execution gas, private state and
outputs. Public query order and replay gas are checked against the manual
program. Untouched neighboring fields retain their seeded sentinels. Malformed
string reads must return the same existing runtime InvalidLedgerCell error; no partial
frame recovery is claimed.

A fourth test isolates the existing noncanonical-input behavior before any
generated call.

No proof or full TS execution is claimed. This fixture qualifies storage
composition, not dynamic opaque memory bounds or broader interface support.

```sh
cargo test --offline --locked -p compact-rust-effect-plan-owned-cells-source-fixture --all-features
```

To refresh retained IR, compile the original source with the pinned ledger-8
frontend and `--target rust --skip-zk`, then compare the complete
`contract/compact-rust-ir.json` before replacing
`tools/compact-rust-backend/tests/effect-plan-owned-cells-schema20-ir.json`.
