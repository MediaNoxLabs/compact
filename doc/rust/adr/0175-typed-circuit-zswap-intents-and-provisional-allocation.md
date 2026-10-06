---
id: RUST-ADR-0175
alias: ADR-0175
title: "Typed circuit Zswap intents and provisional allocation"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-native-only"
topics: ["compiler", "Zswap", "intents"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 1480305303f0a743f8bb3eb4b15d73d82c0119347ce903bcfd817f65c122f14a
---
# RUST-ADR-0175 — Typed circuit Zswap intents and provisional allocation

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-native-only. Typed native Zswap input/output intents preserve source order and cursor effects. This is not a recorded, proved, funded or offer-reconciled flow; later decisions provide those boundaries.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#279 closure](https://github.com/MediaNoxLabs/compact/issues/279#issuecomment-6017702648). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
Original unchanged micro-dao advances beyond qualified Cell.writeCoin and stops at native createZswapOutput/createZswapInput. Compact TS records ordered qualified inputs and coin/recipient outputs plus a currentIndex cursor. The pinned midnight-zswap 8.0.3 local::State instead holds wallet coins, pending maps, a Merkle tree and first_free. Those are different responsibilities and must not be conflated.

### Before / after
```compact
createZswapOutput(coin, recipient);
createZswapInput(input);
```
Before: unsupported native witness action/expression. After: typed IR expressions validate upstream-compatible coin/recipient types and evaluate operands left to right; generated Rust calls context methods and appends one empty aligned private transcript output per native call. A CircuitZswapPlan uses ledger CoinInfo, QualifiedCoinInfo and CoinRecipient, an ordered input/output list, and a bounded provisional cursor. It is independent of wallet local::State.
```rust
context.create_zswap_output(coin_info, recipient)?;
private_transcript_outputs.push(runtime::fab::AlignedValue::from(()));
```

### Allocation policy
Native execution may use a configurable provisional start cursor (default0 for Compact TS parity). Output computes the existing upstream commitment, inserts its provisional index into execution com_indices, increments the cursor, and appends its intent. This is not a validated ledger offer or allocated Merkle leaf. Observed/offer-backed contexts lock provisional allocation: create_output and cursor setters reject explicitly, with no map/plan mutation. Lock is private and irreversible. Later authoritative intent/offer reconciliation needs a separate decision and proof evidence; current ADR0170 funded-offer smoke still requires relaxed balancing (#105).

Inputs append ordered qualified coin intents without wallet mutation or additional public VM operations. Both native builtins emit an empty aligned private transcript value, matching independent generated TS wrappers. No recording/proof claim is made in this slice. RecordingFrame.call_local snapshots and compares every plan and allocation-mode field to prevent the native-helper bridge from hiding side effects.

### Boundaries
Schema18 / runtimeABI43. Rust provisional cursor is u64; checked overflow rejects before all effects. TS currentIndex is bigint and the current runtime even accepts negative and greater-than-u64 values at its JavaScript boundary; Rust deliberately refuses that unbounded domain. Constructor/context/result conversions retain plan and lock state. Existing wallet state remains a separate upstream value.

### Validation plan
Independent TS native source covers input/output ordering, nested helpers and selected branches, left/right recipients, nonzero start indices, private transcript values, zero-query/gas effects, duplicate commitments, writeCoin after provisional output, read result and state. Negative typed IR args/arity, locked contexts and overflow before mutation; audit local-call rejection of new effects. Original micro-dao compilation must advance unchanged and identify the next builtin or operation honestly. Focused fixture freshness, renderer tests and targeted Clippy; no broad old-fixture refresh, no new target cache, no pushes/remoteCI. Signed DCO/GPG delivery and exact receipt.


### Accepted delivery — 2026-10-05
Issue #279, milestone rust-backend-v2. Signed DCO/GPG commit `88ef1dd672125171797a9601618bd7d4ed40be29`; schema18/ABI43.

Implemented typed `CreateZswapInput { coin }` and `CreateZswapOutput { coin, recipient }`, guarded declared types and left-to-right temporary bindings. Runtime owns `CircuitZswapPlan` independently of wallet state and preserves private output order. Public plan access is read-only; cursor setter rejects after output production or when locked. Observed contexts lock provisional output creation; the existing offer allocation map must be copied before locking at ADR0170 integration. No authoritative map is changed by these native methods.

Each output intent includes its provisional index. Constructor transitions retain the log and irreversible lock and reconstruct only that provisional map. `ConstructorResult` does not retain arbitrary offer/call-context allocation maps; no wider roundtrip claim is made. `call_local` compares the entire new plan, including allocation mode.

Nine independent original TS cases passed: direct Unit-return consume/produce, start0/7, both recipient arms, duplicate output commitments through a local helper, conditional selection, two witnesses in coin→recipient order and private-state progression, exact private transcript outputs, zero-query/gas native effects, and subsequent qualified Cell write/read selecting index8. Four runtime tests cover overflow before all effects, cursor reset rejection, plan/conversion retention, nonempty locked-map preservation and helper-audit rejection of input/output/cursor/mode changes. Eight library +12CLI +141existing renderer tests and a new typed-negative renderer test passed; malformed args/arity and pure misuse reject. Native source gate, targeted allfeatures/alltargets Clippy, formatting and one fresh generated fixture passed. Older ABI fixtures await parent combined refresh.

Signed-head receipt: `${LOCAL_EVIDENCE}/compact-focused-88ef1dd6-native-zswap-intents/receipt.json`, passed; 1/5 recorded (read_coin), three native-only exports are proof-not-applicable, flow remains an explicit recording gap. No native-intent proof or funded-offer claim. Compiler `${LOCAL_EVIDENCE}/compact-adr175-compactc`; Scheme `${HISTORICAL_NIX_STORE}/l7xng37s989xbiwchbkdy0pkajxb015b-compactc-binary-nixos/bin/compactc-scheme`.

Original unchanged micro-dao progresses to `<standard library>: Rust backend requires an array-index ledger query path`; source remains unassessed. The next work is explicit kernel/native ledger query admission and later authoritative offer/intent reconciliation, with strict funded-offer validation still tracked by #105.
