---
id: RUST-ADR-0010
alias: ADR-0010
title: "Report total Rust circuit gas across ledger queries"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "witnesses", "gas", "gas-policy"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 9d8e2e7e4e3fded759c5233d473198bfb5f65968ed2253c52491a274945f73df
---
# RUST-ADR-0010 — Report total Rust circuit gas across ledger queries

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept Rust circuit gas as the sum of executed VM query costs, compared with independently captured TypeScript query sums. Keep the TypeScript wrapper's last-query cost and replay/partition gas distinct. This is an explicit reporting policy and sampled equality evidence, not a change to the upstream gas model or a whole-circuit budget.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#104 closure](https://github.com/MediaNoxLabs/compact/issues/104#issuecomment-6017403902). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`3799746a`](https://github.com/MediaNoxLabs/compact/commit/3799746acec6e5898e5e7b121621045d8dac925a). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 10
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: "https://github.com/MediaNoxLabs/compact/issues/104"
```

## Historical decision and amendments

### Problem

The `tiny` Rust generated crate matches TypeScript state bytes and `get` result, but its `CircuitResult.gas_cost` looked much larger than the TypeScript circuit's `gasCost`. For `clear`, TypeScript reports `readTime=85000000` and `computeTime=1233942932`; Rust reports `595000000` and `6201659012`. Comparing those values directly would misdiagnose a VM or cost-model divergence. Issue #104 requires an explicit gas and VM transcript decision, including the `tiny` query-boundary discrepancy.

### Before

```typescript
// runtime/src/circuit-context.ts — each query replaces the wrapper field.
circuitContext['gasCost'] = res.gasCost;
// Generated wrapper returns context.gasCost after the entire circuit.
```

```rust
// Generated tiny Rust already adds each query cost.
total_cost += step.gas_cost;
// Three writes, plus the reads before them, contribute to the circuit result.
```

The earlier vault description conflated the `codegen-rust` Rust oracle with the TypeScript oracle. The `codegen-rust` generated Rust constructor batches its three assignments in one `OpProgramVerify` query. Current compiler 0.31.133 TypeScript output invokes `queryLedgerState` separately for each constructor assignment and circuit ledger operation; its constructor also issues three initial scaffold writes. The local AST Rust constructor uses three typed writes on an already initialized state. These are distinct query topologies and must be compared separately.

### Decision and after

Keep generated Rust `CircuitResult.gas_cost` as the sum of every VM query executed by that circuit. Capture each TypeScript `QueryContext.query` cost and compare its sum with Rust, rather than comparing with the wrapper's last-query `gasCost`. Preserve the full normalized TypeScript public transcript for later operation-by-operation comparison.

```rust
let total: u64 = ts_queries.iter().map(|query| query.gas_cost.read_time).sum();
assert_eq!(rust_clear.gas_cost.read_time, total);
```

The executing `tiny` test now checks all four cost dimensions and private-output counts for `clear`, `set`, and `get`. Measured TypeScript per-query sums match Rust exactly:

| Circuit | Queries | Read time | Compute time | Bytes written | Bytes deleted |
|---|---:|---:|---:|---:|---:|
| `clear` | 5 | 595000000 | 6201659012 | 650 | 718 |
| `set` | 4 | 425000000 | 4951743649 | 850 | 782 |
| `get` | 2 | 340000000 | 2499829721 | 0 | 0 |

The TypeScript wrapper's reported costs equal only the last query: `clear` and `set` report read time 85000000, while `get` reports 170000000. The capture stores query costs and the full normalized public transcript in `runtime-rs/tests/fixtures/tiny-gas-oracle.json`; the reproducible driver is `tools/compact-rust-backend/oracles/tiny_gas_capture.mjs`.

### Alternatives and rationale

Changing Rust to return the last query cost would hide real VM work and understate multi-operation circuits. Batching local queries solely to mimic the `codegen-rust` Rust constructor would change failure boundaries and may diverge from current TypeScript circuit execution. The total-cost contract reflects the executed ledger work and preserves useful gas accounting for consumers. The TypeScript wrapper behavior is recorded as a compatibility fact; a future upstream fix could make direct whole-circuit values equal without changing Rust.

### Emitter and runtime ownership

No typed IR, emitter program, VM operation, generated API shape, or runtime ABI change in this slice. The existing stateful emitter adds individual `CircuitResult.gas_cost` values; `runtime-rs/src/context.rs` now documents that field. The capture/test owns the cross-language acceptance evidence. Full public transcript exposure from generated native circuits, constructor gas reporting, and a decision on batching versus per-statement execution remain separate follow-up work. If those change, amend this ADR with before/after generated Rust, the IR/emitter/runtime boundary, and proof impact.

### Verification and risks

Compiler 0.31.133 and matching JavaScript runtime 0.16.101 generated and ran `tiny_oracle.compact`. The capture is byte reproducible against the checked-in JSON. The targeted Rust fixture test passes; `cargo fmt --all -- --check`, Node syntax check, and `git diff --check` pass. The commit signature and Signed-off-by trailer were verified. The test compares per-query summed gas and private-output counts; it verifies the TypeScript transcript capture is internally consistent, but does **not** claim Rust versus TypeScript public transcript equality. The constructor query topology, gas limits, failed circuits, proof execution, wallet submission, and other fixtures remain open. The TypeScript prototype instrumentation is test-only and depends on the current runtime API.

### Tracking and delivery

- Issue: [MediaNoxLabs/compact#104](https://github.com/MediaNoxLabs/compact/issues/104).
- Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Local commit: conventional GPG-signed and DCO-signed `3799746acec6e5898e5e7b121621045d8dac925a`; branch remains local.
- Delivery state: local `tiny` gas evidence; transcript equality and production exit remain open.

### Amendments

Append dated revisions or write a superseding ADR if the cost accounting contract changes.

Issue delivery record: [#104 comment](https://github.com/MediaNoxLabs/compact/issues/104#issuecomment-5946385095). #104 remains open in `rust-backend-v2`.
