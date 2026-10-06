---
id: RUST-ADR-0234
alias: ADR-0234
title: "Enforce canonical Schnorr signing keys independently of curve provider"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-runtime-fix"
topics: ["TypeScript", "Schnorr", "key-range"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 1c08033979775eeca3858f018cea33af66214275f8d56bfb64b5606e8b3680c3
---
# RUST-ADR-0234 — Enforce canonical Schnorr signing keys independently of curve provider

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-runtime-fix. The TypeScript Schnorr signer checks the documented canonical scalar key range before randomness/provider calls. It leaves global curve multiplication and verification policy untouched; the raw Field/provider boundary remains explicit.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#339 closure](https://github.com/MediaNoxLabs/compact/issues/339#issuecomment-6017806254). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c) · [`9a94248c`](https://github.com/MediaNoxLabs/compact/commit/9a94248cc0007866557dfb915048b995d13baf3c) · [`bb86008c`](https://github.com/MediaNoxLabs/compact/commit/bb86008cfdaff8c1cd93af0a5c5071efbbbbb0d2). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

<!-- compact-adr-final-acceptance:03c03a39a2d39c43c91143b9526ca1c2ffae706c:ADR-0234 -->
### Final remote acceptance — 2026-10-06

At published commit `03c03a39a2d39c43c91143b9526ca1c2ffae706c`, all 10 required remote workflows passed on the same revision; the root-reviewed acceptance record and closed rust-backend-v2 milestone cover this bounded decision. [Issue #339 closure record](https://github.com/MediaNoxLabs/compact/issues/339#issuecomment-6017806254) and the [milestone closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) document the verified scope.

[Final evidence bundle](references.md#private-note-06) · [Architecture successor](references.md#private-note-07)

Earlier status statements, observations, tests and limits below remain historical evidence. Final acceptance supersedes their pending wording and does not expand implementation scope.

### Historical decision and delivery record


Date: 2026-10-06
Status: Accepted for bounded implementation
Milestone: rust-backend-v2

### Problem and observed remote failure

Testing runtime run37416631534 at157d7033 executes70 tests:68pass and two failures in schnorr.test.ts. Prior local registry-backed validation passed. The failure is real provider-dependent acceptance, not a reason to suppress the tests or claim the earlier local gate covered Nix WASM.

The flake's onchain-runtime-v3 input is pinned to midnight-ledger909292455347b9bdbe6476cddd1fc9b09e78fd70. It builds package3.1.0-rc.1. Its transient-crypto/src/fab.rs decodes EmbeddedFr through from_le_bytes_wide; curve.rs explicitly reduces a <=32-byte value modulo the scalar order. The npm package3.0.0 used by earlier local tests rejects noncanonical scalars instead. Both low-level providers reject negative bigint conversion. Actual dynamic evidence is `${LOCAL_EVIDENCE}/compact-adr234/provider-probe.json`, binding WASM hashes and results for−1,0,1,q−1,q,q+1. The Nix provider accepts q/q+1, the registry provider rejects them.

### Decision

Make the existing documented signing-key precondition executable inside `jubjubSchnorrSign`: reject signingKey<0 or signingKey>=JUBJUB_SCALAR_MODULUS with a stable RangeError before randomness, curve calls or message encoding. Preserve allowed zero andq−1 boundaries. Do not change global ecMul/ecMulGenerator semantics, reduction, challenge construction, sampler, signature equation or public verification policy.

Remove the test assumption that low-level ecMulGenerator(q) must reject across providers. Retain the mathematical generator-order identity using canonical `(q−1)G+G=identity`, exact scalar modulus, alias and sampler range checks. Strengthen actual Schnorr signing boundaries, canonical success, deterministic nonce/equation checks and refusal-before-entropy/message encoding.

### Before and after

Before:

```ts
const r = jubjubSampleScalar();
const announcement = ecMulGenerator(r);
const verifyingKey = ecMulGenerator(signingKey); // Provider implicitly decided key admissibility.
```

After:

```ts
if (signingKey < 0n || signingKey >= JUBJUB_SCALAR_MODULUS) {
  throw new RangeError('jubjubSchnorrSign: signing key must be in [0, JUBJUB_SCALAR_MODULUS)');
}
const r = jubjubSampleScalar();
// Existing signature construction unchanged.
```

### Ownership and compatibility

Only TypeScript runtime built-ins and focused Schnorr/sampler tests. No Rust emitter/runtime/ABI/schema, provider/version pin, production dependency or generated code changes. Nix callers previously accepted some out-of-domain keys because the underlying provider reduced them; they now receive the documented error. Registry callers retain refusal with a deterministic domain-specific error instead of provider decode error. `jubjubSchnorrVerifyingKey` intentionally normalizes signed inputs; this distinct API remains unchanged. No new public cryptographic policy beyond the existing canonical signing-key contract.

### Validation

Create independent temporary runtime trees with writable private node_modules cache roots and selected actual registry/Nix provider packages; never modify shared node_modules or dist. First reproduce baseline Nix failures and registry success; then production/test typecheck, all runtime tests and targeted lint/format on both providers. Retain raw outputs, package/wasm/source hashes, tested toolchain and exact reproduction. Include invalid−1,q,q+1,huge positive/negative; canonical0,q−1 signatures; no entropy/message serialization before refusal; deterministic canonical signatures verify. Preserve u128 receive descriptor and scalar reduction/sampling regression coverage. No remote dispatch/push by this delivery; parent integrates signed commit and reruns exact final revision.


### Signed local delivery — 2026-10-06

Issue: https://github.com/MediaNoxLabs/compact/issues/339. Signed GPG/DCO commit `c6eb81e9a2dbbfd2a842887688305dbefb0e46d3` from `bb86008cfdaff8c1cd93af0a5c5071efbbbbb0d2`; clean isolated checkout. Three changed files, no dependency/Rust ABI/schema changes. Root reviewed the bounded API decision before signing.

Baseline reproduced locally: registry 3.0.0 passes 70 tests; Nix 3.1.0-rc.1 passes 68 and fails the same two as remote run 37416631534. Pinned source revision `909292455347b9bdbe6476cddd1fc9b09e78fd70` uses `EmbeddedFr::from_le_bytes_wide` in ValueAtom decode, while the registry boundary rejects q/q+1. Registry WASM SHA256 `edeea319fe8b1dca486a3aeaa051e64d4b7fb96d83b5d2bb15e68fc1ce23369e`; Nix WASM SHA256 `f1a93903983767c6d08633407a416240e2f7ad59679171fbcb9f9f7145559185`. Actual probe and upstream source copies are retained in `${LOCAL_EVIDENCE}/compact-adr234`.

After the fix, both actual provider packages pass production and test TypeScript checks, all **78 runtime tests**, and all **14 generated receive/u128 regression cases**, independently on Node **22.21.1** (the remote Node version) and Node **24.14.0**. TypeScript/Vitest/package hashes are in the receipt. Targeted ESLint and Prettier checks pass for both dependency trees. Invalid −1, −q, q, q+1 and 512-bit signed values reject before entropy/message encoding. Canonical 0 and q−1 preserve signing/verification with a forced nonce. Canonical generator-order identity, signed reduction, sampler rejection sampling and existing real signature negatives remain covered.

Source isolation: each test tree copied runtime sources into a private directory with independent writable node_modules cache roots and symlinks only to immutable/read-only dependency packages. No shared dist or package tree was changed. Low-level noncanonical scalar behavior still differs between providers; this delivery makes no universal Rust/TypeScript/provider parity claim outside canonical signing inputs. Existing verification policy is unchanged.

Receipt: `${LOCAL_EVIDENCE}/compact-adr234/receipt.json`, SHA256 `216fccd278c0274afacefe939d47cea4884549be93d801a4c1f250b29cd8d45d`. Raw baseline failures are preserved. No push or dispatch by this delivery; parent owns integration and final exact-revision remote reruns.


### 2026-10-06 final remote validation and raw Field boundary

At published head `9a94248cc0007866557dfb915048b995d13baf3c`, Testing runtime run `37418355731` passed all 78 tests in four files with Vitest 4.1.4. Sanitized build/test steps and exact-head receipt: `${LOCAL_EVIDENCE}/compact-m2-remote-9a94248c/runtime-build-and-test.log` and `runtime-public-summary.json`. Auth/setup steps were excluded. Remote Node executable version was not printed; fetched store artifacts alone do not establish PATH precedence.

Compact native `ecMul` and `ecMulGenerator` accept `Field`, which includes valid Field values at or above the Jubjub scalar modulus q. Rust `canonical_jubjub_scalar` rejects those noncanonical scalar values, matching the registry onchain-runtime-v3 3.0.0 boundary. The locked Nix onchain-runtime 3.1.0-rc.1 reduces q/q+1 instead. Consequently, passing the runtime suite does **not** establish Rust-versus-Nix parity for every valid Field argument. Canonical scalar inputs are the common domain; `jubjubScalarFromNative` provides the explicit portable reduction path before multiplication. This is a real low-level provider/domain discrepancy, not an invalid Compact source claim. ADR234 changes only the Schnorr signing-key API and leaves this discrepancy unchanged.

Read-only issue-body inventory (all 289 returned issue bodies, `${LOCAL_EVIDENCE}/compact-adr234-issue-scope-inventory.json`) found no explicit milestone requirement to make raw q EC calls match the Nix provider. Relevant #104 requires documented supported/rejected boundaries and representative crypto tests; #68/#67 concern older runtime parity/signature/point validation, #227 concerns bounded hash-to-curve recording, #312 covers canonical subgroup projection, and #339 explicitly preserves global ecMul semantics. This inventory is not a waiver of universal parity: presentation and closure must state the provider/domain restriction, and an expansion to raw Field equivalence needs its own semantic decision and tests.
