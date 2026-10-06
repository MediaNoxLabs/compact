---
id: RUST-ADR-0020
alias: ADR-0020
title: "Preserve ledger-8 per-query gas limits"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "witnesses", "gas", "gas-policy"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: aceca2a2847ce1f8cb0badc16f9be4dc2d9cce77eed7a467d75abeb6c56266bd
---
# RUST-ADR-0020 — Preserve ledger-8 per-query gas limits

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept the existing ledger 8 per-query meaning of gas_limit and separately accumulated observed costs. The evidence resolves an ambiguous proposed cumulative-budget requirement without changing the runtime API. A whole-circuit budget would require a separate decision; milestone closure does not imply such a budget exists.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#121 closure](https://github.com/MediaNoxLabs/compact/issues/121#issuecomment-6017433641). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`7c489892`](https://github.com/MediaNoxLabs/compact/commit/7c4898929a7c9ab8bc357d844a1843c5531129c8). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 20
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 121
```

## Historical decision and amendments

### Problem and evidence

The ABI-10 `WitnessReadMeter` forwards `CircuitContext.gas_limit` to each ledger read while accumulating observed gas separately. The M2 delivery map calls cumulative gas-limit semantics an open gap. This wording risks changing Rust behavior without checking the ledger-8 contract. TypeScript `runtime/src/circuit-context.ts::queryLedgerState` passes `circuitContext.gasLimit` to each `currentQueryContext.query`; the matching `midnight-onchain-runtime 3.0.0` `QueryContext::query` passes that limit to one VM program and checks that query's total. Generated witness projections and ordinary circuit queries both use this path. The limit is therefore a per-query execution guard in the current ledger-8 APIs, while `CircuitResult.gas_cost` is the sum of queries for diagnostics.

### Before and after Rust

Before, the code behaves per query but the generated crate's contract is undocumented and untested:

```rust
let mut context = initial_state(...).into_circuit_context(address);
context.gas_limit = Some(one_read_cost);
let result = write_twice(context, &witnesses, seed)?;
// No assertion explains why two accepted reads may sum above one_read_cost.
```

After, a focused test and runtime documentation make the behavior reviewable:

```rust
let mut context = initial_state(...).into_circuit_context(address);
context.gas_limit = Some(one_query_limit);
let result = write_twice(context, &witnesses, seed)?;
assert!(result.gas_cost.read_time > one_query_limit.read_time);
```

A second case sets a limit below one query and expects `CompactError::LedgerQueryRejected`. The test should cover repeated witness reads and a circuit query under the same configured limit. The public `gas_limit` field and generated method signatures do not change.

### Decision and ownership

Preserve the upstream per-query meaning for `CircuitContext.gas_limit` and `WitnessReadMeter`. The runtime owns forwarding the unchanged limit to each canonical ledger VM query and summing observed costs. The emitter needs no new IR node or transformation; generated witness methods keep using the meter. The runtime README and focused tests make the boundary explicit. Generated/runtime ABI remains 10 and private IR schema remains 8.

If applications need a whole-circuit budget, specify a separate budget API with explicit treatment of witness reads, public queries, writes, failed calls, recording/replay, and all four cost dimensions. Do not silently reinterpret the existing ledger-8 per-query field.

### Alternatives and risks

Subtracting observed gas from the current limit before each query would make Rust reject circuits accepted by TypeScript/ledger-8, and the upstream `RunningCost` has no subtraction API. Checking accumulated gas only at finish would permit side effects before rejection and require new failure/rollback semantics. A blanket policy based on `RunningCost::Ord` is also suspect because its derived ordering is lexicographic while the VM checks read/compute dimensions during execution. A separate budget design needs its own ADR and parity/transaction evidence.

The main risk is that callers interpret `gas_limit` as a total budget. Documentation must name the precise scope; production transaction admission remains the ledger's own replay/partition/validation gate.

### Verification and delivery gate

- Record upstream and TypeScript source references in the issue.
- Test one-query acceptance, repeated-read aggregate above the limit, and one-query rejection through `TryWitnesses`, with exact observed gas.
- Run focused and runtime tests, fixture freshness, and a relevant external consumer/proof gate if behavior changes.
- Append the signed/DCO commit and evidence to this ADR, the issue, and [Milestone 2 — ADR delivery map](references.md#private-note-08). Do not claim remote CI or release from local tests.

### Delivery amendment — 2026-10-02

Local conventional GPG-signed/DCO commit `7c4898929a7c9ab8bc357d844a1843c5531129c8` documents the existing per-query contract in `CircuitContext` and the runtime README. A focused generated-crate test measures one canonical Cell read, applies exactly that per-query read/compute limit, accepts two witness reads in one fallible frame, and asserts the observed total is twice the single-query cost and above the configured read-time limit. It then accepts two ordinary circuit reads with the same limit, and rejects a fallible generated witness when the read-time limit is zero. All 10 `witness-cell-write` fixture tests and the `midnight-compact-runtime` suite pass; `cargo fmt --all --check` and scoped `git diff --check` pass. The generated ABI stays 10 and private IR schema stays 8. No emitter or VM behavior changed.

This resolves the ambiguous cumulative-gas item as a ledger-8 compatibility decision; it does not assert that a whole-circuit admission budget exists. A separate API and ADR would be needed for that. Branch publication, remote CI, registry release and wallet/node exit remain open. Tracking: [#121](https://github.com/MediaNoxLabs/compact/issues/121), parent [#116](https://github.com/MediaNoxLabs/compact/issues/116) and [#104](https://github.com/MediaNoxLabs/compact/issues/104).
