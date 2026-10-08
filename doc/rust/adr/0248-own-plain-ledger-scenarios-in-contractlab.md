---
id: RUST-ADR-0248
alias: ADR-0248
source_sha256: b79d7f96981e0ed02945442a885d9dfb65b03ffd4f826469a3eef85fcb9cc752
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0248 — Own plain-ledger scenarios in ContractLab

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** locally-delivered-first-slice. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: locally-delivered-first-slice
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/373
```

## ADR-0248 — Own plain-ledger scenarios in ContractLab

### Problem
Generated typed calls require repetitive context construction, VM replay, state/effect comparison and checkpoint management. Carrying the previous query into a new call can accidentally carry per-call effects. A developer-facing testkit should make successful commit and failed-call isolation explicit.

### Before and after
```rust
// Before: each test manually constructs, invokes, replays and compares contexts.
let result = counter::recorded::increment_by(context, amount)?;
// ... independently invoke QueryContext::query and compare public state/effects ...

// After: generated APIs remain typed, and the lab owns scenario lifecycle.
let checkpoint = lab.snapshot();
let report = lab.recorded(|ctx| counter::recorded::increment_by(ctx, amount))?;
lab.restore(&checkpoint)?;
```
Examples illustrate intended API; final receipt will record actual signatures.

### Decision and ownership
Add separate midnight-compact-testkit crate with ContractLab<Private>, DefaultDB, typed closure adapters, explicit identity/environment, in-memory versioned snapshots/forks, typed WitnessScript queues and redacted CallReport. Reuse generated native/recorded APIs and upstream QueryContext VM; no second interpreter or compiler dependency. Reconstruct each call from committed charged ledger/private state. Commit only after execution and requested replay validation succeed. A requested replay may never silently become native-only acceptance.

Snapshots check version, source/generated identities, runtime ABI, ledger version and mode before adoption. Caller-provided hashes prevent accidental mismatch but do not authenticate provenance. Private: Clone must own its state or use immutable persistence; shared interior mutation and external witness side effects cannot be rolled back. Script state/journal belongs inside private checkpoints, not a global mutable registry.

### Bounded first mode and threats
PlainLedger rejects nondefault circuit Zswap plans, nonempty local wallets and changed address/clock/identity/cost/gas policy. Constructor and result boundaries are both checked. Recorded calls must match their sealed start and replayed final public state/effects. Empty local replay is allowed without implying transaction preparation or proof readiness. Keep execution gas and replay gas distinct because witness reads may not appear in the public program. Default diagnostics omit private values/transcripts and generic outputs.

### Emitter/runtime impact
No emitted API, ABI or schema change is planned. Reuse the existing runtime and pinned ledger8 dependencies. The default dependency graph still includes crypto/proof dependencies; this slice does not claim a lightweight VM-only graph. Package modules separate environment, identity/checkpoints, execution/replay, reports/errors and witness scripting where practical.

### Alternatives and deferrals
Reject a universal erased contract interface and a new DSL for this slice. Defer funded/offer-bound contexts until complete runtime state can be checkpointed safely. Defer generic databases, durable private files, proof/network adapters and WASM packaging to separately evidenced slices. This child does not close the broader testkit work package.

### Verification
Counter success/repeated calls/underflow rollback; owned restore/fork and identity/version/mode mismatch; actual generated witnessed Cell and collections; ordered private outputs; native-only witness branch support; malformed replay/context/Zswap refusal; private diagnostics redaction; no silent downgrade. Use unchanged generated fixture crates and existing independent TS captures. Local tests/Clippy first. Baseline source pins and initial threat/test boundaries are in the milestone targets and this decision; broader version/adversarial parent packages remain open.

### Internal review: trusted adapter boundary

The closure adapter receives a mutable runtime context. Existing runtime public traces seal query/identity/intents, but not transient cost/gas changes. A closure can clear a gas limit, execute and restore the original value before returning; comparing the final environment cannot detect this. ContractLab is a test orchestration tool for trusted generated-call adapters, not an enforcement boundary around arbitrary user Rust. This first slice checks final policy and sealed starting query/identity/intents, and validates replay state/effects; it does not attest every intermediate adapter action. API docs and a regression must explicitly demonstrate this limit.

Do not substitute an aggregate replay gas limit for per-query execution policy: those are different semantics. Stronger immutable runtime policy ownership is future domain-hardening work; it requires its own runtime API evidence. This clarification preserves honest scope without pretending that arbitrary callbacks are sandboxed. External witness effects and shallow Clone state remain outside rollback guarantees. Internal review is not the independent external coding-agent milestone audit.

### Delivery

Signed/DCO local commit `ceaac44c49ccf361250550d65e170a9c52e1ece2`; [ADR0248 — ContractLab local receipt](references-0.3.0.md#note-015). 15 tests and strict Clippy pass;329/336productionlines exercised97.92%. Initialplain-ledger child complete; broader parenttestkit acceptance remainsopen. Trusted adapter andClone/external-effect limits retained.
