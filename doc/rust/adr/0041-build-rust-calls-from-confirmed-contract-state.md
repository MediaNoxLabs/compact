---
id: RUST-ADR-0041
alias: ADR-0041
title: "Build Rust calls from confirmed contract state"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["wallet", "transactions", "observations"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 045714ae7e52fb4d944be0f1471c37b0b3e867791c4fa4855ca008ac642ae0d4
---
# RUST-ADR-0041 — Build Rust calls from confirmed contract state

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept constructing later Rust calls from confirmed supplied ContractState, keeping observation fetching and wallet duties outside the runtime. Later Counter round 1 → 2 evidence is a specific trusted local flow. Concurrent writers, state freshness and authentication are separate boundaries; an observed snapshot is not a complete authenticated ledger.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#140 closure](https://github.com/MediaNoxLabs/compact/issues/140#issuecomment-6017466781). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`83bb4910`](https://github.com/MediaNoxLabs/compact/commit/83bb4910bc27b794af65fc3da023013a939fbd71) · [`9ee0d987`](https://github.com/MediaNoxLabs/compact/commit/9ee0d987f6bae729914991a6727f04eff78432de) · [`e75f13ba`](https://github.com/MediaNoxLabs/compact/commit/e75f13ba2fdc954a29114177c31994a63e3cbec2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 41
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/140
```

## Historical decision and amendments

### Problem

The ABI-18 live wallet gate proves and submits a counter deployment and call, then observes indexed `round = 1`. But both transaction bodies were recorded and proved from a locally simulated post-deploy ledger before the deployment was confirmed. That pair is valid for a fresh chain and one call; it does not establish a developer workflow for a later call when the on-chain contract state has changed. Replaying a prebuilt call against a different state is unsafe and should fail. A production Rust consumer needs to record and prove against the confirmed contract state at the target address.

The pinned indexer 4.0.1 `contractAction(address)` returns serialized ledger-v8 `ContractState` bytes. On the ABI-18 live run it returned 1,482 bytes at block 8. A local Rust probe used `midnight_serialize::tagged_deserialize::<ContractState<DefaultDB>>` on those exact bytes and `runtime::ledger::read_counter` returned 1. This establishes byte interoperability, not yet a post-confirmation call.

### Before and proposed after

Current proof smoke records the first call from a locally constructed state before network confirmation:
```rust
let initial = counter_contract::initial_state(ConstructorContext::new(()))?;
let context = initial.into_circuit_context(deploy.address());
let recorded = counter_contract::Contract::default().recording.increment(context)?;
let call = prepare_call(recorded, spec)?;
```

Proposed consumer path after indexer confirmation:
```rust
let confirmed: ContractState<DefaultDB> = tagged_deserialize(&mut indexed_bytes)?;
let context = CircuitContext::from_contract_state((), address, &confirmed);
let recorded = counter_contract::Contract::default().recording.increment(context)?;
let call = prepare_call(recorded, spec)?;
// Prove, seal, hand to the wallet for fees/submission, and confirm round == 2.
```

The final helper signature may include ledger cost parameters if the pinned ledger API requires them. It must make the source of state explicit and avoid using `ConstructorResult` in consumer code to disguise a confirmed network state as fresh constructor output. The API should reuse upstream `ContractState`, `ChargedState`, `QueryContext` and ledger-v8 proof primitives.

### Ownership

Runtime context owns the typed transition from an upstream confirmed `ContractState` into a `CircuitContext`; it does not fetch an indexer or claim network finality. Generated `Contract::recording` methods remain the developer-facing circuit API. A small Rust transaction builder owns decoding verified indexer bytes, checking address/operation compatibility, preparing and proving the call with emitted artifacts, and exporting sealed bytes. The wallet driver owns network observation, fees, submission and confirmation. No AST emitter or private IR change is expected; a runtime ABI bump is needed only if generated code must depend on a new method.

### Acceptance

1. Generate a later Counter call from the indexed state with `round = 1`, not the constructor state. Verify that recorded execution starts at 1 and predicts 2, with replay and exact ledger-8 proof verification.
2. Build and export a sealed call using the same contract address and compatible verifier operation. Submit through the pinned local wallet stack after the first call and observe a distinct indexed call with `round = 2`.
3. Reject mismatched address/operation, malformed state bytes and stale state; do not treat indexer action presence alone as a state guarantee.
4. Keep the default offline 57-call proof gate, generated consumer build, TypeScript parity and runtime/package ABI checks green. Document before/after code and remote/finality limits.

### Limits

An indexer snapshot is an observed chain state, not finality or an authenticated full ledger state. The wallet must balance and submit against the live node. This first slice is a counter demonstration; general stateful calls, concurrent writers, rollback handling, key management, remote CI and registry release remain M2 work. The branch stays local and unpushed.

### Decision and local delivery — 2026-10-03

Accept the typed upstream-state bridge for later Rust calls. Local conventional GPG-verified/DCO commit `83bb4910bc27b794af65fc3da023013a939fbd71` implements this Counter slice. The original proposal above is retained as decision history.

#### Before and after in the delivered code

Before, the live proof smoke prepared both deploy and first call before the network confirmed the deployment:
```rust
let initial = counter_contract::initial_state(ConstructorContext::new(()))?;
let recorded = counter_contract::Contract::default()
    .recording.increment(initial.into_circuit_context(address))?;
```
After the first call is indexed, its serialized `ContractState` feeds the next call:
```rust
let confirmed: ContractState<DefaultDB> = tagged_deserialize(&mut indexed_bytes)?;
let context = CircuitContext::from_contract_state((), address, &confirmed);
let recorded = counter_contract::Contract::default().recording.increment(context)?;
let call = prepare_call(recorded, CallSpec::new("increment", verifier, (), Fr::from(0)))?;
// Prove and seal; the wallet balances, finalizes and submits these bytes.
```

#### Ownership and compatibility

- `runtime-rs` re-exports upstream ledger-8 `ContractState` and adds `CircuitContext::from_contract_state(private_state, address, &contract)`. It clones the upstream `ChargedState` into `QueryContext`, starts default local Zswap state, uses the pinned ledger-8 initial cost model and leaves the gas limit unset. It does not fetch, authenticate or finalize an indexer snapshot.
- The AST emitter and generated `Contract::recording` facade are unchanged. Private IR stays schema 8 and generated/runtime ABI stays 18; no generated output or macro changed.
- The checked-in `record_from_confirmed` Rust example decodes the indexer state and Rust deploy bytes, checks the supplied address and installed `increment` verifier against the generated artifact, records `1 -> 2`, prepares/proves/seals the call, validates and applies it on a projected ledger state with fee balancing excluded, and rejects the same sealed call against the projected post-call state. This is a Counter smoke, not a generic wallet SDK.
- The pinned JavaScript wallet driver owns indexer observation, funded wallet balancing, finalization and submission. It invokes the Rust builder only after the first call is indexed, then requires a distinct second `ContractCall` at the same address with decoded `round = 2`. It waits for HTTP proof-server readiness because cold parameter acquisition can outlast container startup.

#### Evidence

- Captured public indexer state is 1,482 bytes, SHA-256 `cfca8f8cd513b4cf82937fc4c3c302a3c16f7cd8c2f72043b036475be131d724`; paired Rust deploy fixture is 1,769 bytes, SHA-256 `9aab17ade2790390803d22f67e45f31ff1688c80768966681ee4c33dfc1eab17`. They are checked in under `tools/compact-rust-proof-smoke/tests/fixtures/` without wallet keys.
- The focused two-test regression decodes the exact indexed bytes, checks the installed operation, records from `round = 1` and observes predicted `round = 2`; truncating the bytes fails deserialization. The Rust builder proved a sealed second call and its local state-precondition replay check passed. Deliberately wrong deployment address and wrong verifier artifact each failed before proof.
- On a fresh isolated stack pinned to node 0.22.3, indexer 4.0.1, proof server 8.0.3 and wallet facade 3.0.0, Rust deploy, first call and confirmed-state second call indexed at blocks 22, 25 and 29 at one address. The indexer reported Counter `round = 1` then `round = 2`. The stack was stopped after the run.
- `cargo fmt --all -- --check`, wallet driver syntax, staged diff check and GPG/DCO commit verification passed. The rebuilt `compactc --consumer --proof` and package gates are tracked in the next amendment after completion.

#### Boundary and follow-through

The indexer response is observed state, not a finality proof or an authenticated full ledger snapshot. The caller must bind the queried address to the response and handle rollbacks and concurrent writers. The local replay check does not prove live network replay rejection. This slice only covers Counter; dynamic network cost parameters, generic stateful circuits, durable state provenance, remote CI, published crates and a fresh remote consumer remain M2 work. The branch is local and unpushed. Focused delivery issue: https://github.com/MediaNoxLabs/compact/issues/140.

### Exact-head validation amendment — 2026-10-03

At signed/DCO `83bb4910bc27b794af65fc3da023013a939fbd71`, a rebuilt Nix `compactc` passed `check_compactc_target.py --consumer --proof`: separate generated consumer checks and all 57 offline ledger-8.0.3 proof, verification, validation and application cases. The focused captured-state regression passed 2/2. `cargo fmt --all -- --check`, wallet JavaScript syntax and staged diff checks passed.

`check_release_packages.py --manifest target/rust-runtime-release-abi18-confirmed.json` packaged and unpacked-compiled the macro and runtime crates, and `--verify-manifest` reproduced their hashes. Macro archive SHA-256: `747cb75ad93208810a7fda3af2183c1b934937b67542f1e6edd5ada8f0c66d6d` (10,905 bytes). Runtime archive SHA-256: `28bfb164d3b870ddec9c1c99c14f2bd44d003b46aa830a65a77ca900940d1ce1` (157,158 bytes). The manifest points to commit `83bb4910` and is dirty only because the separate user-owned `doc/ledger-adt.mdx` edit remains in the checkout. This is a local package rehearsal, not a clean tagged release or registry consumer.

Issue #140 is assigned to `rust-backend-v2` and remains open for the limits above. No branch push or remote CI is claimed.

### Provenance follow-through — 2026-10-03

[ADR-0042 — Bind wallet observations to submitted transaction and finalized block](0042-bind-wallet-observations-to-submitted-transaction-and-finalized-block.md) / [#141](https://github.com/MediaNoxLabs/compact/issues/141), local signed/DCO `9ee0d987`, now binds each indexed action used in the Counter flow to the final merged wallet transaction hash and a node-finalized canonical block. A fresh deploy/two-call run still observed Counter 1→2. This strengthens the state handoff in this ADR, while full state authentication, independent consensus proof, concurrent-writer recovery, generic call API, remote CI and release remain open.

### ABI-28 confirmed-state call delivery — 2026-10-04

On the fresh reset pinned local stack at signed/DCO `e75f13ba`, a current-head build of `record_from_confirmed` received the wallet driver's indexed `ContractState`, exact transaction hash, block hash and height after the first Rust call. It used the generated Counter `increment_call` method and emitted a second proven call; the wallet balanced/finalized/submitted it. Deployment/call/later call indexed at blocks 6/9/13 at the same address; decoded Counter state advanced `round = 1 → 2`. The example checked local projection replay and stale rejection before handoff. See [ABI-28 live wallet admission — 2026-10-04](references.md#private-note-01) for the 2,912-byte first proof, byte sizes, final transaction/block hashes, exact versions and commands. This is a same-head local validation of the existing API, with no emitter/runtime change. Trusted indexer/node input, concurrent state changes, broader contracts, remote CI and registry release remain open under #140 and parent #105.
