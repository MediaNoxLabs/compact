---
id: RUST-ADR-0265
alias: ADR-0265
source_sha256: 8081832e29b7f97777c30330fd08155fdb05f1f91325b719878144d7d26474be
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0265 — Compose opaque-string Set mutations through typed recording

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** not separately declared. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0265 — Compose opaque-string Set mutations through typed recording

Read-only research. No production changes, issue creation, Cargo build, proof generation or acceptance claim. Baseline: ADR0259 committed at 51b9ea43, frozen compiler `/tmp/compact-adr259/bin/compactc`, same Scheme, ABI50/IR20. Source profile remains unchanged DID v0.7.0 compiled by this branch against pinned ledger8.0.3, not promotion of original DID's distinct8.1 dependency graph.

### Recommendation

Record unchanged `setAlsoKnownAs` first. This is the smallest self-contained collection mutation cycle: original constructor leaves the Set empty and this one original export can insert, reject duplicate, remove and reject missing. `removeService` has fewer missing leaf kinds but cannot succeed after original deployment until a service is created; pairing it with `setService` requires Map insertion/composite values and a larger slice. The readonly Schnorr method verifier similarly requires a prior method insertion. Do not seed DID storage just to make either appear to satisfy real-deployment adoption.

Expected bounded result: 4/12 original stateful exports recorded, including the existing three; all twelve original exports remain natively accepted. No name-gated profile, new interpreter, runtime/schema/ABI change, or cryptographic implementation is needed.

### All nine remaining exports

All share existing controller authorization and recordUpdate except the readonly verifier. `inventory.json` contains actual transitive declaration graphs/source locations and complete kind inventory (including types, not merely missing expressions).

| Original export / source line | Missing compositional domains beyond delivered Point/Cell/Counter |
|---|---|
| setAlsoKnownAs / 613 | OpaqueString + Enum parameters/locals/pure digest members; pure Unit assertion call; OpaqueString Set member/insert/remove |
| setService / 791 | Above pure Unit/Enum/String value domain; Map<String,Service> member/remove/insert; struct-valued insertion |
| removeService / 816 | String digest and Map<String,Service> member/remove; no pure Unit assertion helper or Boolean-returning stateful helper |
| setSchnorrJubjubVerificationMethod / 688 | String/enum/pure Unit; Map<String,point-containing struct> mutation; Boolean-returning stateful verificationMethodExists with lazy cross-map membership |
| verifySchnorrJubjubDigestSignature / 742 | String key; Map member/lookup→typed struct→Point projection; current active guard and existing audited local Schnorr; no writes/update/authorization version |
| removeSchnorrJubjubVerificationMethod / 718 | String-key Map member/remove plus five relation-Set membership guards, existing Unit helpers |
| removeVerificationMethod / 670 | Same relation-Set guards with the JWK Map; no extra Boolean-returning stateful helper in this graph |
| setVerificationMethod / 638 | Nested String/Enum JWK struct, pure validation Unit helpers, Map mutation, Boolean-returning exists helper; existing relations compatibility guards across Sets; Enum inequality |
| setVerificationMethodRelation / 754 | String Set mutations on five selected relation slots; Enum inequality; Boolean-returning stateful relationMember/exists helper graph; Map lookup/projection and pure Boolean classification |

A reasonable subsequent dependency order is: opaque Set/pure Unit → Service Map cycle → point-method Map + Boolean stateful helper + readonly verifier → relation-aware removals → JWK/relations. Keep later work separately reviewed; no claim current Set slice closes any Map export.

### Actual source and runtime facts

`did.compact:613–635` evaluates disclosed mutation and alias, constructs authorization digest (id read, expectedVersion, opaque alias+enum commitment), performs controller authorization, checks mutation validity, selects one membership/assert/mutation branch, then calls recordUpdate (operationCount increment, version increment, timestamp witness, updated Cell write).

The original Set declaration is index1, full physical path `[1,0]`; keep typed descriptor/path rather than using index1 as flat layout.

`assertSetMutationDefined` is NOT stateful in IR: frontend classifies it as a pure circuit with Unit result and `Sequence(Assert(If(Equal(enum,Insert),true,Equal(enum,Remove))), Unit)`. Caller uses `StateAction::PureCall`. This explains the first actionable admission diagnostic.

Existing runtime is generic and sufficient: `runtime-rs/src/slots.rs:712,729,737` forwards `SetSlot<T>::record_insert/remove/member` into `RecordingFrame`; `recording.rs:330,605,619` uses existing upstream-backed typed Set programs. Do not recreate hashing, storage representation or op sequences.

Current composition boundaries: `composition.rs:26` rejects OpaqueString/Enum value types; `:145` accepts only value-returning pure helpers; `:196` has no PureCall/Set action handling. Existing shared Plan supports Enum materialization/equality and retained String parameters, but `typed_plan.rs:1003` limits SetMember to bytes32/qualified coins, `:1783` limits insert similarly, and `:1827` limits remove to qualified coins. These are admission gaps, not absent runtime primitives.

### Executed tiny source probes (compiler only)

Three valid unmodified-source reducers are under `reducers/`; actual emitted IR and metadata under `compiled/`, receipts in `reducers.json`.

- `set_string.compact`: standalone string insertion and membership already record; standalone string removal does not; composed mutation has first refusal StateAction::PureCall.
- `pure_unit_guard.compact`: enum pure Unit validation followed by Boolean Cell helper also refuses PureCall; isolates declaration routing without Sets/crypto.
- `opaque_digest.compact`: pure transient hash of String+Enum struct passed through Boolean witness authorization into Boolean Cell write refuses CircuitCall; isolates value-domain composition.

This avoids claiming blanket string/Set support was absent before this slice.

### Smallest maintainable implementation

1. Extend the existing composition **value** domain with OpaqueString and validated Enum declarations (including nested digest structs), retaining exact typed equality/call arguments and declaration identity. Separate `read_type` from expanded `value_type` so this does not silently expand Cell read admission to String/Enum. Cell writes remain the previous Boolean/Uint64/Point policy.
2. Add explicit audited pure Unit guard classification (e.g. `AuditedCall::PureUnitGuard`) in the existing declaration map. Recursively audit every binding/branch/call in its `Sequence/Assert/Unit` body; assert condition must be Boolean, all discarded steps/result Unit, declaration result Unit. Reuse existing pure typed rendering for validation/emission; no evaluator copy. Reject ledger reads/writes, user/native witnesses, cycles and wrong signatures even in unused/unselected forms. Do not accept arbitrary return values as discarded Unit actions.
3. Route StateAction::PureCall only to that declared pure Unit classification, materializing arguments once in caller order. Preserve `?` assertion failure at that exact action position. Existing pure values/stateful Unit/local audited Schnorr routes stay separate.
4. Audit OpaqueString Set member/insert/remove against actual declared Set slot, physical path and exact item type. Extend existing Plan leaves only when the complete composition audit is active and item type is exactly OpaqueString; preserve old standalone and coin/bytes profiles. Use `record_member/insert/remove` on existing typed slot. No broader arbitrary collection admission or global policy flag.
5. Retain root Unit result, real public effect and actual audited stateful helper requirements. The full original graph has recordUpdate; the tiny standalone removal need not be admitted by this bounded composition work.

### Evidence and adversarial gates

Original independent native capture already includes15 alias calls: successful Unicode insertion/removal and empty insertion, duplicate/undefined/missing rejection, late timestamp rollback and following valid insertion, stale/wrong authority/reduction/inactive precedence. Successes have9 TS queries and reduction→timestamp witness order; undefined has4 queries, duplicate/missing5; late timestamp failure has8 queries and must not commit state. Native context after error is consumed: do not invent inaccessible partial gas/state evidence.

Run these original scenario streams through native and recorded/ContractLab with exact before/after data, private outputs/state, witness order, VM operations, sum of successful TS per-query gas and independent VM replay. Existing scenario contains Service calls between alias operations, so either preserve whole native scenario with recorded assertions only for supported exports, or make a new independent alias-only case module with exact initial state and current version. Prefer alias-only for strict sequence, retaining old capture unchanged except necessary provenance metadata.

Add semantic cases for empty/Unicode strings, insert/remove/insert, duplicate/missing/undefined, selected-branch witness behavior, stale version before invalid signature, wrong controller, reduction/timestamp failure and both Counter overflow precedence (reuse established snapshots only for focused failures, not strict successful deployment).

Backend malformed guards: wrong slot/index/path/declaration type; wrong key/result/enum declaration identity; non-Unit PureCall signature or body; Assert non-Boolean; Sequence non-Unit step; argument count/order; pure hidden query/witness/write in unused binding or unselected branch; mutual cycle; escaping/shadowed locals; unsupported Map effects hidden in any branch. Preserve older composition/profile negatives. Test renamed helpers and alternate Set field name as positives to exclude accidental DID-name coupling.

Frozen all-corpus output/capability comparison, scoped tests/strict Clippy/freshness, and untouched native output remain required. Expect exactly setAlsoKnownAs newly recorded in original DID; reducer/other corpus changes must be enumerated and justified.

### Strict actual original-deployment continuation

Extend/reuse ADR0259 deployment harness, keeping constructor source/data and zero stored-id semantics unchanged. Include the setAlsoKnownAs verifier operation at actual deploy time; no direct storage insertion or later fabricated operation state. At least two separately proved default-strict calls on that real deployment: insert a Unicode alias into empty Set, then remove it from the actually applied successor state, and continue with a delivered original rotate/deactivate if useful to demonstrate compatible version/private-state progression. Use one fresh independent TS sequence whose versions exactly follow those calls.

Compare every applied state, counters, updated timestamp and private outputs; require nonempty proof verification + changed binding refusal and exact same-time IntentAlreadyExists replay rejection, as in259. Fund only fees using existing separate Dust helper/default strictness. Include stale public-state transcript refusal against an otherwise-valid prepared call if it can distinguish Set membership binding without fabricating successful history. No live network or constructor-proof claim; no deployed-identity authorization repair in this slice.

Current ledger8.0.3 source profile and raw noncanonical EC provider caveats remain explicit. Do not promote8.1 compatibility or full DID support from four recorded APIs.

### Accepted implementation

Implement the bounded setAlsoKnownAs slice above after coordinating shared Plan ownership with ADR0266. Preserve old Cell read and write admission separately from the expanded value domain. Use existing typed Set/runtime operations; default-strict actual deployment and both mutation branches are mandatory. Original source and constructor semantics remain unchanged. Record full source/capture/compiler/provider identities and every new capability.

### 2026-10-07 — Delivered

Commit `23d5e8cada2adaf11039bb3d731010a4073167fb`, local conventional/GPG/DCO. 313 backend and 34 DID tests, Clippy, 183-source differential checks and five original-constructor strict proof calls pass (two Alias, three Point controls). Root review added real local/transitive pure-Unit guard controls and corrected stale source registration; the full five-source composition manifest gate passes. [ADR0265 — Original DID alias recording and proof receipt](references-0.3.0.md#note-031) and [ADR0265 — Pure Unit guard coverage follow-up](references-0.3.0.md#note-032). Original DID recording is 4/12; parent #353 remains open.
