---
id: RUST-ADR-0022
alias: ADR-0022
title: "Meter Merkle witness VM reads while keeping local projections"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "witnesses", "gas", "merkle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 7a565cc1d002ca929e493deabb586a4adca3cf83b696a189c1ed33426e97d88c
supersedes: RUST-ADR-0021
---
# RUST-ADR-0022 — Meter Merkle witness VM reads while keeping local projections

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept mixed Merkle witness views: local structural projections plus metered is_full/check_root operations delegated to canonical VM queries. This explicitly supersedes ADR0021's incomplete witness-API conclusion. Preserve digest/depth/type checks and the sampled plain/historic evidence rather than claiming all Merkle operations are interchangeable.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#122 closure](https://github.com/MediaNoxLabs/compact/issues/122#issuecomment-6017435295). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Supersedes:** [RUST-ADR-0021](0021-keep-merkle-witness-projections-local.md).

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`6ad550d8`](https://github.com/MediaNoxLabs/compact/commit/6ad550d8a34bcaadbf51994a37d08d0bcf5e08ad) · [`f936ce5f`](https://github.com/MediaNoxLabs/compact/commit/f936ce5fc039d7fd78cabc86ee27dc9392d8cc33). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 22
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 122
supersedes: 21
```

## Historical decision and amendments

### Problem and evidence

A fresh ledger-8 TypeScript compile of `merkle_path_witness.compact` exposes both `isFull` and `checkRoot` as VM queries on witness-visible `ledger.t`, plus the `js-only` root, firstFree, pathForLeaf and findPathForLeaf methods. Historic trees additionally expose local history. The current Rust `LedgerView::t()` returns `MerkleTreeView`, which has only local methods. Rust witness implementations cannot call the two VM read methods and cannot meter them. ADR-0021 correctly identified the local operations but incorrectly assumed VM methods were not witness-visible. This ADR supersedes that conclusion while preserving the local/VM distinction.

### Before and after Rust

```rust
// Before: a witness can inspect a path, but not execute the Compact VM reads.
let tree = context.ledger.t()?;
let path = tree.path_for_leaf(0, leaf)?;
// tree.is_full() and tree.check_root(digest) are unavailable.
```

```rust
// After: one typed view exposes both categories with explicit fallibility.
let tree = context.ledger.t()?;
let path = tree.path_for_leaf(0, leaf)?; // local, no VM gas
let root = GeneratedMerkleTreeDigest { field: path.root().0 };
let full = tree.is_full()?;          // canonical ledger-8 VM query
let known = tree.check_root(root)?;  // canonical ledger-8 VM query
```

Historic views provide the same read methods, with `check_root` consulting the history Map rather than only the current root. Their `history()`, `root()`, `first_free()` and path methods remain local. A failed query returns `CompactError` through `TryWitnesses`.

### Decision and ownership

Add `MeteredMerkleTreeView` and `MeteredHistoricMerkleTreeView` wrapping the existing structural views, the current `WitnessReadMeter`, physical path, and declared depth. Delegate local methods to the structural view; implement `is_full` and digest-bound `check_root(root: Root)` through the already-tested ledger-8 VM query functions. The emitter maps Merkle witness fields to these typed views using `syn` syntax nodes. The runtime meter accumulates exact per-query costs and applies the ledger-8 per-query gas limit (ADR-0020). There is no new VM opcode, duplicate state representation, or IR node. Generated/runtime ABI advances 10→11 because the `LedgerView` method return type changes; private IR schema stays 8.

### Alternatives and risks

Metering all Merkle methods would charge root/path/history work that TypeScript marks `js-only`. Leaving only structural views would omit legitimate witness VM reads. Returning a tuple of structural view and meter would force application authors to manage paths/depth and weaken the developer API. Risks include using the plain root comparison for historic history, assuming a native digest is a Compact `CellValue` instead of using the generated digest type, and failing to preserve nested physical paths. Tests must cover plain/historic query prefixes, local zero-gas calls, rejection, and generated fixture compilation.

### Verification and delivery gate

- Capture fresh TypeScript output for plain and historic witness calls after insertion, including local root/path/first-free/history and VM `isFull`/`checkRoot` query prefixes, four-dimensional gas, private FAB and public transcript.
- Match Rust result, ordered per-query costs, aggregate gas, private FAB and state; show local-only calls cost zero.
- Force a rejected Merkle witness VM query and assert `CompactError` without a partial circuit result.
- Regenerate all affected fixtures, run backend/runtime and all-target workspace compile, 131 fixture freshness, 37 pinned oracle inventory, packaged consumer/proof, and clean-source compatibility gates.
- Append signed/DCO commit and evidence to this ADR, #122 and [Milestone 2 — ADR delivery map](references.md#private-note-08). Keep remote CI/release as separate gates.


### Type-safety refinement (2026-10-02)

The first implementation used `check_root<T: CellValue>(root: T)`. That permits a Boolean or unrelated record to reach a Merkle root query, even though Compact declares a generated `MerkleTreeDigest`. The final view binds `Root` at construction and exposes `check_root(root: Root)`. The emitter chooses `crate::types::MerkleTreeDigest` for both plain and historic fields. This keeps the domain type in generated code and makes a wrong root a compile error. The packaged one-dependency consumer contains a positive call and a negative `check_root(true)` compile probe.

```rust
// First draft: accepted an unrelated CellValue at the API boundary.
fn check_root<T: CellValue>(&self, root: T) -> Result<bool, CompactError>;

// Delivered API: the generated digest is fixed by the ledger field.
MeteredMerkleTreeView<'a, MerkleTreeDigest, DefaultDB>
fn check_root(&self, root: MerkleTreeDigest) -> Result<bool, CompactError>;
```

The local projection methods still delegate to the structural view. Only `is_full` and `check_root` call the ledger VM through `WitnessReadMeter`. TypeScript witness wrapper gas accounting differs from Rust: the wrapper reports zero for these reads, while the captured QueryContext contains the ordered per-query costs. Rust reports their aggregate; the oracle compares each query cost and the result, state and private output separately. This accounting distinction is a deliberate API choice and must be visible in parity reviews.


### Delivery evidence (2026-10-02)

Local conventional GPG-signed/DCO commits `f936ce5fc039d7fd78cabc86ee27dc9392d8cc33` and `6ad550d8a34bcaadbf51994a37d08d0bcf5e08ad` deliver ABI 11 on `codex/rust-backend-ast`. The private Rust IR remains schema 8. Four generated Merkle witness libraries changed after binding `check_root` to `MerkleTreeDigest`; the other 127 generated libraries remain fresh.

The unchanged Compact source was compiled with ledger-8 TypeScript. The oracle captures local plain and historic paths/history, two plain VM reads, and three historic VM reads (including prior root membership), plus ordered four-dimensional query costs, private FAB, public transcript and serialized state after each insert. Rust matches result, path, FAB, state and each query cost; local-only witness calls cost zero. The generated Rust circuit reports aggregate witness query gas; the TypeScript wrapper reports zero while its QueryContext carries the compared costs. Typed query rejection returns an error without a partial circuit result.

Verification: 53 backend renderer and four CLI tests, the complete runtime suite, focused Merkle parity/rejection tests, all-target workspace compile, 131/131 fixture freshness, 37 pinned oracle sources, two compiler rejection probes, a packaged one-dependency Merkle consumer with positive and compile-fail digest calls, and 45 offline proof/verify/validate/apply cases pass. The clean-source ABI-11 release rehearsal writes and verifies `target/rust-runtime-abi11-clean.json` from commit `6ad550d8` with `dirty: false`; macro/runtime archives contain 8/227 entries. The unrelated `doc/ledger-adt.mdx` edit was preserved byte for byte and excluded from both commits.

Focused [#122](https://github.com/MediaNoxLabs/compact/issues/122) is assigned to rust-backend-v2. Branch publication, remote CI, registry publication and wallet/node submission remain open. Wider List element behavior and other parent #116 work are outside this Merkle slice.
