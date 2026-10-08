---
id: RUST-ADR-0319
alias: ADR-0319
source_sha256: 621c28ee21bf8f95bbd19649c87fd42b6db560c1751fbd850bfdb9e818c958c1
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0319 — Exercise generated reducer control exports directly

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered bounded test slice · 2026-10-07 · R030-07/#351 · milestone 0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0319 — Exercise generated reducer control exports directly

Status: delivered bounded test slice · 2026-10-07 · R030-07/#351 · milestone 0.3.0

### Problem statement
The ADR0318 evidence review found no direct runtime invocation for alias-Set insert_control, member_control, remove_control or nested-relation seed. Existing mutate/check cases and dormant dispatch arms do not establish execution of these exported APIs.

### Before and after
```rust
// Before: only the composed alias operation was exercised.
c::mutate(context, value, Mutation::Insert)?;
// After: execute controls and assert state/result semantics directly.
let inserted = c::insert_control(context, value.clone())?;
let member = c::member_control(inserted.context, value.clone())?;
assert!(member.result);
let removed = c::remove_control(member.context, value)?;
```
Nested seed tests insert a distinct nested record, replace the same key, and verify the original constructor entry survives. Validate complete nested values, private state, no unexpected private outputs, and downstream check behavior. Set controls cover absent/present membership and insert/remove while preserving unrelated values and the counter.

### Emitter/runtime ownership
Only fixture behavior tests change; generated implementations, emitter, runtime, ABI and Compact sources remain unchanged. Recorded insert/member are exercised with ContractLab and replay validation where advertised. Native remove and nested seed have no generated recorded entry point: no invented recorded/proof/TS claim or scope expansion. Existing independent TS capture tests remain intact.

### Validation and evidence
Run the two fixture behavior suites locally, formatting for edited files, and strict Clippy for these test targets. Record exact source/generated/test hashes and named test-to-export joins; retain logs in the vault evidence archive. This closes four direct-invocation gaps only, not the remaining inherited joins, numeric coverage gate or parent #351. Do not rerun all proof/render targets for test-only edits. Root owns edits; independent subagent reviews assertions. Preserve stopped ADR0285 boundary and user-owned doc/ledger-adt.mdx. Signed conventional DCO commit; no push/remote CI.

Delivery issue: https://github.com/MediaNoxLabs/compact/issues/444


### 2026-10-07 — Direct reducer control execution (ADR0319/#444)

Signed GPG/DCO commit `d404d8c1abbb5e517d17a4c4b84db604b196618f` adds three test methods in two dedicated controls modules. All four missing auxiliary exports now have named direct execution joins: alias-Set `insert_control`, `member_control`, `remove_control`, and nested-relation `seed`. Set tests check absent/present/removed membership, unrelated data and counter preservation; nested tests check complete Method insertion/replacement, stable map size, original entry preservation and downstream curve checks. Recorded insert/member are compared with native state/effects/gas and replayed in ContractLab. Remove and seed remain native-only. Private state and empty private outputs are asserted.

Five integration methods pass (three new, two existing TS-capture suites), strict targeted Clippy and edited-file rustfmt pass. Independent subagent review found no material findings. An initial test compile error (ChargedState is not PublicStateSource) was corrected to use context/underlying state projections; the failed attempt is retained. Production emitter/runtime, generated libraries and Compact sources are unchanged. No new TS captures, proofs or network qualification.

Evidence archive [ADR0319 — Direct reducer control execution.zip](references-0.3.0.md#note-101), SHA256 `cd1517588a8054d3003f15207fa7167a3ad0b194b01b63f5678f61429274c53b`, includes exact source/generated/test identities, four test-to-export joins, logs and review. The four demonstrated missing invocations are resolved; 122 inherited oracle case-ID normalizations and 13 constructor joins still require reconciliation, as do finite semantic obligations. Prior measured backend coverage remains **8598/9060=94.90%** (not remeasured). #444 closes this slice; #351 remains open. Parent acceptance remains **9/19**. No push or remote CI; user-owned ledger-adt document preserved.
