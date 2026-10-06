---
id: RUST-ADR-0093
alias: ADR-0093
title: "Record pure conditional Field Cell writes"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "control-flow", "typed-slots"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 13ea6ee1857775462e41c63e5bf89c5fcee32faf32a12bd92c6b694e494c3665
---
# RUST-ADR-0093 — Record pure conditional Field Cell writes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted a pure scalar Field conditional bound before a typed Cell write, with both arms audited and lazy runtime selection. Both walkerWrite branches have focused TS/replay/proof evidence. Witnesses, ledger reads or unsupported coercions in those arms remain outside this narrow rule.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#197 closure](https://github.com/MediaNoxLabs/compact/issues/197#issuecomment-6017564403). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`aabf9241`](https://github.com/MediaNoxLabs/compact/commit/aabf924127553afa156ebbe7e24e6e4a1ced92dd). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 93
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/197
```

## Historical decision and amendments

### Problem and acceptance source

`examples/rust_backend/ternary_cond_oracle.compact` exports `walkerWrite(c: Boolean, x: Field)`, whose result is `c ? x : 0` written to `fieldCell`. The exact schema-11 typed IR is `StateAction::Let` binding a Field `Expr::If`, followed by `StateAction::CellWrite`. The pinned baseline compiler marks it `proof_required: true` but `recorded: false`, `observed_call: false`, with the first gap `Expr::If` at `actions[0].bindings[0].value`. Native generated execution and TypeScript state bytes already work. The missing API prevents source-to-proof-to-ledger use of this exported call.

### Decision and boundaries

The AST recorded emitter will lower a Field `Expr::If` only when the Boolean condition and both Field arms are pure scalar sources already accepted by typed `cell_source`: parameters, previously bound locals, and Field literals with exact types (including no-op coercions). It will emit one Rust `if` expression and bind its Field result before the typed Cell write. This keeps branch choice lazy and uses `CellSlot::record_write` and `RecordingFrame` for VM recording. It will refuse branches with ledger reads, witnesses, calls, nested effects, mismatched types, or unsupported coercions. The fallback remains a structured capability gap; it will not claim those paths as proof capable.

No private IR variant, schema/ABI/report version, runtime public API, or ledger/ZK primitive changes. This is a narrow renderer capability over existing typed IR. It does not target `walkerConstAnnotated`, whose `Uint<8>` cast and subsequent Field cast are two distinct blockers, or comparison/struct paths.

### Before and after generated Rust

Before:

```rust
let native = ledger_contract::walkerWrite(context, c, x)?;
// No ledger_contract::recorded::walkerWrite and no typed walkerWrite_call.
```

After complete lowering:

```rust
let value: runtime::Field = if c { x } else { runtime::Field::from(0u128) };
let frame = crate::ledger_slots::fieldCell.record_write(frame, value)?;
let recorded = ledger_contract::recorded::walkerWrite(context, c, x)?;
let call = contract.recording.walkerWrite_call(&confirmed, private, c, x)?;
```

The branch expression introduces no VM operations; the Cell write supplies the existing ledger-8 VM trace and metering.

### Validation

Use an immutable schema-11 Scheme frontend with a freshly built Rust `compactc` in a separate target. Verify exact before/after capability for `walkerWrite` and no unexpected regression in `ternary_cond_oracle`. Capture both TypeScript branches and compare full state bytes, ordered VM operations, all four summed gas dimensions, and private outputs/state with native and recorded Rust, then replay the recorded VM. Compile ZKIR/keys for the source, prove and verify `walkerWrite`, validate/apply it to ledger-8, and check the resulting Cell and unchanged Counter. Run focused renderer, generated crate, targeted Clippy, fixture freshness, and full inventory delta. Root integration owns the combined local full gate. No push or remote CI.

### Tracking

- MediaNoxLabs issue: pending.
- Branch: `codex/m2-ternary-let` from `aabf9241`.
- Commit and exact evidence: pending.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/197 (rust-backend-v2).

### Local delivery — 2026-10-05

Delivered on isolated `codex/m2-ternary-let` at conventional GPG-signed, DCO commit `3c9db7c974a3d653e6583049895a5d2ea85aa844`, based on root `aabf9241`. Only `ternary_cond_oracle.walkerWrite` changed capability: `proof_required: true`, recorded/observed `false/false → true/true`; all other circuits in that source retained status. Full proof-aware inventory across 191 sources and 931 declarations moves `205/296 → 206/296` available and `91 → 90` gaps; zero baseline drift or unmatched compiler rows. Receipt `${LOCAL_EVIDENCE}/adr93-inventory.json`.

The renderer branch accepts only pure scalar typed `Expr::If` for a Field `StateAction::Let`, emits one `runtime::Field` binding, and reuses `fieldCell.record_write`. The generated `walkerWrite` fixture has both `recorded::walkerWrite` and `recording.walkerWrite_call`. No schema, ABI, report or runtime API changed. Other ternary gaps, including `walkerConstAnnotated`'s Uint8 conditional and Field cast, remain separate.

TypeScript `walkerWrite(false, 777)` stores Field `0`; `walkerWrite(true, 777)` stores `777`. Both branches generate ordered public VM `push(false), push(true), ins(false,1)` and no private FAB outputs or private state transition. Summed gas in TypeScript and generated native/recorded Rust: readTime `85000000`, computeTime `1233942932`, bytesDeleted `328`, bytesWritten `262` (false) / `266` (true). Full serialized ContractState, native/recorded effects, all four gas dimensions and recorded VM replay match both TS branches. The typed observed-call prototype matches the manual recorded adapter.

The local Rust `compactc` SHA-256 was `a9eafbf53449a038c5154cb46965435b2ce2cc6fb381d1a04d904ac29b4c6a06`; schema-11 Scheme wrapper `${LOCAL_EVIDENCE}/adr89-schema11-scheme-root/compactc-scheme` SHA-256 `369ee19b62cba4d6e5f13b0a40599d6b7a38f42e12216ec01ed9ed270747a3d4`; pinned ledger-8 ZKIR 2.1.0 executable SHA-256 `9b45827ac4786e383b3bfd93c870afe9a2cbb69af753949fd53549d146c548bc`. The compiler generated `walkerWrite.zkir/.bzkir`, prover and verifier keys. `compact-rust-proof-smoke --conditional-field ${LOCAL_EVIDENCE}/adr93-proof-zkir` replayed, proved, verified, validated and applied both branches to ledger-8 and checked Field Cell `0/777` plus unchanged Counter.

Focused generated crate tests (3), renderer tests (75), 139 fixture freshness (0 stale/failed), targeted Rust 1.99 Clippy `-D warnings`, `cargo fmt --check`, Python gate syntax and `git diff --check` passed. The proof smoke is wired into `check_compactc_target.py --proof` for the later combined local gate. No push or remote CI. Root integration and combined gate remain pending.
