---
id: RUST-ADR-0197
alias: ADR-0197
title: "Match TypeScript shielded coin descriptor to u128 values"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-runtime-fix"
topics: ["TypeScript", "u128", "shielded-coin"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 967408e17b6789181e847f42f64ca5b14f5549fb1d7b60e70ae7917cce8137b7
---
# RUST-ADR-0197 — Match TypeScript shielded coin descriptor to u128 values

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-runtime-fix. TypeScript ShieldedCoinInfoDescriptor uses the declared Uint128 value width, matching ledger coin identity and accepting wide values. Historical ADR195 oracle refusals are preserved as before-fix evidence, not current Rust semantics.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#300 closure](https://github.com/MediaNoxLabs/compact/issues/300#issuecomment-6017737632). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`b8b96c77`](https://github.com/MediaNoxLabs/compact/commit/b8b96c77b1ed838ec46b39a6c96b15506045982f). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

Issue: https://github.com/MediaNoxLabs/compact/issues/300 (rust-backend-v2).

### Problem
The Compact ShieldedCoinInfo value and ledger8 Rust carrier are u128, but TypeScript runtime ShieldedCoinInfoDescriptor uses MaxUint8Descriptor: max2^64−1 and b8 alignment. Generated receiveShielded/createZswapOutput calls reject valid coin values2^64 and u128MAX when constructing commitments. Root pinned probe `${LOCAL_EVIDENCE}/compact-ts-u128-alignment-probe.json` shows that changing only alignment b8→b16 accepts those values while preserving commitments for0,42,u64MAX. This is a TypeScript descriptor defect, independent of Rust emitter/schema/ABI.

### Before / after
```typescript
// Before: value uses an eight-byte descriptor.
export const MaxUint8Descriptor = new CompactTypeUnsignedInteger(18446744073709551615n, 8);
// ShieldedCoinInfoDescriptor alignment/fromValue/toValue all reference MaxUint8Descriptor.

// After: preserve the existing descriptor for compatibility; use the new one for coins.
export const MaxUint16Descriptor = new CompactTypeUnsignedInteger(340282366920938463463374607431768211455n, 16);
// ShieldedCoinInfoDescriptor alignment/fromValue/toValue reference MaxUint16Descriptor.
```

### Decision and ownership
Make the descriptor-only width correction and update EncodedShieldedCoinInfo.value documentation from64 to128bits. Keep MaxUint8Descriptor API and semantics unchanged. Do not change generic CompactTypeUnsignedInteger encoding/range policy: its fromValue checks maximum, while toValue uses canonical Field encoding and the commitment boundary checks alignment. Tests should identify each actual rejection boundary without claiming that toValue alone rejects above-bound values.

Runtime ownership remains upstream `runtimeCoinCommitment` and existing createZswapOutput allocation/append order. No Rust runtime ABI, private IR schema, compiler emitter or ledger funding policy change. Preserve ADR0195's original-runtime failure captures and label new corrected-runtime evidence separately.

### Validation plan
Use pinned onchain-runtime-v3 3.0.0 from the existing installation. Test0,42,u64MAX,2^64,u128MAX for coin descriptor roundtrip, commitments and createZswapOutput, both contract and public-key recipients. Verify old/new narrow commitments are byte-identical and selected-recipient semantics are retained. Reject−1 andu128MAX+1 at descriptor decode/commitment/output boundaries as applicable, with no query-context/index/output mutation on rejection. Test generated receiveShielded wrapper with corrected isolated JS; retain original refusal evidence unchanged. Run TypeScript typecheck, the full affected runtime unit suite and focused formatting/lint where configured.

The evidence proves encoding, commitment and circuit-intent behavior only: no funded wide-value ledger/proof acceptance claim. Record exact source/runtime/compiler/dependency hashes, command receipts and a conventional GPG+DCO local commit; no remoteCI/push.

### Isolation
Own worktree runtime is a real directory, with no dist/node_modules. Build into an isolated /tmp tree; do not run scripts through another owner's dist or shared runtime symlink. Existing installed dependencies may be read through explicit links, with generated JS/cache located only in the isolated tree.


### Focused verification and existing build limitation
The corrected isolated generated receive wrapper passes14 cases (ten accepted calls across accept/accept_renamed for0,42,u64MAX,2^64,u128MAX; four negative/out-of-range rejections), with exact pure generated/upstream commitments for both contract/public-key recipients. Existing MaxUint8Descriptor remains unchanged. Twenty-one new descriptor/output tests and26 existingstdlib tests pass. Corrected capture is separately named `runtime/test/fixtures/shielded-receive-u128-corrected.json`; originalADR195 refusal capture is read-only and its hash is retained. No wide fundedledger/proof claim.

The full runtime suite was executed:47pass/2Schnorr failures. Pristine signed2b3f3d05 baseline has26pass/the same2failures. Pinned onchain-runtime-v3 3.0.0 lacks jubjubSampleScalar, producing TS2339 in built-ins.ts275 and TypeError in both Schnorr tests. Production tsc and test-tsconfig diagnostics are byte-identical baseline/candidate; test-tsconfig also retains two existing number→bigint argument errors at stdlib.test.ts140/146. This is tracked separately at https://github.com/MediaNoxLabs/compact/issues/302. No error suppression or declaration shim; emittedJS checks are useful but full build/suite are explicitly not green.

Isolation uses copied source/test trees under ${LOCAL_EVIDENCE}/compact-adr197-runtime and ${LOCAL_EVIDENCE}/compact-adr197-baseline-runtime with local node_modules directories and read-only dependency symlinks. Repository export-version.ss ran through installed Chez10.4.1 against current compiler field library. All generatedJS/cache is local to /tmp, with no shared runtime dist/node_modules mutation.

### Signed local delivery

Commit `ee4f671c151b101bda41ffcff4f8cccda21b7f7a` is conventional GPG+DCO signed; root integrated it as `b8b96c77`. No push/remoteCI.

Exact clean-worktree receipt: `${LOCAL_EVIDENCE}/compact-adr197-delivery/receipt.json`, SHA256 `dec1f07153b92c5b09b76abb59f74e0cd71f879d6af82314c6a910c44a1af032`. Eleven commands rerun at signed head. Status explicitly `focused_passed_with_verified_preexisting_full_runtime_failures`.

- Twenty-one new descriptor/output tests plus26stdlib tests pass; ESLint/Prettier pass.
- Generated receive passes14cases (ten accepted, four negative/out-of-range refusals); corrected capture rows reproduce exactly, bothrecipient pure/VM commitments agree, narrow commitments unchanged.
- Original-runtime generated receive still refuses wide values with b32b32b8 alignment. OriginalADR195 capture hash remains unchanged.
- Candidate full suite47pass/2Schnorrfail; pristine baseline26pass/same2fail. Production/test TypeScript diagnostics are byte-identical; no new diagnostics. Existing build/Schnorr gap is https://github.com/MediaNoxLabs/compact/issues/302, not suppressed.
- No Rust ABI/schema/emitter edits, generic unsigned validation change, or fundedwide ledger/proof acceptance claim.

ADR0197 accepted in midnight. Receipt records compiler, source, original/corrected runtime and dependency hashes; all generatedJS/cache remains isolated under/tmp.


### Integrated TS coin-width correction — b8b96c77

ADR197/[#300](https://github.com/MediaNoxLabs/compact/issues/300) integrated as signed/DCO `b8b96c77`. ShieldedCoinInfo uses a dedicated u128/b16 descriptor; exported u64 descriptor and generic unsigned encoding policy stay compatible. Schema20/Rust ABI48 unchanged.

Combined main targeted runtime gate: **47 tests pass** (21 new,26 existing), targeted ESLint and Prettier pass. The first direct test attempt lacked generated version.ts; the canonical Chez export-version stage repaired that build prerequisite, then the same tests passed. `${LOCAL_EVIDENCE}/compact-b8b96c77-integration-receipt.json` retains both attempts.

Isolated exact delivery `${LOCAL_EVIDENCE}/compact-adr197-delivery/receipt.json` also records14 generated receive cases, both recipient kinds, unchanged narrow commitments, wide acceptance and atomic invalid-value rejection. Original ADR195 refusal captures remain unchanged; corrected-runtime evidence is separate. These are execution/encoding checks, not funded wide-value ledger application.

**Full TS runtime gate is still failing:** pristine and candidate reproduce the missing pinned `jubjubSampleScalar` API and existing test number/bigint errors, tracked in [#302](https://github.com/MediaNoxLabs/compact/issues/302). ADR198 is researching the separate compatibility fix. No diagnostic suppression or full-gate success claim. Rust corpus remains the previously measured356/368 at15999efb; this TS-only change adds no recorded Rust API. ADR195 proof/application work continues and ADR194 is ready for the shared-emitter handoff. No push, publication or remote CI.
