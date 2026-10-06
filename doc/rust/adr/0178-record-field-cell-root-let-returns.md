---
id: RUST-ADR-0178
alias: ADR-0178
title: "Record Field Cell root Let returns"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Field", "Cell"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: c06d921b26a1ed0d4ecdaad33f7fc00dc1c12c41e4dc36f9a35377cf3d5387eb
---
# RUST-ADR-0178 — Record Field Cell root Let returns

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Bounded Field Cell root-Let returns use shared typed scopes, exact ordered read/write and one evaluation per binding. This closes a narrow recording profile, not arbitrary Field state expression admission.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#282 closure](https://github.com/MediaNoxLabs/compact/issues/282#issuecomment-6017708365). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`2982d602`](https://github.com/MediaNoxLabs/compact/commit/2982d60299d44d175da45c24249f0bc186a22084) · [`c4017bab`](https://github.com/MediaNoxLabs/compact/commit/c4017bab141d457672f529c96efcc05ed1e7a7e6). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
type: adr
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem
ADR-0169 made `root_let_action_return_oracle.step` execute natively with correct lexical scope, but the compiler still reports its proof-required recorded/observed APIs unavailable. A read-bound Field local is incremented and written before an independent parameter is returned.

### Decision — ADR-0178
Extend the existing typed recording plan for a bounded Field Cell/lexical return slice. Reuse typed Cell slots, RecordingFrame, and upstream Field arithmetic. Preserve one evaluation per binding and the single ordered read/write trace; do not add a contract-name switch or duplicate VM program. No private schema or runtime ABI change is expected.

Before:
```rust
let result = ledger_contract::step(context, echo)?;
// No step_recorded / step_call API.
```

After (proposed API):
```rust
let recorded = ledger_contract::step_recorded(context, echo)?;
let observed_call = observed.step_call(echo)?;
```
Exact generated signatures will be documented on delivery.

### Acceptance
- Typed admission checks for Field parameters/result and the bounded Field Cell plan; reject malformed slot/type/scope and unsupported effects.
- Evaluate nested bindings once, preserve lexical visibility, return the input echo after the write.
- Independent TS/native/recorded comparison: two successive calls, result, serialized state, effects, exact program, per-query gas, replay, ordered private data.
- Use actual summed TS query gas; document the existing TS reportedGas omission for this shape.
- Strict Rust recording compile succeeds; preserve welcome/election/bboard capabilities.
- Generate keys, prove, verify and ledger-apply a real recorded call, with the transaction funding policy stated.
- Focused local tests/Clippy and immutable receipt, conventional GPG/DCO delivery; no remote CI or push.

### Engineering history
Design record: midnight vault, `Initiatives/02 Compact Rust emission/ADRs/ADR-0178 — Record Field Cell root Let returns.md`.
This closes one known recording gap; it does not establish full Coracle support or production readiness.

### Implementation plan

The existing recorded/typed_plan.rs already owns typed scopes and lexical bindings. Extend its Field value/Cell support with a narrow explicit acceptance predicate and counters as needed. Ensure nested action locals do not escape into the final return. The current oracle returns the independent echo parameter, so retaining outer bindings must not accidentally select a shadowed or post-write value.

The runtime already supplies Field arithmetic and Cell recording; prefer no runtime API change. Unsupported ReturnPlan effects remain separate ADR-0174 work until independently audited.

### Delivery

Pending implementation, signed commit, exact receipt, and generated before/after excerpt.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/282 (rust-backend-v2).

### Accepted delivery — 2026-10-05

Signed isolated commit2010fc5f integrated as c4017bab; formatting/freshness commit2982d602. Both main commits verified GPG+DCO. ABI46/schema20 unchanged. Main focused receipt ${LOCAL_EVIDENCE}/compact-focused-2982d602/receipt.json passes four fixtures and8/8 recorded APIs (root Let, qualified Cell, qualified Set, welcome).

Actual developer-facing API:
```rust
let recorded = ledger_contract::recorded::step(context, echo)?;
let call = contract.recording.step_call(&observed, private_state, echo)?;
```
Typed plan owns Field literal/addition and scoped Field Cell read/write. Admission requires one Field argument/result and independent parameter return, one same-slot read/write, no extra qualified effects, no shadowing or leaked locals. Runtime APIs are reused unchanged.

Independent TS/native/recorded comparisons pass for two calls: echo9/13, stored0→1→2, exact public program, state, effects, summed query gas, private data and replay. TS aggregate reportedGas omits this shape's read; actual query gas is compared separately from replay gas.

Proof receipt ${LOCAL_EVIDENCE}/compact-integrated-adr178-proof.log: generated trace replay/partition, actual proof verification and ledger application pass under the shared unbalanced smoke policy. This is not funded strict acceptance. Targeted isolated Clippy and integrated backend tests pass. Negative tests retain slot/type/return/shadowing/extra-read/scope rejection. No push or remote CI.
