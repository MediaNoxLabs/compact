---
id: RUST-ADR-0171
alias: ADR-0171
title: "Typed Counter less-than queries and nested reads"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler", "Counter", "query"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 8f38c9e8fed5d9651f6d0968b71b75cbacb135d9b937005c8a85f556f8a446d3
---
# RUST-ADR-0171 — Typed Counter less-than queries and nested reads

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Typed CounterLessThan and nested CounterRead preserve query shape, threshold and metering. The decision narrows compiler admission and does not itself claim full microDAO source recording.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#275 closure](https://github.com/MediaNoxLabs/compact/issues/275#issuecomment-6017695791). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
After ADR0168 removes literal nonce casts, unchanged micro-dao fails at line192 in no.lessThan(yes). The analyzed AST evaluates yes.read() into a Uint64 local, then invokes no.lessThan(local). The generic expression emitter omits CounterRead while the stateful emitter supports it; lessThan itself has no typed Rust IR/runtime operation.

### Before / after
```compact
// Original source; currently rejected at the nested read.
assert(no.lessThan(yes), "Attempted illegal cash-out.");
```
```rust
// Intended generated meaning through typed Counter slots and lexical bindings.
let threshold = yes.read(context)?;
let comparison = no.less_than(threshold.context, threshold.result)?;
```
Recorded code should call the corresponding CounterSlot record_less_than on a RecordingFrame and preserve the observed ledger result. This must execute the actual ledger program dup / idx / push(threshold) / lt / popeq; a Rust-only comparison after reading the Counter has different VM shape, gas and disclosure behavior.

### Decision and ownership
Add CounterLessThan(field,index,threshold) to typed expression IR, schema15. Reuse CounterRead for nested reads with declaration/type/index validation. CounterSlot native and recording methods share the ledger program builder and preserve gathered result alignment. ABI41 is reserved; ADR0170 owns ABI40 and parent will integrate in order and refresh all combined fixtures. No contract-name recognition or qualified coin overlap. Existing branch-return work is ADR0169.

### Emitter/runtime boundaries
Support typed Uint64 threshold inputs including previously evaluated Counter reads and admitted local expressions; preserve left-to-right evaluation and branch-local query execution. Validate Counter declaration, index and threshold type; do not admit unsupported ledger paths or effectful pure calls. Reuse existing bounded types, context cost model, ledger query engine and RecordingFrame. Extend a bounded shared typed-plan recording profile only where its lexical semantics apply.

### Validation plan
Independent generated TS oracle: lower/equal/higher comparisons, zero and u64 maximum thresholds, nested read thresholds, short-circuit branches and assertion rejection. Compare native/recorded query count and order, exact VM/gas, private outputs, resulting state and replay. Prove and ledger-apply representative both outcomes. Negative IR/type/path guards; targeted new fixture freshness and Clippy. Existing fixtures will be temporarily stale by ABI/schema design; parent owns combined refresh. Recompile original micro-dao unchanged and report next actual gap without claiming full source admission.


### Accepted delivery (2026-10-05)
Issue: https://github.com/MediaNoxLabs/compact/issues/275, milestone rust-backend-v2.
Signed+DCO local commit: 0ce70628 (feat(rust): preserve typed Counter less-than ledger queries), based on ADR0168 3c7c09b5. Schema15 / ABI41; parent owns integration after ADR0170 ABI40 and combined fixture refresh.

#### Implementation
- CounterLessThan carries a validated Counter field/index and exact Uint64 expression threshold. Generic nested CounterRead reuses existing typed IR; native lowering evaluates threshold effects before the comparison.
- Shared runtime builder exactly follows ledger8 Counter.lessThan: dup, uncached idx, nonstorage push Uint64 threshold, lt, cached popeq. Native and RecordingFrame methods run the same program; recorded proof program retains the actual observed Boolean FAB alignment.
- CounterSlot exposes less_than/record_less_than; no contract-name checks. Existing typed lexical plan has a bounded read-only Boolean/Unit comparison admission profile, with Boolean/Uint64 parameters and branch-local frames. Pure audits reject the effectful operation; expression visitors recurse through threshold effects.

#### Independent evidence
Fresh generated TypeScript vs native/recorded/replay:
- compare thresholds0 and3 -> false;4 andu64MAX -> true. Each uses one query, five operations; read170000000, compute1251155303, written/deleted0.
- nested high.read() threshold -> true, two queries/eight operations; read340000000, compute2501070156, written/deleted0.
- short-circuit false -> zero queries/gas; true -> same ordered two queries as nested. State/effects/private transcript agree; exact push value/alignment and cached Boolean popeq included in VM-shape comparison.
- checked4 succeeds; checked3 rejects with counter guard. TS partial failure query retained; Rust error API does not expose partial gas/state, so rejected-call partial gas is not claimed equivalent.
- Native/recorded gas totals compared to actual TS query sums, preserving TS aggregate gas separately.
- Both compare false/true proofs verified and ledger-applied. Persistent ${LOCAL_EVIDENCE}/compact-adr171-proof includes all four circuit keys and ZKIR; compare prover41437B/verifier1351B. Runner --counter-less-than in compact-rust-proof-smoke.
- 8 backend library +12 CLI +140 renderer tests; independent generated fixture test; invalid threshold/type/slot/index/schema/pure-effect guards; new fixture freshness; targeted runtime/backend/fixture/proof-runner Clippy; workspace formatting.

#### Exact local tools and fixture boundary
Scheme: ${HISTORICAL_NIX_STORE}/9ywfwdhf30abcid7n65rln1y4wvymmdf-compactc-binary-nixos/bin/compactc-scheme.
Compiler: ${LOCAL_EVIDENCE}/compact-adr171-compactc (native renderer with ABI41; do not use the sibling Nix wrapper whose backend snapshot preceded the ABI constant bump).
Frozen focused receipt: ${LOCAL_EVIDENCE}/compact-focused-0ce70628-counter-less-than/receipt.json (4/4 recorded).
69 existing proof-runner dependency fixtures were temporarily regenerated by the real compiler to execute proofs and targeted Clippy, then restored byte-for-byte. Their mechanical ABI41 updates are intentionally outside this delivery. Parent must perform combined fixture refresh before rerunning the integrated proof runner. Only the new Counter fixture is committed at ABI41; source fixtures otherwise remain untouched.

#### Next source gap
Unchanged original test-center/test-contracts/micro-dao.compact advances to line171 pot.writeCoin unsupported ledger operation. No full eleven-export DAO native/recorded admission is claimed. Qualified coin/writeCoin and further shielded receive/send/mint/merge remain separate work.
