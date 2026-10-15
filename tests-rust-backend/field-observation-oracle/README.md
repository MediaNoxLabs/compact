# Current-generated field-observation-oracle

`build.rs` renders the complete unchanged
`examples/rust_backend/field_observation_oracle.compact` retained IR using the current backend.
The existing tests preserve independent TypeScript observations for Field Cell reads, nested helper calls and selected observations.

```sh
cargo test -p compact-rust-field-observation-oracle-fixture --all-features
```

For proof qualification, `compact-rust-proof-smoke --composite-zswap ORIGINAL TRANSFER`
binds both complete source IR files before opening either set of keys. Cross-source
substitution and changed declarations are rejected. The original planned output
uses value zero; the separate transfer consumes and emits 42. Both retain separate
Dust funding, default strict validation, ledger application and changed proof-binding
rejection. Negatives preserve the exact -42 output-only deficit, extra wallet input
mismatch, missing contract receive claim, unbound intents and spent-input replay.
Field-observation runtime checks are not an additional proof claim. Intent recording
alone does not establish valid wallet settlement or offer acceptance.
