---
id: RUST-ADR-0137
alias: ADR-0137
title: "Record plain Merkle hash append from a typed Bytes32 argument"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merkle", "hash"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 0ed4df6b27859a76f031ddea2780ca2be042fa7dcabff14cd6021d5b7fc0a02e
---
# RUST-ADR-0137 — Record plain Merkle hash append from a typed Bytes32 argument

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted plain Merkle insertion of an already hashed Bytes32 argument through the canonical runtime program, without hashing the bytes again. Original parity and proof/application are established; historic and indexed insertion are separate capabilities.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#239 closure](https://github.com/MediaNoxLabs/compact/issues/239#issuecomment-6017636708). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`ca1798a6`](https://github.com/MediaNoxLabs/compact/commit/ca1798a6cc4e52da17050c43ca5e91e1c42f572b) · [`d9fbd5d9`](https://github.com/MediaNoxLabs/compact/commit/d9fbd5d9b6e419303b84f417feeaa9ecdfacc19e) · [`e22c2bd7`](https://github.com/MediaNoxLabs/compact/commit/e22c2bd799a73891066f660a5e00c539076b2a73). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0137 — Record plain Merkle hash append from a typed Bytes32 argument
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original `examples/rust_backend/merkle_tree_oracle.compact` exports `append_hash(hash: Bytes<32>)`. Schema-12 IR contains one `StateAction::MerkleInsertHash(t, parameter hash)`. The current Rust crate offers only a native call; its capability manifest rejects the recorded and observed APIs at `actions[0]`. Without the ordered public VM program, the call cannot be put into a generated proof-backed transaction. The ledger-8 runtime already executes the corresponding native hash insertion through `merkle_insert_hash`.

### Before and after

Compact source:

```compact
export circuit append_hash(hash: Bytes<32>): [] {
  t.insertHash(disclose(hash));
}
```

Before:

```rust
let native = ledger_contract::append_hash(context, FixedBytes::new([1; 32]))?;
// ledger_contract::recorded::append_hash and an observed-call method are unavailable.
```

After:

```rust
let recorded = ledger_contract::recorded::append_hash(
    context, FixedBytes::new([1; 32])
)?;
let (initial, ordered_verify_ops) = recorded.public.into_parts();
// The generated observed-call method can consume this recorded call for proof.
```

The emitter calls `ledger_slots::t.record_insert_hash(frame, hash)` after checking the declared plain Merkle field, physical path and exact `Bytes<32>` source. The runtime frame executes the same ledger-8 `merkle_insert_hash_program` as native execution and records its ordered Verify ops. The slot remains the typed owner of the field path. No new IR node or ABI is needed.

### Scope and guards

This slice enables only the plain `MerkleInsertHash` action, without treating `HistoricMerkleInsertHash` or indexed hash insertion as equivalent. The emitter rejects a wrong field kind, field index, nested physical path, wrong hash expression/type, or unsupported compound actions. The runtime method accepts `FixedBytes<32>` and the ledger program derives from ledger-8 primitives; it must never substitute a leaf hash computed from the bytes.

### Acceptance

Regenerate the original TypeScript capture freshly. Compare generated native and recorded Rust result, serialized state, all four gas dimensions, ordered VM program, empty private effects, and independent Verify replay with TypeScript. Add a negative renderer guard. Compile a fresh generated crate and test the typed API. Prove/verify and ledger-8 validate/apply the generated observed call using pinned ZKIR 2.1.0 when feasible. Run a focused exact-head local gate. Record capability inventory change and evidence here. No remote CI or push.

### Delivery

Decision recorded before code. Tracking issue and signed commit to follow.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/239 (rust-backend-v2). Created before implementation.

### Local verification (2026-10-05)

- Exact original-source IR: schema 12 has one `MerkleInsertHash(t, hash parameter)` action. Baseline packaged compiler at `ca1798a6` reported `append_hash` unavailable at `actions[0]`. The one-source inventory improved from 4/8 to 5/8 proof-required calls available; `place_hash`, `reset_tree` and the remaining original-source gaps stay explicit.
- Fresh ledger-8 TypeScript capture from the original source changed only the new `nativeQueries.appendHash` entry. The captured call has ten ordered VM operations, zero private outputs, and gas read 935000000 / compute 2276512135 / bytes written 1122 / bytes deleted 1060.
- Generated native and recorded Rust agree on result, serialized state, all four gas values and ledger effects. The recorded Verify program equals the TypeScript operation list, and independent replay reaches the same state, effects and gas. All five generated Merkle fixture tests pass.
- The renderer accepts the exact `Bytes<32>` parameter and rejects mismatched width, field index, historic kind and unsupported conditional hash expression. All 147 generated fixtures are fresh against the local compiler; formatting and diff checks pass.
- Pinned `midnight-zkir 2.1.0` generated prover/verifier keys for the source. The generated `append_hash_call` matched the manual recorded prototype; the local proof verified, ledger-8 validated and applied the transaction, and the resulting state equals native execution.
- Exact integrated-head package and focused gate remain for the parent integration. No remote CI or push.

Signed local commit: `5a5abea8666a10da660070748fd165d15d34be89` (`feat(rust-backend): record plain Merkle hash append`, GPG good, DCO present). Awaiting integration into the milestone branch.

### Integrated checkpoint (2026-10-05)

Agent signed GPG/DCO commit `5a5abea8` cherry-picked cleanly as `e22c2bd7` on `codex/rust-backend-ast`. Combined head passed 110 renderer tests, all five Merkle fixture tests, targeted all-target Clippy, and pinned ZKIR 2.1.0 `append_hash` proof/ledger-8 validate/apply. Exact compiler `${HISTORICAL_NIX_STORE}/qa2cgqwxqda02mraa3ppwpxh782xfqww-compactc` focused gate `${LOCAL_EVIDENCE}/compact-focused-e22c2bd7/receipt.json` passed one fresh source with 5/8 proof calls recorded. Repository inventory `${LOCAL_EVIDENCE}/compact-e22c2bd7-inventory.json` reports 278/316 proof-required available, 38 known gaps, 25 unassessed, 0 unmatched. Prior broad full gate remains the signed `d9fbd5d9` checkpoint. No push or remote CI.
