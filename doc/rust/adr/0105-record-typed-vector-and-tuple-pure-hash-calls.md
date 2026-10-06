---
id: RUST-ADR-0105
alias: ADR-0105
title: "Record typed vector and tuple pure hash calls"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "pure-helpers", "typed-hash"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 8164d3d1c56680879c339b085f7c003461e4847120ae6e409afe4b10aacb7b69
---
# RUST-ADR-0105 — Record typed vector and tuple pure hash calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted a separate closed typed pair/vector hash-call domain, preserving the earlier scalar-only guard. Three original APIs gained recording and TS parity; only the named bridgeTupleIntoVec case supplies the representative proof/application evidence.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#208 closure](https://github.com/MediaNoxLabs/compact/issues/208#issuecomment-6017582704). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`4c462baf`](https://github.com/MediaNoxLabs/compact/commit/4c462baf4baf7f525426ab51ceb92e3eb0292a8d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 105
status: proposed
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/208
```

## Historical decision and amendments

### Problem
`call_arg_declared_type.compact` executes `pureBodyVec`, `bridgeTupleIntoVec`, and `bridgeVecIntoTuple` natively, but each proof-required export lacks recorded and observed APIs at a Field Let binding. The transitive pure bodies hash a two-element Field vector or tuple; two bodies bridge tuple/vector types with explicit IR coercions. The existing scalar-only pure-call guard correctly rejects them.

### Before and after generated Rust
Before: `ledger_contract::pureBodyVec(context)?` is available, while `ledger_contract::recorded::pureBodyVec(context)` is absent. After: both are generated; `let recorded = ledger_contract::recorded::pureBodyVec(context)?;` yields the same Field Cell write and replayable frame. `contract.recording().pureBodyVec_call(...)` gains the typed observed API. The same applies to both bridge exports.

### Decision
Add a separate transitive, type-checked pure hash guard for exactly zero-argument Field results built from closed two-element Field vector/tuple literals, shape-preserving local bindings and tuple/vector coercions, matching pure calls, and transientHash. Invoke the already generated pure function once at the Let binding, then record the existing Cell write. Reuse runtime::transient_hash and ledger-8 recording primitives. Preserve the scalar-only guard and structured unsupported gaps for dynamic, witnessed, ledger-reading, or unknown pure bodies. No schema or runtime ABI change.

### Acceptance
Check TS/native/recorded state, ordered VM, four gas dimensions, private state, and FAB; prove/verify/validate/apply a representative call with pinned ledger-8 tools. Run local focused renderer, fixture, inventory and freshness checks, signed GPG+DCO commit. No remote CI or push.

### Tracking
Issue: https://github.com/MediaNoxLabs/compact/issues/208. Branch: codex/adr105-vector-pure-call-bridge.

### Local delivery, 2026-10-05
The schema-11 local compactc wrapper reports pureBodyVec, bridgeTupleIntoVec, and bridgeVecIntoTuple as proof-required with recorded and observed APIs; the source-local inventory advances from 5/16 to 8/16, and the 191-source full inventory reports 224/296 available, 72 known gaps, 342 nonproof, no unmatched compiler circuits. TS/native/recorded full serialized ledger state, ordered public VM, per-query and reported four-dimensional gas, zero private outputs, and FAB agree for all three. The representative bridgeTupleIntoVec typed observed call matches the manually recorded call, then proves, verifies, validates, and applies against ledger-8 using pinned ZKIR 2.1.0 artifacts. Renderer 80/80, generated declared-type fixture 5/5, 141/141 fixture freshness, four rejection sources, cargo fmt, and targeted Clippy all passed. The exact-head combined gate remains with the root integration lane; no push or remote CI.

Signed GPG+DCO local commit: 40917de2f42b11a8c954fdfc46fe85047c283e66 on codex/adr105-vector-pure-call-bridge (base 4c462baf). The branch is ready for root cherry-pick; milestone issue #208 is attached to rust-backend-v2.
