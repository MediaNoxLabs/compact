---
id: RUST-ADR-0288
alias: ADR-0288
source_sha256: 1917191815682e07b6e74c421f51be8c907f7643c5f5b2a0d8abe2878dc80b61
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0288 — Record original DID digest verification through checked read and local Unit helper

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** locally delivered in signed/DCO commit `8a52a01001894ecb5d0e773463436b78993a119b` on 2026-10-07. The original bounded decision was accepted before implementation and issue #413 was created before code. Baseline is signed ADR0283 commit `770f8dcb546aa3d95455da3a02b9701dcb975191` and its frozen original DID IR at `/tmp/rust030-adr283/did-final/contract/compact-rust-ir.json`. The unchanged source is `examples/rust_backend/did_adoption/packages/contract/src/did.compact`, pinned DID v0.7.0. Current source uses ledger 8.1; local Rust proof environment remains ledger 8.0.3. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0288 — Record original DID digest verification through checked read and local Unit helper

Status: locally delivered in signed/DCO commit `8a52a01001894ecb5d0e773463436b78993a119b` on 2026-10-07. The original bounded decision was accepted before implementation and issue #413 was created before code. Baseline is signed ADR0283 commit `770f8dcb546aa3d95455da3a02b9701dcb975191` and its frozen original DID IR at `/tmp/rust030-adr283/did-final/contract/compact-rust-ir.json`. The unchanged source is `examples/rust_backend/did_adoption/packages/contract/src/did.compact`, pinned DID v0.7.0. Current source uses ledger 8.1; local Rust proof environment remains ledger 8.0.3.

### Choice and evidence

Choose `verifySchnorrJubjubDigestSignature` before `setVerificationMethodRelation`. Both have working native Rust APIs, but neither has recorded/observed-call APIs (10/12 original exports recorded). The digest circuit is a three-stateful-circuit graph: exported verification → `schnorrVerifyDigest` → `schnorrVerify`. Its public side is one Boolean Cell read, one Map membership, and one Map lookup on the same declared `Map<OpaqueString, flat {OpaqueString,JubjubPoint}>`; the callee is a local Unit helper with no public state effect. The original TS capture has five cases (two successful, three rejected). Its original ZKIR key generation reports k=11, 1608 rows (`/tmp/rust030-adr288/verifyDigest-keygen.log`).

Relation mutation traverses 16 stateful nodes / 15 call edges, five branch-selected relation Sets, Boolean relation membership, JWK compatibility Map lookups, and update bookkeeping. The maintained TS capture has 39 cases; original ZKIR key generation is k=12, 2718 rows (`/tmp/rust030-adr288/relation-keygen.log`). Its first missing recorded action is a nested `CircuitCall`, whereas the digest first misses the scoped `MapLookup` Let. Relation work can reuse a proven read-only lookup boundary later, but should receive a separate proposal and authorization.

Two unchanged source reducers compiled with the frozen post-ADR0283 compiler:

- `/tmp/rust030-adr288/point-map-lookup.compact` (SHA-256 `2a9dfd02a754467f78fffeeef80407d11ee53d56e72ae9fd7e77b4d32cbe29fc`) is native-only. Its first recorded gap is `StateAction::Let` at `actions[0].action.actions[1]`.
- `/tmp/rust030-adr288/relation-two-set.compact` (SHA-256 `c275bb37590e08d007c45e157858dc3f894cf379ba43af7ee56a291c04331e80`) is native-only. Its first recorded gap is an `Assert` at `actions[0].action.action.action.actions[1]`.

The original digest circuit is native and typechecked already. Its first recorded gap is `StateAction::Let` at `actions[0].actions[1].action.actions[1]`; the original relation's first gap is `StateAction::CircuitCall` at `actions[0].action.action.action.actions[0]`. These are coarse first gaps, not complete dependency diagnoses.

### Problem and intended developer surface

Before, the source can execute in native Rust but has no recorded call:

```compact
assert(active, "Contract is not active");
const disclosedMethodId = disclose(methodId);
assert(schnorrJubjubVerificationMethods.member(disclosedMethodId), "Verification method does not exist");
const method = schnorrJubjubVerificationMethods.lookup(disclosedMethodId);
Schnorr_schnorrVerifyDigest(digest, signature, method.publicKey);
```

After the bounded admission, the generated crate adds the same method under `ledger_contract::recorded` and its observed-call facade. No source, ABI, IR schema, native method, or runtime API changes are expected. A consumer can call the recorded verifier using the existing circuit context and witness provider, then inspect the exact ordered public query transcript, private reduction witness, gas, and replay result. An unsuccessful assertion or invalid signature remains a typed error; it must not produce an admitted transaction.

### Proposed domain boundary

Add one checked **public-read + local-Unit-verification** profile, preferably in a small recorded module using existing typed scope/Plan renderer and the existing `audited_local::unit_helper` transitive audit. Do not add an evaluator for Schnorr or route by `schnorrVerifyDigest` name. The existing native renderer already emits the imported helper body and uses ledger-backed `transient_hash`, EC operations, and checked Jubjub scalar conversion (`runtime-rs/src/natives.rs`). The audited local helper allows local `TransientHash`, EC operations, field arithmetic, and declared witness calls, and rejects every ledger query/write, unknown call, native witness, and recursive circuit call throughout the helper graph. Before implementation, verify this audit accepts the *actual unchanged imported helper* and that its witness declaration/result types are checked by native rendering; amend this ADR if a distinct domain is required. Do not substitute the earlier `schnorr-attest-oracle` fixture's name-keyed compiler shortcut: that fixture has a different local module and an added identity check absent in the pinned original DID source.

Public prefix acceptance is structural and typed:

1. Unit result/return and scoped ordered actions. An optional required Boolean Cell assertion may precede a disclosed `OpaqueString` local, but no public write/witness or hidden effect may be hoisted. For the original circuit, the active read remains first.
2. A `MapMember` assertion and exactly one `MapLookup` bind must refer to the *same declared field/index and same scoped key value*. The ledger field must be a String-key Map whose value is a nonempty flat named product with OpaqueString and JubjubPoint fields. Match the declared Map value type exactly, not only its shape or the source's `SchnorrJubjubVerificationMethod` name.
3. One terminal local Unit call receives typed arguments evaluated once, in order. A point projection must derive from that bound lookup result; a same-shaped parameter or lookup of another Map cannot stand in for it. The entire transitive callee graph must pass the existing no-public-effects local audit. Parameter/result arity and types, witness declaration/result, lexical references, and cycle detection must pass existing checked render paths.
4. Walk every binding, branch, and argument, including unused/unselected ones. Reject hidden Cell/Counter/Set/Map/Merkle/witness effects outside the audited local graph, a second lookup, a changed key, a different slot, a shadowed/escaped binding, or any public write. Preserve old profile precedence and existing successful output bytes; unsupported inputs keep a precise capability gap.

The first implementation should measure whether this generic shape accidentally admits additional source exports/reducers and review every capability change. It should not require exact action-count/name matching, but should reject reordered or additional effects outside the bounded grammar. Existing `audited_local::unit_helper` currently audits the local graph but its specialized top-level `render` profile does not match this three-parameter Map-read circuit; reuse its audit, not its old top-level shape.

### Verification required before delivery

- Independent corrected TS capture against pinned DID v0.7.0: all five original digest scenarios: missing method, valid digest/signature, wrong key, updated method, and inactive-before-missing. Preserve raw captured public program, query order, gas, private witness values, state, errors, and source/runtime hashes.
- Native vs TS, recorded vs TS, replay and summed query gas for each accepted case. Check failure precedence: active assertion before membership, member before lookup, and reduction/signature failures after ledger reads. A rejected verification leaves state unchanged. The native source must remain unchanged.
- Direct structural negatives: same-shaped wrong Map, member/lookup key mismatch, Map value declaration mismatch, projection from another value, second/hidden lookup, hidden write or witness in unused/unselected binding, recursive/impure/unknown helper, malformed witness argument/result and scoped local escape. Positive reducer should cover non-source field names or reordered flat product where declared identities still match, not hard-code DID names.
- Strict proof+verify+ledger apply for a genuinely seeded original method, with exact retained offer/fee setup and recorded replay; successful read-only call should preserve ledger contract state. A tampered signature should reject at the same source point with no applied state. Add a strict proof/apply for the minimal source reducer if its native and recorded paths are proof-applicable; malformed IR and diagnostic refusal reducers are not proof-applicable. Original digest ZKIR k=11/1608 is a planning artifact, not proof evidence. No claim of native Rust compiling against DID's ledger 8.1 until that protocol pairing is separately verified.
- Default worker stack, focused backend/renderer, strict Clippy, fixture freshness, exact 183-source output/capability comparison (only the intended original digest export plus intentional generic reducers may change). Do not count 11/12 until the original strict proof succeeds.

### Deferred

`setVerificationMethodRelation` remains native-only. Its five selected String Sets, conditional Map lookup and JWK curve compatibility checks, controller authorization, timestamp witness, and order-sensitive update require a separate checked multi-Set relation profile and all 39 maintained TS cases. It should be proposed after the read-only lookup and scoped local helper are proved; no broader Set membership or authorization semantics are implied by this slice.

### Approval and ownership

Root approved this bounded read + local Unit verification profile on 2026-10-07. Work first occurs in an isolated compiler prototype while ADR0286 owns the live compiler. Production port requires explicit source release. No compiler/runtime/DID original source edit, remote CI, push, or issue closure is implied by accepting this decision.


Issue: [MediaNoxLabs/compact#413](https://github.com/MediaNoxLabs/compact/issues/413), milestone `rust-backend-v0.3.0`, created before implementation.


### Isolated pre-port audit — 2026-10-07

A crate snapshot archived from signed ADR0283 `770f8dcb` lives at `/tmp/rust030-adr288/backend-snapshot/tools/compact-rust-backend`. Its direct test confirms that both unchanged imported `schnorrVerifyDigest` and transitive `schnorrVerify` pass `audited_local::unit_helper` (`/tmp/rust030-adr288/helper-audit-test.log`, 1/1). No distinct crypto evaluator or runtime API is required for this original graph. The source has two-segment physical ledger paths `[1,5]` (active Cell) and `[1,8]` (Schnorr method Map), so the checked profile uses actual declaration/index identity and the existing slot renderer, not a flat-path precondition.

The isolated structural prototype admits the unchanged original exported digest circuit, rejects a different same-typed Map declaration, a changed lookup slot, an extra public read, root parameter shadowing, a rebound point projection, a recursive or publicly effectful helper, and reordered typed arguments (`/tmp/rust030-adr288/prototype-tests.log`, 6/6). Scratch-generated DID Rust has only the recorded method and observed facade added relative to frozen ADR0283 (`/tmp/rust030-adr288/generated-diff.patch`, 126 diff lines); its standalone `cargo +1.99.0 check --offline` passed (`/tmp/rust030-adr288/did-generated-check.log`). A copied standalone DID test compares all five maintained original digest TS cases to native and scratch recorded Rust, including successful exact public program, query-summed gas, private transcript, state/effects and failed rollback (`/tmp/rust030-adr288/digest-parity-test.log`, 1 test/5 cases passed). These are pre-port results only; strict proof/application, default full backend, 183-source differential and final source freeze remain required. Scratch compiler runtime resolves ledger 8.0.3, not the source-requested 8.1.


### Local delivery evidence — 2026-10-07

Signed/DCO commit: `8a52a01001894ecb5d0e773463436b78993a119b` (`git verify-commit` good). Earlier re-sign attempts returned bad signatures; the final commit retained the verified source tree. No remote push/CI is part of this local receipt.

Implemented the bounded checked read and audited local Unit helper for unchanged pinned DID v0.7.0 `verifySchnorrJubjubDigestSignature`. The generated crate now has a recorded method and observed call. The original source, native Rust, runtime ABI 50, and IR schema 20 are unchanged. The original DID now has 11/12 recorded/observed exports; `setVerificationMethodRelation` remains native-only. This is one finite source acceptance profile, not a generic authorization mechanism.

The private `ReadDraft` audit verifies ordered active Boolean Cell guard, scoped disclosed String key, Map membership and lookup on the same declaration/key, point projection from that exact bound result, typed helper arguments and full transitive action-free local Unit helper graph. `ReadPlan` contains all mandatory checked fields. Direct negatives cover same-typed other Map, changed key, shadowing/rebound projection, hidden/unselected effect, recursive/effectful helper, and reordered/malformed arguments. The newly admitted renderer uses shared `retained_value` so Copy points are not cloned. No source-name or source-field-count gate was added.

Independent corrected TypeScript capture: seven existing DID oracle scopes were recaptured against the same pinned source/runtime; semantic case objects matched exactly and only the generated Rust library provenance changed. Five original digest cases compare native and recorded complete public program, query-summed gas, private witness output, state/effects, errors and rollback. A maintained renamed/reordered flat String–Point Map reducer has four independent TypeScript behavior cases and Rust native/recorded/replay assertions. Its checked original source and generated library are in the repository; `check_fixture_outputs.py` registers it.

Local verification (all on the final source):

- 35 Rust suites, 405 tests passed on ordinary default test workers; strict Clippy passed for backend, proof runner, original DID fixture and reducer fixture. The proof harness retains its established 64 MiB worker stack; this is separate from ledger default strictness.
- 184 generated fixtures fresh, 0 stale. An immutable 183-existing-source differential against the pre-slice renderer changed only the original DID digest recorded capability/body; all existing native Rust prefixes stayed byte-identical. The new maintained reducer is the separate 184th source.
- Original source gate: six DID scenarios, 16 proof calls, including seeded insert→digest read; original digest k=11/1608, changed FAB rejected, proof verified, default-strict Dust-funded ledger apply, contract data unchanged by read and exact replay refused.
- Maintained generic reducer gate: fresh source compile, four-case package behavior, k=7/64 keygen, 2912-byte proof verified, changed binding rejected, default-strict ledger apply, contract data unchanged and replay refused. Constructor data were deployed; constructor execution itself was not proved.

Evidence: `/tmp/rust030-adr288/delivery-receipt.json` (SHA-256 `7c6617a85f77d259e170cbedd078c0a46568c73d98f70078f68477edec430018`), original gate `/tmp/rust030-adr288/strict-original-gate-final/receipt.json`, reducer gate `/tmp/rust030-adr288/strict-reducer-gate-accepted/receipt.json`, corpus `/tmp/rust030-adr288/corpus-comparison-accepted.json`, and exact command logs linked from the delivery receipt. Prior exploratory proof/fixture attempts and initial Clippy failures are retained separately; final evidence uses the accepted compiler SHA-256 `0091e9cc7c257157ef341a2a2d616dd68766d3665db64a42b40f1d616f8acfd8`.

Limits: the pinned DID source requests ledger 8.1 while this Rust runtime/proof gate uses ledger 8.0.3. This does not establish protocol 8.1 equivalence. The five original and four reducer cases are finite, not exhaustive cryptographic or authorization coverage.


Durable frozen source/evidence: [ADR0288 frozen source and local evidence — 8a52a010.zip](references-0.3.0.md#note-069) (SHA-256 `f3a7b98af3bb70ff6248a525c51af7c7912a394ed2cead2c41ed84d2e90703f1`, 90 entries). Source files in the archive were read from the signed Git object.
