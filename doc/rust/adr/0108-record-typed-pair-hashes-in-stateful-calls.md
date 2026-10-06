---
id: RUST-ADR-0108
alias: ADR-0108
title: "Record typed pair hashes in stateful calls"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "pure-helpers", "typed-hash"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 6e0b0259ea210e9dc24a3564bc971c4eb003d17d6ee08f069d3c08f785ddd3ae
---
# RUST-ADR-0108 — Record typed pair hashes in stateful calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted typed one-argument pair/vector pure hash helper calls with once-only argument evaluation, while preserving the zero-argument path and Vector3 refusal. Three original APIs have TS parity and representative proof cases; the historical conditional capture is not evidence for every branch.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#212 closure](https://github.com/MediaNoxLabs/compact/issues/212#issuecomment-6017589960). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`54d3a8c3`](https://github.com/MediaNoxLabs/compact/commit/54d3a8c3409290c6eed5d2d23b8e9d4bbac49d0d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 108
status: proposed
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/212
```

## Historical decision and amendments

### Problem
The schema-11 Rust target executes impureBare and impureInIfArm, but recording stops in internal storeVec at a Field Let containing pure sumVec(v). ADR-0105 permits only zero-argument closed pair-hash helpers. The same typed pure-call shape blocks pureFromImpure.

### Before and after generated Rust
Before: `ledger_contract::impureBare(context)?` executes, while `ledger_contract::recorded::impureBare(context)` and `Contract.impureBare_call(&observed, ())` are absent. After: the recorder binds the exact `Vector<2, Field>` argument once, invokes `pure_circuits::sumVec(vector)?` in the internal recorded helper, writes the resulting Field through `ledger_slots::fieldCell.record_write`, and exposes recorded and observed APIs. For impureInIfArm, the condition is recorded first and only the selected helper call is emitted. The direct pureFromImpure Field Let uses the same typed pure call.

### Decision
Extend the transitive closed pair-hash guard to a pure Field callee with exactly one declared two-Field vector/tuple parameter. At the Field Let, lower one actual argument with the existing typed Cell source; bind it once with the declared Rust type, then call the generated pure method. Preserve the zero-argument path from ADR-0105 and structured unsupported reasons for mismatched/dynamic arguments, ledger-reading value callees, witness effects and Boolean calls. Use existing RecordingFrame, typed Cell slots and runtime::transient_hash. No IR/schema/runtime ABI change.

### Acceptance
Compare TS/native/recorded full state, ordered VM, four gas dimensions, private state and FAB for impureBare, impureInIfArm and pureFromImpure. Prove/verify/validate/apply representative observed calls with pinned ZKIR 2.1.0 and ledger-8. Run local renderer, fixture, inventory, rejection and format checks; signed GPG+DCO commit, no push or remote CI.

### Tracking
Issue https://github.com/MediaNoxLabs/compact/issues/212 in rust-backend-v2. Branch codex/adr108-call-arg-actions, base 54d3a8c3.

### Local delivery, 2026-10-05
The exact schema-11 compiler at root base 54d3a8c3 plus the local Rust emitter reports pureFromImpure, impureBare and impureInIfArm as proof-required recorded/observed; impureConst, inlinedAssert and witnessBare remain unavailable with precise existing gaps. The full 164-compiled-source inventory advances exactly three, 231/303 to 234/303 proof-required available, 72 to 69 known gaps, no unmatched compiler circuits. TypeScript/native/recorded full state, ordered VM, per-query and reported four-dimensional gas, private/FAB agree for all three; impureInIfArm records its flag read before the selected helper Cell write. Typed observed calls for impureBare and impureInIfArm each matched manual replay, then proved, verified, validated and applied through ledger-8 with pinned ZKIR 2.1.0 artifacts. Renderer 83/83, generated fixture 5/5, all 143 fixtures fresh, four-source strict rejection gate, cargo fmt and targeted Clippy passed. The strict negative oracle now uses Vector<3,Field> to preserve a real unsupported typed hash boundary. Root exact-head combined gate remains after cherry-pick; no push or remote CI.

Signed GPG+DCO local commit: b2c82dfeef493db8d2077f340247380b4f1cb168 on codex/adr108-call-arg-actions (base 54d3a8c3). Ready for root cherry-pick; MediaNoxLabs issue #212 is attached to rust-backend-v2.
