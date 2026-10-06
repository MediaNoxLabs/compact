---
id: RUST-ADR-0018
alias: ADR-0018
title: "Meter List witness reads and map VM Maybe to Option"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "witnesses", "gas"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: a9c9158ad2a3d3fe744a4967999063305410be67eb0b3b53c579e1a2fbb06994
---
# RUST-ADR-0018 — Meter List witness reads and map VM Maybe to Option

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept metered List witness reads and an explicit VM Maybe-pair to Rust Option projection. Keep absence/default payload semantics and local structural views distinct. ADR0023 extends the original Field evidence to selected other element types; this is an extension, not wholesale replacement or proof of every List shape.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#119 closure](https://github.com/MediaNoxLabs/compact/issues/119#issuecomment-6017430201). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`26205ca2`](https://github.com/MediaNoxLabs/compact/commit/26205ca2fb91f1a203f8c19fdcb713a25e2847a4) · [`87e7c3c4`](https://github.com/MediaNoxLabs/compact/commit/87e7c3c40cf4b10efd33109a26a26f851b550189). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 18
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 119
```

## Historical decision and amendments

### Problem and evidence

Generated Rust `LedgerView::items()` returns structural `ListView`. Its `head`, `is_empty`, and `length` inspect the ledger's head/tail/length array without a VM query or gas cost. The unchanged `witness_ledger_list.compact` TypeScript witness invokes those three operations in order on empty, populated, covered, and restored List states. Existing research under [ADR-0015 — Meter witness ledger reads through typed projections](0015-meter-witness-ledger-reads-through-typed-projections.md) observed three read queries per witness and approximately 340,000,000 readTime per query. The current saved List oracle pins result/private FAB but lacks query costs and serialized states; this decision adds that evidence before claiming parity.

### Before and after Rust

```rust
// Before: structural List projection; reads are unmetered.
let items = context.ledger.items().unwrap();
let head: Option<Field> = items.head().unwrap();
let empty: bool = items.is_empty();
let length = items.length().unwrap();
```

```rust
// Decision: each operation runs one canonical ledger-8 VM query.
let items = context.ledger.items().unwrap();
let head: Option<Field> = items.head().unwrap();
let empty: bool = items.is_empty().unwrap();
let length = items.length().unwrap();
```

### Decision and ownership

The runtime adds `MeteredListView<'a, T, D>` backed by `WitnessReadMeter` and the validated root List index. It calls existing `head_list`, `is_empty_list`, and `length_list` ledger-8 query functions. `head_list::<T, (bool, T)>` decodes the VM's Compact Maybe pair; the view converts `(false, _)` to `None` and `(true, value)` to `Some(value)` without another query. `head` requires the same `T: Default` constraint as the canonical VM program. The meter charges each successful query's actual `RunningCost`; failures yield `CompactError` and do not silently return an uncharged value. Structural `ListView` remains for explicit low-level use. The emitter selects the metered view from the typed List declaration; no new opcode, cost constant, or IR variant is introduced.

The current renderer explicitly rejects nested physical List paths, and the existing List VM functions address root indices. This decision covers that supported root shape; it does not claim nested List support. Because `is_empty` changes from `bool` to `Result<bool, CompactError>`, generated/runtime ABI advances 8→9; private IR schema remains 8. The generated witness trait still returns `(Private, T)`, so user witnesses must handle method errors until a separate fallible trait design lands.

### Alternatives and risks

Direct structural reads underreport conditional/repeated witness costs. A fixed emitter surcharge cannot know which methods the user witness calls. Decoding head structurally while metering only emptiness/length leaves the largest query hidden. Returning the VM's `(bool, T)` pair directly would leak representation details into user witness code; `Option<T>` keeps the developer-facing API. The conversion must preserve Compact's absent default behavior and one-query accounting. Verify empty and populated heads, covered/restored transitions, repeated reads, invalid indices and rejected-query costs.

### Verification and delivery gate

- Freshly compile the unchanged TypeScript List source; extend its capture with exact tagged serialized state and ordered four-dimensional query costs for before/after/covered/restored states.
- Compare generated Rust result, private state/FAB, state bytes, each query-cost prefix and total across all four states.
- Test repeated reads, VM Maybe-to-Option conversion, invalid List index and no failed-query accrual.
- Regenerate fixtures; pass renderer/runtime, all-target workspace compile, 131 fixture freshness, oracle/rejection, packaged external consumer, offline ledger-8 proof/application, and clean-source package compatibility gates.
- Append a dated local signed/DCO commit and evidence to this ADR, the issue, and [Milestone 2 — ADR delivery map](references.md#private-note-08). Keep remote CI and publication open until proven.

### Decision history

- 2026-10-02: Proposed from the existing List witness fixture and TypeScript query research. Exact cost/state capture and implementation pending. Tracking: [#119](https://github.com/MediaNoxLabs/compact/issues/119), parent [#116](https://github.com/MediaNoxLabs/compact/issues/116) / [#104](https://github.com/MediaNoxLabs/compact/issues/104), milestone [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


### Delivery amendment — 2026-10-02: ABI-9 List witness VM views

Local conventional GPG-signed/DCO commit `87e7c3c40cf4b10efd33109a26a26f851b550189` implements the decision. Before, generated `LedgerView::items()` returned a structural `ListView`, so `head`, `is_empty`, and `length` inspected state without witness query gas. After, it returns `MeteredListView`: the runtime validates the root List shape, executes the existing ledger-8 `head_list`, `is_empty_list`, and `length_list` VM programs, accumulates each successful query's actual `RunningCost`, and returns errors as `Result`. `head_list::<T, (bool, T)>` decodes the VM Maybe pair; the view converts it to the existing developer-facing `Option<T>` without another query. The generic head method requires `T: Default` and `Value: From<T>`, matching the canonical VM and tuple CellValue bounds. The emitter chooses the view from typed List declarations. Structural `ListView` remains an explicit low-level API. Generated/runtime ABI advances 8→9; private IR schema remains 8; 131 fixture libraries were regenerated.

A fresh TypeScript compile of unchanged `witness_ledger_list.compact` extended the saved oracle with exact tagged serialized state and ordered query costs for empty, after one prepend, covered by a second prepend, and restored after drop. The original result, private state, and FAB fields were unchanged. Each witness executes `head`, `isEmpty`, and `length` as three 340,000,000-readTime queries (1,020,000,000 total) with zero bytes written/deleted. Compute totals are respectively 4,276,690,657; 4,278,577,964; 4,278,577,503; and 4,278,577,964. Generated Rust matches result, private state/FAB atoms and alignment, all four tagged state bytes, and each ordered four-dimensional query-cost prefix and total. Focused tests also verify absent/present head conversion, repeated query charging, invalid-index rejection and zero failed-query accrual.

Three focused List tests, 53 renderer/four CLI tests, the runtime suite, all-target workspace compile, 131/131 fixture freshness, 37 pinned oracle sources, two rejection probes, formatting and scoped diff checks pass. The Nix packaged external consumer and all 45 offline ledger-8 replay/proof/verification/validation/application cases pass. Clean-source manifest `target/rust-runtime-abi9-clean.json` was written and verified from `87e7c3c4` with `dirty: false`; the macro/runtime archives have 8/227 entries and compile after unpacking. The unrelated `doc/ledger-adt.mdx` edit was preserved byte for byte and excluded from the commit.

This is accepted-partial. The compiler still rejects nested physical List paths; only a `Field` List witness fixture exercises the emitted generic view, and wider element types need focused behavioral coverage. The generated witness trait still returns `(Private, T)`, requiring user code to handle `Result` rather than using `?` through a generated circuit. Merkle witness views, cumulative gas-limit semantics, remote CI, registry publication and wallet/node submission remain open. The branch is local and unpushed. Track delivery under [#119](https://github.com/MediaNoxLabs/compact/issues/119), parent [#116](https://github.com/MediaNoxLabs/compact/issues/116) / [#104](https://github.com/MediaNoxLabs/compact/issues/104), in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


### Wider element parity extension — 2026-10-03

[ADR-0023 — Prove typed List witness heads across Compact value shapes](0023-prove-typed-list-witness-heads-across-compact-value-shapes.md) / [#123](https://github.com/MediaNoxLabs/compact/issues/123) closes the earlier Field-only evidence gap for five root List element types: Boolean, Uint<16>, Bytes<3>, enum and struct. Local signed/DCO `26205ca2` pins TypeScript empty/populated VM query and state parity, adds generated one-dependency consumer coverage, and enables recorded typed List pushes through existing slots. Five new proven calls are validated/applied offline. ABI 11 and IR schema 8 remain unchanged. Nested List paths and untested element/expression forms are still open; remote CI and release remain pending.
