---
id: RUST-ADR-0295
alias: ADR-0295
source_sha256: cb0368d5409a83f5a717562ea194dd3d0b39ee9fbd70c0cd63deaaddaeaa88dd
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0295 — Record original DID relation mutation through checked selected Sets and nested JWK reads

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** ** accepted for an isolated prototype by root on 2026-10-07. A source-complete delivery remains conditional on the checked selected-Set and nested Map domains passing the stated gates. This ADR and its issue precede all implementation. Baseline: verified signed ADR0288 `8a52a01001894ecb5d0e773463436b78993a119b`, pinned `midnight-did` v0.7.0 source `examples/rust_backend/did_adoption/packages/contract/src/did.compact` (unchanged), local compiler `/tmp/rust030-adr288/compiler-final/bin/compactc` SHA-256 `0091e9cc7c257157ef341a2a2d616dd68766d3665db64a42b40f1d616f8acfd8`. Source asks ledger 8.1; current Rust proof/runtime profile is ledger 8.0.3. ADR0288 made 11/12 original exports recorded/observed; this is the final native-only export. No cross-instance authorization probe is in scope. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0295 — Record original DID relation mutation through checked selected-Set and nested JWK read composition

**Status:** accepted for an isolated prototype by root on 2026-10-07. A source-complete delivery remains conditional on the checked selected-Set and nested Map domains passing the stated gates. This ADR and its issue precede all implementation. Baseline: verified signed ADR0288 `8a52a01001894ecb5d0e773463436b78993a119b`, pinned `midnight-did` v0.7.0 source `examples/rust_backend/did_adoption/packages/contract/src/did.compact` (unchanged), local compiler `/tmp/rust030-adr288/compiler-final/bin/compactc` SHA-256 `0091e9cc7c257157ef341a2a2d616dd68766d3665db64a42b40f1d616f8acfd8`. Source asks ledger 8.1; current Rust proof/runtime profile is ledger 8.0.3. ADR0288 made 11/12 original exports recorded/observed; this is the final native-only export. No cross-instance authorization probe is in scope.

### Measured gap and complete source graph

`setVerificationMethodRelation` (source line 754) is native Rust, proof-required and recorded=false. Its first reported gap is `StateAction::CircuitCall` at `actions[0].action.action.action.actions[0]`; this is only the first diagnostic, not the full dependency. It is a Unit result after three disclosed locals, controller authorization, mutation/relation/target checks, current relation membership, an insert/remove branch, compatibility checks on insert, and `recordUpdate` on success.

The transitive stateful graph has 11 circuits: `assertControllerCanUpdate → assertController → schnorrVerifyDigest → schnorrVerify`, `verificationMethodExists`, `verificationMethodRelationMember`, `assertVerificationMethodRelationCompatible`, `insertVerificationMethodRelation`, `removeVerificationMethodRelationFromLedger`, and `recordUpdate`. The five relation Sets are selected by one `VerificationMethodRelation` enum; the Boolean selector reads precisely one selected Set, and the corresponding branch inserts/removes precisely the selected Set. The compatibility helper reads a nested `Map<OpaqueString, VerificationMethod { PublicKeyJwk { crv: CurveType, ... } }>`: KeyAgreement requires present X25519, signing relations reject X25519 only if that Map contains the key. `recordUpdate` increments two Counters and writes the timestamp Cell after successful mutation. Authorization and crypto are already used by earlier recorded DID CRUD profiles; reuse the same audited graph and source order.

The independent TS oracle has **42** relation calls: 39 in `oracle/lifecycle.json`, 3 in `oracle/jwk-method-lifecycle.json`; 23 succeed and 19 fail. It covers insert/remove/duplicate/missing for all five relation kinds, X25519 exclusions, undefined/missing-target/mutation error precedence, ordered multi-relation updates, and JWK agreement/signing prestate. Existing native Rust tests exercise these captures, but no recorded or observed facade exists for this export. Original ZKIR keygen measured k=12/2718 rows in ADR0288 research; that is not proof evidence.

Two immutable scratch reducers compiled natively with the frozen ADR0288 compiler and remain recorded=false:

- `/tmp/rust030-adr295/selected-set.compact`, SHA-256 `34d3c7b807f036434e0aff17c0a625c9436c273b1d399b799e9463729e647178`: a two-variant enum selects two String Sets; a read-only Boolean helper chooses member, and Unit helpers choose insert/remove. First gap is `StateAction::Assert` at `actions[0].action.action.action.actions[0]`.
- `/tmp/rust030-adr295/nested-jwk-read.compact`, SHA-256 `a80c58e9eee7703939296f55fc4667c8c4d8900c5c848a2bdb0b4b9d02e81e07`: a nested String/Enum JWK Map read with guarded lookup and short-circuit signing path. First gap is `StateAction::Let` at `actions[0].action.action.then.actions[1]`.

Both gaps are compositional admission gaps, not native/type failures. Scratch source and exact native output are under `/tmp/rust030-adr295/`; no repository file was edited.

### Recommended bounded implementation

Use **one source-complete ADR** for the final original export, but implement and gate two separable checked domains before composing them. A partial first substep may prove the selected-Set reducer without claiming original export acceptance. Do not simply permit `SetMember` under every `read_only_boolean_depth` or remove the current `ReadOnlyBoolean ⇒ product_map_writes` obligation in generic Unit composition. Those flags protect unrelated source cohorts.

1. **Selected String Set family:** a private checked plan maps declared enum variants to distinct declared `Set<OpaqueString>` fields. Audit the entire Boolean `if` selector and both Unit mutation helper branch trees. Require the same scoped enum and key origins and the same variant→field relation for member/insert/remove, all on the same execution path. Nonselected branches must have no executed query/write; syntactically hidden effects are still audited. Undefined/default branch stays false/does no Set write, with the outer source assertion rejecting undefined. Do not use source names, rendered token equality, exact five-branch count, or favorable query totals as proof of coupling. The two-variant reducer proves genericity.
2. **Nested Map compatibility read:** support a String-key Map whose declared value is a checked nested named product containing a declared enum curve projection. Membership and lookup must refer to the same declaration and scoped key. The extracted projection must descend through the bound lookup result by checked field indices/types, not a same-shaped other value. Preserve the original branch's `!member || lookup.curve != X25519` short circuit: absent keys never execute lookup. KeyAgreement's member assertion precedes lookup. Audit all branches, including absent/unselected ones, and reject helper writes/witnesses/recursive or unknown calls. Reuse typed Plan expression lowering and existing slot/runtime methods; the audit is not a second evaluator.
3. **Whole Unit composition:** retain ordered controller authorization, all assertions, the selected Set mutation and `recordUpdate` exactly once on successful paths. Reuse existing controller signature, `currentTimestamp` witness and Counter/Cell action lowering. A rejected path must roll back owned context, with no committed mutation, and retain TS error precedence. The checked plan distinguishes complete source admission from malformed same-shape IR. No runtime method, ABI or IR schema change is expected; re-evaluate if actual typed lowering disproves this.

The current `typed_plan/composition.rs` admits String Set member/insert/remove leaves when `composition_calls.is_some()`, and it already emits effectful `Expr::If` branches with frame handoff. Its audit currently forbids SetMember while `read_only_boolean_depth>0`, and its final `ReadOnlyBoolean` obligation is tied to product Map writes. It does not audit MapLookup in the nested compatibility helper. These are the precise profile boundaries to extend with a separate checked relation domain, not broad global toggles.

### Required negative controls

- Rebind one relation enum or key after member before mutation; swap one Set declaration, enum arm, or insert/remove target while keeping field types; duplicate an arm or route two variants to one Set; execute two writes on one path; move `recordUpdate` before a failing assertion; evaluate nonselected Set member/write. All refuse recording.
- Use Map member from one declaration and lookup from another, same-shaped nested value from a parameter, wrong nested field index/type, missing membership before KeyAgreement lookup, eager lookup on signing path where method is absent, hidden Map query/write/witness in unselected branch, recursive helper or wrong call arity. All refuse or preserve exact TS failure order.
- Valid two- and four-branch renamed reducers are recorded with the declared mapping, proving there is no exact five-set/name gate. Keep short-circuit missing-map case and read/witness ordering observable.

### Acceptance and proof scope

- Maintain unchanged source and original 42 independent TS cases. For all calls, compare native/recorded/replay complete public program, per-query and summed gas, private witness transcript, state/effects, selected Set query/write, error and owned rollback; only normalize genuinely unordered upstream collections, preserving multiplicity. Do not infer full coverage from 12/12 API availability.
- Register two reducers as maintained source fixtures with actual independent TS capture and native/recorded behavior tests. The selected Set and nested JWK reducers are proof-applicable; run default-strict direct proof/verify/apply/replay for each after source/output freeze, not just native checks.
- Original proof: compile/keygen unchanged original `setVerificationMethodRelation` and prove at least each of five selected Set insert paths plus representative remove, an X25519-compatible KeyAgreement and a signing rejection/short-circuit missing-map path with real seeded prior state. Verify strict ledger apply and exact replay refusal for accepted cases; failed assertions must not create an applied mutation. Retain actual k/rows and offer/fee setup. No ledger8.1 compatibility claim from ledger8.0.3 proof.
- Default worker backend/package tests, strict Clippy, complete fixture freshness and frozen 183-existing-source differential. Only the final DID relation recorded capability/body and reviewed new reducer fixtures may change. Preserve ordinary native output bytes and other capabilities.

### Recommendation to root

Accept this as the **final original DID export** slice only if the two checked family rules can be expressed without widening generic Unit composition. If the full source graph exposes another independent primitive, close the selected-Set substep as a separate, honestly partial ADR with reducer proof and record the subsequent original blocker; do not claim 12/12 until original source proof/apply passes. This decision authorizes isolated prototype work after its milestone issue is created. Shared compiler files remain frozen for ADR0294; production port requires explicit ownership release. No 12/12 or ledger8.1 compatibility claim is made at acceptance.

### Issue and delivery

Milestone: `rust-backend-v0.3.0`. MediaNoxLabs/compact [#419](https://github.com/MediaNoxLabs/compact/issues/419) was created after this ADR and before implementation. No source code has been changed for this decision.


### Isolated prototype review, 2026-10-07

The scratch compiler under `/tmp/rust030-adr295/backend-snapshot` admits the unchanged original relation and renamed two-/four-Set and nested-JWK reducers. This is prototype evidence, not a production delivery or 12/12 acceptance. The original generated relation capability is recorded/observed=true; its rustfmt-normalized native prefix remains byte-identical to the checked-in DID fixture (SHA-256 `24d2bf03a4dc8578ec02b318e80924218f974f65f13ddb5c18c92bee1a88cfa6`).

Three review findings were addressed in scratch before production promotion: (1) candidate discovery originally scanned all module declarations in read × insert × remove triples; it now narrows to root-referenced helper identities and still performs the typed family and lexical checks. A hundred unrelated helper declarations leave that candidate set unchanged. (2) branch event histories originally duplicated on every conditional; a finite read → optional compatibility → one write transition rejects a second read/write early, and branch merges deduplicate. Twenty-four sequential no-event branches retain a bounded five-or-fewer event states. (3) an unrecognized Unit helper could transitively call a selected insert while the root checker ignored that event; the reachable stateful call graph now refuses this hidden selected operation. Original and renamed positives and these negative controls pass isolated tests. These are prototype admission findings, not claims of deployed defects.

Remaining gates before a production claim: generated consumer compile, complete original 42-case TS/native/recorded/replay/owned-rollback comparison, wrong-map/key/projection/hidden-branch mutants, maintained reducer fixtures/proofs, strict original ledger proof/apply, and differential across the then-current complete fixture registry. Source ownership remains with root until an explicit port handoff.


### Checked-owner review amendment — 2026-10-07

The isolated prototype established the unchanged original DID relation export and the 42 captured calls; the full 193-source differential changed only this recorded capability. An actual seeded default-strict ledger chain applied six verification-method setup calls and all five relation insert/remove pairs with nonempty verified proofs, altered binding rejection and replay refusal. This is isolated proof evidence until maintained sources and the final gate are sealed.

Before production promotion, candidate discovery will traverse borrowed typed IR. The existing exhaustive expression visitor will collect calls without serializing or cloning whole bodies; an exhaustive action/return walker will distinguish expression calls from direct stateful actions. Root-referenced declarations are classified once into checked read, insert, remove and nested compatibility roles. A single unambiguous read family joins insert/remove by semantic enum type and identical variant-to-declared-Set identity, rather than read × insert × remove loops. Extra matching families cause this bounded profile to refuse, leaving older profiles in order. The root's actual selected-read call determines relation/key formal origins; direct selected writes and the nested compatibility call must use those same scoped origins. Whole-body Audit, transitive selected-call refusal, and the finite read → compatibility → one write automaton remain mandatory. Tests will count role classifications, including referenced same-shaped distractors and ambiguity, and compare emitted bytes over the complete current registry. No new runtime/ABI/schema or helper-name route is authorized.


### Signed joined delivery — 2026-10-07

Release migration and standalone lock: `b6fcb06cec7e8913e91faae167926805c4872ffb`. Relation compiler, maintained reducers and public exporter: `329bf1bc80441125d2800fc9f1dae8b570da5dac`. Both commits are conventional, GPG verified and DCO signed.

The joined source passes 427 backend tests, 43 original DID test methods (including the 42-row relation table), three reducer methods, nine exporter tests, 99 Python checks, strict Clippy, whole-workspace formatting and all 196 generated-fixture freshness checks. Actual Rust 1.88 passes both workspace and freshly isolated standalone backend checks. The unchanged original DID source/import hashes remain pinned to v0.7.0.

The maintained relation gate passes 19 original calls and six reducer calls under default ledger strictness, with changed-binding rejection and replay refusal. All 512 recorded source hashes still match the signed delivery; the receipt records the pre-commit HEAD and explicitly binds unchanged source bytes to the final commits. Existing keys were verified and reused. Original relation TS outcomes match 23 successes and 19 refusals, including ordered programs, per-query/total gas, witnesses, state and ContractLab rollback.

The separate ledger 8.1 receiver passes eleven independent relation snapshots with full-ledger byte equality and unchanged replay refusal. JavaScript passes 66 carrier roundtrips and 132 malformed-input refusals. Eight original setup calls are retained separately. Earlier 14+2 public snapshots are historical evidence, not a fresh 27-row run. Native pins remain ledger 8.0.3; constructor data is deployed but constructor execution is not proved. No live network acceptance is claimed.

[ADR0293-0295-0303-0304 — Signed joined delivery.zip](references-0.3.0.md#note-076) contains 3081 verified entries; SHA256 `9062cfe7afcf8c15f77cb8674ea75cfcb0865a37898cdc31f45e0fb8c6b7c748`. Tool executables, parameter files and reused keys retain original paths/hashes and are not duplicated into this archive. It includes joined source, proof receipts/logs, public carriers/receiver results, release migration evidence and standalone-lock validation. Failed intermediate attempts remain alongside corrected passing runs.

These four child deliveries are complete. Parent evidence reconciliation, final coverage/performance, audit and release qualification remain separate obligations. Accepted parents remain 6/20 pending that reconciliation. No push, registry publication or remote CI. The user-owned ledger document remains byte-identical.
