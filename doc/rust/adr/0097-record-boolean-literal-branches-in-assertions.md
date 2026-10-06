---
id: RUST-ADR-0097
alias: ADR-0097
title: "Record Boolean literal branches in assertions"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "assertions", "control-flow", "asset-registry"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b94ab0be8c8526f89ad0c716b48c09201f830a837167a8720ba902f446975e5e
---
# RUST-ADR-0097 — Record Boolean literal branches in assertions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted Boolean conditionals with effect-free scalar arms while recording the condition once, enabling ordered AssetRegistry writable guards. Successes have focused proofs; actual failure calls are checked for error/witness suppression, while failure trace/gas are separately reconstructed with typed prefix replay because errors do not return a frame.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#199 closure](https://github.com/MediaNoxLabs/compact/issues/199#issuecomment-6017567458). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

### Original source metadata

```yaml
adr: 97
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/199
```

## Historical decision and amendments

### Problem and measured source

In `examples/rust_backend/asset_registry_oracle.compact`, internal `assertWritable` asserts `open` and then `!frozen` before several exported writes. The schema-11 typed IR lowers `!frozen` to `Expr::If { condition: CellRead(frozen), then: Boolean(false), otherwise: Boolean(true) }` inside `StateAction::Assert`. The current recorded Boolean evaluator supports the direct `open` Cell read and the existing typed write operations, but lacks this `Expr::If`. The exact first gap for proof-required `setCustodian` and `tag` is `callee[assertWritable].actions[1]`, `StateAction::Assert`. Both recorded and observed-call APIs are unavailable, while the proof-required `close` circuit already records the shared `recordWrite` helper.

### Decision and boundaries

Extend the AST recorded Boolean expression lowering for `Expr::If` only when both arms are pure Boolean scalar sources accepted by the typed source helper (Boolean literals, exact Boolean parameters or previously bound Boolean locals, including no-op coercions). Recursively lower the Boolean condition before choosing an arm, so an existing typed Cell read or other already-supported condition effect is recorded exactly once in source order. Emit one Rust Boolean `if` binding and feed it to the existing `StateAction::Assert` check. Effects in either branch, composite values and unsupported conditions remain unavailable. Do not move a witness or VM read out of a branch.

No private IR/schema, ABI, capability report, runtime API or ledger primitive changes. The existing `RecordingFrame`, `CellSlot::record_read`, assertion error, Cell/Set typed slots and ledger-8 VM paths remain authoritative. The renderer may expose `setCustodian` and `tag` only if their entire callee and following action graphs are complete; a first-gap movement alone does not count as a capability gain.

### Before and after generated Rust

Before:

```rust
let native = ledger_contract::tag(context, &witnesses, value)?;
// No recorded::tag or typed recording.tag_call; assertWritable second assertion is unavailable.
```

After complete proof:

```rust
let (frame, open): (_, bool) = ledger_slots::open.record_read(frame)?;
if !open { return Err(CompactError::AssertionFailed("registry is closed".into())); }
let (frame, frozen): (_, bool) = ledger_slots::frozen.record_read(frame)?;
let writable: bool = if frozen { false } else { true };
if !writable { return Err(CompactError::AssertionFailed("registry is frozen".into())); }
let frame = ledger_slots::tags.record_insert(frame, value)?;
// recordWrite then records counters and a metered currentTimestamp witness.
let call = contract.recording().tag_call(&confirmed, private, value)?;
```

The same guard precedes the typed `custodian` Cell write in `setCustodian`.

### Validation plan

Use the immutable schema-11 Scheme snapshot and an isolated Rust `compactc`/Cargo target. Confirm exact before/after capability for `setCustodian` and `tag`, plus no corpus regressions. Capture TypeScript success, closed failure and frozen failure with the same seed state and deterministic witness; compare native and recorded Rust full serialized state, private state, private FAB outputs, all four summed query gas dimensions, ordered VM operations, witness call/read order and assertion text. Success must read `open`, then `frozen`, then execute write/recordWrite; closed failure reads only `open`; frozen failure reads both and neither failure writes or invokes `currentTimestamp`.

Compile pinned ledger-8 ZKIR/keys for the proof-required exported calls and use typed observed calls to prove, verify, validate and apply successful `setCustodian` and `tag`, checking the exact Cell/Set mutation and counters. Run focused renderer, generated fixture, strict fixture freshness, Clippy/fmt/diff, and inventory. Root integration owns the combined local full gate. No push or remote CI.

### Tracking

- MediaNoxLabs issue: [#199](https://github.com/MediaNoxLabs/compact/issues/199), assigned to `rust-backend-v2` before implementation.
- Stacked branch: `codex/m2-ternary-let` after signed/DCO `3c9db7c9` (ADR-0093).
- Signed/DCO delivery commit: `1b5eae23a0200ee3bfb3ac465fae4c91e781f1d5`.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/199 (rust-backend-v2).

### Delivery evidence and current limit

The schema-11 source-to-Rust compiler now exposes `recorded::setCustodian`, `recorded::tag`, and both typed observed calls. The generated asset fixture contains a single `open` read, then a single `frozen` read whose Boolean conditional selects an effect-free literal. The shared `recordWrite` helper retains its Counter increments, metered witness reads, private FAB output, and timestamp Cell write.

The deterministic TypeScript oracle covers `setCustodian` and `tag` in success, closed and frozen states. Both successes compare full serialized state, native/recorded effects, private state and FAB, all four gas dimensions, ordered VM instructions, and replayed state. Each failure compares assertion text, unchanged state/private state, no witness or FAB output, and the ordered read prefix and its four gas dimensions. The generated call API returns an error without its partial frame, so the failure transcript is checked through the same declared typed slots as a separate prefix replay; the actual call is checked for its exact error and witness suppression.

Pinned ledger-8 ZKIR keys were generated locally. Both successful calls were replayed, matched to typed observed prototypes, proved, verified, validated and applied through ledger-8. The resulting custodian Cell or tag Set, timestamp Cell, revision and writeCount Counters matched expectations. The proof-required inventory increased from 206/296 to 208/296; 88 remain missing, and asset_registry_oracle still has seven proof-required gaps. The 191-source identity baseline had no additions or removals. All 139 generated fixture libraries were fresh. The renderer's 75 tests, asset fixture's existing and new tests, focused strict Clippy, format and diff checks passed. Root integration still owns the combined local full gate.
