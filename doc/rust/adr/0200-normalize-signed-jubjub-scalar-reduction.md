---
id: RUST-ADR-0200
alias: ADR-0200
title: "Normalize signed JubJub scalar reduction"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-runtime-fix"
topics: ["TypeScript", "JubJub", "reduction"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 7a9427a02eee19abf8abf5ec8491fbc5bb194c6fe13a928db5933c71f52fc037
---
# RUST-ADR-0200 — Normalize signed JubJub scalar reduction

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-runtime-fix. Signed JubJub scalar reduction is normalized to the documented Euclidean range. This is distinct from descriptor width and sampler fixes, and does not claim arbitrary provider raw-Field behavior.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#303 closure](https://github.com/MediaNoxLabs/compact/issues/303#issuecomment-6017743189). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4d27092c`](https://github.com/MediaNoxLabs/compact/commit/4d27092ccfac0b9ffc7a817b9e91f180eddc5008) · [`dd85fa81`](https://github.com/MediaNoxLabs/compact/commit/dd85fa8154e69ab35c23dfaf186d9dc0220617e4) · [`ee912835`](https://github.com/MediaNoxLabs/compact/commit/ee912835169d314851b99fc92c3e543ffaa2fd2e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

Issue: https://github.com/MediaNoxLabs/compact/issues/303 (rust-backend-v2). ADR0199 is reserved for wallet funding and is unrelated.

### Problem
reduceModJubjubOrder(value: bigint) documents a result in [0,q), but JavaScript remainder `value % q` preserves the dividend sign, so−1 maps to−1. ADR0198 explicitly preserved this public-helper question while restoring the pinned sampler and full TypeScript/runtime gates.

### Before / after
```typescript
// Before: negative remainder violates the documented range.
return value % JUBJUB_SCALAR_MODULUS;

// After: Euclidean residue in [0,q), preserving every nonnegative result.
const remainder = value % JUBJUB_SCALAR_MODULUS;
return remainder < 0n ? remainder + JUBJUB_SCALAR_MODULUS : remainder;
```

### Decision and scope
Normalize every bigint to its unique residue in [0,q). Preserve the public signature, positive-input behavior and congruence modulo the exact existing scalar order. No new dependency, Rust emitter/schema/ABI, sampler or signature-equation change. This repairs the reducer's contract, not a broad signing-key or verification-policy change. The signing API continues to expect a canonical scalar; its direct ecMulGenerator call retains the pinned primitive's rejection of negative/noncanonical inputs. The existing verifying-key helper explicitly calls the reducer, so its documented reduction behavior follows this change; do not bypass its established call path.

### Tests and evidence
Check−1,−q,−q−1, large positive/negative multiples and offsets,0,q−1,q,q+1. Assert range, expected exact residue and `(input−result)%q===0`. Retain nonnegative/alias/sampler/Schnorr tests; explicitly preserve signing API negative/noncanonical rejection without widening verification rules. Run production/test TypeScript, full runtime suite, lint/format, and fourteen correctedADR0197receive rows. Keep prior negative-behavior evidence separate; new acceptance is an intentional public-helper correction.

Build in a fresh isolated/tmp runtime with repository-generated version.ts and existing dependency links, keeping shared dist/node_modules untouched. Finish conventional GPG+DCO local commit, exact receipt, issue/vault notes; no push or remoteCI.

### Signed local delivery

Conventional GPG+DCO signed commit `80bbccaaf66500c9b7302f77c73616ec24b36789`, based on signedADR0198. Only reducer/docs and signed-boundary tests change; no push/remoteCI.

Exact clean receipt `${LOCAL_EVIDENCE}/compact-adr200-delivery/receipt.json`, SHA256 `be62bef0bc40c6160dbdb54a80c51c695acfde8c98a92733a1d4384421458d8d`: all seven commands pass with clean before/after status.

- Production and test TypeScript checks pass.
- Full runtime70/70 tests pass, retaining sampler/Schnorr/descriptor coverage.
- Twelve exact signed-boundary cases cover−1,−q,−q−1, large multiples/offsets,0,q−1,q,q+1, range and modulus congruence; all nonnegative cases retain previous remainder behavior.
- Existing verifying-key helper follows its explicit reducer call. Signing still refuses−1/q/q+1; verification/signature equations are unchanged.
- Targeted ESLint/Prettier pass; all14 correctedADR0197receive rows remain unchanged.

The public helper now intentionally normalizes negative inputs into[0,q). No entropy behavior, dependency, Rust emitter/schema/ABI or broader signing/verification-policy changes. Prior failure evidence remains preserved in ADR0197/0198 receipts. ADR0200 accepted in midnight.


### Integrated receive recording and TS boundary fixes — dd85fa81 (2026-10-06)

ADR195/[#298](https://github.com/MediaNoxLabs/compact/issues/298) integrated as signed/DCO `4d27092c`; ADR200/[#303](https://github.com/MediaNoxLabs/compact/issues/303) as `dd85fa81`. The former uses shared typed planning and declaration-audited pure identity helpers for the unchanged receiveShielded wrapper; the latter makes signed bigint reduction satisfy its documented Euclidean range. Earlier196/197/198 changes are retained. Schema20/ABI48 unchanged.

- **171 fixtures fresh**, backend16 library +13CLI +153renderer tests,2 pinned-ledger coin identity tests,34Python tests and targeted strict backend/runtime/fixture/proof Clippy pass.
- Six-fixture main gate passes11/23proof-required recording APIs,0nonproof native-only rows in this selected group: `${LOCAL_EVIDENCE}/compact-focused-dd85fa81/receipt.json`. Original Coracle/microDAO and the native lexical regression are included.
- Both zero and2^64 receive call proofs verify (4480bytes each), with changed-binding rejection. **Only the zero-value contract output is ledger-applied**, under default strictness with separate Night-backed Dust and authoritative index0. Real external wallet input still rejects InputMismatch under the existing exact policy. `${LOCAL_EVIDENCE}/compact-integrated-adr195-proof.log`.
- Combined main production+test TypeScript checks, full **70/70runtime tests**, targeted ESLint/format pass. Signed reduction preserves nonnegative behavior and canonical signing-key rejection. Original TSb8refusal capture and separate corrected197capture remain distinct.
- Exact inventory `${LOCAL_EVIDENCE}/compact-dd85fa81-inventory.json`: **358/370 proof-required APIs available,12explicit gaps,zero unassessed within this corpus**;212sources/739exports/191compiledroots/369nonproof, zero baseline drift/missing/unmatched rows. Gaps:5microDAO,4terminal-lexical regression,3Coracle. Corpus availability is not complete language/semantic parity.

Main receipt: `${LOCAL_EVIDENCE}/compact-dd85fa81-integration-receipt.json`. Latest broad/full and clean portable archive remain ee912835; no later full/package claim. ADR194 advance is implementing; ADR199 explicit wallet funding is runtime-only; ADR201 read-only composite Cell observations is approved for preparation and waits for194shared-emitter handoff. Historical capture harness provenance is being tightened separately. No push, publication or remote CI. User-owned documentation edit preserved.
