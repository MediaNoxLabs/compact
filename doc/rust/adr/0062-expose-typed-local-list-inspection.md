---
id: RUST-ADR-0062
alias: ADR-0062
title: "Expose typed local List inspection"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "generated-api", "state-inspection"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 704733d12d145202569a3ef60e5142cfd1fecb194e58060da7c52279ec7796e4
---
# RUST-ADR-0062 — Expose typed local List inspection

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed local List views while distinguishing Rust Option inspection from Compact Maybe circuit results. Preserve the documented malformed-internal-state inconsistency and constructor-oracle correction. This projection does not silently add VM costs or claim every malformed ledger shape is normalized.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#161 closure](https://github.com/MediaNoxLabs/compact/issues/161#issuecomment-6017503681). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`29964f38`](https://github.com/MediaNoxLabs/compact/commit/29964f386697d2a1044d628d7d1abae6718b2f27) · [`cb439884`](https://github.com/MediaNoxLabs/compact/commit/cb43988455b7e95e486677ddad50d623ef22cbc7). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 62
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/161
```

## Historical decision and amendments

### Problem

A generated List crate exposes a typed `ListSlot<T>` for execution, but no named way to inspect an already held public List state. Consumers copy the compiler-owned physical path and element type into `list_view_at_path`. The imported chunked List fixture uses `[1, 14]`; changing layout requires each caller to notice and update that literal. Local state inspection must remain separate from metered witness reads and circuit execution.

### Before

```rust
let list = runtime::ledger::list_view_at_path::<runtime::Field, _>(
    after.context.query.state.get_ref(), &[1, 14]
)?;
let head = list.head()?;
let length = list.length()?;
```

### Decision and after (proposal)

Generate a declaration-named `PublicStateView` getter for each List field, returning the existing borrowed `ListView<'a, T, D>`. A List-only contract gains `PublicStateView`. The slot delegates to the existing structural decoder:

```rust
let view = ledger_contract::PublicStateView::from(&after);
let head: Option<runtime::Field> = view.items()?.head()?;
let length = view.items()?.length()?;
let empty = view.items()?.is_empty();
```

The `Option<T>` is a local Rust projection. Compact circuit `head` returns its generated `Maybe` representation with `is_some` and `value`; on an empty List the latter can carry a default value. Do not equate or coerce these APIs. The getter borrows public state and makes no VM read, gas charge, transcript entry, or state mutation.

### Alternatives and rationale

Applications can call `list_view_at_path` or `ledger_slots::items.inspect` directly; the generated declaration getter avoids duplicated physical paths and fits the existing Cell/Counter/Set/Map pattern. A new decoder or derive/macro would duplicate a ledger-8 primitive without improving the type boundary. Merkle and historic Merkle inspection need a separate decision because structural `root`/`first_free` and metered `check_root`/`is_full` have different semantics and the current structural view does not carry leaf type or depth.

### Emitter and runtime ownership

`LedgerFieldKind::List { ty }` already carries the typed element and physical path in private Rust IR schema 8. The `syn` renderer in `tools/compact-rust-backend/src/lib.rs` adds only a `PublicStateView` method for the declared name; `ledger_slots` remains the source of the physical path. `runtime-rs/src/slots.rs` adds `ListSlot<T>::inspect<'a, D: DB>(self, state: &'a StateValue<D>) -> Result<ListView<'a,T,D>,CompactError>` under `T: CellValue`, delegating to `ledger::list_view_at_path`. Reuse `runtime-rs/src/ledger/collections.rs` shape and cell decoding. No midnight-ledger or midnight-zk fork, new primitive, proc macro, proof adapter, VM program, or runtime mutation is proposed. Public generated/runtime ABI 31 advances to 32; private schema stays 8. A matching generated crate and runtime must be used together.

### Verification and risks

Check an empty/root List, prepend, pop, reset, and imported chunked `[1,14]` on initial/native/recorded/replayed/applied state where applicable. Compare named view to raw view and metered circuit result while explicitly translating `Option<T>` versus circuit `Maybe`. Assert malformed path, array arity, head cell and length cell errors, and external consumer wrong element type rejection. Verify TypeScript state bytes, FAB, four-dimensional gas and ordered VM transcript parity; run clean exact-head fixture freshness, package/archive consumer, proof verification and ledger application. Measure fixture/source delta and avoid claiming a runtime speedup from the new getter.

The existing structural `ListView::is_empty` checks the tail for `Null`, while `length` decodes a cell; malformed internal states can make those observations disagree. This change does not establish full List invariant validation. `MeteredListView` stays the witness/circuit API.

### Tracking and delivery

- Parent generated API: https://github.com/MediaNoxLabs/compact/issues/110
- Focused issue: pending creation and rust-backend-v2 assignment before implementation.
- Local commits: none for this decision yet.
- State: proposed; no delivery or remote-CI claim.

### Amendments

Append the issue link, any changed design, signed/DCO commits, measured evidence, and remaining gates here. Keep the focused issue open until same-commit remote CI passes.


### Tracking amendment — 2026-10-04

Focused issue [#161](https://github.com/MediaNoxLabs/compact/issues/161) was created and assigned to `rust-backend-v2` before implementation. The pending tracking sentence above preserves the proposal sequence.

### Implementation checkpoint — 2026-10-04

The proposed narrow implementation is in the local working tree pending verification: `ListSlot<T>::inspect` delegates to `list_view_at_path`, the `syn` renderer adds a declaration-named `PublicStateView` getter, and generated/runtime ABI constants advance to 32 with private schema 8 unchanged. Root List tests compare empty/prepend/pop/reset native, recorded and replay states to raw views; chunked `[1,14]` tests compare named and raw views; proof smoke compares the named getter to the raw view after applied proven chunked List calls. A five-crate archive consumer adds a List call and distinct compile-fail probes for Set, Map and List types. Documentation states `Option<T>` versus circuit `Maybe` and the existing structural validation limit. The Nix compiler build and fixture regeneration are in progress. No acceptance, signed commit or remote-CI claim is made yet.


#### Constructor-oracle correction

The first clean chunked test asserted an empty initial head. Its imported constructor actually seeds `Field(1)`, as the existing proof-smoke expectation and new getter both reported. The first test-only correction fixed one assertion but the direct slot assertion retained the same bad assumption. I amended that local test commit before final delivery to signed/DCO `95353631`, correcting both to `Some(Field(1))`. The clean chunked oracle test then passed 1/1. This does not change the runtime or generated getter; it records why fixed expected values must come from the imported oracle rather than from the empty root List fixture.


### Local acceptance — 2026-10-04

ADR-0062 is accepted locally at clean exact HEAD `cb43988455b7e95e486677ddad50d623ef22cbc7` (tree `cc1d885e493e737a0fedeb324d2a6c3a6fbf08e6`). Conventional signed/DCO commits `29964f38` (`feat(rust): expose typed public List inspection`) and `cb439884` (`test(rust): align List oracle and ABI assertions`) both pass `git verify-commit`. The intermediate local test hashes in the constructor-oracle amendment were superseded by the final signed amendment, preserving the review history. Both final commits are local/unpushed.

The generated `PublicStateView::items()` returns the existing structural `ListView`; `ListSlot<T>::inspect` resolves the compiler-owned path and delegates to `list_view_at_path`. ABI 32, private IR schema 8. Root List initial/native/prepend/pop/reset and recorded/replayed inspection passed 3/3 focused tests, including malformed root/array/head/length cases and explicit local `Option<T>` versus circuit `Maybe` checks. The imported `[1,14]` chunked oracle passed 1/1 with `Field(1)` constructor head, named-versus-raw view parity, existing TypeScript state/gas/ordered VM assertions and unchanged metered witness behavior. `cargo check -p compact-rust-proof-smoke` passed with named-versus-raw inspection after applied proven chunked List calls. Renderer suite passed 58/58.

All 137 fixture sources are fresh at the final commit; six List fixtures gained getters. Fixture diff is 388 added and 250 removed lines, net +138, including ABI substitutions across 137 fixtures. This is source-size evidence, not a compile-time or runtime speedup claim. The clean `git archive HEAD` header validator passed 1,521/1,521. Exact-head Nix compiler `${HISTORICAL_NIX_STORE}/q60k5l209bvgzs1lskcmsmnbliijw63f-compactc/bin/compactc` reported version `0.31.133`. The fixture checker now uses an explicitly supplied `COMPACTC` directly, so freshness is measured against that same built compiler without a redundant local backend build.

`target/rust-runtime-release-cb439884.json` was created and independently reverified with `dirty:false`, exact commit/tree above. Macro archive: 9 entries, SHA-256 `f4d1ec48cff0ec6535475a785ea89123bb1564263d4cfe8a0343baf5bb00e13b`; runtime archive: 248 entries, SHA-256 `b442595c4524cd42d6184cbe0399b3d080f5a62a524e4148eb2512482c782e15`. A five-crate archive-only Counter+Cell+Set+Map+List consumer compiled and ran with one packaged runtime; separate wrong Set key, Map key and List element compile probes were rejected. The exact-head `check_compactc_target.py --consumer --proof` exited 0 with 95 replayed/partitioned traces and 95 generated deployment/proven calls validated/applied, including chunked List; full local log `${LOCAL_EVIDENCE}/compact-adr62-proof-cb439884.log`.

The structural `is_empty()`/`length()` inconsistency on malformed internals remains as documented in the decision. Merkle semantics are separately proposed in ADR-0063/[#162](https://github.com/MediaNoxLabs/compact/issues/162). The main checkout's unrelated user-owned `doc/ledger-adt.mdx` remains unstaged. Same-commit remote CI has not run because the M2 branch has not been pushed; [#161](https://github.com/MediaNoxLabs/compact/issues/161) stays open.
