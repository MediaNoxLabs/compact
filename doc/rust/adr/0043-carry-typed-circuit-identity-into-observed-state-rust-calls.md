---
id: RUST-ADR-0043
alias: ADR-0043
title: "Carry typed circuit identity into observed-state Rust calls"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: ac02a8d3ee1712edebee7bab1128dd58c342e40a14abbeccbb36e21794d827d3
---
# RUST-ADR-0043 — Carry typed circuit identity into observed-state Rust calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed observed-call handles and artifact access for complete recorded exports, with caller-supplied observation provenance. Preserve the corrected comparison invariant: equal pre-proof payloads, independently valid proofs and equal applied state, not equal randomized proof bytes. Later live and multi-argument extensions retain separate scopes and trust limits.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#142 closure](https://github.com/MediaNoxLabs/compact/issues/142#issuecomment-6017469883). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`015b4e2d`](https://github.com/MediaNoxLabs/compact/commit/015b4e2ddddf2788df97dd9c76ddc652711d87f1) · [`05d4b6ab`](https://github.com/MediaNoxLabs/compact/commit/05d4b6aba00f1a7f0aa5381b340652d38164f120) · [`8aa2f425`](https://github.com/MediaNoxLabs/compact/commit/8aa2f425a3b6e8470f1c78eeae223c7716f09d98) · [`e75f13ba`](https://github.com/MediaNoxLabs/compact/commit/e75f13ba2fdc954a29114177c31994a63e3cbec2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 43
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/142
```

## Historical decision and amendments

### Problem

ADR-0041 proves that generated Rust can record a later call from indexed ledger-8 state, but `record_from_confirmed.rs` is a low-level consumer example. It manually decodes `ContractState`, creates `CircuitContext`, chooses `recording.increment`, reloads the verifier, repeats the string `increment` and public input in `CallSpec`, creates an intent, proves, seals and exports bytes. `RecordedCircuitResult` contains the typed output and VM trace but not its circuit identity or input. This duplication can pair a valid trace with the wrong entry point, input or verifier, and makes the generated crate harder to use than its source circuit.

### Before

```rust
let observed: ContractState<DefaultDB> = tagged_deserialize(&mut bytes)?;
let context = CircuitContext::from_contract_state(private, address, &observed);
let recorded = contract.recording.increment(context)?;
let verifier = tagged_deserialize(&mut artifact_bytes)?;
let call = prepare_call(recorded, CallSpec::new("increment", verifier, (), rand))?;
// Construct Intent, Transaction, proof provider and sealed bytes manually.
```

### Proposed decision and after

Use an explicit `ObservedContractState<D>` with address, upstream `ContractState<D>` and caller-supplied observation metadata (transaction hash, block hash and height). These metadata are evidence supplied by the caller, not an authentication or finality proof. A generated method for each eligible recorded export retains the typed arguments and circuit identity while reusing the existing recorded body:
```rust
let observed = ObservedContractState::decode(address, indexed_bytes, observation)?;
let prepared = contract.recording
    .increment_call(&observed, private)?
    .prepare(verifier, &mut rng)?;
// A typed one-argument method similarly accepts BoundedUint<65535> once
// exact input FAB encoding has been checked against the current manual path.
let sealed = single_call(network_id, ttl, prepared, &mut rng)
    .prove(provider, cost_model).await?
    .seal(&mut rng);
```

The first implementation slice may stop at `ContractCallPrototype` and retain the upstream intent/prove/seal API. A shared transaction helper may follow only if it removes real repeated code without hiding wallet policy. Keep existing `.recording.<circuit>(context, ...)`, `CircuitContext::from_contract_state` and `prepare_call` as explicit advanced APIs. Do not put a concrete local prover or indexer client into the core runtime.

### Alternatives and rationale

A body-wide procedural macro would obscure control flow and eligibility, and is unnecessary because the AST emitter already knows each recordable export and its declared parameter types. A generic `prepare_call(recorded, string, input, verifier)` is the current adapter and repeats source facts. A wrapper around a raw `ContractState` without observation metadata would perpetuate the ambiguous word 'confirmed'. The generated typed wrapper is additive and makes state provenance and circuit identity visible.

### Emitter and runtime ownership

- The AST emitter (`tools/compact-rust-backend/src/recorded.rs`) generates a typed call method only for `StatefulCircuit` exports whose recorded body is complete. It owns entry-point name and public input encoding from typed IR parameters; it never infers those facts from emitted Rust text. The first pair is Counter `increment()` and counter-parameter `increment_by(BoundedUint<65535>)` after FAB parity evidence. Witnessed borrowed `Contract<W>` needs the same API without cloning witnesses.
- Runtime owns `ObservedContractState`, a `RecordedCall` value retaining the observed state and generated identity/input, verifier compatibility against upstream `ContractState.operations`, replay/partition delegation to `prepare_call`, typed errors, and optional generic intent/prove staging. It must check recorded initial address/state against the observation before preparing.
- Reuse upstream ledger-8 `ContractState`, `ContractOperation`, `VerifierKey`, `AlignedValue`, `Intent` and transaction/proof primitives. No derive or proc macro is needed for this slice. A generated/runtime ABI increase is expected; private IR schema stays 8 if existing `StatefulCircuit.parameters` suffice.

### Acceptance, evidence to collect and risks

1. Compare new and manual Counter zero-arg and counter-parameter one-arg calls: typed result, exact public FAB input, ordered VM transcript, effects, four-dimensional gas, `ContractCallPrototype`, proof and sealed bytes under fixed randomness. Do not assume tuple encoding for multiple arguments without independent parity.
2. Reject malformed serialized state, address mismatch, missing or rotated verifier, stale state, unsupported recorded exports and proof errors. A fresh external consumer with only the generated crate as dependency must compile; include witnessed borrowed API compile coverage.
3. Keep 132 generated fixtures current, source span diagnostics, TypeScript oracle parity, 57 offline ledger proof/application calls, release archives, and local wallet admission. Measure source size and compile time; do not claim an optimization from shorter source alone.

The indexer and node boundary remains ADR-0042. An observed snapshot is not an authenticated full ledger state or finality proof; concurrent writers can invalidate a prepared call. This ADR is proposed design research, not implementation. It has no local commit, branch push, remote CI or published crate yet.

### Tracking

- Focused issue: pending MediaNoxLabs/compact issue in rust-backend-v2.
- Parents: generated crate #103, ledger flow #105, runtime release #106, observed state #140.
- Append dated delivery amendments with signed/DCO commits, before/after if design changes, exact gates and unresolved limitations.

### Delivery amendment — 2026-10-03, ABI 19

Local conventional GPG-verified/DCO commit `015b4e2ddddf2788df97dd9c76ddc652711d87f1` implements the first slice. The decision is **accepted-partial**; the original proposal and acceptance list above remain the decision history.

#### Actual before and after

Before, a caller chose the same source facts twice:

```rust
let context = CircuitContext::from_contract_state(private, address, &contract_state);
let recorded = contract.recording.increment(context)?;
let call = prepare_call(recorded, CallSpec::new("increment", verifier, (), Fr::from(0)))?;
```

After, the generated handle retains those facts and checks the observed contract operation:

```rust
let observed = ObservedContractState::decode(address, indexed_bytes, observation)?;
let call = contract.recording.increment_call(&observed, private)?
    .prepare(verifier, Fr::from(0))?;
// One-argument example: .add_twice_call(&observed, private, BoundedUint::<65535>::new(3)?)?
```

`Observation` holds caller-supplied transaction hash, block hash and height; `decode` accepts only exact tagged ledger-8 `ContractState` bytes. The generated method is gated by the generated crate's `ledger-transaction` feature. It is emitted only for exported, completely recordable circuits with zero or one parameter. The existing `recording.<name>(context, ...)` and `prepare_call` remain available.

#### Ownership and compatibility

- AST emitter: `render_observed_call_method` in `recorded.rs` derives the method name, typed argument, entry-point literal and one-argument FAB input from `StatefulCircuit`, using `syn` items. It reuses the existing recorded method and preserves the borrowed witnessed facade. Two-or-more-argument methods are withheld pending tuple FAB evidence; a collision with an existing exported `<name>_call` is also withheld.
- Runtime: `ObservedContractState` owns address, upstream `ContractState` and metadata. `RecordedCall::prepare` rejects address/state mismatch, missing operation and installed-verifier mismatch before delegating to existing `prepare_call`, ledger replay and transcript partition. It returns the upstream `ContractCallPrototype`. No new VM builder, prover or wallet policy was added.
- Generated/runtime ABI moved 18→19; private typed IR remains schema 8. The 132 fixtures and the feature declarations of 57 fixture crates with generated call methods were refreshed. Counter and counter-parameter fixtures grew by 42 additions and 2 deletions each; no source-size or compile-time optimization is claimed.

#### Evidence

- `cargo test -p compact-rust-backend --test render --locked`: 57 renderer tests pass, including typed zero/one-argument methods, suppressed multi-argument method and unsupported recorded body.
- `cargo check --workspace --all-features --lib --locked --quiet`: passes across generated fixture libraries, including witnessed borrowed methods.
- Fresh local `compactc` with `check_compactc_target.py --consumer --proof`: separate generated consumers and all 57 offline ledger proof/verification/validation/application calls pass. The zero-argument `increment` and typed one-argument `add_twice(BoundedUint<65535>)` prepared prototypes match the manual adapter in complete Debug output before the typed prototypes are proved and applied. The gate rejects trailing state bytes, missing or wrong verifier, wrong address and stale state.
- Exact fixture check: 132 checked, zero stale and zero failed. `cargo fmt --all --check` and scoped `git diff --check` pass. Runtime macro and runtime archives package and compile from their unpacked sources. Commit signature is `G` with DCO. The unrelated user-owned `doc/ledger-adt.mdx` remains unstaged.

#### Remaining acceptance

The specific `counter-parameter.increment_by` fixture compiles with the feature, but the one-argument proof parity probe uses the nested `add_twice` fixture. Exact manual-versus-typed proof and sealed-byte equality under fixed randomness, malformed verifier bytes, a fresh external consumer invoking the new method using only the generated crate dependency, and a same-head live wallet call with ABI 19 remain open. The observed metadata is not authenticated finality; caller/node validation and concurrent state changes remain separate. Remote CI, clean tagged release provenance, registry publication and branch push remain open. No push occurred.


### Follow-up decision proposal — 2026-10-03: artifact access and byte parity

The ABI-19 call handle works inside the proof harness, but a fresh consumer with only the generated crate as a direct dependency can call `increment_call` yet cannot deserialize its emitted `.verifier` artifact through the facade: `VerifierKey` and the tagged decoder live in upstream crates. The first delivery compared complete prototype Debug output and proved the new handle; it did not compare two independently proved, sealed transactions byte for byte. Those gaps prevent closing #142.

Before:
```rust
// Requires an extra direct midnight-serialize dependency and upstream type import.
let verifier: VerifierKey = tagged_deserialize(&mut verifier_bytes.as_slice())?;
let call = contract.recording.increment_call(&observed, ())?
    .prepare(verifier, Fr::from(0))?;
```

Proposed after:
```rust
let verifier = contract_crate::runtime::transaction::decode_verifier_key(&verifier_bytes)?;
let call = contract.recording.increment_call(&observed, ())?
    .prepare(verifier, contract_crate::runtime::Field::from(0))?;
```

Runtime owns exact tagged verifier-key decoding and reexports the upstream `VerifierKey` type; the emitter still owns only circuit identity/input and does not gain parser logic. This is an additive public runtime contract change and therefore requires the next generated/runtime ABI. A separate temp consumer must depend only on the generated crate and invoke both the call handle and `prepare` against emitted proof artifacts. The proof harness must independently construct, prove and seal the manual and typed prototypes with identical network, TTL and RNG seeds, then compare tagged sealed bytes. Exact equality is useful evidence only with a deterministic pinned prover; both paths must also validate and apply to the ledger. No implementation, test result or commit is claimed in this proposal.


#### Evidence correction — 2026-10-03: nondeterministic proof bytes

The proposed exact manual-versus-typed sealed-byte comparison failed for `increment` even with identical seeds: the tagged pre-proof transaction bytes were identical, while 3,303-byte proven transactions first differed at offset 1,012. Inspection of the pinned upstream `midnight-proofs 0.7.0` source, `src/plonk/vanishing/prover.rs` `blind_quotient_limbs`, shows `F::random(OsRng)` at line 142. Independent PLONK proofs therefore need not be byte-identical under a seeded caller RNG. This corrects the earlier acceptance wording; a byte-equality requirement on independently generated proofs would test an upstream randomness accident, not the AST/runtime adapter.

The stronger applicable invariant is exact serialized pre-proof transaction bytes, followed by independent proof verification, ledger validation and application for both paths, with identical resulting contract state. The local Counter probe passed this corrected invariant (457 exact pre-proof bytes). The full gate and local commit are still pending at this point in the decision history.


### Delivery amendment — 2026-10-03, ABI 20

Local conventional GPG-verified/DCO commit `05d4b6aba00f1a7f0aa5381b340652d38164f120` closes the artifact-access and proof-parity research slice. ADR status remains **accepted-partial** because same-head live wallet admission, multi-argument call handles, remote CI and release gates remain open. The branch is local and unpushed.

#### Before and after for engineers

Before, a generated-crate-only consumer could obtain `RecordedCall` but had no facade decoder for its compiler-emitted verifier artifact:
```rust
// Requires extra direct midnight-serialize and midnight-transient-crypto imports.
let verifier: VerifierKey = tagged_deserialize(&mut artifact_bytes.as_slice())?;
let call = contract.recording.increment_call(&observed, ())?
    .prepare(verifier, Fr::from(0))?;
```

After, only the generated crate is a direct Cargo dependency:
```rust
let verifier = contract_crate::runtime::transaction::decode_verifier_key(&artifact_bytes)?;
let call = contract.recording.increment_call(&observed, ())?
    .prepare(verifier, contract_crate::runtime::Field::from(0))?;
```

Runtime owns exact tagged `VerifierKey` decoding, trailing-byte rejection and the upstream key type reexport. The AST emitter's call method remains unchanged; its generated/runtime ABI assertion advances 19→20 because the public runtime contract expanded. The private typed IR remains schema 8. A checked-in `consumers/observed_call.rs` is copied into a fresh temp Cargo consumer whose only direct dependency is the generated Counter crate. The counter-parameter fixture is added to the proof harness without manual output edits.

#### Exact evidence and revised parity invariant

The local `compactc --consumer --proof` gate passed separate consumers and **58** offline calls, each proved, verified, validated and applied with pinned ledger-8/zk dependencies. For `increment()`, `increment_by(BoundedUint<65535>)` and `add_twice(BoundedUint<65535>)`, the generated and manual paths have equal complete prototype Debug output, exact tagged pre-proof transaction bytes (457, 464 and 472 bytes respectively), independently valid proofs, and identical applied upstream `ContractState`. The first attempted exact proof-byte comparison found equal pre-proof bytes but differing 3,303-byte proven transactions at offset 1,012. Pinned `midnight-proofs 0.7.0` uses unconditional `OsRng` quotient blinding in `src/plonk/vanishing/prover.rs`; sealed proof-byte equality is therefore not a valid adapter invariant. This correction is recorded above rather than hidden.

The fresh consumer decoded the emitted `increment.verifier`, prepared the generated observed call, and rejected an artifact with trailing bytes. 57 renderer tests, 132/132 fresh fixtures, all-features workspace library compile, Rust formatting, checked-in consumer formatting, Python syntax, scoped diff and unpacked macro/runtime archive rehearsal passed. The ABI-20 package rehearsal used dirty local source and made no registry publication. The user-owned `doc/ledger-adt.mdx` remains unstaged.

#### Open limits

A same-head ABI-20 wallet/node/indexer call from an authenticated observation has not been rerun; the live ABI-18 history does not prove this new head. Observation metadata remains caller supplied and does not authenticate finality or protect against concurrent writers. Multi-argument FAB encoding, remote CI, clean signed release provenance, registry consumer and branch publication remain open. Issue #142 stays open in `rust-backend-v2` for those requirements.


### Delivery amendment — 2026-10-03, ABI-20 live wallet admission

Local conventional GPG-verified/DCO commit `8aa2f425a3b6e8470f1c78eeae223c7716f09d98` moves the live second-call builder from the manual adapter to the generated observed-state call API. The preceding ABI-20 emitter and runtime contract is unchanged (private IR schema 8). This is a delivery of the live acceptance slice, not closure of ADR-0043 or #142.

#### Problem and before/after

The earlier live example consumed indexed state but manually chose the circuit name and public input again. It also did not carry the transaction and block identity established by the wallet driver into Rust:

```rust
let contract: ContractState<DefaultDB> = tagged_deserialize(&mut state_bytes.as_slice())?;
let context = CircuitContext::from_contract_state((), address, &contract);
let recorded = counter_contract::Contract::default().recording.increment(context)?;
let call = prepare_call(recorded,
    CallSpec::new("increment", verifier, (), Fr::from(0_u64)))?;
```

The wallet driver now passes the indexed action's transaction hash, block hash and height after checking the exact final transaction and canonical finalized block. Rust decodes the exact upstream state and creates the typed handle:

```rust
let observation = Observation { transaction_hash, block_hash, block_height };
let observed = ObservedContractState::decode(address, &state_bytes, observation)?;
let verifier = decode_verifier_key(&fs::read(artifacts.join("keys/increment.verifier"))?)?;
let recorded_call = counter_contract::Contract::default()
    .recording.increment_call(&observed, ())?;
let call = recorded_call.prepare(verifier, Fr::from(0_u64))?;
```

#### Ownership and evidence

- Emitter: no change. Its ABI-20 `increment_call` method owns circuit identity and Unit input; the generated crate remains the call surface.
- Runtime: no change. `ObservedContractState`, exact `.verifier` decoder and `RecordedCall::prepare` reuse upstream ledger-8 state, installed verifier and replay/partition logic. The example retains local proof/validation and stale replay checks, projecting the observed upstream `ContractState` without claiming to reconstruct unrelated wallet balances or fees.
- Wallet driver/example: the driver supplies its checked indexer transaction/block metadata to the Rust example. The example validates hash syntax and height, then uses the generated method. No new prover, wallet or VM abstraction is introduced.

On a fresh pinned local node 0.22.3 / indexer 4.0.1 / proof server 8.0.3 stack, the rebuilt ABI-20 `compactc --consumer --proof` gate passed all **58** offline ledger proof/verification/validation/application calls. The wallet submitted the Rust deploy at indexed block 58 (`0db28e48f444410a65ca65ac5602450079df3f96d95e8d41490da596c3a7eb28`), first call at block 61 (`5bce9bd727c560cde1c4387325efb030cff46d15261c47bff5913f84bf648513`, `round = 1`), and generated observed-state second call at block 65 (`08b1982a957e018a824632a20d4caeef504ab4d0348e1880591b9abe17602018`, `round = 2`). The address was `85e623ca9ada2b6379b5ce1c929467474d7690c9b38e1bec78177bf92b589a2b`. A separate read-only check matched block 65 hash `3f59789e737e92eb7f0a4162433d840166d87d8885a0af9ee660863e62530eb3` against node `chain_getBlockHash(65)` under finalized head 69. The driver matched all three action hashes and blocks to finalized node observations.

`cargo build -p compact-rust-proof-smoke --example record_from_confirmed --locked`, `cargo test -p compact-rust-proof-smoke --test confirmed_state --locked --quiet` (2 passed), four wallet provenance tests, JavaScript syntax, Rust formatting and scoped diff checks passed. The user-owned `doc/ledger-adt.mdx` remained unstaged.

#### Remaining limits

Observation metadata is caller supplied. The driver checks it against the connected indexer and node; this is a trusted-node boundary, not a cryptographic state/finality proof. A concurrent state transition can invalidate the prepared call before admission. Multi-argument FAB encoding and generated handles, clean remote CI, clean signed release provenance, registry publication and branch publication remain open. #142 stays open in `rust-backend-v2`; no push or publication occurred.


### ABI-28 live generated-call confirmation — 2026-10-04

The generated Counter `increment_call` path was retested at signed/DCO `e75f13ba` with ABI 28/schema 8 on a fresh pinned funded local stack. Its current-head `record_from_confirmed` example consumed indexed state and observation metadata at `round = 1`, prepared/proved the typed call from emitted artifacts, and the wallet submitted it. The second call indexed at block 13 with `round = 2`; exact transaction and canonical/finalized block checks passed. See [ABI-28 live wallet admission — 2026-10-04](references.md#private-note-01). This is validation of the already emitted API, not an emitter/runtime change. Generalized state races, indexer trust, remote CI and published runtime remain #142 exit work.


### Generated-crate design probe reassessment — 2026-10-04

A fresh developer-facing probe independently recommended typed call methods that retain circuit identity and input, an observed-state wrapper, runtime verifier matching, and a small path from recorded trace to ledger call. Those are the accepted ABI-19/20 decisions above, rather than a new ADR: the current `render_observed_call_method` emits `increment_call(&observed, private)` and parameterized siblings from typed IR; `RecordedCall` retains the entry point, input and observation, while `prepare` checks address, initial state and installed verifier before delegating to `prepare_call`. The exact tagged verifier decoder is already in `runtime-rs/src/transaction.rs`. The ABI-28 live Counter second call used this path. ADR-0044 / [#143](https://github.com/MediaNoxLabs/compact/issues/143) separately records multi-parameter `AlignedValue::concat` and twelve-Field proof evidence. The independent probe's suggestion that this layer was still absent reflected the low-level `RecordedCircuitResult` and manual portions of the example, not the generated `RecordedCall` surface.

Before for a developer choosing the low-level path (still available):

```rust
let context = CircuitContext::from_contract_state(private, address, &state);
let recorded = contract.recording.increment(context)?;
let call = prepare_call(recorded, CallSpec::new("increment", verifier, (), randomness))?;
```

Current generated path:

```rust
let observed = ObservedContractState::decode(address, &state_bytes, observation)?;
let verifier = contract_crate::runtime::transaction::decode_verifier_key(&artifact_bytes)?;
let call = contract.recording.increment_call(&observed, private)?
    .prepare(verifier, randomness)?;
```

This reassessment changes no emitter, runtime, macro, derived implementation, ledger-8/zk mapping, private IR schema or ABI. It does not warrant a new issue: focused [#142](https://github.com/MediaNoxLabs/compact/issues/142) is already open in `rust-backend-v2`; [#143](https://github.com/MediaNoxLabs/compact/issues/143) owns broader input parity. A proposed `single_call(...).prove(...).seal(...)` convenience layer remains unaccepted: it needs a concrete repeated-code example, clear wallet/prover policy boundary and a before/after consumer gate before an additive ADR/issue. The current API's production limits remain caller-supplied observation trust, concurrent state changes, broader cross-contract evidence, remote CI and registry publication. No new commit or delivery is claimed by this note.



#### Review linkage — 2026-10-04

The generated-crate design-probe reconciliation above is also recorded in focused [#142 comment](https://github.com/MediaNoxLabs/compact/issues/142#issuecomment-5976232169). It is a review of the existing ABI-28 API, with no new commit, ABI/schema change, test claim, or issue closure.
