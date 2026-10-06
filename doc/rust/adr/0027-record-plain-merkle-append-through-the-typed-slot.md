---
id: RUST-ADR-0027
alias: ADR-0027
title: "Record plain Merkle append through the typed slot"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts", "merkle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: f42afbe04affedc9f86a1f6bba29f48437771d408c7e645d010a24e5ba9caa44
---
# RUST-ADR-0027 — Record plain Merkle append through the typed slot

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept recorded plain Merkle append through the typed slot and the same canonical VM builder used by native execution. Preserve complete-circuit eligibility and actual hashed-leaf semantics. Historical source growth adds a proof capability; it is not a generated-size optimization or general indexed/default/reset support.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#127 closure](https://github.com/MediaNoxLabs/compact/issues/127#issuecomment-6017444534). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`4746e8de`](https://github.com/MediaNoxLabs/compact/commit/4746e8dee63b8e87f7510ddffef46103ea9d0459). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 27
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: "127"
```

## Historical decision and amendments

### Problem

The generated `merkle_tree_oracle` crate exposes a typed native `append` circuit, but no `recorded::append`. An engineer can inspect and run the ledger-8 state change yet cannot obtain a replayable public trace for this simplest Merkle write. The compiler has a typed `StateAction::MerkleInsert` and the runtime already executes the corresponding verifying VM program. The gap is in the recording path.

Compact source: `examples/rust_backend/merkle_tree_oracle.compact` (`MerkleTree<3, Uint<8>>`, `append(value)`). The first slice is a root plain tree, a typed leaf value and Unit return. Indexed, hash, default, historic and read operations remain separate acceptance work.

### Before

```rust
// Generated native entry point; no recorded::append is emitted.
let step = crate::ledger_slots::t.insert(context, __compact_param_0)?;
let context = step.context;
```

### Decision and proposed after

```rust
// Generated body, from typed IR only when the complete trace is supported.
let frame = crate::ledger_slots::t.record_insert(frame, __compact_param_0)?;
// Consumer:
let result = generated::recorded::append(context, bounded::<255>(7))?;
let (initial, program) = result.public.into_parts();
// Replay/prove the program with the existing ledger-8 proof path.
```

`MerkleSlot<T, DEPTH, false>::record_insert` delegates to `RecordingFrame::insert_merkle`. The frame uses the same `merkle_insert_hashed_program` builder as native execution, hashes the typed leaf with the existing ledger-8 `leaf_hash` wrapper, calls `QueryContext::query`, accumulates four-dimensional gas and appends the exact verifying operations. No generated VM instruction text or new ledger implementation is needed. The compiler declines to emit a recorded method for any Merkle action it cannot represent completely.

### Alternatives and rationale

An emitter-owned VM program would duplicate Merkle semantics and risk transcript drift. Recording native execution after the fact would lack the verified operation sequence. A macro would hide the explicit data flow from the generated crate. Reusing the runtime builder preserves the typed domain boundary and operation parity already checked in ADR-0026.

### Emitter and runtime ownership

`recorded.rs` validates `StateAction::MerkleInsert` against the ledger field kind/index, plain-tree flag and typed leaf, lowers supported scalar expressions and emits the slot call. `slots.rs` binds the declared `T`, depth and plain-tree kind. `recording.rs` owns frame state, gas and trace. `ledger/merkle.rs` owns the shared hash/program builder and ledger-8 mapping. No macro change; private IR schema stays 8. A new public runtime/generated API requires ABI 12→13 and regenerated fixtures. Keep the old ABI-12 crates incompatible by compile-time assertion; rebuild consumers from source.

### Verification and risks

Before acceptance, compare recorded result, state bytes, four-dimensional gas and full VM program against pinned TypeScript oracle for first append; replay the public program independently; prove/verify/apply at least one packaged call offline; check rejection on malformed/wrong-type consumer calls; regenerate fixture freshness and package manifest. Signed/DCO local commit, exact commands and measured generated source impact will be appended. Branch publication, remote CI, release and wallet/node submission remain separate gates. The current proposal claims no delivered code.

### Tracking and delivery

- Issue: pending focused MediaNoxLabs/compact issue.
- Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Parent issues: [#107](https://github.com/MediaNoxLabs/compact/issues/107), [#108](https://github.com/MediaNoxLabs/compact/issues/108), [#105](https://github.com/MediaNoxLabs/compact/issues/105), [#126](https://github.com/MediaNoxLabs/compact/issues/126).
- Delivery: proposed; implementation uncommitted and unpushed.

### Amendments

Append dated delivery and review evidence without replacing the decision history.

Issue assigned: [#127](https://github.com/MediaNoxLabs/compact/issues/127) in rust-backend-v2.



### Delivery amendment — 2026-10-03

Local conventional GPG-signed/DCO commit `4746e8dee63b8e87f7510ddffef46103ea9d0459` delivers the first slice on `codex/rust-backend-ast`, still unpushed. Issue [#127](https://github.com/MediaNoxLabs/compact/issues/127) is assigned to rust-backend-v2. No public release or remote CI is claimed.

**Before:** `merkle_tree_oracle` exposed only native `append`; no `ledger_contract::recorded` module existed, so a consumer could not obtain a verifying program. **After:** the generated ABI-13 crate emits:

```rust
pub fn append<Private>(
    context: runtime::context::CircuitContext<Private>,
    __compact_param_0: runtime::BoundedUint<255>,
) -> Result<runtime::recording::RecordedCircuitResult<Private, ()>, runtime::CompactError> {
    let frame = runtime::recording::RecordingFrame::new(context);
    let frame = crate::ledger_slots::t.record_insert(frame, __compact_param_0)?;
    Ok(frame.finish(()))
}
```

The native `append` still uses `ledger_slots::t.insert`. `recorded.rs` accepts a root plain `StateAction::MerkleInsert` only when the field kind/index/path and typed scalar expression match; a circuit with another unsupported action is omitted entirely. The slot exposes `record_insert` only on `MerkleSlot<T, DEPTH, false>`, so historic trees cannot call it. The frame executes and retains the same production Merkle append verifying program, using `leaf_hash_for` and `merkle_insert_hashed_program` in `ledger/merkle.rs`. The runtime owns gas and query/effect accumulation; emitter owns static eligibility and Rust AST emission. No proc macro or ledger primitive was added. Private IR schema remains 8; the public runtime/generated ABI changes 12→13, requiring source regeneration.

**Parity and proof evidence:** the new fixture test compares `recorded::append(7)` with the independent ledger-8 TypeScript first-append capture: Unit result, exact serialized state, all four gas dimensions, and the complete ten-operation VM program including operands and leaf hash. It independently replays the public verifying program to the same state. The packaged shared-runtime consumer executes `recorded::append` and replay from one dependency graph. Compile-fail examples reject a `bool` leaf with E0308 and reject unsupported `recorded::place`; the earlier plain-tree history rejection remains. The full `compactc --target rust --consumer --proof` gate generated append proving/verifying artifacts, replayed and partitioned the trace, proved/verified it, validated the transaction and applied it; the resulting tree has first-free index 1 and a path for leaf 7. The previous 52 proof/application calls also passed, for 53 total calls.

**Other local gates:** 54 renderer tests, 4 CLI tests, both plain Merkle fixture tests, 132/132 fresh generated fixtures, 2 rejection probes, 37 pinned oracle source checks, `cargo check --workspace --all-targets`, `cargo fmt --all -- --check`, and staged diff checks passed. A clean-source ABI-13 package manifest `target/rust-runtime-abi13-clean.json` was written and verified for commit `4746e8de`, tree `4a6ac8da`, `dirty=false`; it records 8 macro and 229 runtime archive entries. The unrelated `doc/ledger-adt.mdx` edit was restored byte for byte and excluded from the commit.

**Code size and exposure:** each of `merkle-tree-oracle`, `merkle-path-verify` and `merkle-path-witness` gains a 32-line net recorded `append` surface; the other 129 regenerated fixtures change only ABI assertions. This is a proof capability, not a generated-size optimization. Indexed/hash/default insert, historic mutations, fullness/root reads, other leaf shapes, nested Merkle paths, remote CI, registry publication and wallet/node submission remain open. The issue stays open for those integration and release gates.
