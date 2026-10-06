---
id: RUST-ADR-0015
alias: ADR-0015
title: "Meter witness ledger reads through typed projections"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "witnesses", "gas"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 608dadc1420020bff9bb2dde1e15a8ccfb80ec59f0a220a5ccc2e7f90d5bbad9
---
# RUST-ADR-0015 — Meter witness ledger reads through typed projections

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept an explicit borrowed witness-read meter and declaration-selected projections, initially Cell/Counter and then Set. Meter canonical VM reads and preserve private transitions/order; local structural inspection remains distinct. Later fallible witness and other view decisions extend this foundation rather than making every inspection query chargeable.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#116 closure](https://github.com/MediaNoxLabs/compact/issues/116#issuecomment-6017424818). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`2ca64f6f`](https://github.com/MediaNoxLabs/compact/commit/2ca64f6f26390b17f018ba5319f5330dd4ec3922) · [`98d33ca7`](https://github.com/MediaNoxLabs/compact/commit/98d33ca7d8ee52ab129d861f0604dc0826d929ca). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 15
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 116
```

## Historical decision and amendments

### Problem and evidence

A witness can inspect the current ledger through the generated `LedgerView`. Its Cell getter currently decodes the state directly, while TypeScript runs a ledger VM read query. The new witnessed Cell oracle captures both queries. For one `write_secret` call, TypeScript spends readTime 170,000,000 on the witness Cell read and 85,000,000 on the write; generated Rust reports only 85,000,000. A two-write call doubles the gap. The native frame introduced under [ADR-0005 — Use a typed runtime DSL instead of a body-wide macro](0005-use-a-typed-runtime-dsl-instead-of-a-body-wide-macro.md) correctly sums the operations it sees but cannot observe a direct getter hidden inside the user witness.

### Before and after code

```rust
// Before: a witness projection reads the state without a VM query or charge.
pub fn cell(&self) -> Result<Field, CompactError> {
    runtime::ledger::read_root_cell::<Field, _>(self.state, 0)
}
let (next_private, value) = witnesses.secret(context.witness_context_with(
    LedgerView { state: context.query.state.get_ref() }
), seed);
```

```rust
// Decision: the projection runs the canonical VM read and records its cost.
pub fn cell(&self) -> Result<Field, CompactError> {
    self.meter.read_cell::<Field>(&[0])
}
let (frame, value) = frame.witness_metered(|context, meter| {
    witnesses.secret(context.witness_context_with(LedgerView {
        state: context.query.state.get_ref(),
        meter,
    }), seed)
});
```

### Decision and ownership

Use an explicit `WitnessReadMeter` borrowed by generated ledger views. It runs the matching ledger-8 read program via the existing typed runtime query, accumulates its `RunningCost`, and returns the decoded value. The witness still owns its private-state transition. Native `CircuitFrame` and recorded `RecordingFrame` add the observed witness-read cost in order; the general emitter adds it to its explicit total. The emitter constructs the view and meters Cell/Counter projection methods from typed ledger declarations. Collection and Merkle view methods require their own operation-level metering design, so this ADR is initially a Cell/Counter slice. The generated/runtime ABI must advance from 5 to 6; private IR schema 8 remains unchanged.

An implicit thread-local meter was rejected because it hides effect ownership and is fragile under nested or concurrent witness calls. Charging a fixed read amount in the emitter was rejected because witness implementations may read zero, one or many fields conditionally, and cost depends on the VM query and state.

### Acceptance and risks

- Capture exact TypeScript serialized state, per-query four-dimensional gas, private FAB values/alignment, and ordered public transcript for single, repeated and nested witnessed Cell calls.
- Assert generated native and recorded results against the capture. Test multiple reads, zero reads, and query rejection/error behavior.
- Regenerate all affected fixtures and verify the compiler, external consumer, runtime, workspace, proof/application and package compatibility gates.
- Keep collection/Merkle witness read accounting explicitly open until each view operation can be metered and compared.

The meter borrows the current query and cost model. It does not append a public proof operation for a private witness read. Query failure must not silently produce an uncharged value. The principal costs are a new runtime API, generated view field, ABI change and fixture churn; this is justified by exact cross-target gas parity.

### Decision history

- 2026-10-02: Proposed after the extended witnessed Cell oracle exposed an exact readTime mismatch (Rust 85,000,000 versus TypeScript total 255,000,000 for one call). Implementation and full gate evidence are pending.

Tracking: [focused issue #116](https://github.com/MediaNoxLabs/compact/issues/116), parent [#104](https://github.com/MediaNoxLabs/compact/issues/104) and related frame issue [#110](https://github.com/MediaNoxLabs/compact/issues/110), assigned to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).

### Delivery amendment — 2026-10-02: Cell and Counter slice

Local conventional GPG-signed/DCO commit `98d33ca7d8ee52ab129d861f0604dc0826d929ca` implements the explicit meter. `WitnessReadMeter::read_cell` runs the canonical ledger VM query against the current witness context, decodes the typed Cell, and accumulates the query's `RunningCost`. Generated Cell/Counter getters call it; generated native `CircuitFrame`, recorded `RecordingFrame` and the general emitter add the observed cost in evaluation order. The public Verify program still contains only public circuit operations, so private witness reads do not become replayable public ops. Generated/runtime ABI advances 5→6; the private compiler IR stays schema 8. The compatibility assertion intentionally rejects a new generated crate paired with an old bundled runtime.

**Before/after evidence.** For one `write_secret` call, generated Rust previously reported readTime 85,000,000 (the write alone), while the TypeScript query capture sums 170,000,000 for the witness read plus 85,000,000 for the write. The new Rust result matches 255,000,000 and the other three cost dimensions. `write_twice` and `write_nested_twice` each match four ordered TypeScript query costs and six ordered public Verify operations. The exact serialized contract state before/after, final Cell value, private state and FAB atoms/alignment match the fresh TypeScript capture. A witness-only Counter circuit now records its VM read cost in Rust; TypeScript's wrapper still reports zero because it has no public query, while its captured ledger query has the same four-dimensional cost. Fresh TypeScript compilations reproduced both saved captures byte for byte.

**Verification.** Seven witnessed Cell tests cover native/recorded/TypeScript parity, zero and repeated reads, failed read cost, and short-circuit behavior. The Counter fixture passes its exact query gas test. All 53 renderer and four CLI tests, the runtime suite, 131/131 fixture freshness, 37 pinned oracle sources, two rejection probes and the packaged external consumer plus 45 offline replayed/proven/verified/validated/applied ledger-8 calls pass. The full Cargo workspace test and clean-source package manifest rehearsal are still running as this amendment is written; append their outcomes rather than assuming success. Formatting, diff check, GPG signature and DCO pass. All 131 generated libraries were regenerated because ABI 6 is asserted in each module.

**Limits.** Set, Map, List and Merkle witness views still use direct structural projection methods; this slice does not claim general witness gas parity. A failed projection returns `CompactError` from the meter, but existing user witness trait methods return a value pair rather than `Result`, so generated witness error propagation needs a separate design. Publication, remote CI and wallet/node submission remain open. Track completion under [#116](https://github.com/MediaNoxLabs/compact/issues/116) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2); related frame parity remains [#110](https://github.com/MediaNoxLabs/compact/issues/110).

Package verification update — 2026-10-02: from the clean committed tree at 98d33ca7 (dirty: false), check_release_packages.py wrote and then verified target/rust-runtime-abi6-clean.json. Macro and runtime archives compile after unpacking; the runtime archive has 227 entries. This is a local source/package provenance check, not registry publication, a real candidate tag, or an unpatched remote consumer. The full workspace suite remains in progress.

### Research amendment — 2026-10-02: collection witness query boundary

A fresh TypeScript compiler probe from the unchanged `witness_ledger_set`, `witness_ledger_map`, and `witness_ledger_list` sources confirms that witness collection view methods execute ledger VM queries. Instrumenting `QueryContext.query` showed three read queries per Set witness (`isEmpty`, `size`, `member`; each 170,000,000 readTime), three per Map witness before insertion (`member`, `isEmpty`, `size`; each 170,000,000 readTime), and four after insertion including `lookup` (255,000,000 readTime). List witness reads include `isEmpty`, `length`, and `head` queries (340,000,000 readTime each). The current Rust `SetView`, `MapView`, and `ListView` inspect state structurally without charging these VM costs. Existing canonical query functions in `runtime-rs/src/ledger/collections.rs` provide the operations to reuse. This is local research, not an implementation or parity claim.

The method signature boundary needs review before implementation: Set/Map `member` and `is_empty` currently return `bool`, while VM queries can fail. A metered wrapper must keep the witness-facing API usable and propagate query failure through the enclosing frame. Do not silently discard failures or claim collection parity from structural values alone. Focused [#116](https://github.com/MediaNoxLabs/compact/issues/116) remains open for this slice; record exact before/after, emitter/runtime ownership, ABI impact, TypeScript state/gas/Verify and proof evidence in a later delivery amendment.


### Set projection design slice — 2026-10-02, before implementation

**Problem.** A generated witness currently receives `SetView` whose `is_empty`, `size`, and `member` read the Map structure directly. The TypeScript witness performs three separate ledger VM queries for these methods, so Rust underreports four-dimensional query gas. A plain `bool` return from `member`/`is_empty` cannot represent VM query rejection.

```rust
// Before in a generated witness implementation.
let seen = context.ledger.seen()?;
let present: bool = seen.member(true); // structural read; no VM cost

// Proposed after, using a typed metered projection.
let seen = context.ledger.seen()?;
let present: bool = seen.member(true)?; // canonical VM query + cost
```

The runtime will add a `MeteredSetView` that borrows `WitnessReadMeter`, validates the declared Set path and delegates each method to existing `member_set`, `size_set`, and `is_empty_set` queries. The meter charges each returned `RunningCost`; the generated `LedgerView` returns this view for Set fields. The pure structural `SetView` stays available for explicit low-level inspection. This changes the generated witness-facing method signatures to `Result`, so generated/runtime ABI must advance to 7 if adopted; private IR schema 8 remains unchanged. The witness trait itself still returns `(Private, T)`, so user implementations must handle a failed method (for example with `expect` or an explicit policy). A later fallible witness-trait design is needed for clean circuit-level propagation; do not claim it from this slice.

The first acceptance gate is the unchanged `witness_ledger_set` Compact source: capture exact TypeScript per-query costs and compare Rust before/after witness result, private state/FAB, serialized ledger state and four-dimensional total gas. Also check query rejection behavior at the view API and keep zero/repeated reads distinct. Map/List/Merkle, public Verify, package/proof, remote CI and release remain open until tested. Track this extension in focused [#116](https://github.com/MediaNoxLabs/compact/issues/116) in `rust-backend-v2`.


ABI-6 baseline workspace update — 2026-10-02: `cargo test -q --workspace --exclude compact` completed with exit code 0 on committed source `98d33ca7`. This broad local suite covers the generated fixture crates and runtime; it does not cover the new uncommitted ABI-7 Set work or remote CI. The clean-source package manifest for ABI 6 had already written and verified with `dirty: false`.


### Delivery amendment — 2026-10-02: Set witness projection through the ledger VM

Local conventional GPG-signed/DCO commit `2ca64f6f26390b17f018ba5319f5330dd4ec3922` extends the typed witness-read decision from Cell/Counter to Set. The former generated `LedgerView::seen()` returned structural `SetView`; its `member` and `is_empty` returned plain `bool`, and `size` read the Map structure without VM gas. The new generated method returns `MeteredSetView`, and a witness handles each query result explicitly:

```rust
// Before: structural projection, no witness query gas.
let seen = context.ledger.seen().unwrap();
let present = seen.member(true);

// After: canonical ledger-8 VM query and typed failure.
let seen = context.ledger.seen().unwrap();
let present = seen.member(true).unwrap();
```

`runtime-rs/src/ledger/collections.rs` owns `MeteredSetView`, validates the declared physical path against the meter’s own state, and exposes `member`, `size`, and `is_empty` as `Result`. `runtime-rs/src/context.rs` runs the existing `member_set`, `size_set`, and `is_empty_set` query functions and accumulates their actual `RunningCost`; no opcode or cost constant is duplicated. `tools/compact-rust-backend/src/witness.rs` selects the metered view from typed Set declarations using `syn` output. The pure structural `SetView` remains available for explicit low-level use. Generated/runtime ABI advances 6→7, while private IR schema stays 8; all 131 fixture libraries were regenerated. The generated witness-facing Set methods now require callers to handle `Result`.

**Cross-target evidence.** A fresh TypeScript compile of the unchanged `witness_ledger_set.compact` source reproduced the saved capture byte for byte. It performs `isEmpty`, `size`, and `member` as three VM reads before and after insertion. Each read has 170,000,000 readTime; the total is 510,000,000 readTime and 3,756,612,609 computeTime, with zero bytes written/deleted. Generated Rust matches each prefix and the total in all four dimensions. Its before/after results, private state/FAB atoms and alignment, and tagged serialized initial/final contract state match TypeScript. A separate nested-path test verifies a two-element physical Set path. Repeated reads charge twice; a rejected path returns `CompactError` without charging the meter. The shared Set/Map capture script now records per-query costs and state bytes; the existing Map result/private capture remains unchanged and its fixture test passes.

**Gates.** Five focused Set tests, one Map fixture test, 53 backend renderer and four CLI tests, the full runtime suite, `cargo check --locked --offline --workspace --exclude compact --all-targets`, 131/131 fixture freshness, 37 pinned oracle sources, two rejection probes, formatting and scoped diff checks pass. The Nix packaged compiler/consumer/proof gate passes 45 offline ledger-8 calls replayed, partitioned, proven, verified, validated and applied. After the signed commit, a clean-source package manifest `target/rust-runtime-abi7-clean.json` wrote and verified against `2ca64f6f` with `dirty: false`; the macro archive has 8 entries and runtime archive 227, and both compile after unpacking. The unrelated `doc/ledger-adt.mdx` worktree edit was preserved byte for byte and excluded from the commit.

**Limits and next gate.** A `Result` from a Set view can be handled by the witness implementation, but the generated `Witnesses` trait still returns `(Private, T)`, so it does not yet support `?` to propagate query errors cleanly through the circuit. Map, List, Merkle, cumulative gas-limit semantics, remote CI, registry publication and wallet/node submission remain open under [#116](https://github.com/MediaNoxLabs/compact/issues/116), parent [#104](https://github.com/MediaNoxLabs/compact/issues/104), and milestone `rust-backend-v2`. The branch remains local; no push or remote CI is claimed.
