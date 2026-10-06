---
id: RUST-ADR-0173
alias: ADR-0173
title: "Native qualified-coin Cell writes"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-native-only"
topics: ["compiler", "qualified-coin", "Cell"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 3609b15727cb72d35030600f262202fd997a903ac4ead42b2828261e2cf7f317
---
# RUST-ADR-0173 — Native qualified-coin Cell writes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-native-only. Typed qualified-coin Cell writes reuse ledger commitment and allocation primitives in native code. Recording and proof were explicitly deferred in this slice; allocation-table parity is not an applicable offer.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#277 closure](https://github.com/MediaNoxLabs/compact/issues/277#issuecomment-6017699564). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
Original micro-dao now reaches pot.writeCoin at line171 after ADR0171. Cell<QualifiedShieldedCoinInfo>.writeCoin is a ledger8 operation that appends the transaction-allocated Merkle index to ShieldedCoinInfo before replacing the Cell. Rust currently supports qualified Set insertion but no equivalent Cell operation.

### Before / after
```compact
pot.writeCoin(disclose(coin), right<ZswapCoinPublicKey, ContractAddress>(kernel.self()));
```
Before: unsupported ledger operation. Proposed typed native emission:
```rust
let step = ledger_slots::pot.write_coin(context, coin_info, recipient)?;
```
CoinInfo and CoinRecipient remain midnight-ledger carriers via existing Compact conversion functions. A new CellWriteCoin typed action validates qualified Cell declaration, coin and recipient types and operand evaluation order. Schema16 / ABI42 reserved with parent; ADR0170 owns ABI40 qualified Set recording, ADR0171 ABI41.

### Runtime design
Reuse common validation of QualifiedCoinInfo alignment and an existing coin.commitment(recipient) entry in QueryContext.call_context.com_indices. Never allocate or fabricate an index. Factor shared validation with Set insertion while retaining actual index lookup in ledger VM. Cell program follows upstream: optional parent idx, final-key push, correctly offset context dup, commitment push, context-map idx, coin push, swap, concat91, Cell ins, optional parent ins. Preserve path bounds and reject unsupported depth before executing.

### Recording boundary
This slice initially admits native Cell writes and reports a structured recording gap. Recording requires its own audited composition and proof receipt; no recorded/provable claim will be made for unsupported CellWriteCoin.

### Validation
Independent generated TS oracle for zero/default and occupied replacement, public-key and contract recipients, allocated index values, missing commitment, wrong recipient and malformed Cell type. Compare state, effects, query gas/program and private outputs. Use the same supplied allocation table semantics as ADR0164; production allocation remains ledger-owned. A conditional branch probe covers operand order where bounded. Recompile original source unchanged and report subsequent blockers. New fixture only refreshed; parent handles combined schema/ABI updates. Local tests and signed DCO/GPG delivery; no push/remoteCI.


### Delivery — 2026-10-05
Accepted for native execution; issue #277; signed DCO/GPG commit `e2edd98fc5b27742238c2b41c0b354eff1f93eac`. Schema16 / ABI42.

The emitter uses typed `CellWriteCoin` operands and declared Cell shape; runtime factors `qualified_coin_commitment` with Set insertion. It preserves the actual VM lookup and concat91. The independent original TS compiler generated root and 17-field chunked contracts, so both root path `[0]` (8 VM ops) and nested path `[1,14]` (10 ops) are checked exactly.

Evidence: fresh contract recipient index7, user recipient index11, index0 and occupied replacement index17 all match state, effects, query/program gas and private outputs. Missing/wrong-recipient commitment rejects before any query; malformed ledger shape with a valid allocation reaches a VM rejection after one attempted query. Failed-call gas is not exposed by the Rust error API and is not claimed equivalent. Three native fixture tests, exact-program runtime unit, 8 library +12 CLI +141 renderer tests, typed negative guards, native compiler source gate, targeted Clippy, formatting and one fresh fixture passed. Focused signed-head receipt `${LOCAL_EVIDENCE}/compact-focused-e2edd98f-qualified-coin-cell/receipt.json` reports 1/2 recorded: read_coin only.

Frozen Scheme: `${HISTORICAL_NIX_STORE}/xxa6hlrjvdddgrh9wc112kjzqybvy4v9-compactc-binary-nixos/bin/compactc-scheme`; native compiler `${LOCAL_EVIDENCE}/compact-adr173-compactc`. Older generated ABI fixtures are intentionally left for the coordinated integration refresh. No Cell write proof or recording claim.

Original unchanged micro-dao advances beyond pot.writeCoin to `<standard library>: Rust backend does not yet support this native witness expression`; the remaining family includes createZswapOutput/Input. The oracle takes an explicit recipient to isolate this operation; original kernel.self gas depends on ADR0170. Allocation tables are test inputs matching TS createZswapOutput, not evidence of a ledger-valid offer. The planned conditional operand probe was not needed for this bounded typed action admission and is not part of the delivered evidence. ADR0175 will research typed circuit Zswap intent accumulation separately from wallet state.
