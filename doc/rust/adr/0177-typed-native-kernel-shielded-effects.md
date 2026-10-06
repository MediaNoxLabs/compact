---
id: RUST-ADR-0177
alias: ADR-0177
title: "Typed native Kernel shielded effects"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-native-only"
topics: ["compiler", "Kernel", "ledger"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 5fa9705534a625cde9f40ca50e9e7b705caf1bf4f1412830eaf8fcf2663074c9
---
# RUST-ADR-0177 — Typed native Kernel shielded effects

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-native-only. Native Kernel effects use the special Kernel ADT and a strict upstream VM operation allowlist. Seven proof-required APIs remained unavailable for recording here; ADR184 adds a separate recorded profile.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#281 closure](https://github.com/MediaNoxLabs/compact/issues/281#issuecomment-6017706517). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`24654ee5`](https://github.com/MediaNoxLabs/compact/commit/24654ee56624e2b66a6305328bc2d2822cd3ca4b) · [`e751ddf1`](https://github.com/MediaNoxLabs/compact/commit/e751ddf198bf293cb58b5c5abc8dcffdaa28458a). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
After ADR0175, original micro-dao reaches Kernel ledger operations with empty special paths. Treating Kernel as a normal user ledger array field yields an array-index diagnostic. The source uses mintShieldedToken/receiveShielded/sendShielded helpers, whose native core updates the ledger8 effects frame. It must not become a fictional public Cell or mutate fabricated balance/allocation data.

### Before / after
```compact
kernel.mintShielded(disclose(domain), disclose(amount));
kernel.claimZswapCoinSpend(disclose(commitment));
```
Before: unsupported empty ledger path. After: typed KernelMintShielded(domain Bytes32, amount Uint64) and KernelClaim(kind Nullifier/Spend/Receive, value Bytes32) Unit expressions, usable in actions, local helpers, branches and returns. Typechecked ADT signatures govern coercions; renderer revalidates exact widths.
```rust
let step = context.kernel_mint_shielded(domain, amount)?;
let step = context.kernel_claim_zswap_coin_spend(commitment)?;
```

### Domain/runtime decision
Use the special Kernel ADT identity and a strict operation allowlist before ordinary public-array path validation. No ledger field, index or invented slot appears in the Kernel IR. Shared canonical native builders emit the original ledger8 VM operations: claim nullifier/spend/receive use six operations against effects indices0/2/1; mint uses16 operations with cached membership branching and existing-amount addition at effects index4. Actual QueryContext::query governs state/effects/gas and rejection timing. Typed public inputs reuse upstream Commitment, Nullifier and HashOutput domain identifiers where their meaning is exact.

### Boundaries
Schema20 / runtimeABI45. Native execution reports explicit recording gaps; later recording can reuse the same program builders and must supply proof/ledger evidence. Claim effects do not prove a matching funded transaction exists. Mint effects are not fabricated balances or allocated coins. Unsupported Kernel operations fail closed. Repeated claim behavior and overflowing accumulated mint amounts come from the upstream VM/effect decoder rather than new Rust prevalidation.

### Evidence plan
Fresh independently compiled TS oracle covers each operation, dynamic arguments, zero/max/boundary mint amounts, same/different-domain accumulation, duplicate claims, selected branches and witness/private-output order. Compare full programs, query gas, state/effects, private state/results, plus malformed effects and overflow rejection. Typed IR negatives and no fictional path/field; strict-recording refusal. Recompile unchanged micro-dao and Coracle and retain next diagnostics without full-source acceptance claims. Focused tests, fresh new fixture only, targeted Clippy, signed DCO/GPG commit and exact receipt; no new Cargo target, push or remote CI.


### Accepted native delivery — 2026-10-05
Issue #281, milestone rust-backend-v2. Signed DCO/GPG `c7490bd3667ecea043520eca3b90318d2cf66b6b`; schema20/ABI45.

One Scheme Kernel handler serves normal typed expressions, stateful actions and direct returns, validating the special empty path and exact ADT argument signatures. The renderer checks Bytes32 and Uint64, rejects pure use and malformed/fictional path fields, and preserves argument/witness order. Native public context methods take ledger Commitment, Nullifier or HashOutput plus u64. The actual ledger8 query executes canonical generic-result-mode builders; user ledger slots, allocations and balances are not invented or directly mutated.

Evidence:12 original TS cases cover zero/7/u64MAX mint, each claim, repeated claims, same/different-domain accumulated mints, overflow on the second query, selected false/true, and witness domain→amount ordering. All successful states/effects/private values and query-sum gas match. Claims have exact6-op programs and mint16-op programs. JavaScript Map effects are serialized explicitly to preserve real shieldedMints entries, including u64MAX without numeric precision loss. TS batch aggregate gas contains only its last query; this is retained separately, while native gas is compared against every actual query cost. No failed-call partial gas is claimed.

Three runtime tests confirm exact TS programs, native overflow rejection on the second query without changing the prior immutable context, and malformed effects-frame EffectDecodeError. Eight library +12CLI +142existing renderer tests plus one new typed-negative guard passed; native sourcegate includes strict-recording and unsupported Kernel.checkpoint refusal. Targeted allfeatures/alltargets Clippy, formatting and one fresh fixture passed. Older fixtures await coordinated parent ABI refresh.

Signed-head receipt `${LOCAL_EVIDENCE}/compact-focused-c7490bd3-kernel-effects/receipt.json` passed:1/8recorded (read_state only), seven proof-required Kernel exports remain explicit native-only gaps. Compiler `${LOCAL_EVIDENCE}/compact-adr177-compactc`; Scheme `${HISTORICAL_NIX_STORE}/8br3z4fhzf1432fy0c3bb3xdj0cm1f20-compactc-binary-nixos/bin/compactc-scheme`. No funding, valid-offer, recorded-kernel or proof/apply claim.

Unchanged micro-dao now passes Scheme lowering and reaches renderer `standard-library.compact line125: stateful expression requires stateful evaluation`. Actual coin construction is StructLiteral with color tokenType(domain,Kernel.self) plus Uint64→128 widening; ADR0179 will isolate typed stateful struct evaluation. Coracle on this pre-ADR0169 branch remains at original line294 Cell.write; integrated parent already contains that separate root-let bridge, so this is not a new regression or source completion claim.

### Main integration — ADR177

Integrated as `24654ee5`, verified GPG+DCO. Native Kernel effects use upstream VM programs and typed domain carriers. Seven proof-required Kernel oracle APIs remain intentionally unavailable for recording. The nine combined runtime units cover the canonical program, overflow and malformed effects cases.

Current checkpoint `e751ddf1` is private schema20/runtime ABI45.162 fresh fixtures (zero failures),8 backend units+144 renderer tests,9 runtime units,27 Python tests, targeted strict Clippy and formatting pass. Seven-source frozen focused gate: `${LOCAL_EVIDENCE}/compact-focused-e751ddf1/receipt.json`. Combined Cell proof log: `${LOCAL_EVIDENCE}/compact-e751ddf1-cell-proof.log`.

Full source inventory:333/343 assessed proof-required APIs,10 known gaps,20 unassessed exports across205 sources/717 exports. Receipt: `${LOCAL_EVIDENCE}/compact-e751ddf1-inventory.json`. Native admission expands the assessed gap denominator; retained existing capabilities pass regressions. Latest completed broad gate remains4ef0fa57, not a claimed ABI45 full run. No push/remote CI.
