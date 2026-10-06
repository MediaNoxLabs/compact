---
id: RUST-ADR-0110
alias: ADR-0110
title: "Record typed vector hash Boolean Cell assertions"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "typed-hash", "cell", "assertions"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 1d255f2b7252306184f63412b42b9bb794efe8e00427e6f34c07dcdd641fce01
---
# RUST-ADR-0110 — Record typed vector hash Boolean Cell assertions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted a declared two-Field pair/vector parameter hash only within the bounded Field Cell equality or inequality assertion path. Hash, Cell read, assertion and Counter increment retain source order; dynamic or wider hash operands remain excluded.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#213 closure](https://github.com/MediaNoxLabs/compact/issues/213#issuecomment-6017591605). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`1e521ffc`](https://github.com/MediaNoxLabs/compact/commit/1e521ffcdcfd36d6019f4ef96d86527a48460f73). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 110
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/213
```

## Historical decision and amendments

### Problem
The schema-11 `inlinedAssert` circuit executes natively, but its first `StateAction::Assert` is unavailable in the recorder. The called `vecDiffers(v: Vector<2,Field>): Boolean` compares `transientHash(v) != fieldCell`; the Boolean recorder can enter the value helper and record a declared Field Cell read, but cannot lower that typed pure hash operand. The following Counter increment has no replay path until the assertion is recorded.

### Before and after generated Rust
Before: `ledger_contract::inlinedAssert(context)?` exists while `ledger_contract::recorded::inlinedAssert(context)` and `Contract.inlinedAssert_call(&observed, ())` are absent. After: the generated recorder binds the exact two-Field vector argument once, evaluates `runtime::transient_hash(vector)` before the typed `fieldCell.record_read`, compares the Fields, enforces the assertion, then records `asserts.record_increment`. The public API gains recorded and observed methods.

### Decision and ownership
Inside Boolean equal/not-equal lowering for a declared Field Cell comparison, permit a `TransientHash` operand only when its source is a direct declared two-Field vector or tuple parameter. Obtain the source through the existing typed Cell value path, emit the runtime hash once, and leave the Cell read to the RecordingFrame typed slot. Preserve structured unsupported reasons for wider/dynamic/untyped hashes and other expressions. No schema, generated API shape, or runtime ABI change; the runtime hash and ledger primitives already own encoding and gas.

### Acceptance
Check TS/native/recorded full state, ordered VM, four gas dimensions, private state and FAB, plus typed observed/manual replay. Prove/verify/validate/apply inlinedAssert with pinned schema-11 ZKIR 2.1.0 and ledger-8. Run focused renderer, fixture, rejection, inventory and formatting gates; signed GPG+DCO commit. No push or remote CI.

### Tracking
MediaNoxLabs issue https://github.com/MediaNoxLabs/compact/issues/213 in rust-backend-v2. Branch codex/adr110-declared-type, base 1e521ffc.

### Local delivery evidence
- Exact pinned compiler package qm7cvdcz9v1hjzkbs6j5iamyrmf6jq7j generated 16 circuit keys/ZKIR 2.1.0; typed observed/manual replay, proof verification, ledger validation and application passed for inlinedAssert.
- TypeScript oracle matches native and recorded full state, ordered public VM operations, query gas in four dimensions, private transcript and FAB; no private output.
- 86 renderer tests and all generated call-argument tests passed. The focused renderer rejection keeps non-parameter hash assertions unsupported.
- 145 fixture outputs have zero stale/failure; 4 source rejections and proof capability checks passed; formatting, Clippy and Python syntax passed.
- Inventory changed exactly one row: inlinedAssert unavailable to available. Coverage is 237/305 proof required (68 gaps), versus 236/305 on base 1e521ffc. witnessBare and impureConst remain unsupported.

Delivery commit: 9d6b46f8efb076927239c8b9abb56fc4f70bb11f (verified GPG signature and DCO). Local only; ready for integration into milestone branch.
