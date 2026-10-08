# Original DID Service developer guide

The fixture under `examples/rust_backend/did_adoption/` pins the unchanged `midnight-did` **v0.7.0** source and import closure. The generated Rust crate is `tests-rust-backend/did-adoption`; its `lib.rs` is generated from that source, not hand edited. The compiler/runtime profile is IR schema 20 / ABI 50 on pinned Rust ledger 8.0.3; upstream JS source requests ledger 8.1, so this guide describes the locally tested profile rather than a general cross-version promise.

## Generated Service surface

The generated `types::Service` carries `id`, `typ`, and `serviceEndpoint`, each an `OpaqueString`. Its generated `ledger_contract::recorded` module exposes `setService(context, witness, service, mutation, controllerSignature, expectedVersion)` and `removeService(context, witness, serviceId, controllerSignature, expectedVersion)`. The bound façade also has `setService_call(observed, private_state, ...)` and `removeService_call(observed, private_state, ...)` with `ledger-transaction` enabled. Those call methods produce an observed `RecordedCall` for proof preparation. The generated signatures, exact type names and bounds are at `tests-rust-backend/did-adoption/lib.rs` (search the named function); `tests-rust-backend/did-adoption/support/service_calls.rs` demonstrates concrete typed arguments and witness reuse.

```rust
// Adapted from the actual generated façade and test support.
let result = ledger_contract::recorded::setService(
    context,
    &witness,
    service,                         // types::Service
    mutation,                        // types::MapMutation
    controller_signature,            // types::SchnorrSignature
    expected_version,                // BoundedUint<u64::MAX>
)?;
let replay = result.replay()?;
let program = replay.program();
```

Use `MapMutation::Insert` for a new `id`, `MapMutation::Update` for an existing one. Update checks membership and removes the old value before inserting the new value; `removeService` requires an existing `id`. Controller authorization, expected version and active state are checked before the mutation. A failed call leaves committed ledger state unchanged while already executed external witness activity remains separately observable. Use the generated `RecordedCall` binding and the runtime transaction API for strict proof/application; do not construct a favorable state or substitute an authorization witness.

## Local verification

```sh
CARGO_TARGET_DIR=/path/to/an/owned/warm-target CARGO_INCREMENTAL=0 \
  cargo +1.99.0 test --offline --locked -p compact-rust-did-adoption-fixture
python3 tools/compact-rust-backend/test_did_proof_gate.py
python3 tools/compact-rust-backend/did_proof_gate.py \
  --compiler /path/to/pinned/compactc \
  --scheme /path/to/pinned/compactc-scheme \
  --run-dir /path/to/new/private-run \
  --cargo-target-dir /path/to/owned/warm-target
```

The proof gate freezes source/compiler/material hashes and requires Point, Alias, Service, Schnorr-method and JWK-method scenarios. Service success is the original constructor-derived Insert → Update → Remove sequence. It verifies each proof and uses default-strict ledger application with separately funded Dust; it also checks changed binding and same-time replay refusal. The retained frozen run is “ADR0269 — Strict DID Service proof receipt” (historical vault reference; not bundled here). Independent capture and parity tests are `tests-rust-backend/did-adoption/oracle/service-lifecycle-cases.mjs`, `service-lifecycle.json`, and `tests/service_recording.rs`. The captured rows include Unicode and empty-field updates, missing/duplicate/undefined mutation and late failure boundaries; compare ordered public VM operations, query costs, resulting state/effects, private witness order and replay from the same prestate.

## Generated SchnorrJubjub method surface (ADR0278)

The original `SchnorrJubjubVerificationMethod` has an `OpaqueString` ID and a `JubjubPoint` public key. The generated recorded façade exposes `setSchnorrJubjubVerificationMethod` and `removeSchnorrJubjubVerificationMethod`, plus `*_call` methods under `ledger-transaction`. The actual generated signatures are in `tests-rust-backend/did-adoption/lib.rs` (search the named function); typed call support is in `tests-rust-backend/did-adoption/support/schnorr_method_calls.rs`.

```rust
let call = binding.setSchnorrJubjubVerificationMethod_call(
    &observed,
    private_state,
    verification_method,          // types::SchnorrJubjubVerificationMethod
    types::MapMutation::Insert,
    controller_signature,
    expected_version,
)?;
let replay = call.recorded().replay()?;
```

The checked profile retains controller authorization and version checks, the source-order relation Set reads before removal, and short-circuit membership in the action-free Boolean helper. The immutable 183-source comparison admits only the two intended new capabilities; a broader candidate that changed Asset Registry `setWatch` output was rejected. Independent TS capture and generated Rust tests are `oracle/schnorr-method-lifecycle-cases.mjs`, `oracle/schnorr-method-lifecycle.json` and `tests/schnorr_method_recording.rs` under the DID fixture. The original-constructor Insert → Update → Remove sequence passed three default-strict ledger applications, nonempty proofs, changed-binding refusal and same-time replay refusal. See “ADR0278 — Strict DID Schnorr proof receipt” (historical vault reference; not bundled here) and “ADR0278 — 183-source renderer differential” (historical vault reference; not bundled here).

## Generated nested JWK method surface (ADR0283)

The original `VerificationMethod` nests `PublicKeyJwk` and declared `VerificationMethodType`, `KeyType` and `CurveType` enums. The generated façade now exposes recorded `setVerificationMethod` and `removeVerificationMethod`, plus typed `*_call` methods under `ledger-transaction`. Actual generated types and signatures are in `tests-rust-backend/did-adoption/lib.rs` (search the named function), `:293–343` and `:4943–5048`; `tests-rust-backend/did-adoption/support/jwk_method_calls.rs` supplies concrete arguments.

```rust
let call = binding.setVerificationMethod_call(
    &observed,
    private_state,
    verification_method,          // types::VerificationMethod with nested PublicKeyJwk
    types::MapMutation::Insert,
    controller_signature,
    expected_version,
)?;
let replay = call.recorded().replay()?;
```

The checked composition accepts a nonempty declared named-product tree whose leaves are `OpaqueString`, `JubjubPoint` or a matching declared Enum. It uses the existing typed Map slot recording primitives and audits the entire helper graph, including unused bindings and unselected branches. Direct key-only membership on a nested-value Map outside the read-only Boolean helper is permitted only when that same declared field/index is mutated by the composition. `CurveType != X25519` uses matching declared Enum types; this does not admit mixed enums or unreviewed scalar operations. No contract name or field-count shortcut is used.

The independent TS capture and native/recorded tests are `oracle/jwk-method-lifecycle-cases.mjs`, `oracle/jwk-method-lifecycle.json` and `tests/jwk_method_recording.rs`. The capture includes actual relation prestates and 23 JWK CRUD rows; the existing capture supplies another 38 JWK rows. The original-constructor Insert → Update → Remove sequence passed three default-strict ledger applications with nonempty verified proofs, changed-binding refusal and same-time replay refusal. See “ADR0283 — Nested JWK delivery receipt” (historical vault reference; not bundled here), “ADR0283 — Strict DID JWK proof receipt” (historical vault reference; not bundled here), “ADR0283 — 183-source renderer differential” (historical vault reference; not bundled here), “ADR0283 — Pinned DID source and capture closure” (historical vault reference; not bundled here) and “ADR0283 — Local gate logs and interrupted attempt” (historical vault reference; not bundled here).

## Digest verification and relation mutation

At signed `329bf1bc`, all twelve original DID exports have recorded and observed-call APIs. These two methods complete the surface described above:

```rust
// Excerpt using existing caller-owned values; not a standalone setup tutorial.
let digest_call = binding.verifySchnorrJubjubDigestSignature_call(
    &observed,
    private_state,
    method_id,       // runtime::OpaqueString
    digest,          // runtime::FixedVector<runtime::Field, 4>
    signature,       // types::SchnorrSignature
)?;
let replay = digest_call.recorded().replay()?;
```

Digest verification reads the stored method, checks the original digest/signature helper and preserves its ordered observations. A method's presence alone is not successful signature verification. The original source and its negative cases remain the behavioral authority. See `tests-rust-backend/did-adoption/tests/digest_recording.rs` and the ADR0288 receipt.

```rust
// Independent call example; use the observed state and version for this call.
let relation_call = binding.setVerificationMethodRelation_call(
    &observed,
    private_state,
    relation,             // types::VerificationMethodRelation
    method_id,            // runtime::OpaqueString
    mutation,             // types::SetMutation
    controller_signature, // types::SchnorrSignature
    expected_version,     // runtime::BoundedUint<u64::MAX>
)?;
let replay = relation_call.recorded().replay()?;
```

The generated enum types and bounded version separate these arguments at compile time. The source still decides which key kinds can participate in each relation and checks authorization, active state, version, membership and insertion/removal preconditions. A type-correct argument is not a promise that the call will succeed. The typed recorder preserves the selected Set accesses and nested JWK reads; rejected calls do not become successful recordings.

`tests-rust-backend/did-adoption/tests/relation_recording.rs` covers42 independent TS rows:23 successes and19 refusals. The joined relation gate passes19 original calls and six reduced-source calls with real proofs, default-strict ledger application, changed-binding refusal and replay refusal. This gate complements the earlier lifecycle/digest gates. It does not replace them with a single unqualified “all proved” label.

The function names and argument order above were checked against the unedited generated crate at329bf1bc. They illustrate the existing facade; no generated Args switch, application wrapper or new external-consumer measurement is claimed. See “ADR-0276 — Prototype generated named argument facades” (historical vault reference; not bundled here) for the retained research disposition.

## Current scope

All12 original DID recorded exports are locally accepted. Native Rust pins ledger8.0.3. Independent ledger8.1 public transaction snapshots cover all12 exports, including eleven new relation snapshots with66 JS roundtrips and132 malformed refusals. Those snapshots are independently initialized, not a continuous8.1 chain. Constructor data is deployed; constructor execution remains unproved. This is not live-network acceptance or general compatibility with arbitrary later ledger/compiler versions.

Nested product Maps, selected relation Sets and read-only nested JWK lookups are admitted by their checked profiles. This does not make arbitrary effect graphs recordable. Inspect the capability manifest for each newly compiled source and distinguish native generation, recording, replay and proof applicability. Full current evidence: “DID and minimal-reproducer local acceptance — 2026-10-07” (historical vault reference; not bundled here) and “Joined relation and release delivery — 2026-10-07” (historical vault reference; not bundled here).

Decisions and receipts: “ADR-0269 — Record opaque-string product Maps through shared typed composition” (historical vault reference; not bundled here), “ADR-0274 — Preserve default worker stack in typed Map lowering” (historical vault reference; not bundled here), “DID Service delivery — 2026-10-07” (historical vault reference; not bundled here), “ADR-0278 — Record flat point-product Maps and Boolean membership helpers” (historical vault reference; not bundled here), “ADR0278 — Point-product Map delivery receipt” (historical vault reference; not bundled here), “ADR-0283 — Record nested JWK VerificationMethod Map CRUD” (historical vault reference; not bundled here), “DID JWK delivery — 2026-10-07” (historical vault reference; not bundled here).


Guide refreshed2026-10-07 against signed329bf1bc (later ec10f324 adds only compiler tests). Historical delivery counts above remain tied to their dated receipts. Current API examples are excerpts backed by existing generated-fixture tests, not a newly executed clean external consumer.
