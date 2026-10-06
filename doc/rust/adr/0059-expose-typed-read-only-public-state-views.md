---
id: RUST-ADR-0059
alias: ADR-0059
title: "Expose typed read-only public state views"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "generated-api", "state-inspection"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e9fa43fbba7f0039e1a622b1bb82982d99f4218bc90d89d054142c7e814634a3
---
# RUST-ADR-0059 — Expose typed read-only public state views

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept named fallible read-only public-state projections for Cells and Counters, separate from metered witness execution. The shared source conversion and typed paths avoid consumer duplication without introducing authorization or mutation. Subsequent view families extend the scope; source-size reduction does not establish performance.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#158 closure](https://github.com/MediaNoxLabs/compact/issues/158#issuecomment-6017498588). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`b665860e`](https://github.com/MediaNoxLabs/compact/commit/b665860ea12e0f7d503d2f128ac1d170848bd045) · [`c2ccbd6a`](https://github.com/MediaNoxLabs/compact/commit/c2ccbd6a08a76ed7d7b9014e426a627b9cc4f086). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 59
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/158
```

## Historical decision and amendments

### Problem

A generated Rust crate has typed circuit calls and typed ledger slots, but an application inspecting returned or observed public state must still match the ledger `StateValue::Array`, select a physical index, and invoke a raw decoder. The index is generated knowledge that the consumer should not copy. This is especially awkward for chunked roots and independently generated contract crates. The existing `LedgerView` is for metered witness callbacks during circuit execution; using it for application inspection would mix ownership and charge semantics.

### Before

```rust
let StateValue::Array(fields) = result.context.query.state.get_ref() else { panic!("root") };
let raw = runtime::ledger::read_counter(&fields.get(0).unwrap())?;
assert_eq!(raw, 1);
```

An observed `ContractState` or the post-state of a recorded call requires the same manual projection. A Boolean Cell similarly needs `read_cell::<bool, _>` and a copied index.

### Decision and after

Generate a separate `ledger_contract::PublicStateView<'a, D>` for read-only inspection. It borrows the upstream ledger-8 `StateValue<D>` and has named fallible getters for each declared Cell and Counter. A Counter getter returns Compact's `BoundedUint<{u64::MAX}>`, matching the generated circuit result type; a Cell returns its declared Rust type. Construct it via `From<&StateValue<D>>`, `From<&ContractState<D>>`, and, when available, `From<&ObservedContractState<D>>` and `From<&RecordedCircuitResult<Private, Output, D>>`. Do not invent a second state representation or make a VM query.

```rust
let view = ledger_contract::PublicStateView::from(result.context.query.state.get_ref());
assert_eq!(view.round()?.value(), 1);
let observed = ledger_contract::PublicStateView::from(&observed_contract);
assert!(observed.flag()?);
```

The first implementation slice covers declared Cell and Counter. Set, Map, List and Merkle views need separate semantic design and remain explicitly out of this slice. Only emit the view where these supported declarations exist; avoid empty public APIs on pure contracts. Getter names follow declared identifiers and Rust identifier escaping rules. The return value is owned, so a read view cannot mutate ledger state.

### Emitter and runtime ownership

The typed `syn` renderer derives getters from the existing closed `ledger_fields` model and reuses the `ledger_slots` physical paths. `CellSlot<T>::inspect` and `CounterSlot::inspect` delegate to `ledger::read_cell_at_path`, retaining exact alignment/path checks. `PublicStateView` wrappers only borrow existing upstream `StateValue`, `ContractState` and optional transaction state. This additive generated API requires a runtime ABI bump from 28 to 29 because freshly generated crates call new runtime slot methods; the private IR schema 8 stays unchanged. No derive/proc macro is needed for fixed contract-specific getter names.

### Alternatives and rationale

Direct `ledger_slots::round.inspect(state)` is useful as a lower layer but still asks applications to know which slot maps to each contract field and to translate Counter into Compact Uint. Extending the metered witness `LedgerView` risks implying that local application reads participate in gas or transcripts. Generated getters give the smallest ergonomic surface while the runtime owns physical decoding.

### Verification and risks

Compare each getter against the existing raw decoder for initial, native, recorded, replayed and observed states. Prove wrong root/path/alignment fails through the same `CompactError` as the raw decoder. Inspecting state must not change query context, gas, private outputs, recorded Verify ops or observed metadata. Compile two independently generated crates against a single packaged runtime; run fixture freshness, strict compiler, generated consumers and proof/ledger gate on the new ABI. Check a source declaration whose Rust name requires escaping. The view borrows an externally supplied state and does not authenticate it; observation trust remains the caller's responsibility.

### Tracking and delivery

- Focused MediaNoxLabs issue: pending creation; assign to `rust-backend-v2` before code.
- Parent ergonomic issue: [#110](https://github.com/MediaNoxLabs/compact/issues/110). Existing witness-metering issues [#116](https://github.com/MediaNoxLabs/compact/issues/116) and [#138](https://github.com/MediaNoxLabs/compact/issues/138) remain distinct.
- Status: proposed, no implementation or CI claim yet. Amend this ADR with exact commit, before/after generated code and test evidence at delivery.


### Tracking amendment — 2026-10-04

The focused issue is [#158](https://github.com/MediaNoxLabs/compact/issues/158), created in rust-backend-v2 before implementation. The earlier pending sentence records the proposal sequence. This ADR remains proposed until changed-head verification and delivery evidence are appended.

### Design amendment — shared source projection

The first renderer probe emitted four near-identical `From<&...>` implementations in each generated crate with a Cell/Counter. Across the 137 checked-in fixtures, 80 gained a view and total generated source grew by 4,761 net lines; Counter grew from 9,308 to 11,359 bytes. This is a cost to readers and compile time even though the API is useful. The design therefore moves the four source adapters to one runtime `PublicStateSource` trait with an associated ledger DB type. Generated code now needs one generic `From<&S>` impl plus the field-specific getters. Runtime owns extraction from upstream `StateValue`, `ContractState`, recorded result and feature-enabled observed state. The trait only returns a borrowed state and does not query the VM. The exact post-change source delta and consumer build results must be recorded before acceptance.

A generated ledger field named `from` is legal and produces an inherent `from` getter. A review probe confirmed that `Self::from(...)` inside conversion code resolves to that getter and fails to compile. The shared generic conversion constructs `Self { state: source.public_state() }` directly. Consumers can use `let view: PublicStateView<'_> = (&state).into()` when that getter exists. A renderer regression checks both `r#type` and `from` identifiers.


#### Generated-source measurement after the shared trait

The same 137 fixture sources were regenerated with the one-impl design: 80 contain the new view, and the fixture diff is now 2,441 net lines instead of 4,761, a reduction of 2,320 lines relative to the first probe. Counter grows 986 bytes (9,308 to 10,294) instead of 2,051; Boolean Cell grows 745 bytes (5,989 to 6,734) instead of 1,810. These are source-size measurements only, not compile-time or runtime performance measurements. The one-impl design keeps explicit getters and uses one runtime trait for state holders; it does not add a proc macro.


#### Consumer API refinement

The shared runtime trait also covers `ConstructorResult`, `CircuitResult`, `CircuitContext` and `QueryContext`. This removes the last manual state-path expression from a native call site: `let view = PublicStateView::from(&result); let round = view.round()?;`. The earlier after example remains a valid lower-level conversion from `&StateValue`; the direct result conversion is preferred for application code. It still borrows the same upstream state and adds no query, gas or transcript effect.

### Local implementation checkpoint — 2026-10-04

Conventional, GPG-verified and DCO-signed local commit `c2ccbd6a08a76ed7d7b9014e426a627b9cc4f086` implements ABI 29. It adds `PublicStateSource` adapters and typed `CellSlot`/`CounterSlot` inspection in the runtime, one generated generic conversion plus declaration-named getters in the `syn` renderer, documentation, and all 137 generated fixtures. Private IR schema 8 is unchanged. The first probe's four repeated conversions were replaced before commit.

Changed-source validation: 58/58 renderer tests passed; 137/137 fixtures were regenerated then checked fresh with zero stale/fail; focused Counter (5), Boolean Cell (2), chunked Cell oracle (1), and constructor/observed state (2) tests passed under Cargo. The constructor test uses the feature-enabled observed-state conversion; the Counter test compares malformed path/alignment errors to the raw ledger decoder. A clean checkout at the exact commit is running Nix package, archive and proof gates. No exact-head gate or remote CI claim is made yet. `doc/ledger-adt.mdx` is user-owned and was not staged.


#### Archive-consumer acceptance amendment

Conventional GPG-verified/DCO local commit `b665860ea12e0f7d503d2f128ac1d170848bd045` adds explicit `PublicStateView` calls for Counter and Boolean Cell to the two-crate archive-only consumer. It follows implementation commit `c2ccbd6a` and remains under the same focused issue #158/ADR because it verifies that delivery. The earlier `c2ccbd6a` release manifest verifies its two source archives; final-HEAD archive-only consumer and exact-HEAD package/proof gates are still pending.

### Exact final-head delivery — 2026-10-04

**Identity.** Clean detached final HEAD `b665860ea12e0f7d503d2f128ac1d170848bd045` (tree `64c03616c409f4cd2eb6afdf6d35cd7913f0c83a`) contains signed/DCO implementation `c2ccbd6a` and signed/DCO archive-consumer test `b665860e`. `git verify-commit` reported good GPG signatures for both. Runtime ABI 29; private IR schema 8. Root working tree still has only the user-owned `doc/ledger-adt.mdx` unstaged.

**Exact package.** Clean source built Nix `compactc` `${HISTORICAL_NIX_STORE}/s6igb9r2n15dmz1w0mxdgvwmi5z2sv0f-compactc` (version 0.31.133). The final-head macro/runtime archives in `target/rust-runtime-release-b665860e.json` were packaged and independently reverified; manifest says `dirty:false`, macro 9 entries SHA256 `1aac4f8129895a1a456153bde609bd28222f033629ad33e1b509ce2553cf9ccf`, runtime 248 entries SHA256 `f0819ce355a0d21d4e9a6f4c7ec82d87b68a06e76ede0262332d3610621e1b80`. The archive-only external consumer generated Counter and Boolean Cell contracts from that compiler, used both named `PublicStateView` getters, and resolved exactly one shared runtime archive without editing generated output; exit 0.

**Behavior.** The exact-head packaged `check_compactc_target.py --consumer --proof` gate exited 0. Its retained log `${LOCAL_EVIDENCE}/compact-adr59-proof-b665860e.log` contains 95 replayed/partitioned generated traces, 95 validated/applied proven calls, and ends `compactc target boundary and manifest: passed`. Changed-source renderer 58/58, fixture freshness 137/137, focused Counter 5/5, Boolean Cell 2/2, chunked Cell oracle 1/1, constructor/observed 2/2 and final-commit clean-source header check 1,521/1,521 passed. The wrong-shape/alignment Counter errors match `read_cell_at_path`; chunked Boolean/Field getters match the same raw decoder at nested paths. The read-only projection takes `&StateValue` through the existing slot decoder and has no VM, gas or transcript mutation path.

**Limits.** `PublicStateView` covers Cell and Counter declarations only. Set, Map, List and Merkle inspection need separate decisions; witness `LedgerView` remains the metered execution API. Source size improved relative to the first four-conversion probe, but no compile-time/runtime performance claim is made. Issue #158 stays open for same-commit remote CI; #106/#114 retain registry publication and unpatched registry-consumer gates. No branch push or package publication is claimed.
