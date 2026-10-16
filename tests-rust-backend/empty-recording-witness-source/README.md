# Witness-only recording from Compact source

Source: `examples/rust_backend/empty_witness_guard.compact`. The default-v2 frontend generated the retained whole `empty-witness-guard-schema20-ir.json`. The source is a derivative of guarded arithmetic: it calls the existing freshness guard, then a Unit helper that requests a Boolean `permit` witness and asserts it. No public ledger operation occurs in that root.

Frontend classification is explicit: three pure helper circuits; `recordFreshEnough` and `finish` are stateful; `permit` is a witness. Contract-info says `recordFreshEnough`: **pure=false, proof=false**. Recording/observation availability does not imply proof applicability. The signed597 baseline accepted both APIs through `guard.guarded-struct-pure` with a checked-supported body. `provenance.json` records the exact source/wholeIR hashes, original frontend command and classification.

Two tests cover fresh and unchecked-age successes; future/expired errors before any callback; false witness and provider failure after exactly one callback. Native, recorded and observed execution agree with an independent manual success trace: unchanged public state, zero public program/gas, private state7→8, one exact Boolean FAB output. Empty-transcript preparation is refused through both direct and observed APIs. Failure tests establish error/callback order, not rollback of consumed contexts.

`build.rs` uses the current backend without altering generated text. Source and IR were retained byte-for-byte from authenticated scratch generation, with the source basename preserved. No TS execution capture or proof is claimed here. The serialized empty verifier carrier is not a proof key.

Run: `cargo test -p compact-rust-empty-recording-witness-source-fixture --features ledger-transaction`.

## Historical frontend provenance metadata

The publication successor of `provenance.json` changes exactly four historical command metadata leaves: `$.command[0]`, `[7]`, `[8]`, and `[9]`. Original JSON SHA-256: `cbfbed6f6ce8263feed773a2f0bffd7daff94ca83307b20b267d5b32a29f55ce`; successor SHA-256: `3e8746c5e4ee10b57fc800dcbea4947f432e3bb3dee67def0dc440221dde1356`. The original record is retained privately.

The wrapper and source names carry their already recorded byte hashes as opaque `artifact:sha256:` labels. Runtime/output directory locations carry `historical-runtime-root:undistributed` and `historical-output-root:undistributed`; no directory hash is claimed. These labels are not distributed artifacts, resolvable URLs or runnable command arguments. The historical command metadata is **not runnable**. All other command positions, source/IR/wrapper/Scheme/prior-probe hashes and classification fields are unchanged. This four-field disclosure is separate from the oracle `files[].path` policy and adds no oracle transition row.

The original execution used the older ABI52 wrapper and authenticated Scheme. Its receipt does not attribute frontend generation to the current Rust renderer. Current `build.rs` renders the retained IR with the selected backend. `recordFreshEnough` remains stateful with **proof=false**; no frontend/TS recapture, proof run or empty-transaction claim is introduced by this metadata publication.
