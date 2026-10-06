---
id: RUST-ADR-0034
alias: ADR-0034
title: "Share expression-nested recorded Field calls"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e01766644788bc29cda8972cefe775b7a64cacbaa298332a319ac5d282fa2412
---
# RUST-ADR-0034 — Share expression-nested recorded Field calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept reuse of supported same-frame Field helpers inside typed expressions, preserving source-order evaluation and errors. The same-source size probe is separate from the expanded fixture's total lines, and the historical external-consumer rebuild was deferred. Broader helper eligibility is not inferred from this nested-expression slice.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#133 closure](https://github.com/MediaNoxLabs/compact/issues/133#issuecomment-6017454810). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`eefde98d`](https://github.com/MediaNoxLabs/compact/commit/eefde98dffd8e4d6281f5c4e055ae8904af79a7f). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 34
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/133
```

## Historical decision and amendments

- Status: Accepted, partial local delivery (2026-10-03)
- Focused issue: [#133](https://github.com/MediaNoxLabs/compact/issues/133)
- Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2)
- Parents: [ADR-0016 — Reuse recorded callee bodies within one frame](0016-reuse-recorded-callee-bodies-within-one-frame.md), #117; [ADR-0005 — Use a typed runtime DSL instead of a body-wide macro](0005-use-a-typed-runtime-dsl-instead-of-a-body-wide-macro.md), #110

### Problem

ADR-0016 shares a private frame-taking body when a Field-returning stateful callee is bound directly. The same callee is inlined when its Expr::Call appears inside a Field expression, even if the helper already exists for another caller. Ledger-8 accepts a circuit body such as value = innerValue() + disclose(ordered()), lowered as StateAction::Let with Expr::Add(Expr::Call(innerValue), Expr::Witness(ordered)). Duplicated generated witness and metering code makes the crate harder to review and can diverge from the direct-call path.

### Before and after generated Rust

Before, the expression-nested call repeats the witnessed callee body at the use site:

    let (frame, witnessed) = frame.try_witness_metered(/* innerValue witness */)?;
    let right = /* ordered witness */;
    let sum: runtime::Field = witnessed + right;
    let frame = crate::ledger_slots::value.record_write(frame, sum)?;

After, it calls the same private helper used by a direct binding, with one shared frame and Compact-order evaluation:

    let (frame, witnessed) = __compact_recorded_body_innerValue(frame, witnesses)?;
    let right = /* ordered witness */;
    let sum: runtime::Field = witnessed + right;
    let frame = crate::ledger_slots::value.record_write(frame, sum)?;

The excerpts show the ownership decision; exact emitted temporary names are compiler generated. Public Contract<W>::recording signatures do not change.

### Decision

Discover supported Field callees recursively in Add and Field Coerce expressions in action bindings or Cell writes. Preflight candidate bodies with the existing recorder. Share one emitter-side typed Field-call lowerer between direct bindings and nested expression calls; keep argument evaluation in source order and propagate errors before later witness calls. Use the existing collision-safe helper naming. Unsupported or recursive shapes retain no recorded entry point.

### Ownership, compatibility and alternatives

- Emitter: tools/compact-rust-backend/src/recorded.rs owns candidate discovery, typed argument lowering, helper reuse, and syn statements.
- Runtime: existing RecordingFrame, typed slots, ledger VM operations and TryWitnesses; no new package API or macro.
- Scheme/private IR: no change, schema 8. Generated/runtime ABI stays 15.
- Alternative: inline every nested call. This duplicates metered witness bodies and leaves one callee with two generated implementations.
- Alternative: body-wide macro expansion. It hides statement order and error propagation from ordinary Rust review.
- Scope: Add and Field Coerce expression shapes. Other expression forms still need independent eligibility and parity work.

### Acceptance and local evidence

The new source circuit outerValueExpr calls innerValue() on the left and disclose(ordered()) on the right, then writes their sum. Independent ledger-8 TypeScript capture records private FAB values [7,13] after previous fixture calls, private state 14, Field Cell value 20, exact serialized ContractState, gas readTime=85000000, computeTime=1233942932, bytesWritten=36, bytesDeleted=36, and one full three-op Verify program. Seven Rust fixture tests compare private FABs, private state, native effects, gas, complete Verify program, state bytes and replay. A rejecting left witness proves that ordered() is never called on error. Fifty-five renderer tests and 132 fixture freshness checks pass. The packaged proof gate now proves, verifies, validates and applies outerValueExpr as its 57th offline call; with a fresh private state of 7 that call writes Field 15. Full all-target Cargo workspace check passes.

A same-source temporary renderer probe reported 649 to 641 generated lines after helper reuse, but it is a local probe rather than a reproducible benchmark. The checked-in fixture adds a source circuit and thus grows overall; no generated-size or compile-time benefit is claimed. No live wallet/node submission, remote CI or release is claimed. The branch remains local and unpushed.

### Review and remaining work

Review the recursive eligibility boundary and generated statement order when adding more Field expression variants. Keep issue #133 open for branch publication, clean remote CI, reproducible size/timing evidence and wider expression/negative coverage. Parent #105 still owns wallet/node submission. Preserve this note as the proposal/delivery history and append dated amendments for later changes.


### Commit and gate amendment — 2026-10-03

Delivered locally in conventional GPG-verified/DCO commit eefde98dffd8e4d6281f5c4e055ae8904af79a7f. The packaged compiler/proof gate passes 57 offline calls including outerValueExpr, with fresh-state Cell Field 15. The pinned wallet byte check from ADR-0033 still passes. All-target workspace check, seven focused fixture tests, 55 renderer tests, 132 fixture freshness and independent TS recapture pass. The checked-in fixture adds a circuit; the same-source 649→641 probe is not an overall generated-size benchmark. The external consumer was not rebuilt after this commit due local disk pressure; the prior clean consumer gate and direct generated Rust type check are narrower evidence. Remote CI, release, wallet/node submission and broader expression eligibility remain open. See [#133 delivery](https://github.com/MediaNoxLabs/compact/issues/133#issuecomment-5963862517).
