---
id: RUST-ADR-0128
alias: ADR-0128
title: "Record an annotated closed Uint8 conditional"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "ternary", "unsigned-arithmetic"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 1ec75bf917c5e6138dbbb5051dd18643e9b1d2f11234f9ef1284553c4d6d9c99
---
# RUST-ADR-0128 — Record an annotated closed Uint8 conditional

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the precise annotated Uint8 conditional-to-Field write chain with validated inner and outer bounds. Both Boolean branches are proved; the differently shaped vector-element stream remained a separate gap. Historical temporary ADR127 filenames do not change this ADR identity.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#230 closure](https://github.com/MediaNoxLabs/compact/issues/230#issuecomment-6017621704). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3c75882d`](https://github.com/MediaNoxLabs/compact/commit/3c75882db9d58475f582787c76c0f633ccc8640b). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
initiative: compact-rust-backend
milestone: rust-backend-v2
adr: 128
status: accepted
date: 2026-10-05
base: 3c75882d
```

## Historical decision and amendments

### Problem and source evidence

The original `examples/rust_backend/ternary_cond_oracle.compact` exports `walkerConstAnnotated(c: Boolean)`. It binds `const x: Uint<8> = c ? 1 : 2`, casts `x` to Field and writes `fieldCell`. Native Rust compiled, but at base `3c75882d` the capability report marked recording and typed observed calls unavailable. Schema-12 IR first rejected `StateAction::Let` at `actions[0]`: one `Unsigned<255>` binding from `UnsignedCast<255>(If(c, Coerce<Unsigned<2>>(1), Coerce<Unsigned<2>>(2)))`, immediately followed by one Field binding from `FieldCast(Parameter x)` and a Cell write of that Field.

The other remaining ternary gap, `streamVectorElement`, has a different IR: a two-element Field vector with conditional elements after a recorded flag read. It is outside this decision.

### Decision and ownership

The recorded emitter admits precisely the annotated Uint<8> chain. It requires the outer `UnsignedCast` and binding bound to be 255, both inner coercion and literal bounds to be 2, both literal values within the bound, and a condition sourced from a Boolean parameter or already recorded local. It requires the immediate Field projection to reference the same unsigned local and the following Cell write to reference that projection. An unrecorded ledger read predicate or effectful witness arm remains unavailable. The emitter materializes a typed `runtime::BoundedUint<255>`, converts its value to the existing runtime `Field`, and records the Cell write through its typed ledger slot. This changes only the recorded AST lowering and checked generated crate. Runtime, schema 12, ABI 37, ledger-8 primitives and ZKIR remain unchanged.

### Before and after generated Rust

Before, the generated crate exposes native `walkerConstAnnotated`, but has no `recorded::walkerConstAnnotated` or typed `recording.walkerConstAnnotated_call`. After, its recorded function contains:

```rust
let selected: runtime::BoundedUint<255> =
    runtime::BoundedUint::<255>::new(if c { 1u128 } else { 2u128 })?;
let field: runtime::Field = runtime::Field::from(selected.value());
let frame = crate::ledger_slots::fieldCell.record_write(frame, field)?;
```

The actual generated identifiers are compiler allocated. The crate also exposes the typed observed-call method.

### Acceptance and limits

A minimized schema-12 IR fixture from the original source and a renderer test require the recorded API. The test rejects a direct unrecorded Cell read as the condition and a witness call in an arm. Fresh TypeScript capture and generated native/recorded Rust tests cover both Boolean branches, comparing serialized initial/final state, unit result, four gas dimensions, ordered public VM shape, zero private outputs and replay state/effects. The TypeScript wrapper reports the last query's gas; the test sums each query cost. Pinned ZKIR 2.1.0 compiled binary ZKIR and prover/verifier keys. For both branches, the typed observed call equaled the manual recorded prototype and proved, verified and applied through ledger-8; the stored Field was 1 or 2 and flag/Counter remained unchanged.

### Local evidence

- Focused inventory of the original source moves recorded/observed API availability 19/21 → 20/21, with zero unassessed; `streamVectorElement` remains the sole gap. These counts are API availability, separate from the two actual proofs.
- Renderer 99/99, generated ternary crate 9/9, targeted Clippy, formatting, deterministic TypeScript fixture recapture and focused local parity gate pass.
- Focused gate: `${LOCAL_EVIDENCE}/adr127-focused-gate/receipt.json`; inventory: `${LOCAL_EVIDENCE}/adr127-ternary-inventory.json`. The temporary filenames were allocated before ADR-0128 was reserved and do not change the ADR identity.
- Pinned proof command: `cargo run -p compact-rust-proof-smoke -- --annotated-uint8 ${LOCAL_EVIDENCE}/ternary-const-annotated-proof`. Both branches passed proof/verify/apply with ZKIR 2.1.0 and ledger-8.
- Work was local only; no remote CI or push.

### Tracking

Milestone-v2 issue and signed feature commit will be linked after final review.
### Delivery

[Milestone-v2 issue #230](https://github.com/MediaNoxLabs/compact/issues/230) tracks this slice. Signed conventional GPG+DCO feature commit `58e717f4c3d879bfd6f186f3d895dc9024476fd3` is ready for parent cherry-pick from isolated base `3c75882d`; the worktree is clean. The integrated exact-head package and broad local gate are pending.
