# ADR0359 / #494 — opt-in encoded-byte policy preset

Implemented after the sizing agent confirmed 64 MiB has generous headroom over retained state/key artifacts. Parent owns the sizing report, ADR/CoPS/docs, acceptance history and commits. Only two source files changed by this agent:

- `runtime-rs/src/transaction/decoding.rs`: `EncodedSizeLimit::DEFAULT_MAX_BYTES = 64 * 1024 * 1024`, `Default` and const `max_bytes(self)` accessor. Existing `new(max_bytes)` remains unrestricted by the preset.
- `runtime-rs/tests/transaction_decoding.rs`: two focused tests added to the five existing codec/admission cases. Real small state and deterministic seed353 verifier carrier pass under the preset; explicit budgets 0, preset−1, preset, preset+1 and usize::MAX retain their values and expected small-artifact behavior. No preset-sized allocation or stress input is used.

The preset is an explicit application policy, not a protocol/global cap, maximum supported artifact size or decoded heap/CPU bound. Legacy decoders receive no implicit policy. A source comparison confirms all state/key decoder functions (legacy and explicit limited) are byte-identical to the pre-change implementation. Default only constructs a policy passed to already explicit limited APIs. ABI and dependencies unchanged.

## Focused validation

All commands, source/lock identities and logs are recorded in `receipt.json`.

- `cargo +1.99.0 test --locked --offline -j4 -p midnight-compact-runtime --features ledger-transaction --test transaction_decoding`: **7 passed**, exit0. Shared target `target/compact-rust-parity-gate`, `CARGO_INCREMENTAL=0`. Raw log `transaction-decoding.log`.
- Same package/features/test target with `cargo +1.99.0 clippy ... -- -D warnings`: **passed**, exit0. Raw log `clippy.log`.
- Scoped Rust1.99 rustfmt check and `git diff --check`: **passed**; raw empty success logs retained.

The existing tests retain exact/custom/one-under/zero/preparse-oversize, malformed tag, truncation, trailing and empty-input coverage. No broad coverage/proof/MSRV/consumer dependency build or network test was performed. Passing codec tests do not establish proof validity or aggregate resource containment.

## Handoff

Source is frozen for parent review. Cargo lease released; no additional Cargo or source edits planned. No commits, pushes, vault or unrelated repository edits by this agent. Historical evidence and concurrent0.4.0 documentation remain intact. Broader decoded heap/object/CPU containment is explicitly deferred by the accepted milestone scope.


## Root delivery and independent review

Source delivered as signed conventionalGPG/DCO9ffd7880405ed5cfd62909ed4aac64a638c7fc02. Root inspected actual7-test/Clippy logs and diff. A separate sibling who did not implement the code reviewed API compatibility, opt-in preset, unclamped override and size-only claims; no actionable finding. No total containment, proof or finalcandidate claim. Previous runtime97.83%instrumentation is tied toef352bcd; this small later additive policy has focused execution but no fresh whole-component percentage is claimed.


Verified archive: [ADR0359 — Practical byte policy implementation.zip](decoder-policy-implementation.zip), SHA256 `f581b9913f547d0114b02eceb18aa3f269d2de66c1dedd36f5029a54ff6cf4d4`, 8members.

Root verified both source hashes against workingtree and exact9ffd7880commit, all recorded log/before-source hashes, and unchangedCargo.lock.
