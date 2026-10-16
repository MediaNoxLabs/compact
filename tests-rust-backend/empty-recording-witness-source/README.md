# Witness-only recording from Compact source

Source: `examples/rust_backend/empty_witness_guard.compact`. The default-v2 frontend generated the retained whole `empty-witness-guard-schema20-ir.json`. The source is a derivative of guarded arithmetic: it calls the existing freshness guard, then a Unit helper that requests a Boolean `permit` witness and asserts it. No public ledger operation occurs in that root.

Frontend classification is explicit: three pure helper circuits; `recordFreshEnough` and `finish` are stateful; `permit` is a witness. Contract-info says `recordFreshEnough`: **pure=false, proof=false**. Recording/observation availability does not imply proof applicability. The signed597 baseline accepted both APIs through `guard.guarded-struct-pure` with a checked-supported body. The original command/classification receipt is preserved with the separately reviewed historical evidence; it is not distributed in this core extraction. The retained whole IR and current source remain inputs to the existing execution tests.

Two tests cover fresh and unchecked-age successes; future/expired errors before any callback; false witness and provider failure after exactly one callback. Native, recorded and observed execution agree with an independent manual success trace: unchanged public state, zero public program/gas, private state7→8, one exact Boolean FAB output. Empty-transcript preparation is refused through both direct and observed APIs. Failure tests establish error/callback order, not rollback of consumed contexts.

`build.rs` uses the current backend without altering generated text. Source and IR were retained byte-for-byte from authenticated scratch generation, with the source basename preserved. No TS execution capture or proof is claimed here. The serialized empty verifier carrier is not a proof key.

Run: `cargo test -p compact-rust-empty-recording-witness-source-fixture --features ledger-transaction`.
