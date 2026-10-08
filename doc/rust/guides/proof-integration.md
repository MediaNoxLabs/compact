# From local execution to proof and ledger application

**API reference.** These excerpts describe integration points and prerequisites; they are not a standalone proof tutorial. The guide links existing maintained integration owners and preserves their evidence limits. It adds no transaction ownership, cross-instance or consumer-security assurance.

## Separate the stages

| Stage | Inputs and owner | What to check |
|---|---|---|
| Native/recorded call | Generated circuit, typed arguments, witnesses and context | Exact source rules; output/state/effects; available recorded API |
| Local replay | `RecordedCircuitResult::replay()` or `ContractLab::recorded(...)` | Actual VM replay of the sealed program from the original prestate |
| Observed call | Generated `*_call` facade and `ObservedContractState` | Explicit state/address/entrypoint/input binding; trust in observation comes from its supplier |
| Preparation | `RecordedCall::prepare(...)` and runtime transaction adapters | Matching verifier/material, supplied commitment randomness and supported offer policy |
| Proof | Existing provider/material pipeline | Actual proof creation and independent verification |
| Ledger application | Pinned ledger validation/application harness | Appropriate funding and strict rules; accepted applied state |
| Network | Separate wallet/node integration | Actual submission/observation on the selected network |

Enable the generated crate's `ledger-transaction` feature for observed-call/preparation APIs. Native default already has crypto/proof dependencies; default features are not a promise of a lightweight or transaction-ready build.

Inspect the actual generated schema3 report first. It declares per-export native/recorded/observed APIs and compiler proof applicability; it is not a receipt that a particular call succeeded. A pure passport export does not acquire a proof by enabling this feature.

The existing proof harness uses this shape (excerpt, prerequisites omitted):

```rust
let prepared = binding.apply_call(observed, private_state, point, scalar)?
    .prepare(verifier.clone(), commitment_randomness)?;
```

Names/types here stand for the particular generated circuit and caller-owned inputs. This fence is not part of the runnable snippet set. Use complete maintained code under `tools/compact-rust-proof-smoke/src/` for the actual provider, proof and ledger transaction lifecycle; DID and generic Jubjub gates retain their own source/material/result receipts.

`runtime-rs/src/transaction/` owns observation, preparation, offers and transaction adapters. Supplied observations and local hashes do not establish chain finality. Constructor-derived data can be deployed without a proof of constructor execution. Actual circuit proof/application evidence must retain this distinction.

The approved compatibility policy supports `ledger-transaction` within exact recorded/proof evidence on native ledger8.0.3. It does not qualify network deployment, arbitrary funded applications, browser proving or every possible composition. ADR0285/#409 subsequently passed its scoped external-consumer controls at4c8aebc3; the candidate and audit dispositions are recorded separately in [candidate qualification](candidate-qualification.md).

## Admit encoded state and verifier inputs

The `transaction` module exposes `EncodedSizeLimit::new(max_bytes)`, `ObservedContractState::decode_with_limit(address, bytes, observation, limit)` and `decode_verifier_key_with_limit(bytes, limit)`. Use `EncodedSizeLimit::default()` for the generous **64 MiB** opt-in preset, or `EncodedSizeLimit::new(max_bytes)` for a deployment-specific limit. `max_bytes()` exposes the selected budget. The preset has substantial headroom above the retained ledger8 state/verifier corpus; it is not a ledger protocol maximum. Larger legitimate states can use an explicit override. Apply transport acquisition limits before allocating the input slice. Legacy decoders do not automatically acquire this policy.

Oversized input returns `io::ErrorKind::InvalidData` before upstream parsing, reporting only input length and the selected limit. A value exactly at the limit still undergoes normal tagged decoding. Malformed tags, truncated encodings and trailing bytes remain errors. Limited refusal never retries through the legacy helper. The original `decode` and `decode_verifier_key` APIs remain available with their inherited upstream behavior.

This policy bounds encoded bytes admitted to the decoder. It does not bound total decoded heap, object count or CPU work; the pinned ledger serializer exposes no per-call budget for those resources. It also does not authenticate state provenance or a verifier key. Keep resource admission, provenance and proof verification as separate integration obligations.

The maintained `runtime-rs/tests/transaction_decoding.rs` corpus uses real serialized state and a deterministic verifier carrier to exercise these APIs. Its passing tests establish the named codec/admission cases, not proof validity or universal decoder containment. ADR0353 records byte admission; ADR0359 and CoPS-001 record the64MiB preset and the owner-approved0.3.0 scope: aggregate decoded heap/object/CPU containment is deferred. This is an explicit limitation, not a claim that total containment is implemented.

## Convenient local test setup

The testkit now supplies [ProofLab](local-proof-tests.md) behind its optional `proof` feature. It owns official Resolver and ParamsProverProvider implementations and returns the official LocalProvingProvider. Use its upstream ProvingProvider methods directly; the testkit does not replace ledger/zk domain objects. Existing ContractLab remains the generated execution/replay runner. ADR0360/#497 records this narrow local test slice; broader client and network orchestration stays separate.
