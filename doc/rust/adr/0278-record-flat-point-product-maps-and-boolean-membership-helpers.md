---
id: RUST-ADR-0278
alias: ADR-0278
source_sha256: 37baaf6834dd1dbeb9b659e7681385183694da207374d29a0cfa2f657efb11cb
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0278 — Record flat point-product Maps and Boolean membership helpers

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for bounded implementation, 2026-10-07. Root accepted the checked domain and staged scratch-first ownership while ADR0279/0280 compiler changes freeze. Issue title: `feat(rust): record original DID SchnorrJubjub method Map CRUD`. Parents R030-09/#353, R030-11/#355; follows delivered ADR0269/#393 and stack correction ADR0274/#398 at signed HEAD `a536360ab002149afc7f6b33465c6d3071eb1d12`. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0278 — Record flat OpaqueString/JubjubPoint Maps and read-only Boolean membership helpers

Status: accepted for bounded implementation, 2026-10-07. Root accepted the checked domain and staged scratch-first ownership while ADR0279/0280 compiler changes freeze. Issue title: `feat(rust): record original DID SchnorrJubjub method Map CRUD`. Parents R030-09/#353, R030-11/#355; follows delivered ADR0269/#393 and stack correction ADR0274/#398 at signed HEAD `a536360ab002149afc7f6b33465c6d3071eb1d12`.

### Problem and original source evidence

Unchanged midnight-did v0.7.0 `did.compact` SHA256 `632f34af543924edb185fe9c8eda54aa4ca0503b0b9e8ee3ff06ec470f6d7456` has six remaining stateful recording gaps. Two form a coherent method CRUD pair:

```compact
export struct SchnorrJubjubVerificationMethod {
  id: Opaque<"string">,
  publicKey: JubjubPoint,
}
// Original source: setSchnorrJubjubVerificationMethod and removeSchnorrJubjubVerificationMethod.
// An Insert or Update writes the declared Map, then recordUpdate(); removal
// checks five relation Sets before the Map remove and recordUpdate().
```

The actual source declarations and circuits are `did.compact:95–100,688–739`; the IR is `/tmp/rust030-adr269/original/contract/compact-rust-ir.json`. Both exports compile to native Rust but have no recorded/observed-call API. The present first diagnostics are shallow `StateAction::Let` and nested `StateAction::CircuitCall`; the complete helper graph matters. In particular `verificationMethodExists(id): Boolean` (`did.compact:108`) evaluates a short-circuit membership on both the nested-JWK `verificationMethods` Map and the point-product `schnorrJubjubVerificationMethods` Map. Current ADR0269 composition admits only Unit stateful helpers and Map values with flat OpaqueString fields, so changing the Map insertion predicate alone would still refuse this original graph.

Current independent unchanged-source reducers:

- `/tmp/rust030-adr269/remaining-reducers/point_product_map.compact` SHA256 `9b7eb22d94b241cabe0c32acfd7f17793421c229d1b9a0a7596ebb8d67971633`; compiled with frozen schema20/ABI50 compiler. Its source-shaped `upsert` and `remove` remain native-only; exact capability JSON under `point-product-render`.
- `/tmp/rust030-adr269/remaining-reducers/heterogeneous_member.compact` SHA256 `b24d2fefe7f155ffe4bfb4d7e660c463da7dcbb384ab295ee77b3e3d99a4f2df`; direct membership over nested-JWK-like and flat-point-product values remains native-only, both refusing `StateReturn::MapMember`. This isolates membership, which does not read/decode `V`, from mutation.

### Before and after generated behavior

Before: the original generated crate has native `ledger_contract::setSchnorrJubjubVerificationMethod` and `removeSchnorrJubjubVerificationMethod`, but no corresponding `recorded::...` or `..._call` façade. The source-order, authorization, Set reference checks and Map mutations must not be replaced by a handwritten partial VM program.

After target: the same generated crate gets typed `recorded::setSchnorrJubjubVerificationMethod` and `recorded::removeSchnorrJubjubVerificationMethod`, plus bound observed-call façades. Both delegate actual Map effects to existing `MapSlot<OpaqueString,V>::record_member/record_insert/record_remove`. The read-only Boolean `verificationMethodExists` helper is inlined through shared Plan in its own lexical parameter scope, evaluating the first Map member and only evaluating the second when needed. All five reference Set reads occur before removal, and controller authorization remains before mutation. Generated native bytes, ABI50/IR20 and all previous recorded APIs stay unchanged.

### Proposed checked domain and owners

1. In the existing `recorded/typed_plan/composition.rs` audit, allow `MapMember` for a *declared* `Map<OpaqueString,V>` regardless of `V` shape, only as a Boolean key query. `MapSlot<K,V>::record_member` is already implemented with no `V: CellValue` bound (`runtime-rs/src/slots.rs:818`); its key and full physical path still undergo current declaration/index/type checks. This admits the nested-JWK Map membership used by `verificationMethodExists` without admitting nested value insertion, lookup or projection. The audit must inspect both branches of the short-circuit helper even though execution evaluates one.
2. Admit a **nonempty flat named product** `V` whose fields are in a deliberately finite scalar set initially `{OpaqueString, JubjubPoint}` for `MapInsert/MapRemove`. Do not key off `SchnorrJubjubVerificationMethod`, field names or exact two-field arity. `MapSlot<K,V>::record_insert/remove` already requires typed `V: CellValue`; verify generated struct implementation and exact value identity at render, including same-shape different named product refusal. Existing all-string Service values continue to pass. Empty/nested products, enums, unsigneds, vectors and arbitrary mixed containers remain outside mutation until direct cases warrant them.
3. Add one audited stateful **read-only Boolean-result helper** class for action-free `StateReturn::Expression` bodies in this composition domain. Require exact Boolean result and declared typed parameters; recurse through the complete expression/branch/callee graph, reject witnesses, writes, recursive calls, malformed arity, hidden unused effects and any unsupported value. The existing `Plan::inline_call` already has isolated scope, caller-order argument evaluation and cycle guard; route the validated helper through that machinery rather than creating a second evaluator. Preserve complete prior Unit helper admission and source error precedence.
4. Keep composition's complete-body audit and `public > 0` accounting, but do not use counters as the only allowlist. Direct standalone Boolean `has_nested` may become recorded incidentally under a separate expression-return profile; if so, the 183-source capability differential must attribute and test it. Do not use source name/count suppressors to force exactly two target changes.

Compiler owner: `recorded/typed_plan/composition.rs` and narrowly gated `recorded/typed_plan.rs` Map/Boolean-helper leaves. No runtime, frontend, IR schema or ABI change is expected. Existing runtime primitives are reused. If the generated point product fails `CellValue` or the helper requires unsupported transcript semantics, stop and revise the ADR rather than emulate VM behavior.

### Direct tests and strict acceptance

- Reducer positives: renamed two-field `{OpaqueString,JubjubPoint}`, three/four-field mixed flat products, both field orders and different legal names, changed point in Update, direct nested-value Map membership. Reducer negatives: same shape wrong declared name, wrong point/key type, empty/nested product insertion, MapLookup/size/reset, hidden Map query/write in unused binding or unselected branch, query inside pure helper, recursive stateful helper, malformed argument order/count and lexical escape. Check no unexpected wider profile wins first.
- Independent original TS capture against pinned upstream runtime for Insert, Update, Remove, duplicate/missing, undefined mutation, key collision across the two method Maps, referenced-removal refusal (each relation class where source behavior differs), bad signature/version/inactive state, nonzero/Unicode IDs and point change. Exact native/recorded/replay program, effects, state, per-query/summed gas and private witness order; same source prestate and failure rollback.
- Original strict ledger proof: constructor-derived Insert → Update → Remove with separate Dust and default strictness; verified nonempty proof, application and version/counter/Map state after each call, changed-binding rejection and same-time replay refusal. Retain a referenced-removal negative without seeding impossible successful history. Generate and record original k/rows. Constructor execution and ledger8.1↔8.0.3 compatibility remain separately scoped.
- Full default-stack backend, DID fixture freshness, source-scope registry, DID proof-gate extension from the existing six distinct key operations to eight distinct operations only after both new original circuits prove successfully (expected total call occurrences 8 → 11 if the original Insert → Update → Remove scenario passes), strict Clippy, and immutable 183-source before/after output/capability comparison. Review every incidental capability delta. Existing 6/12 original recorded baseline must remain unchanged except fully verified new exports. No remote CI/push for local slice.

Expected outcome **if** all original gates pass: DID recorded availability 8/12. This is a target, not a current claim. The remaining nested JWK method CRUD, digest-signature MapLookup, and relation multi-Set branch remain separate domains. Do not investigate unrelated cross-instance authorization behavior in this slice.

### Implementation sequencing

Before touching shared compiler files, implement and test in an isolated compiler snapshot of committed HEAD `fca7577b8a56150a869c56d1e7979727cef33059`. ADR0279/0280 own the live compiler layout until explicit handoff. Retain original source, existing schema20/ABI50, runtime and generated library unless a focused frozen compile proves a change necessary. Run the source-admission/reducer guard gate first; run expensive original proofs only after recorded behavior is stable. Exact generated-source/capability differential, default-stack backend and source hash inventory are required at final handoff. Do not infer full DID parity from the target 8/12.


Issue: https://github.com/MediaNoxLabs/compact/issues/404 (milestone rust-backend-v0.3.0).


### Local implementation and verification — 2026-10-07 (integration pending)

The bounded compiler implementation and original-source fixture are ready for root integration at base `d5d4a6c9`. The complete source and evidence manifest is `/tmp/rust030-adr278/delivery-receipt.json` (SHA256 `5f48cecc0f6908a528d16e0b3ec317d6e252b07fcb84cfe35b87d13a89845edf`). This section records local evidence, not a new ledger-version compatibility claim or a committed integration SHA.

The final ownership boundary is narrower than the initial key-only proposal: a nested-value `Map<OpaqueString,V>.member` is admitted only inside the audited action-free Boolean helper when the enclosing composition performs a checked flat-product Map mutation. Flat `{OpaqueString,JubjubPoint}` products have ordinary checked membership and mutation. A first 183-source comparison revealed an incidental rewrite of Asset Registry `setWatch` through the broader Boolean helper profile; this extra bound restored its old emitted Rust byte for byte. The final differential `/tmp/rust030-adr278/corpus-comparison.json` compares all 183 source closures with the frozen ADR0279/0280 renderer. Every native prefix is unchanged; all other 182 complete generated Rust files and capability reports are unchanged; only the two original DID SchnorrJubjub method exports gain recorded/observed-call availability. No contract-name dispatch or new runtime, IR schema or ABI was added.

Independent original TS capture covers nine prestate-based method cases and a 14-step original-constructor sequence. The native and generated recorded runs match checked state, effects, private witness journal, complete public VM program, query-summed gas and replay on their successful paths; failed calls preserve owned ContractLab state. The original-constructor method Insert→Update→Remove proof summary is `/tmp/rust030-adr278/did-proof/did-schnorr-methods-result.json`. The final frozen gate `/tmp/rust030-adr278/full-did-gate-final2/receipt.json` has SHA256 `7d3607014e32849a5882561976143eb52f66a6b3bdc5f15af2178811670526d8`: eight original key operations, four scenarios, eleven nonempty proof-verified calls, separate Dust, default-strict ledger application, changed-binding rejection and exact same-time replay refusal. The new two circuits compiled at k=11 / 2030 and 1831 rows. Original constructor **data** was actually deployed; constructor execution was not proved.

Default-stack backend suite passed 332 tests; focused Map point composition passed six tests; DID provenance and Schnorr recording passed three and two; 44 Python inventory/orchestration tests passed; strict Clippy for backend, DID fixture and proof runner passed. The first full proof run was correctly rejected only because capture provenance changed while its frozen source inventory was active; the final run passed after source freeze. Direct tests cover hidden witness, Counter query, effect, recursion, wrong Boolean result, nested/empty mutation, wrong slot/key/value/name/arity and lexical arguments. They are finite guards, not exhaustive input-domain proof. Original DID remains a ledger 8.1 source while this local runtime/proof uses pinned ledger 8.0.3; that version pairing is still a distinct adoption gate.

### Local integration — signed DCO commit 7d65bd8e, 2026-10-07

Locally delivered in `7d65bd8ee2c2654b9558a8b75077302364fd5d6b` (GPG verified, DCO trailer). The implementation was committed from the frozen source hashes in [ADR0278 — Point-product Map delivery receipt](references-0.3.0.md#note-050). The passing final proof gate is [ADR0278 — Strict DID Schnorr proof receipt](references-0.3.0.md#note-051); the 183-source before/after capability and generated-byte comparison is [ADR0278 — 183-source renderer differential](references-0.3.0.md#note-049). These supersede the earlier *integration pending* checkpoint above without rewriting the decision history.

The final domain bound is deliberate: nested-value Map membership is key-only and exists only inside an audited action-free Boolean helper in the checked flat-product mutation composition. The earlier wider candidate changed Asset Registry `setWatch` generated bytes and was rejected. Original DID is now **8/12** stateful exports with recorded and observed-call APIs; the remaining four are separate. The failed source-freeze and ENOSPC runs are retained as non-passing historical attempts, and the final frozen gate is the sole strict acceptance receipt. No ledger 8.1 compatibility, constructor proof or exhaustive input parity is implied.

Issue closure: MediaNoxLabs/compact#404 was closed as completed after the local delivery comment [#404, 2026-10-07](https://github.com/MediaNoxLabs/compact/issues/404#issuecomment-6027448819). This issue closeout covers only ADR0278’s two original DID exports; the other four remain separate.
