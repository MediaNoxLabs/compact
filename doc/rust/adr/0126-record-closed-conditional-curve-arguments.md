---
id: RUST-ADR-0126
alias: ADR-0126
title: "Record closed conditional curve arguments"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "curve", "ternary"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: bdc926c3a97301b400e61222ae7564bcdb4b9d1eac975e948cc3181300fba297
---
# RUST-ADR-0126 — Record closed conditional curve arguments

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the closed conditional Field-to-curve-to-X-coordinate chain with exact type and source checks. Both walker flags and both seeded stream states are proved; direct unrecorded conditions, witness arms and unrelated native calls remain rejected.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#227 closure](https://github.com/MediaNoxLabs/compact/issues/227#issuecomment-6017616285). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`80f9b6a0`](https://github.com/MediaNoxLabs/compact/commit/80f9b6a043057828c0726b07df634dffa168264c). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
initiative: compact-rust-backend
milestone: rust-backend-v2
adr: 126
status: accepted
date: 2026-10-05
base: 80f9b6a043057828c0726b07df634dffa168264c
```

## Historical decision and amendments

### Problem and source evidence

The original `examples/rust_backend/ternary_cond_oracle.compact` exports `walkerNativeArg(c)` and `streamNativeArg()`. Both use `hashToCurve<Field>(condition ? 1 : 2)`, project the point X coordinate, and write `fieldCell`. The stream obtains its condition from `flag.read()`. At base `80f9b6a0`, both native Rust functions compile, but recording and typed observed-call APIs are unavailable. Schema-12 IR first fails at `StateAction::Let`: `actions[0]` for the walker and `actions[0].action` after the stream's recorded flag read. The shared shape is a `JubjubPoint` binding from `HashToCurve(FieldCast(If(...)))`, followed immediately by a `Field` binding from `JubjubPointX` of that point, then a Cell write of the projected Field.

### Decision and ownership

The recorded emitter admits only that exact native-call projection chain. It requires a Boolean parameter or already recorded Boolean local as the condition; each arm must be a closed `Unsigned<2>` literal with matching coercion and value within its bound. It requires one point binding, the immediate X projection from that same point, and a Cell write from that projected Field. It emits typed calls to the existing runtime's pure `hash_to_curve(Field)` and `jubjub_point_x(JubjubPoint)` primitives. Dynamic, stateful, witness, wider or mismatched unsigned arms and unrelated native calls remain unavailable. This changes recorded lowering and checked generated output only. The runtime, schema 12, ABI 37, ledger slots, ZKIR and ledger-8 primitives retain their existing ownership.

### Before and after generated Rust

Before, the crate exposes native `walkerNativeArg` and `streamNativeArg`, but no `recorded::walkerNativeArg`, `recorded::streamNativeArg` or typed `recording.*_call` APIs. After, the walker recording contains the following actual generated sequence (compiler allocated names shortened here):

```rust
let point: runtime::JubjubPoint = runtime::hash_to_curve(if c {
    runtime::Field::from(1u64)
} else {
    runtime::Field::from(2u64)
});
let x: runtime::Field = runtime::jubjub_point_x(point);
let frame = crate::ledger_slots::fieldCell.record_write(frame, x)?;
```

The stream first records `flag.record_read(frame)?` and uses that retained Boolean for the same typed native calculation. The generated crate now exposes both `recorded::` functions and their typed observed-call methods.

### Acceptance and limits

A schema-12 IR fixture from the real source exercises the admission rule. Renderer tests reject a direct unrecorded Cell read condition and an effectful witness arm. Fresh TypeScript capture and generated native/recorded Rust tests cover walker true/false and stream flag false/true, including an explicit true Cell seed excluded from call gas. They compare serialized initial/final state, unit result, all four gas dimensions, ordered public VM shape, zero private outputs, and replay state/effects. The TypeScript wrapper reports only the final query gas; the test sums per-query costs for multi-query calls. Typed observed calls equal manual recorded prototypes. Pinned ZKIR 2.1.0 generated binary ZKIR and prover/verifier keys; all four branches proved, verified and applied through ledger-8, confirming the stored X coordinate and unchanged flag/Counter. No remote CI or push.

### Local evidence

- Focused renderer 97/97, generated ternary fixture 8/8, formatting and focused local parity gate pass.
- Original source inventory at this isolated head: 21 proof-required exports; recorded and observed-call API availability 17→19, gaps 4→2, unassessed 0. Remaining gaps: `walkerConstAnnotated`, `streamVectorElement`. This inventory is API availability, not a substitute for the four actual proofs.
- Focused gate receipt: `${LOCAL_EVIDENCE}/adr126-focused-gate/receipt.json`; source inventory: `${LOCAL_EVIDENCE}/adr126-ternary-inventory.json`.
- Pinned proof command: `cargo run -p compact-rust-proof-smoke -- --native-curve-arg ${LOCAL_EVIDENCE}/ternary-native-arg-proof`. It passed proof/verify/apply for walker true/false and stream false/true.

### Tracking

Milestone-v2 issue and signed local feature commit will be linked after final review.
### Issue

[MediaNoxLabs/compact#227](https://github.com/MediaNoxLabs/compact/issues/227) is assigned to `rust-backend-v2`.

### Delivery

Signed conventional GPG+DCO local feature commit `395ce8edcdf5a29a7be1b169b98ba070b2ff2144` from isolated base `80f9b6a0`; signature verified and worktree clean. Focused gate, 97/97 renderer, 8/8 generated fixture, targeted Clippy, formatting, fresh TypeScript capture replay and four pinned proof/verify/apply cases passed. The parent branch must still cherry-pick this commit and run its integrated local gate. No push or remote CI.
