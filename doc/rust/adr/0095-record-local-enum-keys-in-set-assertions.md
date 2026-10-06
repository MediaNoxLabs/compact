---
id: RUST-ADR-0095
alias: ADR-0095
title: "Record local enum keys in Set assertions"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "collections", "enums", "lint"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 56168af9a228b7570b31d47760d0b6a1dd054b4472d806edb368040a4f746f0b
---
# RUST-ADR-0095 — Record local enum keys in Set assertions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted matching enum local bindings and Set membership with existing typed slots, preserving query/assertion order. The implementation temporarily allowed only generated bool_comparison lint under separately tracked #200; that allowance is historical and not a general Clippy relaxation. Vector/default/qualified-coin domains are separate.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#196 closure](https://github.com/MediaNoxLabs/compact/issues/196#issuecomment-6017562771). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`aabf9241`](https://github.com/MediaNoxLabs/compact/commit/aabf924127553afa156ebbe7e24e6e4a1ced92dd). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 95
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/196
```

## Historical decision and amendments

### Problem and measured source

On isolated root `aabf9241` with schema-11 Scheme, `examples/adt/tests/set_enum.compact` compiles as native Rust and compiler metadata marks exported `test` proof-required. The schema-3 report has `recorded:false`, `observed_call:false`, first gap `StateAction::Let` at `actions[0]`. The typed IR binds `one = Names.bill` and `two = Names.sally`, then removes/inserts them in `Set<Names>` and checks `isEmpty`, `size`, and `member` across reset. Native Rust already uses typed `SetSlot<Names>`. Recording accepts neither the local enum binding nor enum-valued `SetMember` yet. `set_vector.test` instead fails at its second `StateAction::Let` for a pure `getVector()` call and is explicitly outside this decision.

### Before and after generated Rust

Before, the generated crate exposes only the direct method:

```rust
let native = ledger_contract::test(context)?;
// ledger_contract::recorded::test and contract.recording().test_call do not exist.
```

After, the same source also exposes a replayable recorded/observed call. Its key steps should retain the declared enum type and source order:

```rust
let one: crate::types::Names = crate::types::Names::bill;
let two: crate::types::Names = crate::types::Names::sally;
let frame = crate::ledger_slots::c.record_remove(frame, one)?;
let (frame, empty) = crate::ledger_slots::c.record_is_empty(frame)?;
// Assertion checks empty, followed by one record_size query.
let frame = crate::ledger_slots::c.record_insert(frame, one)?;
let (frame, member) = crate::ledger_slots::c.record_member(frame, one)?;
// The remaining operations and assertions follow the Compact source order.
```

The renderer may use internal temporary names and clone or copy enum values as required. The public API remains an ordinary typed contract method rather than handwritten VM operations.

### Decision and ownership

Extend the recorded AST emitter only for `StateAction::Let` bindings whose declared type is `Type::Enum` and value is a matching typed `Expr::EnumVariant` or already-bound matching parameter. Bind once using the existing `cell_source` and `rust_type` helpers. Permit `Type::Enum` in recorded `Expr::SetMember`, which already validates the Set declaration and index and uses `SetSlot::record_member`. Existing Set remove/insert/reset, isEmpty and size recording, enum `CompactCellValue` derivation, and ledger-8 Set VM primitives are reused. No new private IR node, schema/ABI bump, runtime API or new key encoding is intended. Unsupported enum expressions must keep a precise capability gap; no fallback from native execution to a fabricated recording.

### Acceptance and limits

Freeze schema-11 Scheme and one Rust CLI, compile `set_enum` before/after, and require exactly its proof-required `test` capability to move unavailable to available with no prior-row regressions. Compare fresh TypeScript vs Rust native and recorded state, four gas dimensions, ordered VM transcript and private FAB output (empty for this contract), including replay. Run a pinned source-to-proof-to-ledger application if proving artifacts are available. Check generated crate and renderer tests, focused local Clippy and a fresh compiler inventory. `set_vector` remains unavailable at the pure vector call binding; `set_qualified_coin_info` has a separate compiler rejection. Full suite and fixture refresh belong to root integration. Risks are key type mismatch, duplicate member queries, wrong Set query order, and unsupported enum expressions silently accepted.

### Tracking

- Predecessor: [ADR-0088 — Record nested Set size assertions through typed slots](0088-record-nested-set-size-assertions-through-typed-slots.md).
- Cohort: [#188](https://github.com/MediaNoxLabs/compact/issues/188).
- Focused issue: pending.
- Delivery: proposed; no code or proof claim yet.


### Local delivery and proof evidence — 2026-10-05

The isolated branch `codex/adr95-adt-set-recording` based on `aabf9241` emits `set_enum.test` as a recorded and observed call. The emitter accepts only matching typed enum variants or already-bound enum parameters in `StateAction::Let`, and admits `Type::Enum` keys in typed Set insert/remove/member recording. A renderer test proves the binding precedes insert, member and remove in order; `Expr::Default<Names>` still reports a recording gap. No IR node, schema, ABI or runtime API changed. The generated crate uses the existing `CompactCellValue` enum derivation and ledger-8 Set slot recording.

The source's explicit Boolean assertions (`c.member(one) == true`, etc.) produce `bool_comparison` Clippy diagnostics in both native and recorded generated functions. The production `compactc` Cargo manifest and the checked fixture manifest set only `lints.clippy.bool_comparison = "allow"`; all other warnings remain denied by the gate. Normalizing these comparisons in both native and recorded AST emitters would change a wider fixture cohort and needs separate parity proof; follow-up [#200](https://github.com/MediaNoxLabs/compact/issues/200) tracks removal of this narrow allowance. This is a generated-manifest change, not a runtime primitive or schema change.

Pinned local compiler `${LOCAL_EVIDENCE}/adr95-target/debug/compactc` SHA-256 `1db6dbdcdcf88b736b52ab3116d1577aa94bb8d353595a3c3d5ec1b921889b15` with schema-11 Scheme wrapper SHA-256 `369ee19b62cba4d6e5f13b0a40599d6b7a38f42e12216ec01ed9ed270747a3d4` generated `test` ZKIR/prover/verifier with ZKIR 2.1.0. The checked TypeScript capture records 16 queries and 73 public VM operations. Generated Rust native and recorded paths match its serialized final state, all four aggregate gas dimensions, ordered VM shape, and zero private FAB outputs; recorded Verify replay reaches the same state. `compact-rust-proof-smoke --adt-set-enum ${LOCAL_EVIDENCE}/adr95-set-enum-proof-final` proved, verified and applied the observed typed call through ledger-8. The final bzkir SHA-256 is `90474a2b70f620582f07e0f6a8ad89110fa45d6408df43cf12a612c4dfdd7120`.

The checked fixture script passed 140/140, no stale outputs. Focused gate `${LOCAL_EVIDENCE}/adr95-focused-gate-final/receipt.json` passed one proof-eligible recorded/observed API. All 76 renderer tests, 12 `compactc` binary tests, the generated enum Set test, targeted backend/fixture/proof-smoke Clippy `-D warnings`, standalone generated crate Clippy, workspace rustfmt check, and `git diff --check` passed. Full source inventory `${LOCAL_EVIDENCE}/adr95-full-inventory.json` has 191 sources, 931 declarations, 296 proof-required contract circuits, 206 available and 90 known gaps, with 120 unassessed exports. Compared row-for-row with `${LOCAL_EVIDENCE}/adr91-92-schema11-inventory.json`, exactly `set_enum.test` changes unavailable→available; no other row changes. `set_vector.test` stays unavailable at `actions[0].action` because its pure `getVector()` local binding is separate. Full combined gate after integration remains root's responsibility.


Local signed GPG/DCO conventional commit: `efbe5772c34378e2b11395ec4dbe8b128f99cdc1` (`feat(rust): record enum Set assertions for ADT source`). Working tree clean on isolated `codex/adr95-adt-set-recording`; no push or remote CI. Integration and combined exact-HEAD full gate remain with root.
