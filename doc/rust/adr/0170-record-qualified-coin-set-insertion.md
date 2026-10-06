---
id: RUST-ADR-0170
alias: ADR-0170
title: "Record qualified coin Set insertion"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "qualified-coin", "Set"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: d72bf8f67bab5acaca147d9c1c64d1fa55e354e036fa987e132c8ba6163c4d00
---
# RUST-ADR-0170 — Record qualified coin Set insertion

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The qualified Set insert path shares one ledger commitment/program builder across native and recording. The first original ADT proof applies; its early offer is deliberately unbalanced, so funded default-strict production validity is established only by the later funding decision.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#274 closure](https://github.com/MediaNoxLabs/compact/issues/274#issuecomment-6017694100). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
title: ADR-0170 — Record qualified coin Set insertion
status: delivered
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem and original domain

The new unchanged `qualified_coin_set_oracle.compact` has proof-required `insert_coin` and `contains`. The native Rust backend executes `Set<QualifiedShieldedCoinInfo>.insertCoin`, but recording is unavailable at `StateAction::SetInsertCoin`; `contains` already records. The original ADT source `examples/adt/tests/set_qualified_coin_info.compact` has two proof-required exports, each unavailable at the enclosing `Let`. `test_QualifiedShieldedCoinInfo` inserts/removes/resets two fully qualified values. `test_ShieldedCoinInfo` uses `kernel.self()`, the standard `right` helper, two `insertCoin` calls, membership/size assertions, remove, and reset. Both sources emit schema-14 typed IR. The native adapter reuses ledger-8 `CoinInfo`, `Recipient`, commitment and `call_context.com_indices` to obtain the allocated Merkle index. A missing commitment is rejected before querying the VM.

### Before and intended after

Before, the generated native path calls `ledger_slots::coins.insert_coin(context, coin_info_from_compact(...), coin_recipient_from_compact(...))?`, while `recorded::insert_coin` does not exist. After, the emitted recorded path should use the declared typed slot and the same ledger-8 verifier program:

```rust
let frame = ledger_slots::coins.record_insert_coin(
    frame,
    ledger::coin_info_from_compact(coin.nonce, coin.color, coin.value.value()),
    ledger::coin_recipient_from_compact(
        recipient.is_left, recipient.left.bytes, recipient.right.bytes,
    ),
)?;
```

The original ADT exports additionally need lexical typed bindings for literal coin/qualified coin values and the `right(kernel.self())` recipient, then ordered existing Set read/write/assert operations. The emitted public API should remain a normal typed `recorded::*` and `recording().*_call`, not expose VM instructions to developers.

### Proposed decision

Extract the canonical qualified coin insertion Verify program from `runtime-rs/src/ledger/collections.rs` so native and recording invoke one implementation. Add `RecordingFrame::insert_qualified_coin_set` and `SetSlot::record_insert_coin` with alignment and missing-index validation before appending any public operation. Keep the actual index in ledger-8 `com_indices`; never synthesize a caller-provided or default index. In the emitter, admit `SetInsertCoin` only for a declared `Set<QualifiedShieldedCoinInfo>` slot with exact field/index, a typed `ShieldedCoinInfo`, and typed `Either<ZswapCoinPublicKey,ContractAddress>` recipient. Extend the existing typed lexical planner, or a shared bounded typed plan, for the two ADT sequences, including the pure `right` helper and `kernel.self()` source. Preserve source action order and fail closed on any unsupported branch, binding or effect. Coordinate any runtime ABI change with the parent; no schema bump is currently expected.

### Negative guards

Reject a wrong Set element alignment, field/index/path mismatch, malformed coin or recipient type/provenance, unknown/effectful `right` helper, altered or escaped lexical bindings, reordered ADT operations, and unallocated commitment. A missing index must fail before recording a transaction. The public trace must contain the actual ledger allocated index and exact upstream VM effects.

### Acceptance

Fresh original TypeScript capture and native/recorded Rust/replay for both recipients and indices 7/11, missing-index failure, state/effects/private outputs, ordered VM and all four gas dimensions. Reproduce both original ADT exports against fresh TypeScript with the necessary allocated commitments and verify their full ordered Set traces. Pinned ZKIR 2.1.0 proof/verify and ledger-8 validate/apply for `insert_coin` and both ADT exports. Renderer mutation guards, generated fixture freshness, focused local gate, strict Clippy, conventional GPG+DCO commit. Local only: no push or remote CI.

### Research state

Fresh schema-14 IR captured with `${HISTORICAL_NIX_STORE}/s41cjhai0r3449asc8j4ssldfar6c67z-compactc-binary-nixos/bin/compactc-scheme` on 2026-10-05 before implementation. Delivery is local signed commit `8a6f566d`; the scoped receipt is `${LOCAL_EVIDENCE}/compact-adr170-focused-gate/receipt.json` (two source fixtures, four recorded exports).

### Offer-backed observed-call refinement (2026-10-05)

The original `test_ShieldedCoinInfo` fixes two distinct commitments to `mt_index = 0` and makes no output or receive operation. Its synthetic two-index-zero context can establish source-level TS/native/recorded parity only; a real ledger transaction cannot allocate both distinct outputs at index zero. The original qualified-value lifecycle has a legitimate proof/verify/ledger-apply path and passes it.

For a usable `insert_coin` observed call, the public API is an offer-bound wrapper: `OfferBackedObservedState::new(observed, ledger, offer)` validates that the observed contract address and state equal the supplied ledger snapshot, computes `com_indices` through `ledger.zswap.try_apply(&offer, None)`, and retains the owned offer. `wrapper.observed()` feeds the generated typed `*_call`; `wrapper.into_transaction(call, ...)` consumes the same offer and prepared call. Wrong contract snapshot, invalid offer, missing commitment, or call-address mismatch fail before proof. No caller-supplied index map overload is exposed. Ledger validation still checks the snapshot at application time. The runtime owns this provenance boundary; the emitter's typed call signature remains unchanged.

The native `kernel.self()` path previously read `context.query.address` without its TypeScript VM observation. Both native and recorded paths now execute the exact `dup(2)/idx(cached,path 0)/popeq(cached)` address query, preserving repeated reads, gas, and public order. Fresh TS reports only the final query as circuit gas for this original ADT fixture; parity compares the sum of actual per-call query costs (readTime 2,465,000,000 and computeTime 17,921,617,489 for the qualified lifecycle) against native and recorded totals.

#### Provenance refinement after review

A builder that accepts an arbitrary `ContractCallPrototype` and checks only its address is insufficient: another offer or snapshot for the same address could produce that prototype. The actual API takes a `RecordedCall` still tied by reference to the wrapper's exact `ObservedContractState`, checks that identity and its initial `com_indices`, then performs `RecordedCall::prepare` inside the wrapper. It returns an opaque `OfferBoundPreparedCall` that owns both the prepared call and a clone of the same validated offer; only that value exposes `into_transaction`. A mismatched offer-backed observation at the same contract address must be rejected before preparation. Low-level ledger constructors remain explicit unbound APIs for advanced use, but are not described as the provenanced observed route.

### Delivery boundary and proof receipt (2026-10-05)

The original ADT `test_QualifiedShieldedCoinInfo` passed pinned ZKIR 2.1.0 proof/verify and ledger-8 validation/application. The original `test_ShieldedCoinInfo` deliberately assigns two distinct commitments `mt_index = 0` and creates no outputs. Its fresh TypeScript comparison uses an explicit mock index context; native, recorded, and replay match the source's state, ordered VM, effects, and gas under that same context, but this export has no ledger-valid proof receipt. Its accepted scope is source-level parity, not transaction applicability.

The new `insert_coin` proof smoke creates one real user-owned shielded output with value 42 and derives the inserted index from the exact offer through `zswap.try_apply`. The opaque offer-bound observed API checks snapshot and call identity, proves/verifies with pinned ZKIR 2.1.0, and applies to ledger-8. This output is unfunded: default strict `well_formed` rejects it with `BalanceCheckOverspend` for the shielded token in segment 0, `overspent_value: -42`. The smoke then deliberately sets `enforce_balancing = false` and succeeds at VM/proof/application. It does **not** demonstrate a funded production-valid transaction. Funding this offer and passing default strict validation remains a production backlog item.

The fresh TypeScript qualified-value lifecycle reports per-query aggregate `readTime = 2,465,000,000` and `computeTime = 17,921,617,489`; native and recorded totals match that aggregate, while the source `output.gasCost` contains only its final query. The source-level index-0 case is likewise compared by actual per-query aggregate. The `kernel.self()` query is metered in both native and recording, including repeated reads, rather than treated as a free address lookup.
