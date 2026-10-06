---
id: RUST-ADR-0039
alias: ADR-0039
title: "Read witness ledger state through typed slots"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: affed888c34fb7bab6b343211fe9018e51e4a0778aab15770e713341e02b2611
---
# RUST-ADR-0039 — Read witness ledger state through typed slots

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept declaration-typed slot delegation for witness getters across the delivered Cell, Counter, collection and Merkle families. Runtime meters retain query/gas/error ownership; generated getters stop duplicating physical paths. Scalar Map view bounds and metered versus structural semantics remain explicit, with no new body-wide DSL.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#138 closure](https://github.com/MediaNoxLabs/compact/issues/138#issuecomment-6017463240). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`0f2870f7`](https://github.com/MediaNoxLabs/compact/commit/0f2870f73830c7612ad60656980d4b79aa91a36c) · [`4f229ce9`](https://github.com/MediaNoxLabs/compact/commit/4f229ce9ec081e18dc20cd6440e9909947bcde5a) · [`a45884bd`](https://github.com/MediaNoxLabs/compact/commit/a45884bd193e680b453954e3e752ec4efd3e9fb7). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 39
status: accepted
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/138
```

## Historical decision and amendments

### Problem

ABI-16 generated crates have typed `ledger_slots` constants for native and recorded calls, but `LedgerView` witness getters independently embed physical paths, indexes, type parameters and Merkle depth in low-level read/view calls. A declaration can therefore have two generated representations of its physical location. The public getter API is useful and should stay visible. The repeated internal read plumbing is not useful to a consumer implementing `Witnesses` or `TryWitnesses`.

A read-only inventory of all 132 ABI-16 fixture libraries found 70 `LedgerView` getters in 21 libraries: 48 Cell/Counter reads, 7 Set, 3 Map, 6 List, 4 plain Merkle and 2 historic Merkle views. Every getter uses a raw path or index that already exists on its typed slot. The election fixture has nine such getters, and asset-registry has twenty. The `codegen-rust` tiny snapshot emitted an explicit `OpProgramGather` in its `Ledger::value` getter; the AST backend already moved execution and metering into runtime helpers, and this proposal completes the type/path ownership boundary.

### Before and after generated Rust

Current ABI-16 election excerpt:

```rust
pub fn authority(&self) -> Result<runtime::FixedBytes<32>, runtime::CompactError> {
    self.meter.read_cell::<runtime::FixedBytes<32>>(&[0])
}
pub fn committed(&self) -> Result<MeteredSetView<'a, runtime::FixedBytes<32>, DefaultDB>, CompactError> {
    runtime::ledger::metered_set_view_at_path::<runtime::FixedBytes<32>, _>(self.meter, &[7])
}
pub fn committed_votes(&self) -> Result<MeteredMerkleTreeView<'a, MerkleTreeDigest, DefaultDB>, CompactError> {
    runtime::ledger::metered_merkle_tree_view_at_path::<MerkleTreeDigest, _>(self.meter, &[5], 10)
}
```

Proposed source retains the same public getter names and result types:

```rust
pub fn authority(&self) -> Result<runtime::FixedBytes<32>, runtime::CompactError> {
    crate::ledger_slots::authority.witness_read(self.meter)
}
pub fn committed(&self) -> Result<MeteredSetView<'a, runtime::FixedBytes<32>, DefaultDB>, CompactError> {
    crate::ledger_slots::committed.witness_view(self.meter)
}
pub fn committed_votes(&self) -> Result<MeteredMerkleTreeView<'a, MerkleTreeDigest, DefaultDB>, CompactError> {
    crate::ledger_slots::committed_votes.witness_view(self.meter)
}
```

The public `LedgerView` remains an ordinary Rust struct with typed, discoverable methods. No body-wide macro or dynamic field lookup is introduced. The runtime slot stores the declaration path/index and value type; for Merkle, `MerkleSlot<Leaf, DEPTH, HISTORIC>` stores leaf/depth/kind while the witness view still requires a separate root digest type inferred from the getter return type.

### Runtime API and first increment

The first bounded increment is Cell and Counter, covering 48 of 70 getters. Add `CellSlot<T>::witness_read<D: DB>(self, meter: &WitnessReadMeter<'_, D>) -> Result<T, CompactError>` and `CounterSlot::witness_read<D: DB>(...) -> Result<u64, CompactError>`. Each delegates to the existing `meter.read_cell` with `self.path`; there is no new VM program, cost calculation or error mapping. The generated Counter getter keeps its `u64` to `BoundedUint<u64::MAX>` conversion exactly as today. Emit slot calls only when the corresponding descriptor exists; malformed/private IR still returns a structured renderer error with the declaration location.

Follow-through for Set/Map/List and plain/historic Merkle adds `witness_view` on the corresponding slot, delegating to the existing `metered_*_view` constructor with `self.path` or `self.index`. Preserve the `&'a WitnessReadMeter<'a, D>` lifetime needed by view objects. Keep `MapSlot<K,V>` witness views limited to `V: CellValue`; nested Map values must retain current unsupported behavior. Plain and historic Merkle methods must be separated by the slot const kind and infer the root digest independently of leaf type. The view methods must use the existing constructors so validation and later charged reads remain unchanged.

### Ownership and compatibility

`runtime-rs/src/slots.rs` owns the typed slot delegate methods. `runtime-rs/src/context.rs` and `runtime-rs/src/ledger` continue to own metered query execution, gas and errors. `tools/compact-rust-backend/src/witness.rs` emits slot references from the existing declared field identity and preserves its source-location diagnostics. No new proc macro is justified because the public getter body remains short and meaningful. `ledger_slots` remains the single generated owner of paths, indexes, types and depth. ABI must move to 17 when generated code depends on new runtime methods; private IR schema stays 8 unless implementation finds an actual IR gap. No native/recorded method signature changes are intended.

### Read-only shape probe and limits

A temporary `/tmp` rewrite replaced all 70 getter bodies with proposed slot delegation and ran `rustfmt`, without editing the repository or compiling the transformed libraries. Across the 21 affected fixture libraries, total formatted lines moved 10,741→10,685 (−56); election 1,035→1,021 and asset-registry 1,422→1,402. This is a source-shape estimate, not compile, gas, proof, or correctness evidence. The principal gain is one typed source of physical ledger locations and no raw path in generated witness getters. A standalone external extension-trait typecheck is pending while the parent task runs the ABI-16 proof gate.

### Acceptance and risks

- Before implementation, typecheck a standalone `/tmp` extension-trait prototype against the current runtime for all seven getter families. Confirm lifetime inference, especially Set/Map/List/Merkle, and Merkle root-versus-leaf typing.
- Implement and test Cell/Counter first with a representative composite path, Field/Bytes/enum Cell values, Counter width conversion, a successful witness read, a rejected ledger projection, and unchanged four-dimensional gas. Regenerate all fixtures and compare no-update freshness.
- Complete Set/Map/List/Merkle follow-through only after exact existing view validation, read-meter cost, error and TypeScript oracle parity. Require ordered FAB/Verify transcript and offline proof/application for witnessed calls that exercise each family. Avoid claiming only a source-size win.
- Check that identifiers used for `ledger_slots::<field>` remain collision-safe and that imported/nested physical paths route to the exact declared slot. A public slot method exposes a typed path constructor, not an access-control boundary; this matches current `slots.rs` semantics.
- Run a separate one-dependency generated-crate consumer, packaged `--consumer --proof`, runtime archive and remote CI gates before production closure. Measure same-host compile impact if feasible; no speed claim follows from the shape probe.

### Tracking

- ADR status: proposed; no repository implementation or push.
- Focused issue: pending creation in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Related: [ADR-0002 — Generate typed ledger field descriptors](0002-generate-typed-ledger-field-descriptors.md), [ADR-0015 — Meter witness ledger reads through typed projections](0015-meter-witness-ledger-reads-through-typed-projections.md), [ADR-0038 — Declare generated witness signatures once](0038-declare-generated-witness-signatures-once.md).


### Tracking update — 2026-10-03

Focused issue [#138](https://github.com/MediaNoxLabs/compact/issues/138) is assigned to rust-backend-v2. The earlier pending issue line was the proposal-time state. No repository implementation or push is claimed.


### Standalone lifetime probe — 2026-10-03

A dependency-free `${LOCAL_EVIDENCE}/compact-slot-lifetime-probe.rs` compiled with `rustc --edition=2024` and executed. It models the exact meter borrow shape and two specialized `Slot<Leaf, DEPTH, false/true>::witness_view` methods. The return type `View<'a, Digest, D>` inferred a separate root digest while the slot retained `Leaf`, and both plain/historic const-kind methods resolved without explicit turbofish. This validates Rust lifetime and type inference for the hardest proposed signature in isolation, not compatibility with Midnight ledger types.

A fuller `${LOCAL_EVIDENCE}/compact-witness-bridge-probe/consumer/examples/slot_probe.rs` has extension-trait delegates for Cell, Counter, Set, Map, List, plain and historic Merkle using the actual runtime API signatures. Both Cargo and direct rustc typecheck attempts against the large generated runtime metadata sat at 0% CPU on this host and were stopped; neither produced a pass or diagnostic. The parent task needed the shared Cargo target for a production gate. Keep the dependency-backed typecheck as a pre-implementation gate and do not infer the seven real methods compile from the pure mock. No repository source was edited.


### Cell and Counter implementation slice — 2026-10-03

The first bounded slice is implemented locally in the shared checkout, without a commit or push yet. `CellSlot<T>::witness_read` and `CounterSlot::witness_read` delegate to the existing `WitnessReadMeter::read_cell` using the slot path. The AST emitter now builds every Cell/Counter `LedgerView` getter from `crate::ledger_slots::<field>`; the public getter name, result type and Counter `u64` to `BoundedUint<u64::MAX>` conversion are unchanged. No witness trait, native/recorded call, VM opcode, cost model, FAB or private IR schema changed. Generated/runtime ABI moves from 16 to 17 because generated code needs the new runtime methods; compiler toolchain remains 0.31.133 and private IR schema remains 8.

Actual generated Cell before/after (`witness-ledger-cell`):

```rust
// ABI 16
pub fn flag(&self) -> Result<bool, runtime::CompactError> {
    self.meter.read_cell::<bool>(&[0])
}
// ABI 17
pub fn flag(&self) -> Result<bool, runtime::CompactError> {
    crate::ledger_slots::flag.witness_read(self.meter)
}
```

Actual Counter body change (`witness-ledger-counter`):

```rust
// ABI 16
let value = self.meter.read_cell::<u64>(&[0])?;
// ABI 17
let value = crate::ledger_slots::round.witness_read(self.meter)?;
runtime::BoundedUint::<18446744073709551615>::new(value as u128)
```

The renderer and runtime guides state ABI 17. All 132 fixture libraries were regenerated with the pinned Scheme compiler and the no-update check reports 132 current, 0 stale, 0 failed. Exactly 48 Cell/Counter getters across 15 libraries now call `.witness_read(self.meter)`; the remaining 22 Set/Map/List/plain/historic Merkle view getters retain their old path/index calls for later issue #138 slices. The first slice changes generated fixture total lines by −11, including 132 ABI assertions, so it is primarily a path-ownership change, not a meaningful source-size optimization. No compile-time improvement is claimed.

Focused evidence passed: `cargo test -p midnight-compact-runtime --test typed_slot_witness --locked` (1/1) checks Cell and Counter values, each slot read cost against the direct metered read, two wrong-shape `InvalidLedgerCell` errors, and zero-gas `LedgerQueryRejected`. `cargo test -p compact-rust-backend --test render --locked` passed 56/56, including a 16-field valid chunked layout with physical paths `[0,0]` and `[1,0]`; a two-field composite path was correctly rejected by the existing IR validator during test design and the validator was not changed. `cargo test -p compact-rust-witness-ledger-cell-fixture -p compact-rust-witness-ledger-counter-fixture --locked` passed Cell 3/3 and Counter 1/1 against their existing ledger-8 TypeScript captures, including Counter gas dimensions and private FAB. `cargo check -p compact-rust-election-oracle-fixture -p compact-rust-asset-registry-oracle-fixture --locked` passed on generated ABI 17, covering Bytes/enum Cell types and actual multi-segment declaration paths. `cargo fmt --all -- --check` and scoped `git diff --check` passed. The pre-existing user-owned `doc/ledger-adt.mdx` edit was untouched.

The parent task owns the exact-head packaged `--consumer --proof` run and signed/DCO delivery commit; neither result is claimed in this note yet. Wider ADT `witness_view` methods, same-head compile-time comparison, remote CI, publication and live wallet/node acceptance remain open in [#138](https://github.com/MediaNoxLabs/compact/issues/138). This amends the earlier proposal-only tracking text.


### Signed first-slice delivery — 2026-10-03

Conventional GPG-verified/DCO local commit `4f229ce9ec081e18dc20cd6440e9909947bcde5a` delivers the Cell/Counter slice above. `git log -1 --format=%G?` returned `G` and the sign-off trailer is present. The branch was not pushed; the only remaining tracked worktree edit is the unrelated user-owned `doc/ledger-adt.mdx`.

The exact-head rebuilt Nix `compactc` `check_compactc_target.py --consumer --proof` gate exited 0: separate generated consumers and compile-fail checks passed, and all 57 ledger-8.0.3 offline proof/verification/validation/application calls passed. The focused 56 renderer tests, one runtime typed-slot gas/error test, Cell 3/3 and Counter 1/1 oracle fixture tests, election/asset-registry generated crate checks, 132/132 fixture freshness and format/diff checks also passed. No same-head live wallet/node submission was performed for ABI 17; ADR-0040's funded local deploy/call admission was on the immediately preceding ABI-16 commit and the unchanged transaction/proof tooling path.

`check_release_packages.py --manifest target/rust-runtime-release-abi17.json` packaged and compiled the unpacked macro and runtime 0.1.0 archives; `--verify-manifest` reproduced both archive hashes. Macro SHA-256 `a2e5b5ae080e66f27f0cf43c4fdca4c03a200c352efffce954373343121b6d9f` (10,909 bytes, 9 entries); runtime SHA-256 `fd3036cd8ad6ef31c2acfbbeea39ce8336c7ad8d7df987f793807fd50da888c8` (156,271 bytes, 231 entries). The precommit manifest reports prior HEAD `0f2870f7` and `dirty: true` because ABI-17 source and the user doc were uncommitted; verification proves local archive reproducibility, not a clean release tag or registry availability. Its unpacked runtime check still applies the local unpublished macro patch. Issue #138 remains open for the 22 collection/Merkle witness getters, remote CI, publication, release provenance and broader consumer review.

### Collection and Merkle witness-view implementation — 2026-10-03

Problem: ABI 17 still emitted 22 witness getters with duplicated raw paths, indexes or tree depth: 7 Set, 3 Map, 6 List, 4 plain Merkle, 2 historic Merkle. Their declared slots already carry this metadata.

Before (`committed` Set and `committed_votes` tree):
```rust
runtime::ledger::metered_set_view_at_path::<FixedBytes<32>, _>(self.meter, &[7])
runtime::ledger::metered_merkle_tree_view_at_path::<MerkleTreeDigest, _>(self.meter, &[5], 10)
```
After, with unchanged public getter names and result types:
```rust
crate::ledger_slots::committed.witness_view(self.meter)
crate::ledger_slots::committed_votes.witness_view(self.meter)
```

Emitter: `witness.rs` references the existing field slot for Set, scalar Map, List and both Merkle kinds. It no longer synthesizes a path, index or depth literal for witness getter bodies. The generated/runtime ABI moves 17→18; private IR schema 8 and Compact compiler 0.31.133 do not change.

Runtime: `SetSlot<T>`, scalar-valued `MapSlot<K,V>` and `ListSlot<T>` expose `witness_view`, delegating to the same metered ledger constructors as ABI 17. `MerkleSlot<Leaf, DEPTH, false>` and `MerkleSlot<Leaf, DEPTH, true>` each expose a specialized `witness_view<Root,D>`; the return type infers the root digest separately from the declared leaf type. These methods preserve existing view layout checks, later VM reads, gas and errors. Nested Map values still lack a scalar witness-view method; no new codec, VM program, cost rule, macro or DSL is introduced.

Focused tests initially passed: 57 renderer tests, including a new five-family getter regression, and two runtime slot tests comparing Set/Map/List/plain/historic Merkle read results and four-dimensional gas against direct metered constructors, wrong-layout errors and zero-gas rejection. Full fixture, oracle, consumer, proof and archive evidence will be appended after execution. Related issue: [#138](https://github.com/MediaNoxLabs/compact/issues/138), assigned to rust-backend-v2. Branch remains local and unpushed.
### Signed collection and Merkle delivery — 2026-10-03

Conventional local commit `a45884bd193e680b453954e3e752ec4efd3e9fb7` delivers the second typed witness-slot slice. `git log -1 --format=%G?` returned `G` and the commit has a DCO Signed-off-by trailer. The branch remains local and unpushed; only unrelated user-owned `doc/ledger-adt.mdx` remains modified outside the commit.

The pinned ledger-8 compiler regenerated all 132 fixture libraries. A no-update check found 132 current, 0 stale, 0 failed. Exactly 22 Set/Map/List/plain/historic Merkle getters in nine libraries now call `.witness_view(self.meter)`, alongside the existing 48 Cell/Counter `.witness_read(self.meter)` getters. No generated library calls `runtime::ledger::metered_*` directly in a witness getter. Public getter signatures are unchanged. The 132 generated libraries have 262 added and 307 deleted lines, including 132 ABI assertion edits; this is a source-shape observation, not a compile-time speed claim.

Focused evidence passed: 57 renderer tests; 2 runtime typed-slot tests comparing direct metered views for all five families, four-dimensional gas, shape failures and zero-gas rejection; all tests in the nine changed witness-view fixture crates (asset registry, election, Merkle path witness/verify, witness Set/Map/List/list shapes and Zerocash), including existing TypeScript state, query-cost, nested-path and rejection cases. `cargo fmt --all -- --check` and staged `git diff --check` passed. The Nix `check_compactc_target.py --consumer --proof` gate passed with a separate generated consumer, compile-fail checks, and the 57 offline proof/verification/validation/application calls. This is local ledger-8 application, not same-head live wallet/node admission.

The ABI-18 archive rehearsal packaged and compiled the unpacked 0.1.0 macro and runtime crates, then `--verify-manifest` reproduced both hashes at `target/rust-runtime-release-abi18.json`. Macro SHA-256 `136d4b36e0641e4e6a49218e30aec6ba1c1521cc8a5aeeadd63bf3c71363ecd7` (10,906 bytes, 9 entries); runtime SHA-256 `7ce5049333681c5c5eecc8e1efbf0a496ae5d8094d2e8f16655e6992b642811f` (156,978 bytes, 231 entries). The precommit manifest records prior HEAD and `dirty: true` because ABI-18 sources and the user doc were uncommitted during rehearsal. The unpacked runtime check still uses the local unpublished macro patch. Thus this is reproducible local packaging evidence, not a clean tagged release or registry consumer.

Issue [#138](https://github.com/MediaNoxLabs/compact/issues/138) remains open in rust-backend-v2 for remote CI, clean release provenance, registry publication and broader consumer/live-network acceptance. The AST emitter and typed slots now own all 70 generated witness getter paths/indexes; runtime metered constructors still own layout validation and VM reads. The private IR schema remains 8 and compiler toolchain 0.31.133.
### Reviewer examples for each ABI-18 view family

The public `LedgerView` methods continue to return the same `Result<Metered*View<...>, CompactError>` types. These excerpts show only the expression inside each getter; names and declared types come from their generated fixture libraries.

| Family | ABI-17 getter expression | ABI-18 getter expression | Slot-owned metadata |
| --- | --- | --- | --- |
| Set | `runtime::ledger::metered_set_view_at_path::<FixedBytes<32>, _>(self.meter, &[7])` | `crate::ledger_slots::committed.witness_view(self.meter)` | element type and path |
| Map | `runtime::ledger::metered_map_view_at_path::<K, V, _>(self.meter, &[0])` | `crate::ledger_slots::table.witness_view(self.meter)` | key/value types and path |
| List | `runtime::ledger::metered_list_view::<T, _>(self.meter, 0)` | `crate::ledger_slots::items.witness_view(self.meter)` | element type and root index |
| Plain Merkle | `runtime::ledger::metered_merkle_tree_view_at_path::<MerkleTreeDigest, _>(self.meter, &[5], 10)` | `crate::ledger_slots::committed_votes.witness_view(self.meter)` | leaf type, path and depth; root type remains inferred from return type |
| Historic Merkle | `runtime::ledger::metered_historic_merkle_tree_view_at_path::<MerkleTreeDigest, _>(self.meter, path, depth)` | `crate::ledger_slots::historic.witness_view(self.meter)` | leaf type, path, depth and historic kind; root type separately inferred |

The `K`, `V`, `T`, `path` and `depth` tokens in this table are schematic where a fixture uses different concrete names. The Set and plain-tree rows reproduce the election declarations described above. The emitter change removes only the low-level constructor calls from generated getters; the runtime slot delegates to those same constructors. No DSL or macro is needed for these short bodies. Map witness views remain available only when `V: CellValue`; nested Map values stay outside this scalar-view API.
