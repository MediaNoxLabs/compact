---
id: RUST-ADR-0078
alias: ADR-0078
title: "Record Counter reset through the typed ledger slot"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 4f364d04490f0365367871634bdff491e7b9cb25651dee01d7e822e0f83c71e4
---
# RUST-ADR-0078 — Record Counter reset through the typed ledger slot

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept recorded Counter reset through the declared CounterSlot and canonical zero Cell write. The initial proof uses a seeded prior state and does not prove a preceding increment in the same deployment. Preserve ABI 35 history and separate later lifecycle evidence.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#177 closure](https://github.com/MediaNoxLabs/compact/issues/177#issuecomment-6017530371). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`e82b6202`](https://github.com/MediaNoxLabs/compact/commit/e82b6202094909209cb69a496cdf34627c74ad42). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 78
status: accepted-local
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/177
```

## Historical decision and amendments

### Problem

`counter_parameter.compact` exports `reset_round(): [] { round.resetToDefault(); }`. The native Rust method uses `CounterSlot::reset` and the pinned ledger Cell write path. The schema-8 IR already names `StateAction::CounterReset`, yet `recorded.rs` has no lowering for it. The generated crate omits `recorded::reset_round` and the observed `reset_round_call`, so a Rust consumer cannot prove that source circuit. The 2026-10-05 local capability inventory finds 174 of 333 exported oracle circuits without those APIs; this ADR solves only the Counter reset case.

### Before and after

```rust
// Before: native execution exists, but no recorded::reset_round.
let result = ledger_contract::reset_round(context)?;

// After: one replayable reset and a typed transaction-ready handle.
let recorded = ledger_contract::recorded::reset_round(context)?;
let verify_ops = recorded.public.verify_ops();
let call = ledger_contract::recorded::Contract::default()
    .reset_round_call(&observed, ())?;
```

The generated body should be a small typed slot call:

```rust
let frame = runtime::recording::RecordingFrame::new(context);
let frame = crate::ledger_slots::round.record_reset(frame)?;
Ok(frame.finish(()))
```

### Decision and ownership

At `StateAction::CounterReset`, the AST emitter validates the declared field kind and physical index, then emits `CounterSlot::record_reset`. The runtime slot delegates to existing `RecordingFrame::write_cell(path, 0_u64)`, which uses the pinned ledger Cell write VM program and the same semantics as native `CounterSlot::reset`. Generated code never owns VM opcodes. The change adds a public generated/runtime recording API, so increment runtime ABI 34 to 35 and regenerate fixture outputs; private IR schema 8 stays fixed. No new macro, ledger or midnight-zk primitive is needed.

### Acceptance and limits

Create a focused MediaNoxLabs issue assigned to `rust-backend-v2` before implementation. Verify wrong field kind/index rejection in the renderer and absence of accidental capability for unsupported shapes. Verify native/recorded reset from a nonzero Counter, exact state, effects, gas, ordered Verify operations and replay; compare available TypeScript oracle data or capture the exact source circuit oracle if needed. Extend the local proof and ledger application gate to prove, verify, validate and apply `reset_round` using the pinned ledger-8/zk toolchain. Regenerate all fixtures, compile an external consumer, and run focused local gates followed by full gates justified by the ABI and proof change. Commit conventionally with DCO and verified GPG signature. The branch remains local; remote CI is deferred by the user's local-first policy.

This decision covers direct Counter reset actions. It does not imply recording support for nested resets, other container resets, arbitrary pure-call composition or the 173 other unsupported circuits in the inventory.

### Tracking

- Parent proof capability issue: [#152](https://github.com/MediaNoxLabs/compact/issues/152).
- Focused issue: pending creation before implementation.
- Delivery: proposed; no implementation or proof claim.


### Tracking amendment — 2026-10-05

Focused [#177](https://github.com/MediaNoxLabs/compact/issues/177) was created and assigned to `rust-backend-v2` before source changes. The proposed state and pending line above preserve the decision chronology.


### Local implementation checkpoint — 2026-10-05

The runtime adds `CounterSlot::record_reset` via existing `RecordingFrame::write_cell(path, 0_u64)`. The AST emitter validates the declared Counter/index and emits the slot call; runtime/backend ABI is 35, private schema stays 8. All 137 fixture libraries were refreshed. `counter_parameter.reset_round` now has recorded and observed-call methods, and the strict `--rust-require-recording` compiler gate accepts all three exported circuits. The corpus capability count moves from 159/333 to 160/333, with no regression.

A reproducible TypeScript capture script and JSON oracle record state bytes before, after increment by 7 and after reset, four gas dimensions, and ordered `push, push, ins` public operations. The focused generated-crate test matches state, native/recorded/replay effects and gas, transcript shape, and empty private outputs. Sixty-six AST renderer tests and 137 fixture freshness checks pass. The pinned local `check_compactc_target.py --consumer --proof` gate passed, including reset from a synthetically seeded value of 3: manual and typed observed call prototypes matched, independent proof verification and ledger validation/application succeeded. This tests the reset against a nonzero deployment state but does not claim a separately proven increment-before-reset history in the same contract.

Exact Rust 1.99 all-target/all-feature Clippy and Nix packaging are still running at this checkpoint. Commit, signature, packaged consumer and final delivery claim follow those results. The user-owned documentation edit remains unstaged.


### Local acceptance — 2026-10-05

Conventional GPG-verified/DCO commit `e82b6202094909209cb69a496cdf34627c74ad42` (`feat(rust-backend): record typed Counter reset`, `Refs: #177`) passed all scoped local gates. The existing ledger Cell write VM program owns reset semantics; `CounterSlot::record_reset` and schema-8 AST lowering expose the generated recorded and observed-call API at ABI 35, with private IR schema 8 unchanged. The 137-source capability inventory changes only `counter_parameter.reset_round`: 160/333 recorded after, 173 missing, zero regressions.

The checked-in TypeScript oracle captures serialized state at 0→7→0, reset gas (read 85000000, compute 1233942932, written 36, deleted 36), ordered `push(false), push(true), ins(false,1)` and zero private transcript outputs. The focused Rust test matches native, recorded and replay state/effects/gas and those TypeScript values. Sixty-six AST renderer tests, three generated Counter tests, 137/137 fixture freshness, formatting, staged diff checks, exact Rust 1.99 workspace all-target/all-feature Clippy with `-D warnings`, pinned ZKIR 2.1.0 `check_compactc_target.py --consumer --proof`, and Nix packaged compiler `${HISTORICAL_NIX_STORE}/pvwmk79f78h54kjafymkhsix3a3g8pca-compactc` with packaged `--consumer` pass. The proof gate verified, validated and applied reset from a nonzero synthetically seeded deployment state, including manual/observed prototype parity; it does not prove a prior increment in the same deployed contract.

`git verify-commit` reports a good signature, and the DCO trailer is present. Only the unrelated user-owned `doc/ledger-adt.mdx` edit remained unstaged after this commit; the Nix source was therefore dirty, not a clean release candidate. [#177](https://github.com/MediaNoxLabs/compact/issues/177) has delivery evidence and stays open under the local-first remote-CI policy.
