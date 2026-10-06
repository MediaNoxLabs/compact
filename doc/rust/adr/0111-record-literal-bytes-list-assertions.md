---
id: RUST-ADR-0111
alias: ADR-0111
title: "Record literal Bytes List assertions"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "list", "bytes"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 463f3cdd39664426b555e5cef50ca14782651214fed85a0496344f8641694732
---
# RUST-ADR-0111 — Record literal Bytes List assertions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted exact-length literal Bytes List-head assertions through matching Maybe types and validated literal or scoped values. Original ListBytes128 parity and proof/application are demonstrated; opaque or dynamic byte construction remains outside this decision.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#214 closure](https://github.com/MediaNoxLabs/compact/issues/214#issuecomment-6017593374). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1e521ffc`](https://github.com/MediaNoxLabs/compact/commit/1e521ffcdcfd36d6019f4ef96d86527a48460f73). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 111
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/214
```

## Historical decision and amendments

### Problem and source evidence

The TypeScript ADT suite compiles `examples/adt/tests/list_bytes.compact::test`, a proof-required circuit over `List<Bytes<128>>`. On integrated Rust backend `1e521ffc`, schema-11 source compilation succeeds and native Rust is emitted, but recorded capability stops at `actions[0]`: a scoped `Bytes<128>` Let. The Scheme frontend already lowers `pad(128, "0")` and `pad(128, "1")` to exact 128-byte `Expr::BytesLiteral` values. Recorded lowering has typed Bytes Let and List push support but `cell_source` does not admit that closed literal; the later `head().value == zero/one` comparisons also lack a Bytes head guard. The source has no parameters, witness, dynamic Bytes operation or pure call.

### Before and after generated Rust

Before, the generated native `test` accepts `FixedBytes<128>` and executes the List assertions, while `recorded::test` and `Contract::recording.test_call` are omitted by capability analysis.

After, a typed closed literal is bound once and reused by each ordered recorded List operation:

```rust
let zero: runtime::FixedBytes<128> = runtime::FixedBytes::new([48, 0, /* exact 128 bytes */]);
let (frame, observed): (_, crate::types::Maybe) =
    crate::ledger_slots::c.record_head::<crate::types::Maybe, _, _>(frame)?;
if observed.value != zero {
    return Err(runtime::CompactError::AssertionFailed(message.into()));
}
```

The emitted AST uses hygienic identifiers, generated literal bytes, and the precise `Maybe<Bytes<128>>` representation. The example omits only repeated zero bytes for readability.

### Decision and ownership

Keep schema-11 typed `Expr::BytesLiteral`, `StateAction::Let`, `Expr::ListHead`, and `Expr::StructField(value)` without a new IR kind. `cell_source` admits a BytesLiteral only when its length exactly matches declared `Type::Bytes`; it calls the existing typed expression renderer and retains the resulting `FixedBytes<N>` value. The recorded head comparison admits Bytes only when the declared List element and compiler head result match exactly; the expected side remains a validated same-type literal, parameter, or scoped local. Reuse existing `ListSlot<FixedBytes<N>>`, runtime codec, recording frame and ledger-8 VM semantics for state, gas and proof inputs. No Scheme or runtime API change is proposed. Dynamic Bytes operations, mismatched lengths and opaque elements remain unavailable.

### Acceptance and limits

Capture a fresh TS oracle and compare generated native, recorded and observed calls for result, initial/final serialized state, all four gas dimensions, ordered public VM and private output; Verify replay. Run pinned ZKIR 2.1.0 proof generation, verification and ledger-8 application with final state matching TS. Add typed renderer negative guards for byte length mismatch and opaque List head; admit only this source into checked acceptance after full parity. Expected exact inventory movement from integrated `1e521ffc`: +1 exported proof-required circuit and +1 available, with missing/unassessed totals unchanged. Other ADT List source gaps remain separate.

### Tracking

- Predecessors: [ADR-0101 — Record nested List queries in ADT assertions](0101-record-nested-list-queries-in-adt-assertions.md), [ADR-0106 — Record List enum head comparisons](0106-record-list-enum-head-comparisons.md), [ADR-0109 — Record List Field vector head comparisons](0109-record-list-field-vector-head-comparisons.md).
- Issue: [#214](https://github.com/MediaNoxLabs/compact/issues/214).
- Delivery: signed GPG/DCO local commit `4c9b9043e988b901cd006cc0ea5b0db1f3984d52` based on integrated `1e521ffc`; parent integration pending.

### Local validation, 2026-10-05

- Frozen schema-11 frontend SHA-256 `2851285051fcfa9f70e637cb091132ddefdfdd83a50b09d612ad6a3f043580ed` and final Rust `compactc` SHA-256 `b6d480d8f85b26f53312bae24b1b0408de5883ef4f17462857af5a16d6a4ad6a` compiled TS and Rust source. Compiler metadata confirms `test` is `pure:false, proof:true`; generated capability is `recorded:true, observed_call:true`. Schema 11 and runtime ABI 37 are unchanged.
- Fresh TS oracle `runtime-rs/tests/fixtures/adt-list-bytes.json` has 16 queries, 160 ordered public VM operations, zero private outputs and unit result. Aggregate gas: readTime 85,000,000; computeTime 1,234,450,028; bytesWritten 212; bytesDeleted 412. Generated crate test passed native and recorded result, initial/final serialized state, effects, four TS gas dimensions, VM shape/order and Verify replay. Generated locals `__compact_recorded_bytes_0/1` retain exact `FixedBytes<128>` literals and are reused in pushes and head assertions.
- Pinned ZKIR 2.1.0 emitted `test.bzkir` SHA-256 `4276734ae27be80561bf6310754004158824633ccb248b89d56400be1aae294f`. Proof generation, verification and typed observed call application through ledger-8 passed; final List of `FixedBytes<128>` is empty. Artifacts: `${LOCAL_EVIDENCE}/adr111-list-bytes-final-proof`.
- Renderer negative guards reject mismatched Bytes literal length and leave OpaqueString List head recorded/observed unavailable. The exact final compiler reproduced the checked-in fixture byte-for-byte. 146 generated fixtures had zero stale/failed; 86 renderer tests and 22 inventory tests passed. Checked singleton source scope accepted TS/Rust/proof metadata; focused deterministic gate `${LOCAL_EVIDENCE}/adr111-focused-gate/receipt.json`, targeted generated crate test, rustfmt, Clippy `-D warnings` and `git diff --check` passed.
- Exact integrated-base inventory movement: declarations 934→935, proof-required 305→306, available 236→237, missing 69 unchanged, unassessed exports 36 unchanged. The one added row is `examples/adt/tests/list_bytes.compact::test`.
- No Scheme IR, runtime, ledger package or user documentation was changed. Other List source blockers remain separate.
