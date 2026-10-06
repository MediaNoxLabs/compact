---
id: RUST-ADR-0153
alias: ADR-0153
title: "Record typed hash assertions through helper formals"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "typed-hash", "lexical-scope", "gas"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: c93e5d83844b12263f62c9cc1f3ef4508e56cbeb6c164c341d4179ce9ac7ece5
---
# RUST-ADR-0153 — Record typed hash assertions through helper formals

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted typed scalar/Field-vector helper-formal hashing before ordered Cell assertions and writes. All three newly supported calls are proved; formal name collisions cannot change encoding. Preserve separate last-query, complete-call and concatenated-replay gas scopes, and the fact that the preceding main checkpoint excluded this later slice.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#255 closure](https://github.com/MediaNoxLabs/compact/issues/255#issuecomment-6017662404). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`89f33962`](https://github.com/MediaNoxLabs/compact/commit/89f33962b78133d8b7c1fed2207afde95bf57ee0). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0153 — Record typed hash assertions through helper formals
status: proposed
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

Three proof-required exports in inline_type_scope_oracle compile natively but lack recording. Each assertion calls an internal Boolean helper whose whole body compares persistentHash of its typed argument against a Bytes32 Cell read, then the caller writes a Field vector. Scalar/aggregate formal-name collisions and differing vector lengths must not change the hash encoding.

### Before and after

Compact: assert(aggHelper(z), message); fieldVec = disclose(w); where aggHelper(w: Vector<4, Field>) hashes its own four-element formal and reads hashCell.

Before: only native checkAggScope(context, w, z) exists. After: recorded::checkAggScope plus the observed-state check_agg_scope_call preserve the helper formal type, record its Cell read, assert the result, then record the caller write. Equivalent scalar and two-element helper cases share the structural lowering.

### Decision

Admit a closed, action-free internal Boolean helper with one parameter of Field or a Field vector and exact persistent-hash-versus-Bytes32-Cell equality. Resolve the actual argument against the helper formal type, not caller name bindings. Reuse runtime persistent_hash and the existing typed Cell read/write recording APIs. Reject effectful helper actions, other primitive operations, unsupported argument expressions, mismatched slots/types and alternate return bodies. No runtime, IR schema or ABI change.

### Acceptance

Fresh TypeScript captures for all three original exports; compare native/recorded result/state, typed hash, four gas dimensions, ordered VM/private output and replay. Negative wrong-hash guards must fail before caller writes. Renderer tests cover scalar/vector formal types, name collisions and rejection boundaries. Prove, verify and ledger-8 apply all three observed calls. Strict original-source proof gate, generated fixture freshness, signed commit, focused inventory and integration record.

### Status

Proposed before implementation; tracking issue follows. Work remains isolated while the main checkpoint is frozen. No push or remote CI.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/255 (rust-backend-v2), created before implementation. Lowering extends the existing typed hash/helper comparison mechanism shared with transient Field-pair comparisons; persistent Field/Field-vector hashing keeps the exact helper formal type and Bytes32 read type. All four original exports now compile with strict recording in the isolated draft. Fresh capture and executing parity are in progress; no delivery claim yet.

### Gas ownership and replay decision (2026-10-05)
The original ledger-8 TypeScript contract reports only the final write query gas for these circuits: readTime 85,000,000; computeTime 1,233,942,932; bytesWritten 156; bytesDeleted 156. Its Cell-read query costs readTime 170,000,000; computeTime 1,249,915,363; zero bytes written/deleted. The Rust native/recorded circuit result correctly sums both queries: readTime 255,000,000; computeTime 2,483,858,295; bytesWritten/deleted 156. A combined Verify replay executes the concatenated VM instructions as one query, with readTime 255,000,000; computeTime 1,409,371,826; bytesWritten/deleted 156. These three measures have different scopes, so parity compares each against the corresponding fresh TypeScript capture instead of equating combined replay gas with the two-query circuit total. The capture snapshots `preCheckState` and `start = queries.length` only after seed `setHash`, then replays `rawQueries.slice(start)`; seed costs are excluded. The same exact values hold for scalar, aggregate and no-collision cases.

### Delivery (2026-10-05)
Verified GPG+DCO local commit: 47c6b3c6f2428500e9de1e2de65b8e4a0b09e6b4. Issue #255. The original inline_type_scope_oracle source compiles under --rust-require-recording with all 4/4 exports recorded. Fresh original TypeScript capture and generated Rust agree on scalar, four-Field vector and two-Field vector helper formal hashing, result, state, ordered Cell read/write VM, private outputs, and each of four gas dimensions at the appropriate query scope. Wrong hashes reject before caller writes. Three fixture tests, 123 renderer tests, Clippy with warnings denied, and 148 generated fixture checks passed. Pinned ZKIR 2.1.0 proofs for checkScalarScope, checkAggScope and checkNoCollisionScope verified; ledger-8 validated/applied all three with expected typed Cell values. Exact-head focused gate passed: ${LOCAL_EVIDENCE}/compact-focused-47c6b3c6/receipt.json. Proof artifacts: ${LOCAL_EVIDENCE}/compact-adr153-proof. No Nix package rebuild, push or remote CI.


### Main integration evidence — 2026-10-05

Integrated as verified signed/DCO89f33962. Renderer123 and inline-type-scope3 tests pass; focused ${LOCAL_EVIDENCE}/compact-focused-89f33962/receipt.json passes4/4 exports. Independent slice proofs are at ${LOCAL_EVIDENCE}/compact-adr153-proof. The immediately preceding main4360 consumer/proof pass does not include this slice; its own focused proof evidence does.
