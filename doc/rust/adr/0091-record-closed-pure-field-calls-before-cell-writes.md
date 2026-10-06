---
id: RUST-ADR-0091
alias: ADR-0091
title: "Record closed pure Field calls before Cell writes"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "pure-calls", "scope", "typed-slots"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 9af7c67fb220ace9a1d4dd03cd6784c06307a683eb294a4de41caa0ef8a0b0c6
---
# RUST-ADR-0091 — Record closed pure Field calls before Cell writes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted audited acyclic zero-argument pure Field calls bound before a Cell write, using generated pure functions once and existing slots. The bounded arithmetic/call domain excludes witnesses, ledger effects, hashes and unsupported primitives. The named complete API has focused TS/replay/proof evidence; broad Field-call combinations are not inferred.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#193 closure](https://github.com/MediaNoxLabs/compact/issues/193#issuecomment-6017557387). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
adr: 91
status: proposed
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem and measured scope

`call_arg_declared_type.compact::pureBodyFieldOnly` is an exported proof-required circuit. Its typed stateful IR has a `StateAction::Let` binding `Field` from `Expr::Call(fieldOnlyFromPureBody, [])`, then writes that binding to `fieldCell`. The callee is a zero-argument pure Field circuit whose body calls a Field identity circuit with a Field-only literal. Native Rust runs, but the recorded API is absent with `unsupported_expression` at `actions[0].bindings[0].value`. This prevents source-to-proof-to-ledger use even though the only ledger effect is the Cell write. The other 12+ gaps in this fixture have different shapes and are outside this decision.

Before:

```rust
let native = contract.pureBodyFieldOnly(context)?;
// contract.recording.pureBodyFieldOnly_call(&confirmed, private_state) is absent.
```

After:

```rust
let call = contract.recording.pureBodyFieldOnly_call(&confirmed, private_state)?;
let proven = prover.prove(call)?;
ledger.apply(proven)?;
```

Generated replay body should use a typed pure value followed by the existing slot operation:

```rust
let field: runtime::Field = crate::pure_circuits::fieldOnlyFromPureBody()?;
let frame = crate::ledger_slots::fieldCell.record_write(frame, field)?;
```

### Decision and ownership

The emitter may lower a zero-argument `Expr::Call` bound to `Field` when the target is a declared pure `Field` circuit and its transitive body belongs to a checked, effect-free scalar Field subset: Field literals, Field parameters, Field coercions, Field arithmetic and other checked pure Field calls. Reject all hashes, witnesses, ledger observations, collection operations, opaque primitives, non-Field parameters and recursion. The check follows typed `PureCircuit` IR, never circuit names or generated text. Evaluate the existing generated pure function once, bind as `runtime::Field`, and pass the local to existing typed Cell recording. This preserves the Field-only literal representation and source evaluation order. The renderer continues to use `syn` syntax nodes.

`midnight-compact-runtime` already owns `Field` and typed `CellSlot<Field>::record_write`; ledger-8 owns the VM operation, gas and proof semantics. No new runtime API, typed IR variant, capability schema or ABI change is intended. Pure functions that perform a transient hash or other primitive need a separate parity decision; this ADR does not claim them.

### Acceptance and limits

Freeze a compiler binary and report the exact capability delta. Compare the pinned TypeScript and Rust outputs for post-state bytes, ordered public VM effects, private FAB state and all four gas dimensions for `pureBodyFieldOnly`. Run generated crate tests and a source-to-proof-to-ledger verification and application with matching ZKIR/keys. Add a focused negative capability case for an unsupported pure primitive and verify zero loss across the compiled capability inventory. Run local focused gates; leave remote CI deferred under the user direction. No global fixture refresh in the isolated branch; the integration owner will refresh the one affected fixture.

### Tracking

- Source: `examples/rust_backend/call_arg_declared_type.compact`, lines 83–85 and 103–105.
- Baseline: `${LOCAL_EVIDENCE}/adr90-call-arg-after/contract/rust-capabilities.json`, root compiler ABI37/IR9.
- Focused issue: pending creation in `MediaNoxLabs/compact`, milestone `rust-backend-v2` before code.


Focused issue created before code: [#193](https://github.com/MediaNoxLabs/compact/issues/193), assigned to `rust-backend-v2`.

### Local validation checkpoint — 2026-10-05

The isolated branch adds checked zero-argument pure Field call recording with an exact negative hash case. Frozen `compactc` SHA-256 `cf74afcd826d7f9d4dcbd22e97ffc06bf9e69775d52a978ad161f387ce68f55d`, matching schema-10 Scheme SHA-256 `0032952e62bf60226341f941cae89f98b8e446f6f7452c3547af0367a476c290`. Full inventory `${LOCAL_EVIDENCE}/adr91-inventory.json` moved 203/296 to 204/296 proof-required APIs available, 93 to 92 missing, with exactly one changed row (`pureBodyFieldOnly`) and zero membership/losses versus `${LOCAL_EVIDENCE}/adr88-schema10-inventory.json`. Renderer tests 73/73, generated fixture focused gate `${LOCAL_EVIDENCE}/compact-local-focused-adr91/receipt.json`, Rust 1.99 Clippy and TypeScript state/ordered VM/private count/four gas dimensions all passed. The generated `pureBodyFieldOnly.zkir` SHA-256 matches the existing pinned prover/verifier source artifacts byte-for-byte; the independent proof smoke replayed, proved, verified, validated and applied the observed call on ledger-8. Integration owner will run the combined full local gate after cherry-pick. No push or remote CI.

Signed/DCO isolated commit: `bc9f5f83` (`feat(rust): record closed pure Field calls before Cell writes`). Focused issue evidence: [#193 comment](https://github.com/MediaNoxLabs/compact/issues/193#issuecomment-5984164718). No push; integration pending.
