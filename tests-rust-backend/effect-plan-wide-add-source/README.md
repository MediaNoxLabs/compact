# Wide addition from genuine Compact source

The pinned ledger8 frontend compiles `examples/rust_backend/effect_plan_wide_add.compact` into the complete retained schema20 IR. The build script emits the current backend directly and requires recorded and observed support for all four source exports. Compiler ownership controls separately establish the checked algebra owner; capabilities alone do not establish ownership.

## Runtime coverage

Four tests cover 21 finite cases: nine successful ordinary calls, two post-add narrowing refusals, eight selected/unselected witness calls, and two callback failures. Native, recorded, observed and independently assembled Counter frames compare result, state, effects, gas, private state/output FAB, transcript and replay for successful calls. Literal public atoms and alignments verify carry across bit128, mixed Uint129/Uint8 addition, the maximum Uint129 result, and Uint128 narrowing. Witnesses read the Counter and expose left-before-right evaluation, once-only calls, lazy branch suppression and failure prefixes.

The source addition result bounds are inferred large enough for valid operands. Its failures are **explicit narrowing after addition**, not overflow of that inferred addition result. Direct arithmetic bound overflow, malformed types, and the maximum248-bit boundary are separate compiler/runtime controls. Inputs use checked unsigned carriers, so this fixture does not claim a constructible invalid public unsigned argument.

## Same-source TypeScript capture

`oracle/typescript.json` retains 21 historical executed cases: 15 successful runtime/preimage serializations and six refusals (four narrowing and two callbacks). It preserves source/frontend/generated TS/runtime artifact hashes, public and private atoms/alignments, callbacks and query costs. Artifact labels identify retained content; they are not fetchable paths or a claim that the original files are distributed. The original capture producer and source-command receipt remain private historical evidence. The separate portable successor and current-profile experiment below preserve that original capture; they do not recreate its missing tool/runtime epoch.

Official compact-runtime0.16.101 wraps callback ledger views in separate query contexts. Its circuit wrapper gas excludes callback reads. The capture retains that wrapper gas and independently sums every actual query; Rust gas is compared with this measured `queryCostSum`, not an adjusted or inferred wrapper number.

These tests prepare public calls with a deterministic test verifier key; they do not generate or verify proofs. The separate proof adapter qualifies a funded `add128(u128::MAX, 1)` execution when run by the parent gate. This fixture itself makes no proof claim.

## Portable producer and separate current-profile observation

[`oracle/capture.mjs`](oracle/capture.mjs) is the portable successor described in
[`oracle/PRODUCER.md`](oracle/PRODUCER.md). It keeps the complete original 21-case
assertion body and genuine QueryContext gas instrumentation, verifies the actual
selected runtime module, pins input bytes and protects existing outputs.
The original producer remains identified by SHA256
`cb21e82cfbaf9b0ffa441f41cc024fe5c0e8756c7e4b846604c18e530f80737a`.

A new current-profile run on 2026-10-16 used compiler 0.31.134, ledger8.1.3,
Node24.14.0, compact-runtime0.16.101, onchain-runtime3.1.2 and TypeScript6.0.3.
[`oracle/current-ledger813-typescript.json`](oracle/current-ledger813-typescript.json)
is that distinct capture, SHA256
`b6d2de03ff405ed923cfbb7a252ad48b46c960f1e130dc69fc51bb0c113fff99`.
The historical capture used onchain-runtime3.0.0. Both artifacts remain intact;
**tests still load the historical `oracle/typescript.json`**, not this new file.
The new capture is an observation and input/tool binding, not a replacement test
oracle or an authority repin. Its content labels are not fetchable artifact URLs.
The private invocation and input manifest are not distributed; their hashes do
not supply missing historical input bytes.

Observed finite results, independently accepted in terminal review:

- Fresh TS and Rust compilation with `--skip-zk`; complete fresh schema20 IR was
  byte-identical to retained SHA256
  `e11ffee3b080b66be4654de98b4a8fc4fbf7d9c5f8da556defe629eae155299d`.
  No source path, timestamp or arbitrary metadata normalization was used.
- All 21 executed TypeScript rows (15 successes, six refusals) matched the
  historical rows in full, in order and with exact scalar types, including gas,
  errors, transcripts, witness events and private state. Version metadata remains
  separate; this does not reproduce the historical onchain3.0.0 setup.
- Three private producer controls refused a different runtime module, a changed
  pinned generated input and an existing output; the sentinel was preserved.
- All four Rust fixture methods passed with `ledger-transaction` in a fresh own
  target. Actual `OUT_DIR` code was byte-identical to the fresh CLI Rust output;
  executed artifacts joined to current runtime and backend source inputs.

Independent coding-agent terminal review accepted this finite current-profile
successor: `wide-add-current-profile-terminal-independent-review.json`, SHA256
`f05978c9d7c3b1353d39c0cb35f544d20ab19ca2a3769dd224a46a4df34e305c`.
That receipt is retained external evidence, not distributed here. The old
onchain3.0.0/tool epoch remains historical and is not reconstructed. Publication
and final composed-source acceptance are separate. No proof, key generation,
full workspace, universal TS parity, production network or overall #508 acceptance
is asserted. The fixture's deterministic test verifier key is unchanged.
