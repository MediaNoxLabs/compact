---
id: RUST-ADR-0283
alias: ADR-0283
source_sha256: 2687f4bdb374fd413e92da7b0264c1cdd5c9595b0dc4f93789067a160af705af
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0283 — Record nested JWK VerificationMethod Map CRUD

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for bounded implementation by root, 2026-10-07; issue creation follows before production edits. Implementation base `1f9b13944c1ac06d983e952cb1a130750e2a9ee1` includes ADR0281/0282 renderer fixes and preserves the ADR0278 eight-of-twelve original DID baseline. The read-only research base was `7d65bd8ee2c2654b9558a8b75077302364fd5d6b`. Original pinned `midnight-did` v0.7.0 commit `4e7f6b0f69bf4e2c8506a9693f8d0c3dfe68e550`, `did.compact` SHA256 `632f34af543924edb185fe9c8eda54aa4ca0503b0b9e8ee3ff06ec470f6d7456`, and `schnorr.compact` SHA256 `f072731d730d72f8b76b183df2c5187b233a0838d9462894cfb0a6d2f0f66bff`. Frozen compiler `/tmp/rust030-adr278/compactc-final` SHA256 `d5314191dca63894bab2994f3a6a6063d2c42e615e3a08b2299949428a6c8491`; Scheme `/tmp/compact-adr251/source-gate-final2/bin/compactc-scheme`. Machine-readable IR/capability graph and reducer checks: `/tmp/rust030-adr283/analysis.json` SHA256 `e11840cd8a26282df16c51836a332585fc70dfaf304e839448856771de359ebf`. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0283 — Record nested JWK VerificationMethod Map CRUD

Status: accepted for bounded implementation by root, 2026-10-07; issue creation follows before production edits. Implementation base `1f9b13944c1ac06d983e952cb1a130750e2a9ee1` includes ADR0281/0282 renderer fixes and preserves the ADR0278 eight-of-twelve original DID baseline. The read-only research base was `7d65bd8ee2c2654b9558a8b75077302364fd5d6b`. Original pinned `midnight-did` v0.7.0 commit `4e7f6b0f69bf4e2c8506a9693f8d0c3dfe68e550`, `did.compact` SHA256 `632f34af543924edb185fe9c8eda54aa4ca0503b0b9e8ee3ff06ec470f6d7456`, and `schnorr.compact` SHA256 `f072731d730d72f8b76b183df2c5187b233a0838d9462894cfb0a6d2f0f66bff`. Frozen compiler `/tmp/rust030-adr278/compactc-final` SHA256 `d5314191dca63894bab2994f3a6a6063d2c42e615e3a08b2299949428a6c8491`; Scheme `/tmp/compact-adr251/source-gate-final2/bin/compactc-scheme`. Machine-readable IR/capability graph and reducer checks: `/tmp/rust030-adr283/analysis.json` SHA256 `e11840cd8a26282df16c51836a332585fc70dfaf304e839448856771de359ebf`.

### Four remaining original exports

| Export | Current first diagnostic | Actual graph beyond that first diagnostic |
|---|---|---|
| `setVerificationMethod` (`did.compact:638`) | `StateAction::Let` at `actions[0]` | Named nested `VerificationMethod { id: OpaqueString, typ: VerificationMethodType, publicKeyJwk: PublicKeyJwk { kty: KeyType, crv: CurveType, x/y: OpaqueString } }`; `assertSupportedVerificationMethod` pure Unit enum/curve guard; `assertExistingVerificationMethodRelationsCompatible` reads five String Sets on Update; checked Map member/remove/insert; existing `verificationMethodExists` Boolean helper; authorization digest and `recordUpdate`. Reachable helper graph has 13 nodes/12 edges. |
| `removeVerificationMethod` (`did.compact:670`) | nested `StateAction::CircuitCall` | Authorization, Map member/remove by OpaqueString key, five reference Set guards, `recordUpdate`; 10 nodes/9 edges. No JWK value is decoded by remove, but the runtime `MapSlot::record_remove` is currently in a `V: CellValue` impl. |
| `verifySchnorrJubjubDigestSignature` (`did.compact:742`) | nested `StateAction::Let` | Point-product Map **lookup**, projection, then `schnorrVerifyDigest → schnorrVerify`, including transient hash, reduction witness, unsigned bounds and crypto checks. This is a different read/crypto domain; 3 nodes/2 edges. |
| `setVerificationMethodRelation` (`did.compact:754`) | nested `StateAction::CircuitCall` | Enum-selected five Set member/insert/remove branches, read-only Boolean relation helper, curve compatibility and conditional nested JWK **lookup**, authorization and `recordUpdate`; 16 nodes/15 edges. This is a different multi-Set/lookup domain. |

The first diagnostics are coarse profile refusal, not proof that accepting Let or CircuitCall alone closes a graph. The source has genuine `KeyType`/`CurveType` authorization guards. A key-only Map remove could be a narrower compiler change, but strict application of original `removeVerificationMethod` requires a legitimately inserted JWK method; without recording its insert, a positive original-source proof would depend on a favorable injected prestate. That makes the CRUD pair the smallest coherent *source-proved* slice.

### Proposed checked domain and implementation boundary

Before, the same original source emits native Rust for both JWK method exports but no `ledger_contract::recorded::setVerificationMethod` / `removeVerificationMethod` or corresponding `*_call` methods. Existing composition admits only nonempty flat products with OpaqueString/JubjubPoint fields at Map mutation. It already has typed `value_type` recursion through Struct and Enum, source-order scoped `Plan`, pure Unit guard audit, source-order String Set reads, and a bounded read-only Boolean membership helper. The generated `PublicKeyJwk`, `VerificationMethod`, `KeyType`, `CurveType` derive `CompactCellValue`; `MapSlot<K,V>::record_insert/remove` already reuse ledger recording primitives. Do not duplicate a codec or substitute helper names.

After target, admit `Map<OpaqueString,V>` insert/remove when **declared V** is a nonempty, acyclic named-product tree with nonempty named-product children and leaves only from the already audited `{OpaqueString, JubjubPoint, declared Enum}` scalar set. Arbitrary legal field names/counts and structurally nested products are allowed; no `VerificationMethod` or contract-name route, fixed arity, or field-name condition. Preserve exact declared type identity, key type, index/physical path and emitted `V: CellValue` bound. Explicitly exclude empty products, tuples, vectors, Map/Set/Coin values, arbitrary unsigned/Field/Boolean leaves, MapLookup and standalone nested key-only membership expansion. The existing flat Service/point product profiles stay accepted. The complete action/expression/helper audit must recurse through *both* branches and all pure bodies, including unused lets; pure guards may assert on enum/curve fields but may not query ledger, witness or mutate. Retain helper cycle/arity/lexical-scope checks and evaluate arguments once in source order. If the recursive product walk increases debug stack, extract a small helper and require default-worker full backend as ADR0274 did; never raise stack size to make this pass.

Compiler owners would be `recorded/typed_plan/composition.rs` type/effect audit and `recorded/typed_plan.rs` scoped Map insert/remove gate only. Runtime, generated ABI50, IR schema20 and frontend should stay unchanged. The small domain should not admit digest verification or relation mutation by implication; inspect exact 183-source diff before making that claim. If a full original helper graph still refuses for a distinct leaf, stop and amend the decision rather than adding contract-specific routing.

```rust
// Illustrative generated consumer surface after acceptance; use actual generated
// types and call signatures in tests rather than hand-constructing VM operations.
let call = binding.setVerificationMethod_call(
    &observed, private_state, verification_method, MapMutation::Insert,
    controller_signature, expected_version,
)?;
let replay = call.replay()?;
```

### Direct baseline reducers and expected guards

All three sources compiled with the frozen current compiler using `COMPACTC_SCHEME=... compactc --skip-zk --target rust --compact-path compiler --rust-runtime-root . SOURCE OUT`:

- Existing `tools/compact-rust-backend/tests/map-composition/nested_map.compact` SHA256 `79772d73d0f802c8dcfd0d1204b02f2f1e0226dedeb1d9eb586d9c2714052037`: nested String product `put` native, recording unavailable at `StateAction::MapInsert`.
- `/tmp/rust030-adr283/reducers/flat-enum-crud.compact` SHA256 `3c7873d72df410379397e8813b895c56a63d385a79e6fe8fa3d85cf3a500b565`: flat String/Enum product plus enum assertion, `put/remove` native-only; first coarse refusal at root Let.
- `/tmp/rust030-adr283/reducers/nested-enum-crud.compact` SHA256 `9be89300f16afe87853910611bf5e8d84f702d0756b1158b2d3749fa342550aa`: nested String/Enum product, pure enum guard, Insert/Update/Remove, Counter bump; `upsert/remove` native-only; first coarse refusal at root Let. Capability reports and hashes are in `analysis.json`.

After change, require positive original JWK CRUD and these three *structural* reducer families. Add two/three-field renamed products and field-order variants, nested child with more than the source's four fields, exact same-shape different named-product refusal, malformed enum variant/key/value/index, unsupported empty or container leaf, hidden query/witness/write in pure helper or unselected branch, recursive helper, malformed arity/order and escaped local. Keep the ADR0278 point-product/Boolean helper negatives and prior Service behavior. Explicitly test that `MapLookup` and relation multi-Set branch remain unavailable until their own bounded decisions.

### Original source acceptance

Capture the pinned TS simulator independently for real constructor-derived Insert→Update→Remove (Unicode/nonzero ID and x/y), OKP Ed25519/X25519 and EC P256 accepted curves, invalid `typ`, incompatible kty/crv, duplicate/missing/undefined mutation, wrong controller signature/version, inactive contract, and referenced removal. Preserve actual source order and full public program, query list/summed gas, state/effects, private witness journal and replay. Relation-prestate negatives must originate from actual TS source calls; do not fabricate impossible source state. Compare native and generated recorded result/rollback with exact declared JWK type and enum value, including point/alias prior behavior.

For strict Rust acceptance, prove/apply original constructor-derived Insert→Update→Remove at default strictness with separately funded Dust; require nonempty verified proofs, retained offer and ledger state at each step, changed-binding rejection and same-time replay refusal. Record new k/rows, exact locked source/material/compiler hashes and a passing frozen gate receipt. Constructor execution remains unproved unless separately tested. No proof claim for relation-prestate branch until a legitimate relation transaction is supplied. Run full backend on default worker, focused typed reducer and DID tests, strict Clippy, fixture freshness and source-scope registry; compare all 183 source closures against frozen `7d65bd8e`/post-ADR281 renderer and review every Rust byte/capability delta. Expected target is 10/12 only after both original proofs and full gates pass. The original DID requests ledger 8.1, while the local backend proof profile uses ledger 8.0.3; that compatibility boundary remains explicit. No remote CI/push in this bounded slice.

### Approval and ownership

Root approved the exact checked domain and acceptance gates on 2026-10-07, with no contract-name routing. Compiler ownership is limited to the shared composition audit and typed Plan Map mutation gates; test and DID proof-gate additions are in scope. Source freeze and source-hash inventory precede final full gates. Root reviews/stages and commits; this ADR does not authorize a push or remote CI.

Issue: [MediaNoxLabs/compact#407](https://github.com/MediaNoxLabs/compact/issues/407), milestone `rust-backend-v0.3.0`, created before implementation.

### Bounded implementation clarification — 2026-10-07

The original `setVerificationMethod` Update helper compares a declared `CurveType` with `CurveType.X25519` using `!=`. A typed IR omission probe showed that the existing composition Field-only inequality leaf, rather than Map insertion, was the final refusal after recursive product admission. This ADR now explicitly admits `!=` for **matching declared Enum types** as well as the previously admitted Field operands, through the same single-evaluation typed Plan. Mixed enum names, mismatched types, unreviewed Boolean/unsigned values and pure-body ledger effects still refuse. This is a source-required enum guard, not a new general expression evaluator.

Direct key-only membership on a nested-product Map is admitted outside the prior action-free Boolean helper only when that **same declared Map field/index** is also mutated by the audited Unit composition. The audit records both sets of physical owners and requires nested direct reads to be a subset of Map writes. The ADR0278 negative in which an unused Let reads unrelated `nestedMethods` while `pointMethods` mutates must still refuse. The helper route retains its prior checked-map-write requirement. Add a negative with an unrelated nested Map read in an unused binding and an unselected branch; retain exact slot/key/value type and full expression traversal. An early candidate that allowed any nested Map read whenever *some* product Map wrote was rejected by that ADR0278 regression.


### Local delivery — signed DCO commit 770f8dcb, 2026-10-07

Locally delivered in conventional GPG- and DCO-signed commit `770f8dcb546aa3d95455da3a02b9701dcb975191`. The source-bound implementation receipt is [ADR0283 — Nested JWK delivery receipt](references-0.3.0.md#note-062). The frozen strict gate [ADR0283 — Strict DID JWK proof receipt](references-0.3.0.md#note-064) passed with ten key operations, five original-source scenarios and fourteen nonempty proof-verified calls. JWK Insert → Update → Remove from actual constructor-derived state passed default-strict ledger application, changed binding rejection and same-time replay refusal. The two JWK circuits measured k=12/2359 rows and k=11/1831 rows respectively.

The independent TS capture and typed Rust tests compare 38 pre-existing JWK rows plus 23 new JWK CRUD rows, including nested enum/key variants, actual relation prestates, complete public VM program, query-summed gas, state/effects, private witness journal and recorded replay. Default-worker backend passed 341 tests; DID parity passed three; proof-gate orchestration passed nineteen; inventory passed 25; strict backend/DID/proof Clippy and generated fixture freshness passed. The full logs, including the intentionally interrupted optional cold source-scope attempt, are [ADR0283 — Local gate logs and interrupted attempt](references-0.3.0.md#note-061). The pinned original source/capture closure is [ADR0283 — Pinned DID source and capture closure](references-0.3.0.md#note-063).

[ADR0283 — 183-source renderer differential](references-0.3.0.md#note-060) confirms all native prefixes unchanged and exactly two DID capability/generated-file changes, `setVerificationMethod` and `removeVerificationMethod`; the other 182 complete source outputs are byte-identical. The original DID now has 10/12 recorded and observed-call exports. `setVerificationMethodRelation` and `verifySchnorrJubjubDigestSignature` remain unavailable. The direct nested Map membership and enum inequality extensions retain the structural bounds above; an early broader membership candidate was rejected by inherited tests. A comment in `checked_product_value` still describes the older flat-only membership limit and is slated for the next compiler source slice; executable behavior and frozen proof output are unaffected.

This finite ledger8.0.3 proof profile does not establish general compatibility with the original DID ledger8.1 request, prove constructor execution, or exhaust the authorization input domain. The separate optional `check_positive_source_scope.py` run was interrupted at exit 130 during a cold generated Cargo check and is not counted as a pass; fixture freshness, 183-source comparison, default backend, parity and the frozen strict proof gate passed. No remote CI or push occurred for this slice.


Issue closeout: [MediaNoxLabs/compact#407](https://github.com/MediaNoxLabs/compact/issues/407) closed as completed after the [local delivery evidence comment](https://github.com/MediaNoxLabs/compact/issues/407#issuecomment-6027834860). Parent DID adoption/milestone 0.3.0 remain open.

### Provenance follow-up found by ADR0286 gate — 2026-10-07

The broader DID package gate correctly refused six prior capture records referencing the pre-JWK generated library hash91700ff9 instead of committed2fb7a3dc. The JWK behavior/proof/compiler evidence remains intact, but fixture provenance needs refresh. Root will rerun all seven unmodified branch TS capture scopes against the frozen verified generated TS module, require full recursive scenarios/cases equality before accepting provenance-only deltas, add the JWK capture to the existing provenance checker, and extend the reviewed artifact matrix with the new case/support/test paths. No generated Rust/source/runtime changes or altered expected semantic values. Focused provenance tests will rerun; no identical proofs are required for metadata changes. Issue407 reopened until corrected.
