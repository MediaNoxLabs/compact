---
id: RUST-ADR-0157
alias: ADR-0157
title: "Record typed asset Map writes with class and count guards"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "map", "asset-registry", "mutation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 173de791eee9b872e7181b0b84040ef0c82578a420efcd524b9dc6742e161e4c
---
# RUST-ADR-0157 — Record typed asset Map writes with class and count guards

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the shared asset Map-write extension with audited class guard, cross-map absence and Insert-only Counter increment. Unicode-note Insert and Update are both proved; guard order, branch action count, key/value and slot provenance remain checked. This does not itself admit setWatch.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#261 closure](https://github.com/MediaNoxLabs/compact/issues/261#issuecomment-6017672100). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
title: ADR-0157 — Record typed asset Map writes with class and count guards
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original ledger-8 asset registry `setRecord` is a proof-required exported write but generated Rust exposes only native execution. Its nested scoped `Let` chain contains an additional pure `assertRecordClassKnown(record)` after writable/mutation checks; Insert alone increments `recordCount` after the `recordExists` absence check. ADR-0151 records the related custody-grant Map write but does not admit this more demanding structural shape.

### Before and after

Before: `ledger_contract::setRecord(context, witnesses, id, record, mutation)` performs the typed write, while the capability reports `StateAction::Let` unavailable for recorded execution. After: `ledger_contract::recorded::setRecord(...)` and `contract.recording().setRecord_call(...)` should produce the same typed Insert/Update transition and a proof-ready observed call. The generated API keeps `OpaqueString`, `AssetRecord`, and `RecordMutation` parameters. A nonempty Unicode `note` remains an opaque value inside `AssetRecord`.

### Decision

Extend the ADR-0151 typed opaque-key/struct-value Map-write recognizer structurally. Validate scoped bindings and exact declared Map/Counter slots, transitive `assertWritable` and `recordWrite` helper bodies, a pure class assertion on the bound struct before branching, and the ordered Insert-only Counter increment of literal one. Reuse existing typed ledger-8 Map and Counter recorder primitives and the generated pure circuit; do not add runtime ABI, IR schema, or contract-name special cases. The Update branch must check membership and remove before insert; Insert must check cross-map absence and increment exactly once before insert. Preserve source guard order and messages.

### Negative guards

Reject changed key/value provenance, wrong class-assert argument or altered pure body, reordered class/mutation/writable checks, changed Counter field/index/amount, Counter increment in Update, absent or extra branch actions, wrong Map declaration/key/value, unsupported mutation, and unexpected effectful helpers. Runtime failures must match TypeScript for invalid class, missing Update, duplicate Insert, invalid mutation, closed, and frozen states without applying a write.

### Acceptance

Freshly compile the original Compact source to TypeScript and Rust. Compare nonempty opaque key and Unicode note on Insert and Update against native Rust, recorded Rust, and independent Verify replay: serialized state, ordered VM program, four gas dimensions, private effects/outputs, and witness calls. Exercise all guards, renderer negative mutations, generated fixture freshness, and focused local gate. Produce pinned ZKIR 2.1.0 proofs for both branches; verify and ledger-8 validate/apply. Record a conventional GPG-signed DCO commit and the exact local receipt. No push, remote CI, or Nix rebuild for this slice.

### Delivery

Decision recorded before implementation. Issue and commit to follow.


### Local delivery — 2026-10-05

Issue: https://github.com/MediaNoxLabs/compact/issues/261 (rust-backend-v2). Conventional GPG-signed and DCO commit `77f4b2baaa12096670513955f37bd5ead6ab3c03` extends the shared ADR-0151 recognizer, with no new runtime ABI or IR schema. It validates the pure class assertion on the bound struct, both distinct Map operands in the cross-map existence guard, and the exact declared Insert-only Counter-one action. `setRecord` now emits `recorded::setRecord` and `contract.recording().setRecord_call`; `setWatch` remains unavailable.

Fresh original TypeScript capture with opaque key `record-α-1` and Unicode notes `検査資料 🔒` / `更新済み café` equals the checked-in oracle. Insert and Update match native Rust, recorded Rust, and independent Verify replay on serialized state, 35/31 ordered VM operations, four gas dimensions, private state/effects/outputs, and witness calls. Original TypeScript, native and recorded failures agree for invalid class, missing Update, duplicate Insert, invalid mutation, closed and frozen. Renderer mutations reject changed class provenance/body, order, Counter amount/declaration and collapsed cross-map guard.

Pinned ZKIR 2.1.0 generated `setRecord` keys and circuits; both branches were proven, verified, validated and applied through ledger 8 using the generated observed call. Artifacts: `${LOCAL_EVIDENCE}/compact-adr157-proof`. Local checks: 127 renderer tests, six asset-registry integration tests, strict Clippy, 152 fixture outputs with zero stale/failed, format/diff checks, and exact-head focused gate 9/10 recorded (receipt `${LOCAL_EVIDENCE}/compact-focused-77f4b2ba/receipt.json`). Broad local gate now requires the capability, keys and both-branch proof/apply selector. Branch is local and unpushed; parent full integration gate is pending.
