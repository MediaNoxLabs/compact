---
id: RUST-ADR-0115
alias: ADR-0115
title: "Record typed closed ternary struct members"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "struct", "ternary"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 49fe52908bf16e27f43380c89d374481edfe108ad59a1d540c240800d5c680b0
---
# RUST-ADR-0115 — Record typed closed ternary struct members

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted exact one-Field-member struct construction and projection from closed conditional Field values, retaining genuine typed generated structs. Historical cases cover walker both flags and stream false; arbitrary composites, dynamic arms and effectful conditions remain refused.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#218 closure](https://github.com/MediaNoxLabs/compact/issues/218#issuecomment-6017600507). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`4087ed08`](https://github.com/MediaNoxLabs/compact/commit/4087ed08278d89db46ff11551c59191f2471e032). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 115
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/218
```

## Historical decision and amendments

### Problem and source evidence

At integrated `4087ed08`, eight proof-required calls in `examples/rust_backend/ternary_cond_oracle.compact` still lack a complete recording. `walkerStructMember(c)` and `streamStructMember()` have the same schema-11 sequence: bind a `Box { f: Field }` struct whose sole member is a Field cast of a closed unsigned-literal ternary; bind `b.f` as Field; write `fieldCell`. The stream form first records `flag.read()` into `f`. Native Rust compiles and executes, but the recording action walker has no typed StructLiteral/StructField Let path, so both calls lack recorded and observed APIs.

### Before and after generated Rust

Before, generated native code constructs `crate::types::Box { f: runtime::Field::from(...) }` and writes its member, while the `recorded` module omits both calls. After, a typed recorded call should preserve the source-level struct and member access:

```rust
let selected: runtime::Field = if c { runtime::Field::from(1u64) } else { runtime::Field::from(2u64) };
let b: crate::types::Box = crate::types::Box { f: selected };
let member: runtime::Field = b.f;
let frame = crate::ledger_slots::fieldCell.record_write(frame, member)?;
```

For the stream call, `flag.record_read(frame)` precedes the pure selection. Names are illustrative; the emitter allocates hygienic locals.

### Decision and ownership

Add a narrow typed lowering for the exact nested Let chain: one `Type::Struct` binding whose `StructLiteral` type agrees, one Field member at index 0 from a closed `FieldCast(If)` of compiler-typed bounded literals, followed by a matching `StructField` Field binding. Require the condition to be an existing Boolean parameter or already recorded local, and validate the declared struct field name/type/index. Materialize a real generated Rust struct and project its member, then let existing Cell write recording own the VM effect. Reuse schema-11 IR, `syn`/`quote`/`prettyplease` emitter, `runtime::Field`, existing typed struct, ledger slot and ledger-8 proof/VM/gas. No runtime API or schema change. Do not admit general struct construction, dynamic arms, effectful conditions, arbitrary projections, or unrelated ternary Lets.

### Acceptance

Capture fresh TypeScript oracle executions for walker true/false and stream false from the same constructor state. Compare generated native/recorded/observed result, serialized initial/final state, four gas dimensions, ordered public VM, private outputs, and Verify replay. Prove and apply both calls through pinned ZKIR 2.1.0 and ledger-8. Add a negative renderer guard for nonliteral or mismatched member forms. Check exact capabilities: only these two gain recorded/observed, the six other ternary gaps remain, and proof inventory denominator is unchanged. Run focused local source gate, renderer/fixture tests, 146 fixture snapshots, rustfmt and Clippy. Do not edit user-facing docs.

### Tracking

- Predecessor: [ADR-0113 — Record closed unsigned ternary comparisons](0113-record-closed-unsigned-ternary-comparisons.md).
- Milestone issue: pending.
- Delivery: pending.

### Local validation (2026-10-05)

- Frozen schema-11 compilation changed only `walkerStructMember` and `streamStructMember` from recording unavailable to recorded and observed. Six other ternary Let first blockers remain unavailable. The focused parity gate passes with 15/21 recorded calls.
- Full inventory at this branch has 935 declarations, 306 proof-required exports, 242 available and 64 missing. Relative to integrated `4087ed08`, this slice adds exactly two available calls (240→242), with no denominator change.
- Fresh TypeScript oracle and generated Rust native/recorded calls agree in unit result, serialized initial/final state, all four gas dimensions, ordered public VM, zero private outputs, and Verify replay for walker true/false and stream false. TypeScript reports the final query gas; Rust call gas equals the sum of captured query gas.
- Pinned ZKIR 2.1.0 compiled all 21 circuits. Generated observed calls for all three cases proved, verified and applied through ledger-8, with typed and manual traces equal.
- A valid dynamic value in the struct Field ternary remains recording unavailable. The exact member type/index/projection and pure Boolean guard in the emitter prevent general struct fallback. 89 renderer tests, six ternary fixture tests, 146 fixture snapshots, focused parity gate, rustfmt and Clippy pass.
- Milestone issue: https://github.com/MediaNoxLabs/compact/issues/218. Delivery commit: pending signed local commit.

### Delivery

Signed local commit `9549409f17306c05b54f3fbdb45094d5885d798c` (`%G? = G`, DCO trailer present) is one commit based directly on `4087ed08`, ready for parent integration. No branch push or remote CI was used.
