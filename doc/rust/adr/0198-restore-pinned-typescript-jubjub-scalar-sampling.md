---
id: RUST-ADR-0198
alias: ADR-0198
title: "Restore pinned TypeScript JubJub scalar sampling"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-runtime-fix"
topics: ["TypeScript", "JubJub", "randomness"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 404661035ce3394b075edee6bb9f990bf67c43788409b5bcceee2e33c7c6e940
---
# RUST-ADR-0198 — Restore pinned TypeScript JubJub scalar sampling

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-runtime-fix. The pinned TypeScript JubJub sampler restores correct scalar sampling at its provider boundary. It does not change Rust code generation or globally relax scalar validity.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#302 closure](https://github.com/MediaNoxLabs/compact/issues/302#issuecomment-6017741396). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`f58fabde`](https://github.com/MediaNoxLabs/compact/commit/f58fabde95d2623daa151e8a9c4c1dc4678631c9). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

Issue: https://github.com/MediaNoxLabs/compact/issues/302 (rust-backend-v2).

### Problem and baseline
The public jubjubSampleScalar implementation calls ocrt.jubjubSampleScalar, absent from pinned onchain-runtime-v3 3.0.0. Production TypeScript check reportsTS2339; both Schnorr unit tests fail with TypeError. ADR0197 preserved pristine/candidate evidence under ${LOCAL_EVIDENCE}/compact-adr197-delivery and did not suppress this gap. Test typechecking additionally finds two existing number literals passed to bigint-typed convertBytesToUint.

### Decision
Use randomBytes from the existing @noble/hashes/utils.js dependency, whose implementation uses platform crypto.getRandomValues and throws when unavailable. Rejection-sample the exact existing JubJub scalar modulus: request32fresh bytes, mask only unused high bits (derive the bit count from the modulus), decode little-endian using available pinned valueToBigInt, and accept only candidates below q. On rejection draw fresh entropy. No modulo reduction, entropy fallback, new cryptographic dependency or signature/challenge change. The documented [0,q) range, including zero, and sampleJubjubSchnorrSk alias remain unchanged.

The pinned midnight-curves0.2.1 scalar modulus limbs equal the existing TypeScript constant0xe7db4ea6533afa906673b0101343b00a6682093ccc81082d0970e5ed6f72cb7. This integer has252bits. Do not use onchain-runtime sampleSigningKey, which belongs to the Bip340 domain. Runtime field/point operations and Schnorr verification remain delegated to their existing canonical implementations.

### Before / after
```typescript
// Before: unavailable pinned API.
return ocrt.valueToBigInt(ocrt.jubjubSampleScalar());

// After: standard rejection sampling, with fresh entropy on each attempt.
for (;;) {
  const bytes = randomBytes(32);
  bytes[31] &= scalarHighByteMask;
  const candidate = ocrt.valueToBigInt([bytes]);
  if (candidate < JUBJUB_SCALAR_MODULUS) return candidate;
}
```
The exact checked code will derive width/mask from the modulus; this sketch highlights ownership and ordering. No test-only entropy injection parameter is added to the production API.

### Validation
Deterministic tests mock only the entropy provider: accept0/q−1; rejectq and the maximum masked candidate before a valid redraw; verify high-bit masking, alias behavior, fresh entropy use and propagated provider failure. Real entropy tests check bounds and existing Schnorr sign/verify, invalid response/message/key behavior. No statistical test is presented as a proof of randomness.

Fix the two existing test-only255literals to255n matching the already-declared bigint API. Require production tsc, test-tsconfig, complete runtime Vitest, focused formatting/lint, and retained ADR0197 descriptor/receive checks. Preserve earlier failure logs as baseline evidence; do not edit originalADR0195 refusal captures. Build in a new isolated/tmp runtime tree with generated version.ts and localcache; no shared dist/node_modules writes. No Rust emitter/schema/ABI changes, fundedledger/proof claim, push or remoteCI.

### Signed local delivery

Conventional GPG+DCO signed commit `39db50ba0059d03c0a962c267598677491f3ad91`, based on ADR0197 `ee4f671c`. Worktree clean; no push or remoteCI.

Exact receipt `${LOCAL_EVIDENCE}/compact-adr198-delivery/receipt.json`, SHA256 `9ce95b868477d80e694ae3b4cd25e8c1d60c11bcda1e1a2bd687260d51c7962a`: all seven commands pass at signed head with clean before/after status.

- Canonical repository version export, production TypeScript and test TypeScript checks are green.
- Complete runtime suite:57/57 tests across four files.
- Six deterministic sampler cases cover0/q−1, q and maxcandidate rejection/redraw, highbit masking, preserved meaningfulhighbit and entropy-provider failure.
- Real Schnorr roundtrip, invalid response/message/key tests pass; pinned curve rejects noncanonical q, while `(q−1)G + G` yields identity and corroborates the scalar order.
- ESLint/Prettier pass; all14 ADR0197 corrected generated receive rows remain byte-for-byte equal at the semantic row level, retaining descriptor bounds/commitments/output behavior.

No missing-API shim, test suppression, modulo bias, entropy fallback, new dependency, Rust schema/ABI or signature/challenge change. Prior pristine build/Schnorr failure logs and ADR0197 receipt remain unchanged. Two small formatter-only hunks in built-ins.ts accompany targeted strict formatting; the only production behavior change is scalar sampling.

The separate negative-input contract of reduceModJubjubOrder remains unchanged (`-1→-1`) and is tracked at https://github.com/MediaNoxLabs/compact/issues/303. ADR0198 accepted in midnight. Isolated TS artifacts remain under/tmp; shared dist/node_modules were not mutated.


### Integrated TS runtime build repair — f58fabde

ADR198/[#302](https://github.com/MediaNoxLabs/compact/issues/302) integrated as GPG/DCO-verified `f58fabde`. The missing scalar sampler now uses the already pinned @noble/hashes CSPRNG with unbiased rejection sampling over the exact upstream JubJub scalar order. Entropy failures propagate; no signature/challenge/reduction semantics changed. Two old test literals now match the existing bigint API.

**Combined main passes production and test TypeScript checks, all57runtime tests, targeted ESLint and formatting.** `${LOCAL_EVIDENCE}/compact-f58fabde-integration-receipt.json`. Isolated delivery `${LOCAL_EVIDENCE}/compact-adr198-delivery/receipt.json` also confirms14 corrected receive rows unchanged. The prior baseline failures remain recorded historically under197; they are resolved by198.

Negative bigint reduction is separately tracked in ADR200/#303. Proposed ADR199/#304 covers explicit wallet-funded receive;195 proof delivery is finishing and194 is queued for its emitter handoff. Rust source/capability matrix remains356/368 as measured at15999efb, without a new Rust inventory claim for this TS-only commit. No push, publication or remote CI.
