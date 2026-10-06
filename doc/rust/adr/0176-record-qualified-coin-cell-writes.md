---
id: RUST-ADR-0176
alias: ADR-0176
title: "Record qualified coin Cell writes"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "qualified-coin", "Cell"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 8ce9851024bb0e20413431bb94adc1d3da06d3a4f5a5bb4dafe61aac3f63ea5c
---
# RUST-ADR-0176 — Record qualified coin Cell writes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The shared typed plan records root and chunked qualified-coin Cell writes with exact types and common ledger program construction. The initially unfunded smoke remains distinct from ADR180's default-strict funded proof.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#280 closure](https://github.com/MediaNoxLabs/compact/issues/280#issuecomment-6017704475). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`e751ddf1`](https://github.com/MediaNoxLabs/compact/commit/e751ddf198bf293cb58b5c5abc8dcffdaa28458a) · [`eab9d6f8`](https://github.com/MediaNoxLabs/compact/commit/eab9d6f8d131e759cf2d9d3992b0d722a4ccf4ec). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0176 — Record qualified coin Cell writes
status: delivered-locally
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem and domain boundary

ADR-0173 added exact typed native `Cell<QualifiedShieldedCoinInfo>.writeCoin` with ledger-8 commitment qualification, allocated `com_indices`, and upstream Cell replacement VM semantics. The unchanged `examples/rust_backend/qualified_coin_cell_oracle.compact` still reports `write_coin` as recording unavailable at `StateAction::CellWriteCoin`, so a developer cannot prepare a proof-required observed call for this export. ADR-0170 already established an opaque offer-bound observation API and a recorded qualified Set insertion. This decision extends the same ledger-owned allocation rule to Cell writes. A provisional native Zswap cursor is not proof of a valid allocation; observed recording must use indices derived from the owned ledger offer.

### Before and after developer code

Before, generated code exposes only a native call:

```rust
let next = ledger_contract::write_coin(context, coin, recipient)?;
// ledger_contract::recorded::write_coin and observation.write_coin_call are unavailable.
```

After, the generated crate should expose the same typed parameters through recording and an offer-bound observed call:

```rust
let recorded = ledger_contract::recorded::write_coin(frame, coin.clone(), recipient.clone())?;
let bound = OfferBackedObservedState::new(observed, &ledger, offer)?;
let call = bound.observed().write_coin_call(private, coin, recipient)?;
let prepared = bound.prepare(call, verifier, commitment_randomness)?;
let transaction = prepared.into_transaction(&mut rng, network_id, ttl);
```

The generated method should call the declared typed slot, not present raw VM instructions:

```rust
let frame = ledger_slots::pot.record_write_coin(
    frame,
    ledger::coin_info_from_compact(coin.nonce, coin.color, coin.value.value()),
    ledger::coin_recipient_from_compact(
        recipient.is_left, recipient.left.bytes, recipient.right.bytes,
    ),
)?;
```

### Emitter and runtime decision

Admit `CellWriteCoin` in the existing shared typed planner only when its field/index/path resolves to a declared `Cell<QualifiedShieldedCoinInfo>`, the coin has the exact `ShieldedCoinInfo` shape, and the recipient has the exact `Either<ZswapCoinPublicKey, ContractAddress>` shape. Preserve source operand order and lexical scope. Add `CellSlot::record_write_coin` and `RecordingFrame::write_qualified_coin_cell`. Extract ADR-0173's canonical `qualified_coin_cell_write_program` as a shared runtime builder, and invoke it in both native and recorded paths; never copy its root/nested path VM operations into a second implementation. Use the common `qualified_coin_commitment` preflight to enforce value alignment and offer-allocated commitment presence before mutating a frame. The VM itself reads the actual index from `com_indices`. Reserve runtime ABI 44 if this generated-facing frame/slot API lands after ADR-0175's ABI 43; no new IR variant or schema bump is expected.

### Negative guards and provenance

Reject wrong Cell declaration/alignment, field/index/path mismatch, coin or recipient type mismatch, escaped/altered binding, missing commitment, wrong recipient, malformed state, and unsupported nested depth. A missing index or invalid offer must fail before any recorded public query. The observed route must retain exact owned offer, observed identity, and initial context checks from ADR-0170; no caller-supplied index map or unbound prototype is accepted by the bound API. Root and nested Cell programs must match the upstream path and stack depth exactly. Replacement must preserve state, effects, and private transcript order.

### Acceptance and known proof limit

Capture fresh original TypeScript for both recipients, root/nested paths, zero/default and occupied replacement, and actual allocated indices; compare generated native, recorded, and replay results for state, VM sequence, public effects, private outputs, and all four gas dimensions. Check missing index, wrong recipient, malformed state, wrong slot/field/index/type, and nested-depth failures. Refresh only affected generated fixtures; run focused renderer, Cargo, Clippy, and local parity gate.

Use pinned ZKIR 2.1.0 proof/verify and ledger-8 validation/application with an offer-derived index. If the offer is unfunded as in ADR-0170's smoke, assert default-strict `BalanceCheckOverspend`, then label any balancing-disabled application explicitly as a proof/VM/application smoke. A funded default-strict transaction remains issue #105 work. Do not claim the source-level mock context alone proves transaction applicability. Deliver a conventional GPG+DCO signed local commit; do not push or run remote CI.

### Delivery evidence (2026-10-05)

The shared runtime builder is `qualified_coin_cell_write_program_for_context`, which performs common ledger commitment and allocation preflight and delegates the VM construction to ADR-0173's one canonical `qualified_coin_cell_write_program`. Native and recorded paths call that same entry. The typed planner accepts one exact qualified Cell replacement with exact coin and recipient types; generated root and chunked `write_coin` now have both `recorded` and `observed_call` capability. A Cell-specific field audit permits the compiler's validated one- or two-segment physical path and exact field/index, while earlier typed plan families explicitly require zero qualified Cell writes. The chunked generated crate passed `--rust-require-recording` and standalone Cargo check from `${LOCAL_EVIDENCE}/compact-adr176-chunked-rust`.

Fresh TypeScript was recompiled from both the unchanged root oracle and the chunked fixture with the combined schema-16 Scheme frontend. Both resulting captures exactly equal the stored independent TypeScript JSON. Native, recorded, and Verify replay match each other and the TypeScript query's complete ordered VM program, public state/effects, zero private outputs, and all four gas dimensions for right/left recipients, indices 7/11/0, and occupied replacement at 17. For root right7: readTime 85,000,000, computeTime 1,246,766,404, bytesWritten 78, bytesDeleted 56. For nested right7: readTime 255,000,000, computeTime 1,430,161,265, bytesWritten 620, bytesDeleted 598. Replacement differs as expected and is asserted from the source capture. Missing or wrong recipient allocation, wrong Cell alignment, malformed VM state, too-deep path, wrong emitter field/index/type, escaped binding, and invalid path are rejected.

The final focused local parity receipt is `${LOCAL_EVIDENCE}/compact-adr176-final-gate/receipt.json` (one source fixture, two of two exports recorded). The persistent pinned ZKIR 2.1.0 artifacts are `${LOCAL_EVIDENCE}/compact-adr176-cell-proof`. `write_coin` replayed, proved, verified, and applied with the actual offer-derived index. Its test output 42 is unfunded: default strict validation rejects `BalanceCheckOverspend` in segment 0 with `overspent_value: -42`; application succeeds only with `enforce_balancing = false`. This is a proof/VM/application smoke, not a funded production-valid transaction. Strict Clippy, renderer guard, runtime canonical program test, generated fixture freshness, and both root/nested parity tests pass. Temporary ABI-only fixture refresh was restored, leaving only the qualified Cell fixture in local GPG+DCO commit `07b00b57`.

### Main integration — ADR176

Integrated as `eab9d6f8`, verified GPG+DCO. Root and chunked Cell writes support recorded/observed calls. Prior typed-plan families reject accidental qualified Cell composition. Combined proof smoke verifies/applies and retains the default-strict unfunded rejection; funded acceptance is ADR180/#284 linked to#105.

Current checkpoint `e751ddf1` is private schema20/runtime ABI45.162 fresh fixtures (zero failures),8 backend units+144 renderer tests,9 runtime units,27 Python tests, targeted strict Clippy and formatting pass. Seven-source frozen focused gate: `${LOCAL_EVIDENCE}/compact-focused-e751ddf1/receipt.json`. Combined Cell proof log: `${LOCAL_EVIDENCE}/compact-e751ddf1-cell-proof.log`.

Full source inventory:333/343 assessed proof-required APIs,10 known gaps,20 unassessed exports across205 sources/717 exports. Receipt: `${LOCAL_EVIDENCE}/compact-e751ddf1-inventory.json`. Native admission expands the assessed gap denominator; retained existing capabilities pass regressions. Latest completed broad gate remains4ef0fa57, not a claimed ABI45 full run. No push/remote CI.
