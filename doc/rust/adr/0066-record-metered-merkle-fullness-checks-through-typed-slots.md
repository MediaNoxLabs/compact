---
id: RUST-ADR-0066
alias: ADR-0066
title: "Record metered Merkle fullness checks through typed slots"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts", "merkle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 2c7879e8aeb81cef005b82d3d1d5821f0cdb521faaba40e9670d0a607efa0ab2
---
# RUST-ADR-0066 — Record metered Merkle fullness checks through typed slots

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept direct typed plain/historic Merkle fullness recording through the shared canonical query and declared field identity. Preserve the pinned ZKIR 2.1.0 proof evidence separately from an earlier cached 2.2.0 run. Other Merkle expressions and operations are not implicitly admitted.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#165 closure](https://github.com/MediaNoxLabs/compact/issues/165#issuecomment-6017510319). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`03bb3daa`](https://github.com/MediaNoxLabs/compact/commit/03bb3daa3bc5caed5c06bf474695c9c0a4b5fa02). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 66
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/165
```

## Historical decision and amendments

### Problem

Both plain and historic Merkle oracle contracts export `full(): Boolean { return t.isFull(); }`. Native Rust generation uses the declaration-typed `MerkleSlot::is_full` and the pinned ledger-8 VM query, but no generated `recorded::full` method exists. A caller can inspect `PublicStateView::t()?.first_free()` locally, yet that structural view is neither metered nor a complete Verify transcript. The native-only gap prevents a generated Rust consumer from proving a fullness read with the same Compact semantics.

### Before and proposed after

```rust
// Before: native result only; no transaction-ready public trace.
let native = merkle_contract::full(context)?;
let full: bool = native.result;

// After: the same named declaration slot records the VM read and result.
let recorded = merkle_contract::recorded::full(context)?;
let full: bool = recorded.execution.result;
let verify_ops = recorded.public.verify_ops();
```

Do this for both plain and historic `full` circuits. The return type remains `bool`. The local structural view from ADR-0063 remains separate; it does not gain `is_full()` or VM behavior.

### Decision and ownership

The existing `StateReturn::MerkleIsFull` and `HistoricMerkleIsFull` IR variants already preserve field name/index and Boolean result. The `syn` emitter should admit only a direct return on the matching declared Merkle kind and physical index; other shapes retain their current unsupported-recording boundary. `MerkleSlot<Leaf,DEPTH,HISTORIC>::record_is_full` delegates to a `RecordingFrame` method using the same pinned ledger `historic_is_full`/`merkle_is_full` query. Refactor the existing `is_full_program` only enough to parameterize its `Popeq` read result for `ResultModeVerify`; do not duplicate the tree-capacity VM sequence. Record the exact observed read event, successor query context, four-dimensional gas, and ordered Verify operations as other typed read slots do. No new ledger/zk primitive, macro, witness behavior or local state decoding is needed.

This additive generated/runtime call API requires ABI 33→34 and fixture regeneration; private Rust IR schema remains 8. The compiler owns support eligibility and source diagnostics. Runtime owns VM program construction, metering, and trace state.

### Acceptance and risks

Create and assign a focused MediaNoxLabs issue to `rust-backend-v2` before code. Prove initial and post-append `full` results for both tree kinds against existing TypeScript oracle outputs and native Rust; compare state, effects, four-dimensional gas, and exact ordered VM operations. The recorded read must not mutate a tree. Test malformed path/depth and gas-limit failures, and reject a wrong tree kind/index in renderer tests. Regenerate all fixtures and measure generated source delta. Compile an external generated consumer; prove, verify, validate and apply at least one recorded fullness call with the pinned ledger-8/zk stack. Run focused local gates first and expand to the full proof suite when the new trace requires it. Preserve conventional GPG/DCO commit and exact compiler/runtime compatibility evidence in this ADR. Remote CI is deferred by user direction until the local backlog is complete.

### Alternatives and limits

Returning `first_free == capacity` from `PublicStateView` would skip the VM and gas charge. A handwritten Verify program in generated code would duplicate ledger ownership. A body-wide macro would obscure operation order. This slice covers only direct `isFull()` returns; `checkRoot`, historic history mutation, indexed/hash inserts and nested Merkle expressions need separate acceptance.

### Tracking

- Parent plain/historic Merkle recording: [#127](https://github.com/MediaNoxLabs/compact/issues/127), [#128](https://github.com/MediaNoxLabs/compact/issues/128).
- Parent complete trace: [#107](https://github.com/MediaNoxLabs/compact/issues/107).
- Focused issue: pending creation before implementation.
- Delivery: proposed; no code or proof claim.

### Tracking amendment — 2026-10-04

Focused [#165](https://github.com/MediaNoxLabs/compact/issues/165) was created and assigned to `rust-backend-v2` before implementation. The pending sentence above preserves proposal chronology.


### Implementation checkpoint — 2026-10-04

The AST emitter now admits direct Boolean `MerkleIsFull` and `HistoricMerkleIsFull` returns only when the declared field kind and physical index agree and the circuit has no actions. The generated recorded method calls the named `MerkleSlot::record_is_full`; the observed-call handle comes from the existing transaction emitter. The runtime slot selects plain or historic recording, while `RecordingFrame` executes the pinned ledger query, stores the observed `GatherEvent::Read` result in the same `is_full_program` built in Verify mode, and preserves successor context, gas and ordered operations. The existing native Gather path uses that shared builder. Runtime/compiler ABI is 34; private IR schema remains 8.

All 137 generated fixture outputs were refreshed; only the two Merkle oracle libraries gain recorded `full` methods beyond the ABI guard update. The staged source delta is 151 files, 744 insertions and 274 deletions. Fifty-nine renderer tests pass, including wrong kind/index rejection. Focused plain and historic oracle tests pass at empty, one append and capacity eight: native result, state, effects, four-dimensional gas, ordered TypeScript VM program and Verify replay agree. Malformed path/depth and zero gas limit reject as expected. Runtime all-features check, formatter, fixture freshness, external generated Cargo consumer and negative compiler probes pass. The packaged proof and ledger-application run is still in progress; no proof acceptance is claimed in this checkpoint.

`nix develop .#compiler` resolves the project compiler Scheme binary to `${HISTORICAL_NIX_STORE}/sd3wkamq8xk51h1fb5z48pm9cp8lgl0n-compactc/bin/compactc-scheme` and its pinned ZKIR to `${HISTORICAL_NIX_STORE}/zlxf4bz9rd53wn6rawyahn0x65sqsjw7-zkir-2.1.0/bin/zkir`. The initial local proof run used cached ZKIR 2.2.0 while resolving the pinned shell; final compatibility evidence must use the pinned version or explicitly state the narrower result. The unrelated user edit to `doc/ledger-adt.mdx` is excluded from staging.


### Local acceptance — 2026-10-04

Conventional GPG-signed/DCO commit `03bb3daa3bc5caed5c06bf474695c9c0a4b5fa02` (`feat(rust-backend): record typed Merkle fullness reads`, `Refs: #165`) contains the emitter, runtime, tests, docs and 137 regenerated fixture libraries. `git verify-commit HEAD` reports a good signature; the sign-off trailer is present. The local runtime and backend agree on ABI 34; the private IR schema remains 8. After committing, `git status --short` shows only the unrelated user-owned `doc/ledger-adt.mdx` edit.

The pinned local gate passed using the source-built `target/debug/compactc`, Nix Scheme compiler `${HISTORICAL_NIX_STORE}/sd3wkamq8xk51h1fb5z48pm9cp8lgl0n-compactc/bin/compactc-scheme`, and project ZKIR `${HISTORICAL_NIX_STORE}/zlxf4bz9rd53wn6rawyahn0x65sqsjw7-zkir-2.1.0/bin/zkir` (`midnight-zkir 2.1.0`). Command: `check_compactc_target.py --consumer --proof` with those paths in `COMPACTC`, `COMPACTC_SCHEME` and `PATH`. The external consumer, source negative probes, generated key artifacts, replay, proof, verification, validation and ledger application passed, including separate plain and historic `full` calls. An earlier local gate with cached ZKIR 2.2.0 also passed, but the pinned 2.1.0 run is the compatibility evidence. A final no-update fixture check reported `Checked 137 fixtures; 0 stale; 0 failed`; `cargo fmt --all -- --check` and staged diff checks passed.

The slice remains local and unpushed by user direction. Same-commit remote CI, branch publication, and wider Merkle operations remain open in rust-backend-v2. Issue #165 remains open until its remote CI criterion is met.
