---
id: RUST-ADR-0188
alias: ADR-0188
title: "Record native Zswap intents against authoritative offers"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-runtime-policy"
topics: ["Zswap", "offer-binding", "recording"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: cdee250f3ed3b233ff9add1c8107fdf53f1c9dad4be286d485b640e85e5827e9
---
# RUST-ADR-0188 — Record native Zswap intents against authoritative offers

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-runtime-policy. The default ABI48 offer binder requires exact contract-owned input coverage and ordered output equality, rejecting wallet inputs, change and transients absent an explicit later policy. A later correction superseded only this ADR's early API/parity-count receipt; the strict default policy remains in force.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#292 closure](https://github.com/MediaNoxLabs/compact/issues/292#issuecomment-6017724466). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3fa3b0ba`](https://github.com/MediaNoxLabs/compact/commit/3fa3b0ba87400f9ebe0f31264637dac4e8ff16b6). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: delivered-locally
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem and research boundary
ADR0175 records native intent plans but create_zswap_output refuses any observed allocation-locked context, and recorded APIs do not admit the intent forms. Original native_zswap_intents_oracle.flow is one proof-required gap; stateful_struct_oracle.planned has an intent-valued member. Research/design only until parent review. Branch codex/adr188-native-zswap-recording starts at integrated ABI47 fixture baseline 3fa3b0ba; no repository code edited.

### Confirmed upstream behavior
Fresh execution of the original TS artifact ${LOCAL_EVIDENCE}/compact-adr175-ts/contract/index.js via the existing capture script exactly matches committed nine-case native-zswap-intents-oracle.json; fresh capture is ${LOCAL_EVIDENCE}/compact-adr188-ts.json. flow(true) appends TWO identical output intents through pair; pot.writeCoin uses the existing commitment index and adds no third intent. At start7, outputs2, cursor9 and written qualified index8. This applies to both user and contract recipients. Intents themselves create no public VM queries or synthetic private witness outputs.

Pinned ledger-v8 8.0.3 reproduction ${LOCAL_EVIDENCE}/compact-adr188-duplicates.mjs and ${LOCAL_EVIDENCE}/compact-adr188-duplicates.log: merging identical singleton offers rejects `attempted to merge non-disjoint coin sets`; applying an already present commitment rejects `faerie-gold attempt with commitment` (upstream CommitmentAlreadyPresent). With the ledger cursor also7, a deduplicated singleton allocates index7 and advances to8, disagreeing with TS index8/cursor9. Upstream Rust try_apply sequentially rejects duplicate commitments in raw output arrays as CommitmentAlreadyPresent. Do not deduplicate/reindex this call into acceptance.

Local midnight-js packages/contracts/src/utils/zswap-utils.ts maps outputs by serialized CoinInfo (recipient absent from the key), and may normalize input/output pairs into transients. That inspected checkout is midnight-js-contracts3.0.0 and imports ledger-v7; this is version-qualified source research, not evidence of a current ledger-v8 SDK execution. The independent pinned8.0.3 primitive reproduction above establishes actual allocation and duplicate behavior. No wallet/network calls were made.

### Proposed bounded decision (pending review)
1. ABI48 is justified by new generated-facing RecordingFrame create_zswap_input/output methods and explicit offer-intent reconciliation errors/state. Schema20 already carries both forms and needs no change. Reuse CircuitContext native intent operations, adding an internal authoritative-allocation mode only for OfferBackedObservedState. Ordinary observed contexts remain locked; provisional native execution retains existing cursor semantics.
2. OfferBackedObservedState retains the exact validated offer, ledger start cursor and authoritative output map plus the contract-owned input evidence needed for reconciliation. Output intents must match the ordered offer output commitments and their actual indices; duplicates, missing/mismatched recipients/coins, reordered outputs and cursor/index disagreement reject explicitly. Never overwrite upstream com_indices. The bounded initial scope excludes transients and third-party/user input evidence; contract input nullifiers derive from upstream CoinInfo::nullifier(SenderEvidence::Contract(address)), and qualified indices must pass upstream Merkle path validation against the ledger tree. No proof-preimage layout parsing, copied hashing or generated VM programs.
3. Record intent operations as ordered local effects without public op/gas/private-output fabrication. Keep witness evaluation order and failure prefix. The frame's initial replay context is the offer's authoritative map from the start. Provisional recorded execution may be inspected, but intent-bearing transaction preparation must go through OfferBackedObservedState reconciliation; generic unbound preparation must not silently discard the plan. Empty public transcripts keep ADR0184 EmptyTranscript semantics.
4. Use an isolated recorded/zswap_plan.rs for bounded Unit intent Sequence/Let/If/direct forms, exact coin/recipient values, zero-argument typed witnesses, qualified Cell write and audited inline pair-style Unit helpers. Canonical Kernel claim/mint recording may be composed for a valid fixture. No call_local adoption of intent/query effects. Leave typed_plan.rs to ADR0187; planned composite-return admission requires a later explicit integration hook/review after187 delivery and is not claimed from Unit-only evidence.
5. Preserve the original duplicate flow as native/recorded/witness-order parity plus exact preparation/offer rejection. Add a DISTINCT-coin same-primitives transfer fixture for positive proof/application: explicit contract-owned funded input, one same-token/value output, actual Kernel nullifier/spend claims, qualified output write and Night-backed Dust fee spend. Claim values must match actual upstream input/output; ledger remains authoritative. This avoids forcing original deliberately unmatched/duplicate data into a valid transaction.

### Required evidence after approval
Original nine TS cases vs native/recorded intent arrays/cursor/state/effects/witness order and exact VM trace; duplicate raw offer/JS merge and deduplicated index mismatch; missing/wrong/reordered output, wrong recipient, unknown/mismatched input coin/index/nullifier, repeated nullifier and authoritative-map immutability; cursor overflow before side effects; empty selected branch exact preparation refusal; malformed operand/type/scope/helper rejection. Strict funded valid proof + independent verification/tamper rejection + default strictness + ledger application + exact commitment/index and spent-nullifier replay rejection. Preserve original source capability boundaries and report planned separately until joint integration evidence exists.

### Risks / unresolved review choices
Contract input comparison needs original ledger Merkle evidence, since an Offer input exposes nullifier/root but not its qualified coin index. Use upstream path validation and nullifier derivation, not private proof encoding. Offered user funding inputs may coexist with contract intents; reconcile only the explicitly bounded contract-owned input slice and document which unrelated wallet funding inputs are permitted. Offer output ordering may include wallet change: initial scope should require the circuit-produced output prefix to match exactly while explicitly handling or rejecting any trailing outputs. SDK normalization can destroy duplicate/index intent semantics and is not adopted. Generic preparation bypass must be closed without changing empty-transcript precedence or legacy offer-only callers. These points need parent review before implementation.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/292 (rust-backend-v2). Proposal sent for parent review; no repository implementation edits. Research hashes: duplicates.mjs c6ecc8a7d1e940acffa579369c39b451735c949ebb95cb468abc3172b1ced86a; duplicates.log b2898b9033774db4aa45fac71f976277b203e79c34f8e6ee65d8b84a9d8a4599; fresh TS capture 4901145a95feb961f7bf65f04a7f6ba4318ce6534d7d2292f06f9b2d4e059628.
### Research addendum: normalization and input-index validation
Upstream Offer::new calls normalize(), which sorts input/output/transient arrays without deduplicating them. Reconciliation must compare the preserved circuit output order/cursor with the normalized authoritative offer order/indices, not the input Vec used to create the offer. Reject mismatch; do not reorder the circuit plan. Input intent order is retained as execution evidence but input matching can use a unique bijection to contract-owned nullifiers because offer normalization sorts them independently.

Upstream MerkleTree::path_for_leaf takes the caller-supplied leaf and provides siblings; its success alone does NOT validate that coin/index pair. Use the public upstream tree.index(mt_index) to check the stored commitment hash and contract ownership exactly, or calculate upstream path.root and compare the actual tree root. This avoids any proof-preimage decoding and is stronger than merely checking index bounds. These corrections supersede the proposal's shorthand “path validation.”

Minimal initial offer scope recommendation: exact circuit-output list (no trailing wallet output/change, no transients), while permit unrelated user funding inputs only as explicitly separate offer inputs; match all inputs owned by this contract exactly to the circuit intents. An empty intent plan keeps legacy offer-backed callers unchanged. Any broader change-output/prefix or third-party contract behavior needs separate evidence.

### Approved implementation scope — 2026-10-05
Parent approved ABI48/schema20 and the first Unit-only slice. For calls with any intent, require exact equality of contract-owned input intents to ALL offer inputs: no extra wallet funding inputs. Require exact ordered output intents against normalized offer outputs and authoritative indices: no trailing change outputs. Reject transients. Dust fee funding remains separate. Empty intent plans retain legacy offer-backed behavior for ADR0180 funded Set/Cell and ADR0184 funded mint.

Use private typed state to distinguish provisional native, ordinary locked observation and authoritative offer-bound allocation. Never overwrite authoritative com_indices, deduplicate or reindex. Verify input coin/index against upstream stored commitment and owner, and derive nullifier with upstream SenderEvidence::Contract. Generic unbound preparation rejects nonempty plans after the existing EmptyTranscript check. Reconciliation must validate final plan/cursor/index map against the initial bound observation to close mutable-context bypasses. Preserve witness/intent order and exact failure prefixes.

Implement isolated recorded/zswap_plan.rs and coordinate with ADR0187; planned composite-member admission is deferred to a later reviewed joint integration. Keep original duplicate flow as execution parity and exact negative evidence; add distinct-coin funded positive transfer with proof, unchanged default strictness and ledger apply. Extra wallet inputs, change outputs and transients remain explicit unsupported composition scope. No push or remote CI.

### Delivered locally — 2026-10-05

Signed GPG+DCO commit `d83af8f564b5e678588904545719a99d315eef21`, verified with git verify-commit, base3fa3b0ba. Runtime ABI48 / IR schema20; explanatory commit body includes problem/change/tests and ADR0188/#292. Worktree clean. No push or remote CI.

The generated native `context.create_zswap_input(...)` / `context.create_zswap_output(...)` sequence now has typed recorded counterparts `frame.create_zswap_input(...)` / `frame.create_zswap_output(...)?`, through isolated recorded/zswap_plan.rs. Ordered Unit helpers are inlined in the same frame; typed_plan.rs is untouched. Recording preserves each native builtin's empty aligned Unit PRIVATE transcript entry alongside witnesses. This corrects the research shorthand suggesting intents had no private outputs: there are no invented public VM queries/gas, but the existing Unit private outputs are mandatory and independently TS-checked.

Runtime allocation is private typed Provisional / Locked / OfferBound state. Nonempty plans require exact offer inputs (all owned by this contract) and normalized ordered output commitments/actual indices, with no extra funding inputs, change outputs or transients. Input coin/index checks use upstream tree.index commitment+owner and upstream contract nullifier derivation. PublicTrace seals initial/final plan snapshots: replacing mutable execution context or hiding a pre-recording prefix rejects; final cursor/map/allocation cannot diverge from the bound observation. Generic unbound preparation rejects nonempty plans. EmptyTranscript remains earlier than intent reconciliation in the preparation core; existing address/operation/verifier and observation identity checks retain their precedence. Empty plans preserve legacy offer-only callers.

#### Accepted evidence
- 15 runtime units; 9 backend units and151 renderer tests; nine original plus two transfer independent TS/native/recorded cases.
- Explicit wrong/missing/duplicate/extra input, wrong index/coin/owner/nullifier, reordered/trailing/missing output, transient, hidden-prefix, map mutation and context-erasure rejection coverage. Original malformed types, pure placement and unsupported helper/escaped-binding guards remain.
- Focused source gate passes all5 original plus2 new recorded exports. Inventory has4 proof-required capability rows, all assessed; these capability counts are not claims that every invalid call proves/applies. Only2 new baseline identities (transfer/read_coin), no old identity changes.
- 2fixture freshness passes. Targeted all-target/all-feature Clippy -D warnings passes. Standalone runtime `cargo check --no-default-features` passes without warnings; fmt and git diff checks pass.
- Dedicated transfer4480-byte proof independently verifies and rejects changed binding. Its exact contract-owned genesis input funds distinct output42 of the same token; Night-backed Dust fees are separate. Unchanged WellFormedStrictness::default and ledger.apply pass at actual index1; final qualified Cell coin/index, output commitment and exact NullifierAlreadyPresent replay are checked.
- Wrong Kernel nullifier claim rejects exact upstream NullifiersNEClaimedNullifiers; balancing is disabled only for this isolated negative.
- Legacy ABI48 controls all pass: ADR0180 funded Set and Cell default strictness/application, and ADR0184 seven nonempty call proofs plus fundedmint42 default strictness/application. Their existing unfunded/claim negatives and empty selected branch remain explicit.

#### Original duplicate-flow boundary
Originalflow(true) retains TWO provisional outputs and last index/cursor behavior, with no successful proof/application claim. Binding to a singleton offer rejects exact ZswapOfferOutputMismatch; generic raw preparation rejects UnboundZswapIntents. flow(false) returns exact EmptyTranscript. A raw duplicated output offer rejects exact upstream CommitmentAlreadyPresent. Pinned8.0.3 JS merge rejects non-disjoint coin sets; deduplicating to one output gives index7/cursor8 versus originalTS index8/cursor9 at start7. No silent normalization/reindexing is performed. The transfer source uses the same primitive operations with distinct valid input/output instead.

#### Receipt and integration
`${LOCAL_EVIDENCE}/compact-adr188-delivery-receipt.json`, SHA256 `bb9d7ccc00379b7802449d6bf0c2e35c63592eea8ba0e3c395e50f240128d019`, records exact source/compiler/zkir/proving-artifact/binary/log hashes, commands, feature checks and boundaries. Key logs: proof-final.log, runtime-tests-3.log, backend-2.log, original-parity-3.log, transfer-parity-3.log, source-gate.log, freshness.log, clippy-final.log, runtime-default-final.log and legacy-{set,cell,kernel}.log under ${LOCAL_EVIDENCE}/compact-adr188-. Failed build/gas assertion iterations remain retained; corrected gas comparison uses TS query sum, with TS last-query aggregate kept distinct.

All164 unrelated temporary ABI47->48 fixture support edits for proof-smoke/Clippy were restored before commit. Only the original native-Zswap fixture and new transfer fixture are delivered. Parent owns full ABI48 refresh and combined integration. `stateful_struct_oracle.planned` stays outside this Unit slice; ADR0187/joint integration is separate. Extra wallet inputs/change/transients, general multi-contract/wallet composition, network submission and finality are not covered. Funded evidence requires MIDNIGHT_LEDGER_TEST_STATIC_DIR and ADR0180's explicit Night/genesis/time setup.


### Proof applicability correction (2026-10-05)
Integration of the actual local_parity_gate exposed an ADR188 admission regression: produce, consume and witness_order were recorded despite compiler contract-info marking all three nonproof. The prior 5 original + 2 transfer API claim was incorrect; only flow/read_coin and transfer/read_coin are proof-required. The bounded Zswap planner now requires both a native intent and a structurally present public query (CellWriteCoin or canonical Kernel claim/mint). Intent-only exports remain native-only; helpers may still inline into a proof-required caller. No VM activity is fabricated, and an empty selected flow branch retains EmptyTranscript. All nine original TS/native execution rows remain; only the three flow rows have recorded parity.
The real selected-two local_parity_gate passed, including Cargo tests, with 4/7 recorded exports and all 4 proof-required exports covered. Receipt: ${LOCAL_EVIDENCE}/compact-adr188-correction-local-gate/receipt.json. Source admission guard and 151 renderer tests passed; added negative verifies removing the flow Cell query rejects recording even with direct input and inlined pair output intents. ABI48/schema20, runtime reconciliation and funded proof semantics are unchanged. Earlier ADR188 receipt is superseded for the API and parity-count claims only.

Correction delivery: c6454ed5b1f3e8283a3d2c7da828ea8143efcf71, conventional explanatory body + GPG verified + DCO. Final receipt ${LOCAL_EVIDENCE}/compact-adr188-correction-delivery-receipt.json, SHA256 6ef477dee4752c6544310d1f4e7babfd03e54a5c49d55f5381b6c9b9d6ccf607. Focused three-package all-target/all-feature Clippy with warnings denied and formatting pass. No proof rerun for this admission-only correction; prior runtime funded proof evidence unchanged.
