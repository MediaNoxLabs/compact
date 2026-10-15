# Internal tuple interfaces and public boundaries

The genuine Compact source is `examples/rust_backend/effect_plan_internal_interfaces.compact`. Its complete schema20 IR retains an internal `touch` with one tuple12(OpaqueBytes, eleven Fields) parameter and the same result, plus internal `project` with that tuple parameter and a small named result. The frontend does not flatten either interface. Both helpers increment a Counter and invoke `trace`; the public wrapper returns only an encodable named product.

The build script generates current Rust, checks preserved internal interfaces, and requires exactly one public capability (`run`, recorded + observed). There are no internal public call facades.

## Five tests, distinct claims

Two unit tests call the real generated `pub(crate)` native helpers directly. Three value rows include empty, canonical and terminal-zero bytes; the tests inspect the complete twelve-element result, mutate an owned clone, and call the projection helper. Two failure rows check direct callback refusal.

Three integration tests execute the public wrapper across twelve rows: four canonical branch/value combinations, two noncanonical raw byte rows, and six callback-failure/branch controls. They compare native/recorded/observed/manual results, state, effects, private state/outputs, gas, full verification operations and replay. Canonical public atoms and alignment are explicit. Native/recorded preserve raw terminal-zero bytes; the observed public facade rejects them before callbacks. Failure callback3 is unreachable on the false branch.

**The wrapper was already recordable before #563.** These integration tests are composition regressions, not proof of new internal-root support. Compiler tests independently check/render the source's internal roots under retained internal authority and reject public/absent-authority counterparts. A separately compiled direct-root rendered harness qualifies the new checked plan. No public tuple12 encoding, witness tuple12 encoding or helper-discovery expansion is implied.

The only Clippy allowance is generated tuple-signature type complexity: complete internal tuple12 signatures are intentionally retained rather than replacing the emitted source with fixture-specific aliases. Test support uses its own named `LocalTuple` alias.

## Evidence boundaries

No new proof or TS parity claim is introduced. Random verifier keys in tests support preparation only. The pre-change whole generated output and capabilities are retained in `/tmp/rust040-internal-interfaces/retained-baseline/`, with source/IR/output hashes in `retained-baseline-receipt.json`; parent qualification compares final output against that baseline.

An initial helper returned tuple projection `input[11]` directly as Field; the pinned frontend refused that stateful return shape. The final source wraps identical projections in named Piece. The original draft/log are preserved at `/tmp/rust040-internal-interfaces/direct-field-projection.*`; this separate limitation is not claimed fixed here.

Local gate: `cargo test -p compact-rust-effect-plan-internal-interfaces-source-fixture --all-features --offline`.
