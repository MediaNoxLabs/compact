---
id: RUST-ADR-0184
alias: ADR-0184
title: "Record bounded Kernel shielded effects"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Kernel", "proof"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e31c7f2235b156d16f106c200ed350b1d3c8794d90f4a777a35df23df959da13
---
# RUST-ADR-0184 — Record bounded Kernel shielded effects

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. A separate recorded Kernel profile executes canonical ledger queries and Verify operations in source order. Seven nonempty calls prove and verify; only the seeded funded mint passes default-strict application in this checkpoint, and other claim/empty-branch limits remain explicit.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#288 closure](https://github.com/MediaNoxLabs/compact/issues/288#issuecomment-6017718090). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`35a9c59f`](https://github.com/MediaNoxLabs/compact/commit/35a9c59f1d3305212a326d41a320cc844721ec80) · [`3fa3b0ba`](https://github.com/MediaNoxLabs/compact/commit/3fa3b0ba87400f9ebe0f31264637dac4e8ff16b6) · [`b433acec`](https://github.com/MediaNoxLabs/compact/commit/b433acec08f29c0d3e4571d6c25c8d642334396d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: delivered-locally
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem
ADR0177 admits native Kernel mint and claim operations through their actual effects frame. Seven proof-required exports in the unchanged kernel_shielded_effects_oracle.compact remain without recorded or observed APIs: mint, nullifier, spend, claim_receive, batch, selected and witness_order. read_state is already recorded. The source inventory identifies these seven as explicit gaps.

### Before and after
```compact
kernel.mintShielded(disclose(domain), disclose(amount));
kernel.claimZswapCoinSpend(disclose(commitment));
```
Before, generated native code calls CircuitContext Kernel methods, and --rust-require-recording refuses the Kernel exports. After, generated recording uses typed runtime methods:
```rust
let frame = frame.kernel_mint_shielded(runtime::ledger::HashOutput(domain.into_array()), amount.value() as u64)?;
let frame = frame.kernel_claim(runtime::ledger::KernelClaim::CoinSpend(commitment))?;
```
Generated code never embeds VM programs, invents ledger slots, inserts balances or supplies fictional coin allocation. Observed calls use the existing common recorded-call adapter.

### Bounded emitter decision
Add a separate recorded/kernel_plan.rs module. Admit Unit Kernel return expressions and ordered Kernel action/Sequence/Let/If forms whose values are exact Bytes32, Uint64 or Boolean parameters, or declared typed zero-argument witness results with lexical scope. Require at least one statically present Kernel operation. Reject public Cell/collection effects, native Zswap intents, stateful helper composition, unsupported expression/result types and effectful return plans. Preserve left-to-right operands, branch-local frames and metered witness private transcript ordering. This module deliberately does not broaden the separate shared typed planner or ADR0182's effectful-return admission.

### Runtime decision
RecordingFrame methods execute the upstream QueryContext query and append the existing ADR0177 canonical generic-result-mode Kernel program as Verify operations, carrying resulting context and observed gas. Both native and recorded paths share exactly the same program builders. Overflow, duplicates and malformed effects remain upstream VM decisions. Reserve runtime ABI47 for this generated-facing recording API; IR schema20 is unchanged. No Scheme change is expected.

### Evidence and transaction boundary
Extend all twelve independently captured TS cases to compare native, recorded and replayed states/effects, exact ordered public VM programs, witness/private results and all four actual query-sum gas dimensions. Keep TS batch last-query aggregate separately. Include duplicate claims, zero/max/same-domain/different-domain mint values, overflow, false/true selected branches and domain-before-amount witness order. Add typed renderer rejection guards and runtime failure/zero-budget checks where they prove the shared VM boundary.

Generate pinned ZKIR2.1.0 proofs for all seven admitted call shapes, independently verify cryptographic proof statements and reject altered binding inputs. Arbitrary claim bytes in the source oracle do not establish an accepted ledger offer: matching coin commitment/nullifier semantics remain mandatory. Assert exact upstream transaction rejection for unmatched claims; any positive application evidence must state its actual matching offer and fee prerequisites. Call proof acceptance alone is not funded transaction acceptance, network submission or finality. Do not relax coin claim semantics to make this fixture apply.

### Delivery
Create the linked GitHub issue in rust-backend-v2 before source edits. Coordinate with ADR0182's typed_plan.rs owner; use separate Kernel module/runtime ownership. Existing target/adr157, focused Rust tests/Clippy/freshness/source gate/proofs only, GPG+DCO conventional commit with explanatory problem/change/validation body. Parent owns combined fixture refresh and integration. No push or remote CI.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/288 (rust-backend-v2). Parent approved bounded design and ABI47. Include a positive nonzero mint transaction with its minted output offer and Dust fee funding, while unmatched claim fixtures must reject exactly upstream. Matching-offer claim composition remains native Zswap integration work.
### Accepted local delivery — 2026-10-05

Signed conventional GPG+DCO commit `86df6044a6437348f858bec89771721ca1ebe404` (base b433acec), verified with git verify-commit; explanatory body includes problem/change/tests and ADR0184/#288. Schema20 unchanged; ABI47. The new runtime methods call the existing shared apply_verify_program executor with canonical Kernel constructors. Only the affected Kernel fixture is committed; 164 temporary ABI-only fixture support edits used for proof-smoke/Clippy were restored before delivery. Parent owns combined regeneration/integration.

Passed: 9 backend units, 148 renderer tests, 12 independent fresh-TS/native/recorded/replay cases, 2 runtime duplicate/overflow/zero-budget boundaries, strict source guard, 1-fixture freshness, targeted Clippy (-D warnings), fmt and diff checks. Seven nonempty API shapes independently prove/verify (2912 bytes each) and reject changed binding inputs. A separate mint42 transaction carries the actual custom-token output plus Night-backed Dust funding; unchanged WellFormedStrictness::default and ledger.apply both pass, with output commitment/index0 and contract state checked. Fee setup retains ADR0180's explicit genesis/Night/time prerequisites and requires MIDNIGHT_LEDGER_TEST_STATIC_DIR. This establishes offline application only.

Arbitrary unmatched claims are independently proof-verified, then their unproven transaction forms reject exact upstream NullifiersNEClaimedNullifiers, CommitmentsNEClaimedShieldedReceives or AllCommitmentsSubsetCheckFailure. Negative validation disables balancing only to isolate claim semantics. No matching-claim transaction acceptance is claimed. Matching-offer claim composition remains native Zswap integration work.

#### Zero-query boundary comparison
`selected(false)` has execution parity and zero effects/public queries. It refuses observed preparation with exact PrepareCallError::EmptyTranscript and is NOT proof-covered. Pinned @midnight-ntwrk/ledger-v8 8.0.3 JavaScript reproduction `${LOCAL_EVIDENCE}/compact-adr184-empty-ts.mjs` shows partitionTranscripts(empty) returns two absent sections; ContractCallPrototype/Intent/Transaction retains one call; wellFormed rejects “Calls cannot have empty guaranteed and fallible transcripts” (upstream CallHasEmptyTranscripts). `${LOCAL_EVIDENCE}/compact-adr184-empty-ts.log` preserves the result. Upstream ledger/src/construct.rs split_at converts empty programs to None, and ledger/src/verify.rs rejects both absent. Local midnight-js contracts/src/utils/ledger-utils.ts createUnprovenLedgerCallTx always adds the prototype; there is no skip in that builder. No actual wallet submission was performed. Rust's earlier preparation refusal is consistent with this boundary; no transcript was fabricated and no separate bug is asserted.

#### Gas and retained failed evidence
Replay at original query boundaries equals native/recorded/TS query-sum gas. A single combined replay preserves state/effects but shares upstream cache and therefore has different batch gas; initial false gas-equality failures remain in parity.log/parity-diagnose.log and are replaced by the explicit segmented comparison. Initial proof.log ended at selected(false) EmptyTranscript; proof-2.log tests that refusal explicitly and passes the full intended scope. Initial keygen.log recorded missing zkir; pinned keygen succeeded. All logs remain in ${LOCAL_EVIDENCE}/compact-adr184*.

#### Receipt
`${LOCAL_EVIDENCE}/compact-adr184-delivery-receipt.json`, SHA256 `72043643bd403bc05b4a0f23c87b5f0a9691b5e31ec801d11a0bf45cf40b00d1`, records commit, commands, exact source/compiler/zkir/key/artifact/log hashes, temporary-build support, accepted evidence and exclusions. Main proof result `${LOCAL_EVIDENCE}/compact-adr184-proof-2.log`; Clippy `${LOCAL_EVIDENCE}/compact-adr184-clippy-final.log`; source/freshness and backend/runtime logs are hashed there. No push, remote CI, network submission or finality claim.


### Main integration checkpoint — 3fa3b0ba

Signed 35a9c59f plus fixture refresh 3fa3b0ba, runtime ABI 47 / schema 20. Seven-source focused gate passes 20/20 recorded APIs; all seven nonempty Kernel calls prove and verify. Funded mint42 passes default strict validation and ledger application. Arbitrary claims retain exact ledger rejection; selected(false) retains EmptyTranscript. Logs: ${LOCAL_EVIDENCE}/compact-integrated-abi47-kernel-proof.log and ${LOCAL_EVIDENCE}/compact-focused-3fa3b0ba/receipt.json. Backend/runtime Clippy and formatting pass. Broad gate remains b433acec; no remote CI/push.
