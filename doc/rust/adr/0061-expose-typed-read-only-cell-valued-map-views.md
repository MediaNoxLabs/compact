---
id: RUST-ADR-0061
alias: ADR-0061
title: "Expose typed read-only cell-valued Map views"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "generated-api", "state-inspection"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 3b01cdf9384b68196420729527f55b33733c1b58f1357dbc41c0ae859914758e
---
# RUST-ADR-0061 — Expose typed read-only cell-valued Map views

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept local public-state Map views only where both key and value support CellValue, using declaration-owned paths. Nested Map nodes receive no scalar getter. Keep missing/invalid-state errors and local inspection distinct from circuit lookup/witness metering; source changes do not demonstrate speedup.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#160 closure](https://github.com/MediaNoxLabs/compact/issues/160#issuecomment-6017501919). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`9a3b2780`](https://github.com/MediaNoxLabs/compact/commit/9a3b27802c72cf6d4783e6172790c0124ef8542e) · [`b89f43b0`](https://github.com/MediaNoxLabs/compact/commit/b89f43b0482b05af6692ad781b9dce662e8022c1). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 61
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/160
```

## Historical decision and amendments

### Problem

Generated `PublicStateView` covers Cell, Counter and Set declarations, but a consumer inspecting an already held Map state still repeats its compiler-owned physical path and key/value types. The chunked Map fixture and proof smoke copy `[1, 14]`. A Map-only crate has no generated public view. The metered witness `LedgerView` belongs to execution and cannot stand in for local inspection.

### Before

```rust
let map = runtime::ledger::map_view_at_path::<bool, runtime::Field, _>(
    state.data.get_ref(), &[1, 14]
)?;
let value = map.lookup(true)?;
```

### Decision and after

For a declared Map with a `CellValue` key and `CellValue` value, generate a declaration-named `PublicStateView` getter returning the existing borrowed `runtime::ledger::MapView<'a, K, V, D>`. Emit `PublicStateView` for Map-only contracts in this supported shape. Reuse ADR-0059's generic `From<&S: PublicStateSource>` conversion and `ledger_slots` physical path.

```rust
let view = ledger_contract::PublicStateView::from(&state);
let value = view.table()?.lookup(true)?;
assert!(view.table()?.member(true));
```

`member` and `is_empty` are local `bool`; `size` remains fallible for `Uint<64>` overflow. `lookup` returns `CompactError::InvalidLedgerCell` when the key is absent and uses the existing value decoder/alignment checks. Do not invent a default-on-absence rule. The view borrows public state; it does not query the VM, charge gas, mutate state, or authenticate provenance.

### Emitter and runtime ownership

Add `MapSlot<K,V>::inspect<'a,D:DB>` only in the existing `impl<K: CellValue,V: CellValue>`; delegate to `ledger::map_view_at_path` and return `MapView`. The `syn` renderer adds getters for `LedgerFieldKind::Map` only when its value is a supported cell value. It must **not** offer this getter for `Type::LedgerMap` nested values: `map_slot_types` names those as `MapNode<K,V>`, which has no `CellValue` codec or scalar lookup. Preserve the generated `MapSlot` descriptor and its existing structural operations for nested Maps. This additive generated/runtime contract advances ABI 30 to 31; private IR schema 8 is unchanged. No derive/proc macro, new Map decoder, VM operation or proof logic is required.

### Alternatives and boundaries

Applications may call `map_view_at_path` or `ledger_slots::table.inspect` directly, but named public getters avoid copying compiler-owned layout and align with Cell/Counter/Set inspection. A generic getter for nested `MapNode` would imply value lookup semantics that the runtime does not provide, so it is excluded. List head/default and Merkle root/local-vs-metered behavior need separate ADRs.

### Review and delivery criteria

Compare generated and raw views for root Boolean→Field and Field→Field Maps and a chunked `[1,14]` Map on initial, native, recorded, replayed and applied states where applicable. Verify missing keys, malformed root/path/map shape, value alignment errors and wrong-key compile rejection. Assert that nested Map nodes do not gain a misleading getter. Keep existing TypeScript state bytes, gas, effects and Verify ops unchanged. Regenerate all fixtures, measure generated-source delta, compile an archive-only external consumer with one shared runtime, and run the clean exact-head packaged compiler/proof gate. Record before/after output, signed DCO commits, package identity and actual evidence here and in the focused MediaNoxLabs issue. Leave that issue open until same-commit remote CI is green.

### Tracking

- Parent generated-API issue: https://github.com/MediaNoxLabs/compact/issues/110
- Related execution issue: https://github.com/MediaNoxLabs/compact/issues/147 (recorded chunked Map, distinct from local inspection).
- ADR-0059/#158 and ADR-0060/#159 establish the source conversion and collection-view pattern.
- Focused issue and milestone assignment: pending, before implementation.


### Tracking amendment — 2026-10-04

Focused issue [#160](https://github.com/MediaNoxLabs/compact/issues/160) was created and assigned to `rust-backend-v2` before implementation. The pending sentence above records the proposal sequence.


### Local implementation checkpoint — 2026-10-04

Conventional DCO-signed local commit `9a3b2780` implements ABI 31. The first signing attempt at `ac906fb8` failed `git verify-commit`; it was amended before any clean-head delivery check, and `git verify-commit 9a3b2780` reports a good GPG signature. The `syn` emitter adds named getters only for cell-valued Map declarations, while nested `MapNode` declarations keep their structural slot and no scalar getter. `MapSlot<K,V>::inspect` delegates to the existing `map_view_at_path`; private IR schema 8 and ledger VM behavior are unchanged. The four-crate archive consumer now includes a Map-only contract and a wrong-key compile rejection. Root Boolean→Field, Field→Field, chunked and applied-state tests are added.

All 137 fixture sources were regenerated and a follow-up freshness check found zero stale outputs. Eleven cell-valued Map fixtures gained views; fixture diff is 450 added and 250 removed lines, net +200 (the other changes are ABI assertions). The Map-only Boolean→Field fixture grows 825 bytes, Field→Field oracle 854 bytes, and chunked Map 286 bytes; nested Map fixture has zero net byte growth beyond ABI substitutions. These are source-size measurements only. Changed-source renderer suite passed 58/58. Focused clean-head Cargo, exact package, external consumer and proof gates are in progress; no final acceptance claim yet. The main checkout's unrelated user-owned `doc/ledger-adt.mdx` remains unstaged.


#### Declared-name test correction

The first clean focused test run failed only because the newly added Field→Field oracle assertion called `table()` while that contract declares the Map as `m`. The generated API correctly used `m()`. Signed/DCO conventional follow-up `b89f43b0` changes those two assertions to `m()`. The original test error is retained here as a reviewable example of why the generated getter must follow declaration names. Focused clean-head tests are being rerun; no success is inferred from the correction alone.


### Clean focused-check checkpoint — 2026-10-04

Clean detached commit `b89f43b0482b05af6692ad781b9dce662e8022c1` (tree `2d57dd2e66fc737a15259c34e02c8d721b8b833a`) contains the signed/DCO ABI-31 implementation and declared-name test correction. The clean focused Cargo run passed three root Boolean→Field tests, one chunked Map oracle test and one Field→Field Map oracle test (5/5). It covers initial/native/recorded/replay views, raw error parity for missing/incorrect shape and absent key, wrong stored value alignment, TypeScript oracle state/gas/VM parity and declaration naming. `cargo check -p compact-rust-proof-smoke` passed with the new applied-state generated-vs-raw Map view assertion. The clean `git archive` header validator passed 1,521/1,521. The final-head source archives in `target/rust-runtime-release-b89f43b0.json` were packaged and independently reverified with `dirty:false`; macro 9 entries, runtime 248 entries. The exact Nix compiler, archive-only external consumer and proof/ledger gate remain pending.


### Local acceptance — 2026-10-04

ADR-0061 is accepted locally at clean exact HEAD `b89f43b0482b05af6692ad781b9dce662e8022c1` (tree `2d57dd2e66fc737a15259c34e02c8d721b8b833a`). Conventional commits `9a3b2780` (`feat(rust): expose typed public Map inspection`) and `b89f43b0` (`test(rust): use declared Map name in oracle view`) both pass `git verify-commit`, carry the developer's GPG signature and include DCO `Signed-off-by`. They remain local and unpushed.

The final clean detached checkout passed five focused Map tests (three root Boolean→Field, one chunked `[1,14]` oracle, one Field→Field oracle), proof-smoke crate checking with generated-vs-raw applied Map inspection, 58 renderer tests, all 137 generated fixture freshness checks with zero stale/failed, and the `git archive HEAD` header validator on 1,521 files. Exact Nix-built compiler `${HISTORICAL_NIX_STORE}/8w2bwflgrrwr9fycbzjg9hb9775rvmcd-compactc/bin/compactc` reported version `0.31.133`. `check_compactc_target.py --consumer --proof` exited 0 at this HEAD: 95 traces replayed/partitioned and 95 generated deployment/proven calls validated/applied. Full log: `${LOCAL_EVIDENCE}/compact-adr61-proof-b89f43b0.log` on this machine. Its parity cases cover state, FAB/pre-proof bytes, four-dimensional gas and ordered VM effects for the existing corpus; this getter itself performs no metered read.

`target/rust-runtime-release-b89f43b0.json` was created and independently reverified from `dirty:false` source. Its macro archive has 9 entries, SHA-256 `544a0ed98803c783458244e768edba67dc7ccc81c3662d6222808d987dac74db`; runtime archive has 248 entries, SHA-256 `590a081813d33425b5fef128fe45ee565457fb388eaed6becc56b4e046c70545`. A four-crate archive-only Counter+Cell+Set+Map consumer compiled against one packaged runtime; wrong Set and Map key types were rejected. The fixture source delta is 450 added/250 removed lines, net +200; eleven cell-valued Map fixtures gained getters. These source-size figures do not establish compile-time or runtime speedup.

The ABI is 31 and private IR schema remains 8. Nested Map nodes receive no scalar getter. The unrelated user-owned `doc/ledger-adt.mdx` was neither staged nor changed by these commits. Same-commit remote CI has not run because the M2 branch has not been pushed; [#160](https://github.com/MediaNoxLabs/compact/issues/160) stays open. Merkle and List inspection remain separate designs (List now proposed in ADR-0062/#161).
