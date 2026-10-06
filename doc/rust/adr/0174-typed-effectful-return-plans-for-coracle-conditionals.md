---
id: RUST-ADR-0174
alias: ADR-0174
title: "Typed effectful return plans for Coracle conditionals"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-native-only"
topics: ["compiler", "ReturnPlan", "effects"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b8b9766c99d7e79980f9ff5aa8890d4f55a93aa24cf88320b4a01f09825a1847
---
# RUST-ADR-0174 — Typed effectful return plans for Coracle conditionals

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-native-only. A single-owner recursive ReturnPlan preserves ordered effects, lexical scope and one evaluation of conditional results in native Rust. The new effectful form is native-only here; later ADR182 separately audits recording.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#278 closure](https://github.com/MediaNoxLabs/compact/issues/278#issuecomment-6017701182). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`6f8c6d17`](https://github.com/MediaNoxLabs/compact/commit/6f8c6d170e2e803fa3704d7fc279e15f1c6d5169) · [`e751ddf1`](https://github.com/MediaNoxLabs/compact/commit/e751ddf198bf293cb58b5c5abc8dcffdaa28458a). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: Accepted for isolated delivery · 2026-10-05 · rust-backend-v2

### Problem

After ADR-0169, the complete unchanged `test-center/test-contracts/coracle.compact` stops at line 191 (`state = State.red_started`) inside `start`'s value-returning conditional. Its `Lnodisclose` body has assertions and two `receiveShielded` calls, nested `let* b/sk/n/cb`, then a condition that reads `state`, followed by branch-local ledger/witness actions and `Player.red` or `Player.blue`. The current `stateful-return-ir` treats the entire final `if` as a pure expression. It descends into a branch Cell write and reports an unsupported nested query. Lifting the writes separately and re-reading the condition at return would be unsound: the chosen branch mutates `state`.

All nine original Coracle exports remain unassessed until the complete source compiles. This decision targets the shared conditional-return shape; shielded `writeCoin` and other operations remain separate semantic bridges.

### Before and after

Original Compact excerpt:

```compact
if (state == State.no_game) {
  state = State.red_started;
  local_set_board(cb);
  return Player.red;
} else {
  state = State.blue_started;
  local_set_board(cb);
  return Player.blue;
}
```

Current Rust lowering tries to render writes as parts of a return expression. The intended generated structure evaluates the condition once, at its source position after prior actions, then executes and returns from only the chosen branch:

```rust
let condition = ledger_slots::state.read(context)?; // metered once
let (context, result) = if condition.result == State::no_game {
    let step = ledger_slots::state.write(condition.context, State::red_started)?;
    local_set_board(/* branch-local witness effect */)?;
    (step.context, Player::red)
} else {
    let step = ledger_slots::state.write(condition.context, State::blue_started)?;
    local_set_board(/* branch-local witness effect */)?;
    (step.context, Player::blue)
};
// total_cost and private transcript outputs include only the chosen path.
```

The sample omits other Coracle operations; it illustrates evaluation order and result ownership, not a complete emitted file.

### Decision

Add schema-17 `StateReturn::Effectful { body: ReturnPlan }` with a recursive typed plan:

```text
ReturnPlan::Value(Expr)
ReturnPlan::Let(bindings: [LocalBinding], tail: ReturnPlan)
ReturnPlan::Sequence(actions: [StateAction], tail: ReturnPlan)
ReturnPlan::Conditional(condition: Expr, then: ReturnPlan, otherwise: ReturnPlan)
```

For this bounded lowering, Scheme lowers the *whole ordered circuit body* into one plan and emits `actions=[]`. The renderer rejects a nonempty `actions` array paired with `Effectful`, preventing duplicate effects. Ordinary `StateReturn` forms and action extraction remain unchanged. Each Let owns the lexical scope of its tail. Sequence performs actions before its tail. Conditional evaluates its Boolean condition once, then renders exactly one branch's actions and typed value in the same Rust block. Both branch results must equal the declared circuit result type. Branch-local values cannot escape their plan scope. No synthetic source name, action/result marker pair, or condition reevaluation is used.

The renderer reuses the existing action lowering for plan Sequence steps and accounts for context, gas, and private transcript outputs once. It must recursively inspect plan bindings, action sequences, conditions, and values in all effect/call/witness analyses. Recorded lowering explicitly reports this new return form unsupported until its replay semantics are audited; strict recording rejects it. This is an IR schema change from the version-16 line reserved by ADR-0173, so use schema 17 and reject prior schema inputs. The generated runtime public API is unchanged; ABI remains 42 after ADR-0173. Coordinate integration order if those version reservations move.

### Guards and acceptance

1. Structural source admission only for terminal value-returning conditionals whose arms have an ordered action prefix and typed final value. Other nested effects fail closed. Do not route by Coracle source or helper name.
2. A minimal oracle must put a prior mutation before the condition, read a Cell in the condition, mutate that same Cell in the chosen arm, and put a witness plus another write in the untaken arm. Compare TypeScript and native values, state, query order/meter, ledger effects, witness counts, and repeated calls through both branches. A re-read after mutation or eager opposite branch must fail the test.
3. Renderer/parser negative tests: non-Boolean condition, unequal/wrong branch types, duplicate `actions` ownership, stale schema, malformed nested plan, escaped branch-local name, and unsupported effectful nested value. Preserve original pure-if, ADR-0138/0169 root Let, bboard, welcome, election, and recorded availability.
4. Recompile the full unchanged Coracle source; record the next diagnostic and exact nine-export inventory attribution. Do not claim native acceptance, recording, or proof if another shielded operation blocks metadata.
5. Focused compiler, renderer, oracle, source, inventory and fixture gates; broaden only for actual shared-risk failures. Deliver local conventional GPG-signed DCO commit; no push or remote CI.

### Alternatives considered

A paired `IfResult` action and `ActionResult` marker would have required cross-node pairing and terminal placement validation. A synthetic condition binding in the existing action/return pair would lose nested `b/sk/n/cb` scopes or move the condition ahead of earlier effects. The single return plan owns the body, branch effects, and value together.

Issue: https://github.com/MediaNoxLabs/compact/issues/278

### Delivery evidence — 2026-10-05

Accepted as an isolated schema-17 / runtime-ABI-42 slice. The structural trigger is limited to non-Unit terminal conditionals with branch Cell writes; welcome, bboard and election IR/capability rows are unchanged apart from schema. The new oracle has nested outer and branch locals, a precondition write, a condition that reads the same Cell the selected branch writes, and an untaken witness/write. Two sequential TypeScript and native calls agree on result, serialized state, ledger effects, four metered queries per call, private state and witness transcript count. TypeScript reportedGas covers only its last query for this shape, so the native full cost is compared with the sum of captured query costs.

Focused receipt: ${LOCAL_EVIDENCE}/compact-adr174-focused/receipt.json (1 fixture, 0/1 recorded); source gate and positive-scope check pass; 142 renderer tests, native fixture test, 27 Python harness tests, targeted Clippy and formatting pass. Full local inventory: 203 sources, 704 exported circuits, 333 proof-required, 327 proof APIs available, 6 known proof gaps, 20 unassessed exports. All nine original Coracle exports remain unassessed: its next compiler diagnostic is `<standard library>: Rust backend does not yet support this native witness expression`. Recorded Rust deliberately refuses Effectful; no proof claim. Root integration after ADR-0175 schema 18 will assign the combined next schema 19, with ABI 42 preserved for this slice. No push or remote CI.

Signed isolated delivery: `4b8b4875fcc78fca54d223e823bdd78226d5596d` (good GPG signature and DCO); exact-head clean focused receipt `${LOCAL_EVIDENCE}/compact-adr174-focused-signed/receipt.json`. Issue delivery comment: https://github.com/MediaNoxLabs/compact/issues/278#issuecomment-5994133800.
### Main integration — ADR174

Integrated as `6f8c6d17`, verified GPG+DCO. Single-owner ReturnPlan is integrated at schema19, then carried into schema20. Native TS parity passes; recording remains explicit unavailable. Combined Coracle advances to standard-library.compact207 Uint129, beyond the isolated native-witness blocker.

Current checkpoint `e751ddf1` is private schema20/runtime ABI45.162 fresh fixtures (zero failures),8 backend units+144 renderer tests,9 runtime units,27 Python tests, targeted strict Clippy and formatting pass. Seven-source frozen focused gate: `${LOCAL_EVIDENCE}/compact-focused-e751ddf1/receipt.json`. Combined Cell proof log: `${LOCAL_EVIDENCE}/compact-e751ddf1-cell-proof.log`.

Full source inventory:333/343 assessed proof-required APIs,10 known gaps,20 unassessed exports across205 sources/717 exports. Receipt: `${LOCAL_EVIDENCE}/compact-e751ddf1-inventory.json`. Native admission expands the assessed gap denominator; retained existing capabilities pass regressions. Latest completed broad gate remains4ef0fa57, not a claimed ABI45 full run. No push/remote CI.
