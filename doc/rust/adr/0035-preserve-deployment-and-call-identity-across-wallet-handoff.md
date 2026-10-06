---
id: RUST-ADR-0035
alias: ADR-0035
title: "Preserve deployment and call identity across wallet handoff"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["wallet", "transactions", "observations"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: c59fe962d2c83a9763710346087dad52908b30fc0d3b55ffb647188bb7e797c4
---
# RUST-ADR-0035 — Preserve deployment and call identity across wallet handoff

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept exporting and checking deployment and subsequent call as separate sealed upstream transactions with one contract identity. Preserve the correction that the exported deployment is the one applied. Local decode/roundtrip and offline application do not themselves establish network TTL, wallet balancing or confirmed state.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#134 closure](https://github.com/MediaNoxLabs/compact/issues/134#issuecomment-6017456472). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`606a9b3a`](https://github.com/MediaNoxLabs/compact/commit/606a9b3a89214a18f331ebefff9cd0237593e33d) · [`ed05c0d8`](https://github.com/MediaNoxLabs/compact/commit/ed05c0d88d21ca6c9cb01663b4d7c30fa41b7a3b). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 35
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/134
```

## Historical decision and amendments

- Status: Proposed (2026-10-03)
- Milestone: rust-backend-v2
- Parent: MediaNoxLabs/compact #105
- Related: #132
- Focused issue: pending

### Problem statement

The proof smoke currently validates a generated-contract deployment, inserts its state directly into an offline ledger, then proves and exports only the subsequent call. A wallet receiving only the call bytes cannot deploy the referenced address. The Rust and JavaScript ledger-v8 packages must agree on the address and transaction markers for both stages. A byte round-trip alone cannot prove that a call targets the exported deployment.

### Before

```rust
let deploy_tx = Transaction::from_intents("local-test", deploy_intents);
deploy_tx.well_formed(&empty_ledger, relaxed, Timestamp::from_secs(0))?;
ledger.contract = ledger.contract.insert(address, deploy.initial_state);
let proven_call = call_tx.prove(provider, cost_model).await?;
write_call_only(proven_call.seal(rng));
```

The direct insert skips deployment application; only call bytes cross the wallet boundary.

### Decision and after

Keep transaction assembly and encoding in midnight-ledger and midnight-serialize, outside generated contract code. Export a separately sealed deployment transaction beside the already proven call. Validate and apply the deployment through ledger semantics before validating/applying the call. Deserialize both with the pinned JavaScript ledger-v8 package, require exactly one deploy and one call, assert identical contract addresses, and byte-for-byte reserialize each.

```rust
let deploy_proven = deploy_tx.prove(provider, cost_model).await?;
let deploy_sealed = deploy_proven.seal(deploy_rng);
write_tagged(deploy_path, &deploy_sealed)?;
let (deployed, outcome) = empty_ledger.apply(&deploy_verified, &context);
ensure_success(outcome)?;
let call_proven = call_tx.prove(provider, cost_model).await?;
write_tagged(call_path, &call_proven.seal(call_rng))?;
```

```ts
const deploy = ledger.Transaction.deserialize('signature', 'proof', 'binding', deployBytes);
const call = ledger.Transaction.deserialize('signature', 'proof', 'binding', callBytes);
const deployAction = deploy.intents.get(1).actions[0];
const callAction = call.intents.get(1).actions[0];
if (deployAction.address !== callAction.address) throw new Error('address mismatch');
```

The exact JS action access and serialization checks will be confirmed against @midnight-ntwrk/ledger-v8@8.0.2 before delivery. These are two sequential offline transactions, not a single atomic deployment and call. A wallet must balance, finalize, validate, submit, and observe the deployment before building or submitting a call against its confirmed state.

### Ownership and compatibility

- Emitter: no change; typed recording and generated constructor stay intact.
- Runtime: no wrapper or ABI change; prepare_call remains the Rust ledger adapter.
- Integration harness: creates canonical ledger transactions, proves/seals both, checks both Rust and JS decoding, state transition and address linkage.
- Wallet/node: application owns network ID, current TTL, funds/keys, fee balancing, final validation and submission. The offline fixture remains local-test with timestamp zero and balancing disabled.
- Schema 8 and ABI 15 remain unchanged. Pin both ledger implementations to 8.0.2.

### Alternatives and risks

A custom JSON envelope duplicates ledger types and can hide a changed address. Combining deploy and call in one transaction would alter sequencing and needs separate ledger proof before adoption. Directly mutating LedgerState in the smoke bypasses the deployment path we need to exercise. Serialized offline transactions still do not establish wallet/node admission or live TTL compatibility.

### Acceptance

1. Deployment and call each pass Rust proof/validation, sealed tagged round-trip, pinned JS decoder and exact byte round-trip.
2. Their sole actions have the same address; changing either transaction or swapping the pair fails the handoff check.
3. Deployment is applied through LedgerState::apply and the call is validated/applied against that result, with the expected counter state.
4. Reproducible commands and wallet sequencing are documented; #105 remains open until live balanced/current-TTL submission, remote CI and release evidence.

### Evidence and limits

Proposed from local midnight-ledger 8.0.2 prove/seal, ContractDeploy::address, Transaction::deploys/calls, and pinned JS ledger-v8 8.0.2 ContractDeploy/ContractCall types. Delivery evidence will be appended without rewriting this proposal.


### Issue assignment — 2026-10-03

Focused issue [#134](https://github.com/MediaNoxLabs/compact/issues/134) is assigned to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2). Related call-only boundary [#132](https://github.com/MediaNoxLabs/compact/issues/132) remains open for live wallet/node work. Delivery evidence will be appended below.


### Local delivery amendment — 2026-10-03

Local conventional GPG-verified/DCO commit `606a9b3a89214a18f331ebefff9cd0237593e33d` implements the paired offline handoff under [#134](https://github.com/MediaNoxLabs/compact/issues/134). The counter fixture now proves and seals a separate deploy and the recorded call with midnight-ledger 8.0.2. The exact sealed deploy exported to disk is validated and applied through `LedgerState::apply`; the subsequent call is validated and applied against that result. For the other 56 offline fixture calls, the validated unproven deployment is applied through ledger semantics instead of direct state insertion. Both exported transaction bytes round-trip through Rust tagged serialization and the pinned JavaScript `@midnight-ntwrk/ledger-v8@8.0.2` decoder. JavaScript sees exactly one `ContractDeploy` and one `ContractCall` in segment 1, matching address `85e623ca9ada2b6379b5ce1c929467474d7690c9b38e1bec78177bf92b589a2b`. The exported lengths are 1,765 and 3,372 bytes. A swapped pair fails the action-type check; a decoded deployment whose bytes were altered to change its address fails the address check.

Verification: `cargo check -p compact-rust-proof-smoke --locked`; `cargo fmt --all -- --check`; packaged `check_compactc_target.py --proof` with both handoff env paths (57 offline calls); pinned Node paired decoder; `node --check`; Ruby YAML parse; scoped `git diff --check`. The first gate passed, then a final gate passed again after ensuring the sealed exported deploy itself is the transaction applied. Remote CI, fresh external consumer, actual network/TTL, wallet fee balancing/signatures and node observation have **not** been validated; branch remains local/unpushed. Runtime/emitter/schema/ABI unchanged. The earlier `--consumer` proof gate passed at `ed05c0d8`; a fresh separate consumer rebuild was deferred because its target directory approached local disk capacity.
