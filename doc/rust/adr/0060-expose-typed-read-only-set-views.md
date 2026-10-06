---
id: RUST-ADR-0060
alias: ADR-0060
title: "Expose typed read-only Set views"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "generated-api", "state-inspection"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: aeb1d2d7916cc645b3eeabd4ba6109240bc5c7062de097d582aa1131621a3c12
---
# RUST-ADR-0060 — Expose typed read-only Set views

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed local Set inspection through declared slots and the shared PublicStateView conversion. Local member/is_empty and bounded fallible size are inspection semantics, not VM queries. Preserve malformed-state checks and applied-state comparison as bounded evidence rather than metered execution parity.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#159 closure](https://github.com/MediaNoxLabs/compact/issues/159#issuecomment-6017500284). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`618379d6`](https://github.com/MediaNoxLabs/compact/commit/618379d6f26cadfbb39a927e3e0e97b030165aba) · [`8cd1f990`](https://github.com/MediaNoxLabs/compact/commit/8cd1f99030869d43f1e8cd5f80dc79f8d1fe9e9e) · [`bcb47924`](https://github.com/MediaNoxLabs/compact/commit/bcb47924cc66c1c5713c4c474693cb3922f1f3a7). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 60
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/159
```

## Historical decision and amendments

### Problem

ADR-0059 gives generated Cell and Counter declarations named public-state getters. Set-only contracts still expose no `PublicStateView`, so a consumer repeats the physical ledger path and key type when inspecting returned state. Root and chunked Set fixtures duplicate array/map matching. Witness `LedgerView` is metered and belongs to circuit execution, whereas an application already holding public state needs local inspection.

### Before

```rust
let set = runtime::ledger::set_view_at_path::<(runtime::Field, bool), _>(
    result.context.query.state.get_ref(), &[1, 14]
)?;
assert!(set.member((key, true)));
```

In the root Boolean Set fixture, callers manually unpack `StateValue::Array`, select index zero, and check the map size.

### Decision and after

For each declared Set, generate a declaration-named `PublicStateView` getter that returns the existing borrowed `runtime::ledger::SetView<'a, T, D>`. Generate `PublicStateView` for Set-only contracts. Reuse the generic `From<&S: PublicStateSource>` conversion from ADR-0059.

```rust
let view = ledger_contract::PublicStateView::from(&result);
assert!(view.keySet()?.member((key, true)));
assert_eq!(view.keySet()?.size()?.value(), 1);
```

`member` and `is_empty` on a successfully projected view return `bool`; `size` remains fallible for the ledger's `Uint<64>` bound. The getter is fallible for missing/wrong-shaped paths. This API is for locally held public state and does not authenticate it, run VM queries, or charge gas. It does not replace the metered witness view.

### Emitter and runtime ownership

The `syn` renderer uses the closed `LedgerFieldKind::Set { ty }` model and existing `ledger_slots::<name>` descriptor. It adds one getter per declaration and activates `PublicStateView` when at least one Cell, Counter or Set exists. `SetSlot<T>::inspect` delegates to `ledger::set_view_at_path` and returns the existing `SetView`; the runtime owns structural path validation and key alignment. This is an additive generated/runtime contract, so runtime ABI advances 29 to 30. Private IR schema 8 is unchanged. There is no derive/proc macro or second Set decoder.

### Alternatives and boundaries

Calling `set_view_at_path` directly copies compiler-owned paths into application code. Calling `ledger_slots::keySet.inspect` is a valid lower layer, but the public view makes the local-state intent and declaration name consistent with Cell/Counter. A broad collection derive would obscure the different Map, List and Merkle semantics; those remain separate design decisions. The generated getter borrows the state and has no mutation capability. `SetView` exposes no iteration, because upstream ledger Map iteration/ordering is not part of this decision.

### Review and delivery criteria

Compare generated and raw views for Boolean/Field root Sets and a chunked composite-key Set on initial, native, recorded, replayed and applied states where applicable. Check malformed root/path/map shape and a compile-fail wrong-key call. Show unchanged state/effects/gas/Verify ops from read-only inspection. Regenerate all fixtures, measure generated-source delta, compile an archive-only external consumer with one shared runtime, and run the exact-head packaged compiler/proof gate. Record signed DCO commits, package identity and actual results here and in the focused MediaNoxLabs issue. The issue stays open until remote CI confirms the same commit.

### Tracking

- Parent generated-API issue: https://github.com/MediaNoxLabs/compact/issues/110
- ADR-0059 and its issue #158 establish the generic public-state source and leave collections open.
- Focused issue and milestone assignment: pending, before implementation.


### Tracking amendment — 2026-10-04

Focused issue [#159](https://github.com/MediaNoxLabs/compact/issues/159) was created and assigned to `rust-backend-v2` before implementation. The earlier pending line records the proposal sequence.


### Local implementation checkpoint — 2026-10-04

Conventional, GPG-verified and DCO-signed commit `bcb47924cc66c1c5713c4c474693cb3922f1f3a7` implements the ABI 30 Set getter in the typed `syn` renderer and `SetSlot<T>::inspect` in the runtime. All 137 generated fixtures were regenerated and a subsequent 137/137 freshness check found zero stale outputs. Thirteen fixture sources gained Set views; across all fixture source diffs the change is 468 added and 250 removed lines, net +218 (the remaining fixture edits are ABI assertions). Set-only Boolean/Field fixture grew 1,058 bytes; chunked composite-key fixture grew 290 bytes. These are source-size data, not execution or compile-time benchmarks.

Changed-source renderer suite passed 58/58. Focused fixture tests and exact clean-head Nix package, archive consumer and proof gate are running; no completion or remote CI claim is made yet. The root checkout retains the user-owned `doc/ledger-adt.mdx` unstaged.


#### Replay-source amendment

The first focused compile found that upstream `QueryResults<ResultModeVerify, D>` is returned by replay but is not a `PublicStateSource`. A test using `PublicStateView::from(&replay)` failed with E0277. The runtime adapter now implements `PublicStateSource` generically for `QueryResults<M, D>` where `M: ResultMode<D>`, borrowing `replay.context.state.get_ref()`. This keeps the generated API identical and avoids making every consumer traverse `.context.state` after replay. It neither changes the ledger replay nor adds a query. The follow-up signed commit and final-head results will be recorded below.


The replay-source amendment is signed/DCO conventional commit `618379d6f26cadfbb39a927e3e0e97b030165aba`. `git verify-commit` reported a good GPG signature for both local commits. The focused test rerun and clean final-HEAD package gates are still pending.


### Exact final-head delivery — 2026-10-04

**Identity.** Clean detached final HEAD `618379d6f26cadfbb39a927e3e0e97b030165aba` (tree `e91f392fa256f73f81b9f1d2e2dc8821fe23c7a3`) contains signed/DCO conventional commits `bcb47924` and `618379d6`; `git verify-commit` reported good GPG signatures for both. Runtime ABI 30; private IR schema 8. The main checkout still has only the user-owned `doc/ledger-adt.mdx` unstaged.

**Exact package.** Clean Nix `compactc` `${HISTORICAL_NIX_STORE}/f8w12ilcv3gc11x05m0wr2d1zm1kbzkm-compactc` reports version 0.31.133. `target/rust-runtime-release-618379d6.json` was generated and independently reverified with `dirty:false`: macro archive 9 entries/SHA-256 `9b8519fe6f2377bdcaa6559b50dafd5b35e7da3fdba48088dbd5dd20c94e9cc9`; runtime archive 248 entries/SHA-256 `05cc555f6b6834179ee0ec15bd266d1142260010264fa362187cd2abf716dc16`. No registry publication is claimed.

**Generated API and behavior.** Set-only Boolean/Field contracts now generate `PublicStateView::seen()` and `fields()`; the chunked composite-key contract generates `keySet()`. Getters delegate through `SetSlot<T>::inspect` to existing `set_view_at_path`; there is no new Set decoder, VM program, gas path, derive or macro. `PublicStateSource` also accepts upstream replay `QueryResults`. The focused clean final-head Cargo run passed one chunked Set oracle test and four Boolean/Field Set tests, including raw-view parity on native state, initial/recorded/replay views, malformed path/map-shape error parity, TypeScript state/gas/replay comparisons in the existing chunked oracle, and Boolean/Field membership and size. The archive-only external consumer generated Counter, Cell and Set crates from the exact compiler, ran them with one shared runtime archive, and verified a wrong-key Set call fails Rust type checking; exit 0.

**Other gates.** Renderer 58/58 passed after the formatting-sensitive assertion was corrected. All 137 fixtures regenerated; a 137/137 freshness check found zero stale outputs. Thirteen gained Set views, with +218 net generated-source lines across fixture diffs; root Set +1,058 bytes and chunked Set +290 bytes. The clean exact-commit `git archive` header validator passed 1,521/1,521. `check_compactc_target.py --consumer --proof` exited 0 from the clean final-head Nix environment; `${LOCAL_EVIDENCE}/compact-adr60-proof-618379d6.log` records 95 replayed/partitioned traces, 95 validated/applied proven calls and the final passed marker. These are local source, compile, state and proof checks, not runtime performance benchmarks.

**Limits and acceptance.** `PublicStateView` still does not generate Map, List or Merkle getters; those require separate semantic decisions. The Set view borrows local state and does not authenticate its provenance or freshness. Issue [#159](https://github.com/MediaNoxLabs/compact/issues/159) stays open for same-commit remote CI once the branch is published. Current M2 branch has not been pushed; public registry package resolution remains a separate M2 gap. This ADR is accepted for the local design and implementation slice.


### Applied-state acceptance amendment — 2026-10-04

The preceding `618379d6` exact-head delivery was a passing local checkpoint, then review found that the proof smoke compared applied chunked Set state only through the raw `set_view_at_path`. Signed/DCO conventional commit `8cd1f99030869d43f1e8cd5f80dc79f8d1fe9e9e` now also obtains `chunked_set_contract::PublicStateView::from(&applied_contract).keySet()?` and compares size and membership with the raw view before the existing expected-state assertion. The clean final source tree is `4ab1c91d6b413064b55a8d05d4d7d8ef99bdcef9`; `git verify-commit` reports a good GPG signature. A clean `cargo check -p compact-rust-proof-smoke` and 1,521-file header validation passed. Exact-commit package, archive-only consumer and full proof gates are being repeated; the earlier package hashes and log belong to the prior checkpoint and must not be cited as final.


#### Final exact-head gate — 2026-10-04

The pending `8cd1f99030869d43f1e8cd5f80dc79f8d1fe9e9e` gates completed from a clean detached checkout. Nix `compactc` remains `${HISTORICAL_NIX_STORE}/f8w12ilcv3gc11x05m0wr2d1zm1kbzkm-compactc` (version 0.31.133) because the final test-only commit did not alter its packaged source. `target/rust-runtime-release-8cd1f990.json` was generated and independently reverified against tree `4ab1c91d6b413064b55a8d05d4d7d8ef99bdcef9`, `dirty:false`: macro 9 entries/SHA-256 `f6acb055175c3fe4871c6b872da494c13d2b50e69d0c730d7e5b6c54d3b9268a`; runtime 248 entries/SHA-256 `26a8c32af9716679dcbc96ce5d4d879fa1f6b38c5ed4b8f1baf8a2d6ba5b436e`. A three-generated-crate archive-only external consumer passed with one shared runtime and compile-fail rejection for the wrong Set key. The clean proof-smoke crate compiled, and the full exact-head `check_compactc_target.py --consumer --proof` exited 0. Its saved `${LOCAL_EVIDENCE}/compact-adr60-proof-8cd1f990.log` contains 95 replayed/partitioned generated traces and 95 validated/applied proven calls, including the generated-vs-raw chunked Set view comparison after application. The clean final-HEAD header check passed 1,521/1,521. These results supersede earlier `618379d6` package hashes for the final delivery. Issue #159 has six local acceptance boxes checked and remains open for same-commit remote CI.
