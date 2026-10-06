---
id: RUST-ADR-0063
alias: ADR-0063
title: "Separate typed local Merkle inspection from metered checks"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "generated-api", "state-inspection", "merkle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b59da33ceb2a2d85445827f81d3afd1b77a7f2e82cf8034be1f8792e0d4355c9
---
# RUST-ADR-0063 — Separate typed local Merkle inspection from metered checks

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept distinct typed local plain/historic Merkle wrappers with declared leaf/depth constraints, separate from metered checks and without Deref to an unrestricted raw view. Preserve root/history/path boundaries and the dated local-first/cancelled-CI history. Local projections are not charged circuit validation or consensus evidence.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#162 closure](https://github.com/MediaNoxLabs/compact/issues/162#issuecomment-6017505276). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1495bf64`](https://github.com/MediaNoxLabs/compact/commit/1495bf64f9b45e26ff543a50e1b29241cc2019ca). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 63
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/162
```

## Historical decision and amendments

### Problem

Generated Merkle declarations have typed `MerkleSlot<Leaf, DEPTH, HISTORIC>` descriptors, but a consumer inspecting held public state still repeats a physical path and calls a raw structural view. The raw `MerkleTreeView<D>` and `HistoricMerkleTreeView<D>` do not carry the declared leaf type or depth. A named getter that simply exposes the raw view could imply stronger leaf/depth validation or equivalence to Compact's VM-charged `check_root` and `is_full` than it actually provides. Plain and historic trees also have different local history capabilities.

### Before

```rust
let tree = runtime::ledger::merkle_tree_view_at_path(
    after.context.query.state.get_ref(), &[0]
)?;
let digest = tree.root();
let first_free = tree.first_free()?;
```

Historic callers use `historic_merkle_tree_view_at_path` and a separate path. The existing `MerkleSlot::witness_view` instead creates a metered view for witness/circuit reads.

### Decision and after (proposal)

Add distinct declaration-scoped local wrappers, `PlainMerkleStateView<'a, Leaf, DEPTH, D>` and `HistoricMerkleStateView<'a, Leaf, DEPTH, D>`. Generated `PublicStateView` getters call `ledger_slots::t.inspect(state)` and `ledger_slots::h.inspect(state)`. Keep the wrappers separate from metered witness views; do not implement `Deref` to the raw structural views.

```rust
let view = ledger_contract::PublicStateView::from(&after);
let digest: Option<runtime::ledger::MerkleTreeDigest> = view.t()?.root();
let next = view.t()?.first_free()?;
let path: runtime::ledger::MerklePath<Leaf> =
    view.t()?.path_for_leaf(index, leaf)?;
let historic = view.h()?;
let seen = historic.contains_root(digest.expect("nonempty tree"));
```

`root()` is an optional local digest, not a Compact `check_root` call. Local wrappers expose `root`, `first_free`, `path_for_leaf(index, leaf: Leaf)` and `find_path_for_leaf(leaf: Leaf)` with `Leaf: BinaryHashRepr` on path methods. Only the historic wrapper exposes `history` and `contains_root`. Neither wrapper exposes `check_root` or `is_full`: these are VM-charged operations on `MeteredMerkleTreeView` and `MeteredHistoricMerkleTreeView`.

### Alternatives and rationale

Returning the raw views directly is a smaller code change but drops the compiler-declared leaf type and depth at the public boundary. Adding `Deref` from typed wrappers would recreate the same ambiguity and allow generic leaf paths. Reimplementing root/history or the VM checks would duplicate ledger-8 behavior. Separate wrappers retain the plain/historic distinction in Rust's type system and delegate structural operations to existing ledger primitives.

### Emitter and runtime ownership

The existing `LedgerFieldKind::MerkleTree { ty, depth }` and `HistoricMerkleTree { ty, depth }` in private IR schema 8 determine `Leaf`, `DEPTH`, physical path and historic kind. The `syn` renderer adds declaration-named `PublicStateView` getters with concrete plain/historic wrapper return types. `MerkleSlot<Leaf, DEPTH, false/true>::inspect` delegates to existing `merkle_tree_view_at_path` or `historic_merkle_tree_view_at_path`, then checks actual ledger tree height against `DEPTH`. The pinned `midnight-transient-crypto` tree exposes `height() -> u8`; add a structural height accessor in `runtime-rs/src/ledger/merkle.rs` so this validation uses the upstream tree rather than parsing bytes. Reject wrong height with `CompactError::InvalidLedgerCell` before returning a typed wrapper. Existing raw views and metered views remain available. No new midnight-ledger or midnight-zk primitive, derive, proc macro, VM operation, proof adapter or state mutation is proposed. This additive generated/runtime API advances ABI 32 to 33; private Rust IR schema remains 8.

### Semantics and limits

The ledger stores hashes, and `insert_hash` exists, so a typed wrapper does not prove every stored leaf was inserted from `Leaf`. The existing `path_for_leaf` checks the index but does not assert the provided leaf matches storage; a mismatched leaf yields a path with a nonmatching root. `root()` may be `None` for an unrehashed tree. Historic `contains_root` is local map membership and is not a VM `check_root` substitute. Both wrappers borrow existing public state; they do not authenticate its provenance or charge gas. Height validation detects depth mismatch, not every possible internal invariant.

### Verification and risks

For plain and historic initial, append, indexed insert, hash insert, reset and forget states where supported, compare named and raw root/first-free/path/history results. Cover wrong array shape, missing historic map, malformed first-free cell, wrong actual tree height, out-of-range path, and mismatched leaf behavior. Compile-fail probes must reject wrong leaf argument, plain history access, and local `check_root`/`is_full`. Typecheck all Merkle fixture crates and an archive-only external consumer. Verify unchanged TypeScript state bytes, FAB, four-dimensional gas, ordered VM transcript, replay, proof and ledger application; run fixture freshness, source delta and clean exact-head Nix/package gates. A shorter call site is not a performance claim.

### Tracking and delivery

- Parent generated API: https://github.com/MediaNoxLabs/compact/issues/110
- Focused issue: pending creation and rust-backend-v2 assignment before implementation.
- Local commits: none for this decision yet.
- State: proposed; no delivery or remote-CI claim.

### Amendments

Record issue assignment, design changes, signed/DCO commits, exact tests and measured evidence here. Leave the focused issue open until same-commit remote CI passes.


### Tracking amendment — 2026-10-04

Focused issue [#162](https://github.com/MediaNoxLabs/compact/issues/162) was created and assigned to `rust-backend-v2` before implementation. The pending tracking sentence above preserves the proposal sequence.

### Implementation checkpoint — 2026-10-04

The proposed runtime/emitter implementation is in the local working tree pending clean exact-head gates. Plain and historic wrappers in `runtime-rs/src/slots.rs` hold the existing raw structural view, retain `Leaf` and const `DEPTH`, expose only local methods, and reject mismatched actual height through upstream `MerkleTree::height()`. The `syn` emitter adds declaration-named getters and advances generated/runtime ABI to 33 without changing private IR schema 8. No VM or proof logic changed. Plain and historic oracle helpers now compare generated/raw root, first-free, path and history across existing TypeScript-backed operation sequences; applied proof smoke adds generated/raw Merkle comparisons. The archive consumer adds a six-crate Merkle contract with wrong-leaf and unavailable local-method compile probes.

The changed-source runtime Cargo check and renderer suite passed, the three focused Merkle fixture crates passed 2+3+2 tests, proof-smoke crate checking passed, and 137 regenerated fixtures are fresh. Seven Merkle fixtures gained getters; the fixture delta is 406 added/250 removed lines, net +156 including ABI substitutions. One new malformed-state test initially attempted to move a ledger array out of `StateValue` (which implements `Drop`); borrowing the array fixed that test-only ownership error. The first renderer assertion assumed a one-line `prettyplease` return type and was changed to assert the emitted type path. Archive-only consumer, package manifest and exact-head proof gates are not yet claimed at this checkpoint. The main checkout's unrelated user-owned `doc/ledger-adt.mdx` remains unstaged.

### Local acceptance — 2026-10-04

Conventional GPG-signed/DCO commit `1495bf64f9b45e26ff543a50e1b29241cc2019ca` (`feat(rust): expose typed local Merkle inspection`, `Refs: #162`) delivered the runtime wrappers and `syn` getters. `git verify-commit` reports a good signature. Clean exact HEAD tree is `15cea0cdb3bd75113180ac1cf2f033222eb57c52`; generated/runtime ABI is 33 and private Rust IR schema remains 8. The implementation uses the pinned upstream tree `height()` and existing raw ledger-8 path/hash primitives; VM and proof adapters were not changed.

The clean Nix compiler is `${HISTORICAL_NIX_STORE}/c09kjdk3rapk46pms7yva6blh0hqhncq-compactc/bin/compactc` (0.31.133). The runtime release manifest `target/rust-runtime-release-1495bf64.json` was independently reverified with `dirty:false`, 9 macro archive entries (SHA-256 `8276f927115404a8f25648da5c8efe1ecadacd7c2dd6e69a0f38c31d8c844e76`) and 248 runtime archive entries (SHA-256 `9b544d840e8c8623d8e97d6e7d94aeb09680986b6b629678c22ffed5b57037c9`). All 1,521 archive headers passed the clean-source validator. The final-head renderer suite passed 58 tests; focused historic/path/plain Merkle fixtures passed 2+3+2 tests; 137 generated fixtures were fresh; proof-smoke checking passed. Seven Merkle fixtures gained getters; fixture source delta is +406/-250 lines, net +156. An archive-only six-generated-crate consumer used one packaged runtime and rejected wrong Set/Map/List/Merkle types plus plain-history and local-`check_root`/`is_full` access at compile time.

With `COMPACTC` set to the exact Nix store compiler, `nix develop .#compiler --command python3 tools/compact-rust-backend/check_compactc_target.py --consumer --proof` exited 0 at clean HEAD. The gate produced 95 replayed/partitioned traces and 95 independently validated/applied proven calls, including generated/raw Merkle inspection on applied state. Log: `${LOCAL_EVIDENCE}/compact-adr63-proof-1495bf64.log`. TypeScript-backed plain/historic oracles compare generated/raw views across supported initial, append, indexed/hash insert, reset and forget states. This is local acceptance only: the branch remains unpublished and same-commit remote CI is pending, so issue #162 stays open. Local wrappers still do not authenticate state provenance, prove the stored hash came from `Leaf`, or charge VM gas.

### Publication checkpoint — 2026-10-04

The earlier sentence records the pre-publication local-acceptance state. The full `codex/rust-backend-ast` branch was subsequently pushed to `MediaNoxLabs/compact` at the same clean commit `1495bf64`. All 121 commits after the prior remote milestone passed conventional-subject, DCO, and GPG verification before the fast-forward push. The repository does not run `Compiler Build` on branch push; [workflow run 37193647381](https://github.com/MediaNoxLabs/compact/actions/runs/37193647381) was dispatched explicitly on this branch. Its outcome is pending and must be checked at exact `head_sha` before marking remote acceptance.

### Delivery-order amendment — 2026-10-04

The user directed the initiative to skip further remote CI and finish the backlog with local verification first. Remote acceptance remains unchecked; no later local commit should be described as remote-green. The in-progress `Compiler Build` dispatch above was requested for cancellation. The local Merkle acceptance evidence remains valid at its exact commit.
