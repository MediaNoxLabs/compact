# Wide addition from genuine Compact source

The pinned ledger8 frontend compiles `examples/rust_backend/effect_plan_wide_add.compact` into the complete retained schema20 IR. The build script emits the current backend directly and requires recorded and observed support for all four source exports. Compiler ownership controls separately establish the checked algebra owner; capabilities alone do not establish ownership.

## Runtime coverage

Four tests cover 21 finite cases: nine successful ordinary calls, two post-add narrowing refusals, eight selected/unselected witness calls, and two callback failures. Native, recorded, observed and independently assembled Counter frames compare result, state, effects, gas, private state/output FAB, transcript and replay for successful calls. Literal public atoms and alignments verify carry across bit128, mixed Uint129/Uint8 addition, the maximum Uint129 result, and Uint128 narrowing. Witnesses read the Counter and expose left-before-right evaluation, once-only calls, lazy branch suppression and failure prefixes.

The source addition result bounds are inferred large enough for valid operands. Its failures are **explicit narrowing after addition**, not overflow of that inferred addition result. Direct arithmetic bound overflow, malformed types, and the maximum248-bit boundary are separate compiler/runtime controls. Inputs use checked unsigned carriers, so this fixture does not claim a constructible invalid public unsigned argument.

## Same-source TypeScript capture

`oracle/typescript.json` retains 21 executed cases: 15 successful runtime/preimage serializations and six refusals (four narrowing and two callbacks). It records exact source/frontend/generated TS/runtime hashes, public and private atoms/alignments, callbacks and query costs. `oracle/capture.mjs` is the exact capture script, with scratch/dependency paths explicit; regenerate TS with the pinned frontend before executing it. The source command is recorded in `/tmp/rust040-wide-add/source-receipt.json` during this delivery.

Official compact-runtime0.16.101 wraps callback ledger views in separate query contexts. Its circuit wrapper gas excludes callback reads. The capture retains that wrapper gas and independently sums every actual query; Rust gas is compared with this measured `queryCostSum`, not an adjusted or inferred wrapper number.

These tests prepare public calls with a deterministic test verifier key; they do not generate or verify proofs. The separate proof adapter qualifies a funded `add128(u128::MAX, 1)` execution when run by the parent gate. This fixture itself makes no proof claim.
