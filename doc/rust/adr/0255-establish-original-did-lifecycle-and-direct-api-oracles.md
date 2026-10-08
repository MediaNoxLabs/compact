---
id: RUST-ADR-0255
alias: ADR-0255
source_sha256: ad26e53cdf9bd801467e4de8a445093940d133dbde6e30f03d1badae12f74804
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0255 — Establish original DID lifecycle and direct API oracles

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for implementation, 2026-10-07. Parents #353, #351 and #355. Test-only slice; no emitter/runtime, ABI or schema changes. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR-0255 — Establish original DID lifecycle and direct API oracles

Status: accepted for implementation, 2026-10-07. Parents #353, #351 and #355. Test-only slice; no emitter/runtime, ABI or schema changes.

### Decision

Accept the constructor-to-native lifecycle and all twelve direct pure API tests described below. Original Compact source and generated Rust remain unchanged. Use independent compiled TypeScript captures and typed generated Rust calls through ContractLab. Record actual state, effects, query gas, private outputs and witness order, including failure prefixes and rollback limits. Do not count transitive calls as direct export coverage.

Native progression is not proven ledger history. Recording remains 1/12 until a separate emitter slice is accepted. The subsequent recording sections below are research, not implementation authorization. Exact constructor self-address semantics must be preserved and investigated before sequential ledger deployment claims.

### Before/after

```rust
// Before: selected deactivate scenario starts from a labeled seeded prior state.
// After: tests start at the unchanged constructor and invoke generated APIs.
let initial = did::initial_state(constructor_context, &witnesses)?;
let mut lab = ContractLab::from_constructor(identity, environment, initial)?;
lab.native(|context| did::rotateControllerKey(
    context, &witnesses, next_key, signature, expected_version,
))?;
```

The example shows intended ownership; compiled tests establish the exact generic signatures. Test adapters own deterministic synthetic keys and captures; generated code owns contract semantics; upstream runtime owns VM, primitives and cryptography.

### Detailed acceptance design

## ADR0255 proposal — original DID lifecycle and direct pure API acceptance

Research snapshot: 775907eadf1122b7acd1c0657520ae651c642abd, IR20 / ABI50. Read-only proposal; no production changes or new measurements. Source is the exact pinned midnight-did v0.7.0 package already introduced by ADR0251. Machine-readable signatures, transitive call graph, node kinds and exact IR effect paths: `/tmp/rust030-did-next-export-inventory.json`.

### Decision requested

Implement independent TypeScript capture and typed Rust **constructor-to-native lifecycle** tests for the unchanged DID, including direct calls to all 12 exported pure functions. Use existing ContractLab native snapshots/rollback, generated witness traits and actual ledger VM. Keep source and runtime/emitter unchanged. Recording extension and sequential strict transactions are a subsequent decision, described below; this tranche must not describe native state progression as proven ledger history.

Before: 24 ADR0251 cases cover reduced crypto/original deactivate from captured prior states; only deactivate is recorded/proved. Other exports compile but have no checked-in complete lifecycle acceptance. Pure authorization functions are used transitively by TS signing but are not each directly compared to their generated Rust public API.

After: an independent TS sequence starting at the original constructor supplies exact state, query order, witness inputs/outputs, private-state evolution and failures. Rust directly invokes each generated API in the corresponding sequence. All 12 stateful exports are executed; all 12 pure exports are called directly. The recording capability remains **1/12** unless a separately accepted emitter slice changes it.

### Original exported stateful API inventory

All return Unit. `Sig` is the exact imported SchnorrSignature; `U64` is Uint<64>.

| Source line / export | Parameters | Additional reusable boundary beyond ADR0251 |
|---|---|---|
| 577 rotateControllerKey | Point, Sig, U64 | Point projections + Field inequality in lazy guards; Point Cell write |
| 595 recoverControllerKey | Point, Sig, U64 | Same operations, recovery-key authorization |
| 613 setAlsoKnownAs | String, SetMutation, Sig, U64 | String/enum typed values; pure Unit mutation guard; String Set member/insert/remove |
| 638 setVerificationMethod | VerificationMethod, MapMutation, Sig, U64 | String/enum composite Map insert/member/remove; Boolean-returning existence helper; relation Set reads; pure validation guards |
| 670 removeVerificationMethod | String, Sig, U64 | Map member/remove plus five ordered relation Set membership refusals |
| 688 setSchnorrJubjubVerificationMethod | {id:String, publicKey:Point}, MapMutation, Sig, U64 | Composite Map insert/member/remove, two-map lazy existence helper, Point coordinates in pure digest |
| 718 removeSchnorrJubjubVerificationMethod | String, Sig, U64 | Map member/remove plus five relation guards |
| 742 verifySchnorrJubjubDigestSignature | String, Vector<4,Field>, Sig | Active read, Map member/lookup, Point projection from stored struct, existing local Schnorr verifier; no timestamp/counter updates |
| 754 setVerificationMethodRelation | Relation, String, SetMutation, Sig, U64 | Boolean effect-return helpers, typed Map lookup, lazy multi-slot member checks and selected one-of-five Set writes |
| 791 setService | Service{id,typ,serviceEndpoint:String}, MapMutation, Sig, U64 | Pure Unit guard; Map member/remove/insert of String struct |
| 816 removeService | String, Sig, U64 | String Map member/remove |
| 833 deactivate | Sig, U64 | Already covered by ADR0251 |

All mutation calls evaluate their authorization digest and source argument disclosure before authorization helper execution; common successful mutation tail is operationCount increment, version increment, then timestamp witness/write (source440). Do not move validation before authorization or combine/remove repeated reads. The read-only verifier is a transaction circuit, not a mutation or pure function.

### Native acceptance matrix

Use deterministic public test keys/controller secret1, distinct recovery secret3, later controller secrets5/7 and distinct method keys. Signing must implement the exact pinned Schnorr scheme and field/subgroup boundaries, using the independent TS runtime; do not call emitted private methods as the only oracle. Store synthetic signatures openly as test data, never production key material.

Main sequence starts with `initialState`, then drives source APIs, without direct ledger slot seeding:

1. Constructor: controller/recovery/timestamp witness order, initial flags, counters, complete state bytes, id, created/updated. Equal controller/recovery rejection precedes timestamp. Witness failures retain original error/order.
2. Alias insert/remove; duplicate insert, missing remove and Undefined mutation on independent snapshots, with valid signatures for the deliberately invalid payloads.
3. Service insert/update/remove; missing update/remove, duplicate insert, Undefined mutation; empty and Unicode strings as distinct cases.
4. Generic verification method insert/update/remove. Exercise accepted OKP Ed25519/X25519/BLS12381G1/BLS12381G2 and EC P256/Secp256k1 branches; rejected unsupported method type, RSA/oct, EC Jubjub, invalid curve/type combinations. Preserve source validation ordering.
5. All five relation selectors: insert/remove, duplicate/missing relation, Undefined relation, absent target. Signing relation rejects X25519; KeyAgreement requires an existing X25519 generic method. Mutation of a referenced method retains compatibility checks. Removal refuses each remaining reference in source order.
6. Schnorr method insert/update, direct read-only signature verification, relation add/remove, referenced removal refusal, final method removal. Cross-map duplicate IDs must reject in both insertion directions. Assert read-only verification leaves full state/version/operationCount/updated unchanged, consumes exactly reduction witness and no timestamp.
7. Rotate controller; old controller signature fails afterward; recover using unchanged recovery authority; wrong authority signature fails. Same-current and recovery-equal new keys reject at the original point-check locations. Use correctly signed invalid requests to reach each specific guard.
8. Deactivate using latest controller/version; state flags and counters advance once. Subsequent mutations reject inactive; stale expectedVersion rejects before inactive where source does so. Read-only verifier tests active check before member lookup.

For each selected rejection, fork or restore a constructor-derived snapshot after real preceding calls. Assert ContractLab committed public/private snapshot unchanged, while separately retaining witness callback trace and successful TS query prefix. ContractLab does not undo external callback side effects. Maximum-Counter overflow may use explicitly labeled seeded stress cases from ADR0251; do not count these as the unseeded lifecycle sequence.

Capture full VM programs and individual query gas in TS. Native Rust exposes state/effects/gas/private transcript rather than a recorded program for the 11 gaps: compare those dimensions honestly, and retain TS operation traces as independent expected execution evidence. For deactivate, keep the existing additional recorded-program/replay checks. No fabricated Rust transcript or claim that native calls prove themselves.

### Direct pure exports

Every result is Vector<4,Field>. Call these Rust APIs directly and compare independent compiled TS results, with typed case-level arguments:

- controllerAuthorizationDigest(ContractAddress,U64,Field,Field), line191.
- rotateControllerKeyAuthorizationDigest(ContractAddress,U64,Point), line212.
- recoverControllerKeyAuthorizationDigest(ContractAddress,U64,Point), line230.
- setAlsoKnownAsAuthorizationDigest(ContractAddress,U64,String,SetMutation), line248.
- setVerificationMethodAuthorizationDigest(ContractAddress,U64,VerificationMethod,MapMutation), line264.
- removeVerificationMethodAuthorizationDigest(ContractAddress,U64,String), line283.
- setSchnorrJubjubVerificationMethodAuthorizationDigest(ContractAddress,U64,SchnorrMethod,MapMutation), line298.
- removeSchnorrJubjubVerificationMethodAuthorizationDigest(ContractAddress,U64,String), line319.
- setVerificationMethodRelationAuthorizationDigest(ContractAddress,U64,Relation,String,SetMutation), line334.
- setServiceAuthorizationDigest(ContractAddress,U64,Service,MapMutation), line355.
- removeServiceAuthorizationDigest(ContractAddress,U64,String), line371.
- deactivateAuthorizationDigest(ContractAddress,U64), line384.

Boundary corpus: versions0/1/u64MAX, distinct contract IDs, all defined enum variants including Undefined (digest construction itself need not reject), distinct keys, empty/Unicode strings, and every nested payload field changed independently. Assert exact four Field values and domain/argument sensitivity for concrete pairs. Do not claim collision resistance from finite tests. Constructor metadata or indirect signing calls do not count as direct-export coverage.

### Ownership and validation

New files under `tests-rust-backend/did-adoption/{oracle,support,tests}` plus capture script/fixture and dev-dependency on existing `testkit-rs` if needed. Split witness/signing adapter, lifecycle dispatcher, pure API cases, and provenance validation. Rust dispatch must call typed generated functions explicitly, not bypass them with hand-written VM or a second interpreter. Pin source/import SHA256, compiler, generated module, runtime implementation hashes and capture hashes. Preserve existing ADR0251 capture/proof receipts.

Run selected new and existing DID tests, strict fixture Clippy, deterministic TS recapture, original source hash check and focused fixture freshness. Case counts derive from the reviewed matrix and actual runs; no percentage or full-path claim. No proof/keygen/network run required for this test-only tranche.

### Subsequent recording slices (not implementation authorization here)

#### First: typed Point Cell authorization transitions — two new exports

Extend the existing Unit composition audit narrowly for Point Cell writes, Point X/Y projection, Field inequality, and their lazy Boolean/If guard composition. Pure digest audit permits those query-free coordinate projections. Shared Plan materializes typed operands once and calls existing runtime functions (`runtime::jubjub_point_x/y`), preserves callee scope and source query order. No crypto reimplementation, new runtime primitive, ABI bump, contract-name whitelist or arbitrary operation-count rule.

This is the smallest structural slice: rotate+recover depend only on already supported Cells/Counters and local Schnorr. Reduced examples should isolate `(a)` Point Cell write; `(b)` two lazy inequality checks reading distinct Point Cells; `(c)` same Point passed through pure hash helper and typed Unit authorization helper. Reject hidden ledger reads in supposedly local/pure helpers, wrong point/Field type, cycle/arity/scope errors, and unselected unsupported effects. Reuse native lifecycle oracle before adding proof.

Strict acceptance should progress from an **actual ledger-applied ContractDeploy** containing the original constructor result, then rotate→recover→deactivate sequentially, each proven/default-strict/separate Dust and each next context from applied ledger state. No direct contract-state insertion after deploy. Keep failed-call rollback and changed-signature/version/state binding negatives. Deployment validates an initial state; it is not a proof of constructor execution. Source constructor `kernel.self()` runs with TS dummy address (generated TS index.js1268); Rust matches. Retain the exact stored id and investigate upstream deployment semantics before claiming it equals actual deployment address—never silently rewrite it to obtain a desired digest.

#### Later: collections with high shared fanout

A closed String/enum/composite value domain plus audited **pure Unit guard calls** is a prerequisite currently missing from composition.rs (value_type26, action194). These guards are IR `PureCall`, not stateful Unit calls; transitive inventory includes them. Then add Map member/lookup/insert/remove and Set member/insert/remove through existing typed slot APIs (`runtime-rs/src/slots.rs712,729,737,818,904,924,939`), respecting full chunked paths and owned String lifetimes.

A conservative first collection vertical can cover alias and service mutations (3 exports) with pure guards and String Set/Map effects. Next add `RecordedValue` Boolean helpers whose return is a closed expression: verificationMethodExists uses lazy OR over two Map members; verificationMethodRelationMember uses selected one-of-five Set member reads. Their source return is `StateReturn::Expression`; no new arbitrary ReturnPlan language is needed for those exact shapes. Shared Plan still owns value evaluation and scopes. This enables the broader verification-method/relation family, subject to real compile/trace/proof results; do not claim all remaining exports based on this inventory alone.

### Existing reusable boundaries / coordination

- `recorded/typed_plan/composition.rs26,129,194,244`: current value/write policy, declaration-derived call audit, action audit and lowering.
- `recorded/audited_local.rs181`: complete query-free native Unit audit; keep Schnorr local.
- `recorded/typed_plan.rs637`: typed lazy If; 995 currently SetMember is limited to Bytes32/qualified-coin, not String. No Map expression leaf exists in this evaluator yet.
- `stateful.rs` native rendering already supports original code; ADR0254 is extracting IR analysis only into circuit_analysis with reexports preserved. Future recording slice should use those reexports or coordinate a deliberate import update, not edit extraction-owned code.
- `testkit-rs/src/lab.rs57,77,93,146,159`: constructor adoption, snapshot, restore, native and recorded calls. Its trusted-adapter/privacy/rollback limits remain explicit.

Architecture debt: the closed profile collection is growing. Share typed value/effect leaves and declaration audit policy rather than duplicate planner/scoping logic. This adoption work does not adopt a new DSL, sandbox arbitrary callbacks, or establish network-level DID interoperability.

### Local delivery —2026-10-07

`9ec520a3af2273b2642d4e383245af85e9a154ff`. Eleven constructor scenarios, 122 native calls and 102 direct pure vectors; all twelve stateful and twelve pure exports exercised.28tests and strict Clippy pass. Branch-generated TypeScript for pinned upstream source is the oracle (compiler0.31.133/runtime0.16.101), not yet the original release profile. Native progression is not proven ledger history; recording remains1/12 and constructor identity needs qualification. [ADR0255 — Original DID native lifecycle local receipt](references-0.3.0.md#note-022).
