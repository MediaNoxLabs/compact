---
id: RUST-ADR-0169
alias: ADR-0169
title: "Preserve effectful ledger writes in value-returning Coracle helpers"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-native-only"
topics: ["compiler", "Let", "Cell"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 72dff3e159bdd36188b0d3e8a565c4bf29c30094380cedd60f53eb1cc628f4af
---
# RUST-ADR-0169 — Preserve effectful ledger writes in value-returning Coracle helpers

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-native-only. A narrow root-Let extraction preserves Cell.write side effects when the returned value does not use the binding. It adds a native source-level oracle, while the complete Coracle source was still unassessed at this historical checkpoint and recording was not admitted.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#273 closure](https://github.com/MediaNoxLabs/compact/issues/273#issuecomment-6017692040). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

## Historical decision and amendments

Status: Implemented in isolated branch · 2026-10-05 · rust-backend-v2 · [Issue #273](https://github.com/MediaNoxLabs/compact/issues/273)

### Problem

The unchanged `test-center/test-contracts/coracle.compact` compiles for TypeScript, while Rust on integrated schema 14 / ABI 39 stops at line 294, `return sendShielded(...).sent`, with `Rust backend does not yet support this nested ledger query: __compact_Cell.write`. The complete source has nine unassessed exported circuits. `sendShielded` in `compiler/standard-library.compact` is a value-returning circuit that creates shielded inputs/outputs and writes kernel claims before returning `ShieldedSendResult`. Dropping the write, evaluating it twice, or moving it past the return changes state and gas. The first diagnostic does not establish that the whole Coracle source is otherwise supported. `--trace-passes` identifies the precise `Lnodisclose` form: outer `let* sk`, then `let* lb`, then `seq` of assertions, `Cell.write(state)`, and `elt-ref(sendShielded(...), sent)`. A separate minimal helper that writes a Cell and returns a value already compiles. The failure is root-Let extraction: `stateful-body-ir` only lifts a Let when the final return is one of its bound locals. Coracle uses the locals in preceding actions, while its final returned value does not name them.

### Before and after

Before Compact:

```compact
return sendShielded(
  red_deposit,
  left<ZswapCoinPublicKey, ContractAddress>(ownPublicKey()),
  red_deposit.value
).sent;
```

The Rust emitter already recognizes `__compact_Cell.write` as top-level `StateAction::CellWrite`. The proposed after shape lifts the outer lexical bindings and ordered actions through the existing `StateAction::Let` and retains the separate typed return expression, conceptually:

```rust
let sent_result = send_shielded(&mut context, input, recipient, value)?;
let sent = sent_result.sent;
return Ok(CircuitResult { context, value: sent, gas_cost: total_cost });
```

A minimal independent oracle should test a value-returning helper that writes a Cell before returning a struct member, then compare TypeScript and Rust value, state, ordered effects, and gas. It is a semantic probe, not a substitute for the complete Coracle source.

### Decision to investigate

Lift a root Let when its action suffix contains a structural `__compact_Cell.write`, even if its final returned expression does not name a bound local; retain the earlier returned-binding rule. Reuse existing typed `StateAction::CellWrite`, `Let`, `Sequence`, and stateful return representation. Preserve each binding across every action that uses it; allow it in the final return only while its lexical scope remains live. This is an existing schema-14 shape and needs no IR or runtime ABI bump. An initial broader predicate also lifted assert-only Lets in `welcome.compact`, causing recorded `add_participant` and `add_organizer` to become unavailable. The structural mutation guard preserves their existing expression form and recording. This is a compatibility boundary pending a unified typed effect/value plan; it must not infer that every assertion requires action extraction. Reject unsupported nested forms rather than silently lowering them as pure expressions.

Generated helpers must preserve the existing runtime context/gas contract and upstream ledger query behavior. Reuse ADR-0164's `CoinInfo`/recipient carriers only where the exact qualified coin semantics match. Shielded creation, kernel claims, `writeCoin`, and `sendShielded` require separate ledger-8 semantic checks; this ADR must not claim them merely because the first nested write compiles.

### Guards and acceptance

1. Test root Let with action-local bindings and an independent final return, alongside the existing root Let whose final return uses a bound local. Renderer tests cover prior actions, nested shadowing, and rejection when a binding escapes lexical scope. The effectful conditional remains a separate blocker.
2. Compare the independent TypeScript and native Rust oracle for return value, serialized state, ordered query operations, effects, and gas; test repeated calls to catch duplicated effects.
3. Recompile the full unchanged Coracle source after each bridge. This bounded bridge advances its first diagnostic from line 294 to line 191, where `start` has a separate effectful branch return. All nine exports remain unassessed; no metadata or proof is claimed. The branch condition must be captured before writes if a later bridge lifts those actions. Retain full `welcome.compact` recorded/observed availability as a regression gate.
4. Run focused Scheme/Rust renderer, fixture freshness, source target, inventory and strict recording checks. Broaden only for concrete shared-expression risks. Deliver conventional signed DCO commit locally, no push or remote CI.

### References

Original source `test-center/test-contracts/coracle.compact`; TypeScript generated `_sendShielded_0` and `_red_concede_0`; `compiler/standard-library.compact:161`; ADR-0164 qualified Set insertion.

### Probe results

The new `root_let_action_return_oracle.compact` fails at its Cell write on the prior compiler and compiles after this change. Two sequential TypeScript and native calls return independent echo values while the bound locals cause one read and one write per call; serialized state and ledger effects match, and native four-dimensional gas equals the sum of both TypeScript query meters. TypeScript `reportedGas` omits the read meter for this shape, an existing capture quirk also recorded in ADR-0138. The oracle export remains native-only (`StateReturn::Expression` recording gap), so no proof is claimed. The unchanged Coracle source still fails before metadata at line 191.

### Local gate receipt

Final compiler `${HISTORICAL_NIX_STORE}/7mvk12pjvbd0fpbgqa22vfsr3hxdj0f5-compactc-binary-nixos/bin/compactc-scheme` with current Rust wrapper passed: 139 renderer tests; native oracle parity; 24 Python inventory tests; focused source gate including unchanged Coracle line-191 rejection, strict oracle recording rejection, and all three welcome recorded/observed exports; TypeScript capture recapture comparison; Clippy; workspace format; and all 155 generated fixtures fresh. Full compiler inventory baseline passed: 199 sources, 176 Rust-compiled, 691 exported circuits, 325 proof-required, 321 proof-available, 4 known proof gaps, 20 unassessed exports. Nine Coracle exports remain unassessed because complete source still stops before metadata. The new oracle adds one assessed native-only export. No Rust runtime ABI or private IR schema change.

The first broad predicate was intentionally discarded after fixture freshness exposed a welcome recording regression. The final structural Cell-write predicate preserves the three welcome recorded/observed exports. The source fixture and inventory test assertion were updated to match that existing recorded cohort; formatting of the proof-smoke CLI was normalized for the workspace format gate.

Signed local delivery: `f5daf0504d7292d570849775a6c8c95c1caa0dde` (GPG + DCO); [issue receipt](https://github.com/MediaNoxLabs/compact/issues/273#issuecomment-5993188337). No push or remote CI.
