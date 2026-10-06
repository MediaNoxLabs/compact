---
id: RUST-ADR-0049
alias: ADR-0049
title: "Record Cell calls at chunked ledger paths"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: cf2f6a5c789462adae6b0bd32b1afd216450dc1de1e5ee62c581fb8ad6e5001f
---
# RUST-ADR-0049 — Record Cell calls at chunked ledger paths

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed Cell recording at complete declared chunked paths through the existing slot and frame methods. Keep exact declaration/value and result-shape checks. The delivered source/proof cases do not establish all expression returns or broad performance improvement.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#148 closure](https://github.com/MediaNoxLabs/compact/issues/148#issuecomment-6017480561). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`fc599c1d`](https://github.com/MediaNoxLabs/compact/commit/fc599c1dcccd2b5ebf51507d8c54cde82e8981f3). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 49
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/148
```

## Historical decision and amendments

### Problem and evidence

A ledger-8 contract with seventeen scalar fields followed by `active: Boolean` places `active` at physical path `[1,14]`. Packaged ABI-26 `compactc --target rust --skip-zk` emits `CellSlot<bool>::new(&[1,14])`, a constructor and native `set_active`, `get_active` and `active_equals` methods. It emits no `Contract::recording` API for these circuits. Four Cell-specific one-element path guards in `recorded.rs` reject write, direct read, expression read and equality shapes. The existing `chunked_ledger_oracle` fixture proves constructor state and pathful native VM read/write; `CellSlot`, `RecordingFrame`, the witness meter and ledger cell VM builders already take complete paths.

### Before and proposed after

Before:

```rust
assert_eq!(ledger_slots::active.path(), &[1, 14]);
let native = contract.set_active(context, false)?;
// contract.recording.set_active(...) and set_active_call(...) are absent.
```

Proposed after:

```rust
let recorded = contract.recording.set_active(context, false)?;
let prepared = contract.recording
    .set_active_call(&observed, (), false)?
    .prepare(verifier, randomness)?;
```

### Decision proposal and alternatives

Permit typed Cell recording at compiler-assigned chunked paths by removing only the four Cell-specific root-depth checks. Keep declaration index, exact Cell type, supported expression and result-shape validation. Generate the same typed slot method calls that root Cells use. Reuse runtime `CellSlot<T>`, `RecordingFrame` and ledger-8 `Op` builders for complete paths; do not emit a parallel hand-written VM sequence. Manual `RecordingFrame` calls would expose low-level plumbing and leave the generated crate incomplete. No new derive or macro is justified. Generated/runtime ABI should advance from 26 to 27 because public recorded methods appear; private IR schema remains 8.

### Emitter and runtime ownership

`tools/compact-rust-backend/src/recorded.rs` owns Cell eligibility and typed method emission; `src/lib.rs` owns slot declaration and ABI. Runtime `slots.rs`, `recording.rs`, `context.rs`, `ledger/cell.rs` and witness meter already own pathful execution. Ledger-8 `Op`, `QueryContext`, FAB and gas model remain authoritative. If parity reveals a protocol discrepancy, fix it in the shared runtime rather than generated raw operations.

### Verification and risks

Check in a source with two-segment ledger, constructor-seeded `active` and circuits for write, direct read, equality and an expression containing a read. Capture TypeScript result/state, ordered public transcript, input FAB atoms/alignment and four-dimensional per-query gas. Compare Rust native, recorded, replayed and metered witness behavior. Check a one-dependency generated consumer, wrong-typed argument rejection and neighboring-path isolation. Require exact manual/generated pre-proof bytes and independent proof, verify, validate and apply for every new call using a fresh packaged compiler. Keep root Cell and all fixtures green. Report generated source size and compile-time impact honestly. Remote CI, clean release, registry and wallet/node remain separate gates.

### History

- Baseline probe: ignored `target/chunked-cell-probe.compact` compiled with packaged ABI-26 `${HISTORICAL_NIX_STORE}/2bvkn2m1y366qhqa1avhn5k6w7k2l3bs-compactc`; native methods and pathful slot exist, recording API absent.
- Related: ADR-0001/0002 for recording and typed slots, ADR-0046/0047/0048 for chunked Set/List/Map.
- Proposal only. Append dated delivery evidence with signed/DCO commit; preserve this rationale.

### Implementation and verification amendment — 2026-10-04

Decision accepted for typed Boolean and Field Cell recording at compiler-assigned chunked paths. Signed/DCO local commit `fc599c1dcccd2b5ebf51507d8c54cde82e8981f3` removes only the four Cell-specific root-depth guards in the AST recording emitter. Declaration index, exact Cell type, supported expression and result checks remain. Generated/runtime ABI is 27; private IR schema stays 8. No runtime VM, ledger primitive, FAB encoder, derive or macro change was needed: `CellSlot<T>`, `RecordingFrame`, witness meter and shared ledger-8 Cell builders already carry complete paths.

The delivered public API is:

```rust
let recorded = contract.recording.set_active(context, false)?;
let prepared = contract.recording
    .set_active_call(&observed, (), false)?
    .prepare(verifier, randomness)?;
let asserted = contract.recording.assert_active(context, true)?;
let updated = contract.recording.add_amount(context, Field::from(7_u64))?;
```

The checked-in eighteen-declaration source places Boolean `active` at `[1,14]` and Field `amount` at `[1,13]`, seeded by the constructor. Six circuits cover write, direct read and equality assertion for Boolean, plus write, direct read and read-add-write expression for Field. The direct-return `active.read() == expected` and `amount.read() + delta` probe circuits remained native-only because exported return-expression eligibility is separately restricted; the delivered equality and Field expression shapes are exercised in an assertion and action. That remaining generated-API gap needs its own decision or a later expansion of this issue, with proof parity before enabling it.

The TypeScript ledger-8 capture and Rust fixture agree on results, serialized state, ordered public VM shape, input FAB atoms/alignment and all four gas dimensions per query. Native, recorded and replayed results/state/effects/gas agree; metered witness Cell reads reach the correct paths, an assertion failure is rejected, and a write to one path leaves the neighboring Cell unchanged. The generated-only consumer passes all six observed call methods, with Boolean/Field wrong-type calls rejected at compile time.

Fresh ABI-27 packaged `compactc` at `${HISTORICAL_NIX_STORE}/y0yfb8knq4accawssvk5mfybc7r7qkim-compactc` passed the complete generated consumer/manifest and **91-call independent proof, verification, validation and application gate**. Manual/generated pre-proof bytes match: set_active 503, get_active 484, assert_active 491, set_amount 535, get_amount 516, add_amount 636. The focused fixture, 57 renderer tests, 137 fresh fixture comparisons, formatting and a bounded all-features check of backend, proof-smoke and new fixture pass. A full all-features workspace check was stopped before completion when broad parallel compilation reduced disk free space to 12 GiB; no result is claimed for that gate. Generated chunked Cell source is 593 lines / 27,304 bytes; no like-for-like root Cell source or compile-time benchmark was measured.

The runtime macro and runtime crate archives package and verify with 9 and 247 entries; a second run reproduced `target/rust-runtime-release-abi27.json` at commit `fc599c1d`. The manifest is dirty solely because pre-existing user-owned `doc/ledger-adt.mdx` remains unstaged. The branch is local/unpushed. Remote CI, clean signed release, registry publication, wallet/node submission and broader expression-return acceptance remain open; issue #148 stays open.
