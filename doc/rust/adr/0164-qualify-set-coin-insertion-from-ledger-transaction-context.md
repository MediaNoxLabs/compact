---
id: RUST-ADR-0164
alias: ADR-0164
title: "Qualify Set coin insertion from ledger transaction context"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-native-only"
topics: ["compiler", "qualified-coin", "Set"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 9a4fdf9270e41ac0e660bae36f871749798d4952e45ee8e293d26ab7a48d3cdc
---
# RUST-ADR-0164 — Qualify Set coin insertion from ledger transaction context

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-native-only. Typed qualified-coin Set insertion uses ledger commitment/index qualification and admits the previously unassessed original ADT exports natively. The insertion remains explicitly unavailable for recording in this decision; no insertion proof is claimed here.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#267 closure](https://github.com/MediaNoxLabs/compact/issues/267#issuecomment-6017681670). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

## Historical decision and amendments

Status: Implemented in isolated delivery · 2026-10-05 · rust-backend-v2 · [Issue #267](https://github.com/MediaNoxLabs/compact/issues/267)

### Problem

The unchanged `examples/adt/tests/set_qualified_coin_info.compact` compiles for TypeScript but Rust stopped at line 58 (`c.insertCoin(...)`), leaving both proof-required exported circuits unassessed. Ledger-8 `Set<QualifiedShieldedCoinInfo>.insertCoin` commits a coin for a recipient, looks up the transaction-allocated Merkle index in `QueryContext.call_context.com_indices`, and inserts the qualified coin through a metered VM program. Ordinary Set insertion or a fixed index changes semantics.

### Before and after

Original Compact remains unchanged:

```compact
c.insertCoin(coin, right<ZswapCoinPublicKey, ContractAddress>(kernel.self()));
```

Before, the Scheme Rust IR pass rejected this ledger operation before metadata. After, the compiler emits a typed `SetInsertCoin` state action. The Rust renderer evaluates coin then recipient once, retains their source scope, and emits a call equivalent to:

```rust
let coin = /* typed ShieldedCoinInfo */;
let recipient = /* typed Either<ZswapCoinPublicKey, ContractAddress> */;
let step = crate::ledger_slots::c.insert_coin(
    context,
    runtime::ledger::coin_info_from_compact(coin.nonce, coin.color, coin.value.value()),
    runtime::ledger::coin_recipient_from_compact(
        recipient.is_left, recipient.left.bytes, recipient.right.bytes,
    ),
)?;
```

`CoinInfo` and `CoinRecipient` are upstream ledger-8 primitives. The same `CoinInfo` supplies the commitment and the VM payload, preventing mismatch between the looked-up commitment and inserted coin. The runtime checks `Set<T>` alignment against upstream `QualifiedInfo`, requires the commitment in the transaction map, and executes the exact ten-op ledger VM insertion (`idx, dup, push, idx, push, swap, concat, push, ins, ins`). The query consumes the transaction-allocated index, including nonzero indices. Both contract and user recipients are supported.

### Compiler and runtime boundary

The new private JSON action requires IR schema 14 (from 13); old-schema input is rejected. The public generated-crate helper requires runtime ABI 39, coordinated with ADR-0163's planned ABI 38. This isolated delivery changes only the new generated fixture to ABI 39; the integrated branch must refresh earlier generated fixtures after combining both ABI changes.

The Scheme pass admits only the ledger-qualified Set operation and emits typed coin and recipient operands. The Rust IR parser, renderer, and source gate check exact field/index, `Set<QualifiedShieldedCoinInfo>`, `ShieldedCoinInfo`, `Either<ZswapCoinPublicKey, ContractAddress>`, source scope, and stale or malformed schema. Unsupported shapes fail closed. Existing ordinary Set behavior remains distinct.

### Evidence and attribution

The complete original source now compiles for Rust, with both exported proof-required circuits reported accurately as native-only: `recorded=false`, `observed_call=false`; strict recording rejects them. The separate oracle has a recorded `contains` circuit and a native-only `insert_coin` circuit. No insertion proof is claimed. Its independent TypeScript capture and Rust fixture match serialized state, empty effects, ordered query operations, and all reported/per-query gas dimensions for a contract recipient at allocated index 7 and a user recipient at index 11. Missing commitment and wrong Set alignment reject before VM execution.

Focused checks passed: 126 backend renderer tests; native oracle parity; 24 inventory tests; exact ADT Set source gate; focused fixture freshness; all five ADT Set sources compile for both TS and Rust with five proof-required exported circuits; focused Clippy and workspace format. The isolated inventory reports 198 sources, 175 Rust-compiled, 690 exports, 324 proof-required, 300 proof-available, 24 known proof gaps, and 20 unassessed exports. The two original qualified-Set exports moved from unassessed to assessed native-only. The broader integrated gate and all-fixture ABI refresh belong to the parent integration checkpoint.

### Delivery

Conventional GPG-signed DCO commit, local only. No push or remote CI.
