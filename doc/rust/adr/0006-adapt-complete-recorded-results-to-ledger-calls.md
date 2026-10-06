---
id: RUST-ADR-0006
alias: ADR-0006
title: "Adapt complete recorded results to ledger calls"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e14af55a22847825804b0fdeb6615e8f68af4f6427e5b5f68f7abc52e0a1f543
---
# RUST-ADR-0006 — Adapt complete recorded results to ledger calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept feature-gated preparation of complete recorded results using upstream replay, partition and state/effect checks. Consumers still own artifacts, transaction assembly and submission. Later Counter wallet evidence is a separate dated integration result; empty or inconsistent traces remain refused, and preparation alone is not consensus or funding validation.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#105 closure](https://github.com/MediaNoxLabs/compact/issues/105#issuecomment-6017405703). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`99aaeb1f`](https://github.com/MediaNoxLabs/compact/commit/99aaeb1fd32f583f0943dbad71c8515aac5a0a90) · [`aaece8fd`](https://github.com/MediaNoxLabs/compact/commit/aaece8fd387efda22777a4f3522585a27627c00e) · [`e75f13ba`](https://github.com/MediaNoxLabs/compact/commit/e75f13ba2fdc954a29114177c31994a63e3cbec2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 0006
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 105
```

## Historical decision and amendments

### Problem

The proof smoke previously reimplemented replay, partitioning, effect checks, and `ContractCallPrototype` construction. A consumer repeating that work could submit a trace whose effects disagree with native execution or attach the wrong verifier/input. Ledger transaction dependencies also increase the core runtime graph for pure and native-only consumers.

### Before and after

```rust
// Before: test-local transaction assembly (abbreviated).
let replay = recorded.public.initial().query(recorded.public.verify_ops(), None, &cost_model)?;
let (context, program) = recorded.public.into_parts();
let transcripts = partition_transcripts(&[PreTranscript { context, program, comm_comm: None }], &params)?;
let call = ContractCallPrototype { /* verifier, input, output, transcripts, ... */ };

// After: one runtime adapter with explicit artifact inputs.
let spec = CallSpec::new("increment", verifier_key, (), Fr::from(0u64));
let call = runtime::transaction::prepare_call(recorded, spec)?;
```

### Decision and ownership

Expose `transaction::prepare_call` only behind the runtime's `ledger-transaction` Cargo feature; generated crates forward it. The adapter rejects empty traces, replay failures, effects/state mismatches, and partition effects mismatches before constructing the ledger-8 call. `CallSpec` carries entry point, verifier key, public input, key location, and communication-commitment randomness. The caller still owns artifact loading and transaction submission. The generated circuit body stays independent of `midnight-ledger` construction types.

Emitter: only complete recorded methods are exposed. Runtime: `recording.rs` captures operations; `transaction.rs` validates and adapts them. Consumer: supplies artifacts and chooses a prover/wallet path.

### Evidence, consequences, and remaining work

Local commits `aaece8fd` and `99aaeb1f`. The packaged proof smoke uses the adapter to prove, validate, and apply Counter increment and Boolean Cell write/read calls offline. This is a reusable seam, not a complete wallet SDK. Add typed artifact discovery, parameterized public input parity, witness proofs, multi-call/rollback behavior, and a supported wallet/node submission example. Preserve the exact ledger-8 value-field communication commitment used by `Intent::add_call`.

### Tracking

- Issue: [M2 source-to-proof-to-ledger #105](https://github.com/MediaNoxLabs/compact/issues/105).
- Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Delivery: three offline calls local; end-to-end network submission open.


### Decision history

- 2026-10-02: Created ADR-0006 from the generated-crate research probes with status `accepted-partial`; linked MediaNoxLabs/compact#105 in `rust-backend-v2`. Local delivery evidence is recorded above. Future changes should append a dated amendment or link a superseding ADR.


### ABI-28 same-head local ledger admission — 2026-10-04

At local signed/DCO `e75f13ba2fdc954a29114177c31994a63e3cbec2`, exact-head packaged `compactc --target rust` emitted complete Counter ZKIR and keys. The current proof smoke produced a verified 2,912-byte Counter proof, locally applied the sealed deploy/call, and exported a 1,769-byte deploy plus 3,376-byte call for `undeployed` with a future TTL; pinned JS ledger-v8 8.0.3 byte-roundtripped both at one address. On a fresh pinned local stack, wallet facade 3.0.0 balanced/finalized/submitted both; indexer decoded `round = 1`. A second fresh run used the generated observed-state call builder to prove and submit a later call from finalized indexed `round = 1`, reaching `round = 2` at block 13. Full images, commands, hashes and trust limits: [ABI-28 live wallet admission — 2026-10-04](references.md#private-note-01). No emitter/runtime/ABI/schema edit in this validation slice. This advances parent #105 beyond the earlier ABI-20 admission, while public registry, remote CI, production wallet/finality trust and branch publication remain open.
