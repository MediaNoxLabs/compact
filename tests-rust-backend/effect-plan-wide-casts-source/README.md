# Source-derived wide unsigned casts

The complete schema-20 IR comes from `examples/rust_backend/effect_plan_wide_casts.compact`, compiled by the pinned ledger8 frontend. The build script renders current Rust and requires recorded + observed support for every export. Compiler tests separately assert actual algebra ownership.

Three tests cover 30 rows: 15 successful public casts/boundaries, five narrowing refusals, and ten conditional witness rows (seven successful, three refused). Successful executions compare generated native, recorded, observed and independently assembled Counter/witness frames: values, state, effects, private state/outputs, gas, complete verification transcript and replay. Prepared public atoms and widths are checked against literal expectations. Private outputs/public atoms also match retained same-source official TS executions. Rust gas matches the sum of measured official TS query costs, including witness ledger reads.

- `widen8`: Uint8 → Uint129; `widen129`: Uint129 → Uint248.
- `narrow8`: Uint129 → Uint8; `narrow129`: Uint248 → Uint129.
- `max248`: successful maximum Uint248 literal boundary (frontend folds the literal cast).
- `witnessed`: increment Counter, conditionally call `nextWide`, narrow Uint248 → Uint8, then call `marker`. Failure must stop before marker; unselected must skip nextWide even when its configured value would fail. Each executed callback reads the incremented Counter exactly once.

The two safe-carrier rejection controls test invalid Uint129/Uint248 construction. These are not observed-facade rejection tests: safe Rust unsigned carriers cannot represent values outside their declared bound. Narrowing a valid source carrier to a smaller range is exercised through genuine generated calls. Low-limb overflow preserves `UnsignedOutOfRange`; high-limb overflow preserves `InvalidUnsignedValue`.

`oracle/typescript.json` records 30 executed official TS rows, source/frontend/generated-code/runtime hashes and versions (Compact runtime 0.16.101, onchain runtime 3.0.0). Twenty-two successful rows also passed official preimage serialization. The other eight report source-located cast failures with exact callback prefixes. This capture is execution/serialization evidence, not proof verification. Rust random verifier keys are preparation-only carriers. Parent qualification owns any real proof work.

A draft source with a second Counter increment after the witness/cast expression was rejected by the pinned frontend as an unsupported nested ledger query. The draft and exact log are retained under `/tmp/rust040-wide-casts/source-two-increments.*`. The final fixture uses one leading increment; it makes no claim to cover that separate frontend limit. Wide arithmetic, field casts and noncanonical bounds are outside this fixture.

Local gate: `cargo test -p compact-rust-effect-plan-wide-casts-source-fixture --all-features --offline`.

## Gas accounting boundary

The generated TS wrapper reports only its own Counter update cost. Each witness receives a separate ledger-view QueryContext; accessing `ctx.ledger.calls` executes an official read query whose cost is absent from the wrapper total. Rust native/recorded/observed callbacks meter those reads. The oracle preserves `gasCost` verbatim and captures each real `QueryContext.query` result in `queries`; `queryCostSum` is the arithmetic sum of those measured costs, with no assumed adjustments. Tests compare Rust total gas with that sum. Unselected witnessed has two queries (update + marker read); successful selected witnessed has three (update + nextWide read + marker read).

The first capture and failed direct wrapper-gas comparison are preserved under `/tmp/rust040-wide-casts/original-wrapper-gas-capture/` and `tests-qualified.log`. Recapture keeps the same source, values, runtime and callback behavior; it changes observation only. This fixture does not claim equality of the TS wrapper's reported total with Rust's total.
