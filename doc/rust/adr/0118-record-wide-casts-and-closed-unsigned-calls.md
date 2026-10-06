---
id: RUST-ADR-0118
alias: ADR-0118
title: "Record wide casts and closed unsigned calls"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "unsigned-arithmetic", "casts", "pure-helpers"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: f851e74fca66446e50084ca9deafc5f49d4ea4dd9bd38ae131e61ba81c193f51
---
# RUST-ADR-0118 — Record wide casts and closed unsigned calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted direct declared unsigned-parameter Field conversion and transitively closed checked unsigned helper calls, including an unused result whose errors must be preserved. Both target circuits have parity and proof/application evidence; Gather execution gas remains distinct from Verify replay cost.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#220 closure](https://github.com/MediaNoxLabs/compact/issues/220#issuecomment-6017603747). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`01671999`](https://github.com/MediaNoxLabs/compact/commit/01671999cbd3bd2e3cf21efd8a9929cb8c4e03df) · [`4c5b1d04`](https://github.com/MediaNoxLabs/compact/commit/4c5b1d040a69d5c9819cccf0f704ff8a4816b87c) · [`c6071253`](https://github.com/MediaNoxLabs/compact/commit/c6071253dbf6e35a75ca549bf2f7c0dcd29f4fe1). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 118
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/220
```

## Historical decision and amendments

### Problem and source evidence

On schema-12 IR and runtime ABI 37 at `01671999`, `examples/rust_backend/field_cast_uint128.compact::save` is proof-required, but `recording_unavailable` reports `unsupported_expression`, `Expr::FieldCast`, `actions[0].bindings[0].value`. The `Uint<128>` parameter is cast to `Field`, written to a Cell, then read back. `examples/rust_backend/widening_arith_oracle.compact::recordArea` reports `unsupported_action`, `StateAction::Let`, `actions[0]`. It binds `areaOf(w, h): Uint<32>` before incrementing `lastArea`, even though the pure local is not used after the call. Both source declarations already compile and run natively; neither has a complete recorded or observed-call API.

### Before and after generated Rust

Before, native `save` writes `Field::from(value.value())` and reads the Cell, but the generated `recorded` module has no `save`. Native `recordArea` calls `pure_circuits::areaOf(w, h)?`, then increments the Counter, but has no recorded method. After, the emitter retains the same typed values and order:

```rust
let cast: runtime::Field = runtime::Field::from(value.value());
let frame = stored.record_write(frame, cast)?;
let (frame, observed): (_, runtime::Field) = stored.record_read(frame)?;

let w: runtime::BoundedUint<65535> = w;
let h: runtime::BoundedUint<65535> = h;
let area: runtime::BoundedUint<4294967295> =
    crate::pure_circuits::areaOf(w, h)?;
let frame = lastArea.record_increment(frame, 1u16)?;
```

The exact generated names are compiler allocated; the example shows their types and effect order. Keeping the pure call retains its checked arithmetic and potential error behavior, even though `area` is unused by later actions.

### Decision and ownership

Recorded `Expr::FieldCast` accepts a direct public `Expr::Parameter` only when its declaration is `Type::Unsigned`; it uses the runtime `Field::from(BoundedUint::value())` conversion already used by native lowering. The underlying `u128` and `Field` primitive own bounds and conversion. The existing literal ternary Field cast path remains separate.

For an unsigned `StateAction::Let` binding a pure call, require an exact result type match, exact argument count and declared argument types, and a transitive body consisting only of unsigned parameters/literals, typed unsigned coercions/casts, bounded add/subtract/multiply or similarly closed unsigned pure calls. Emit a call to the generated pure Rust method with `?` before later VM actions. Hashes, witnesses, stateful effects, arbitrary conditionals and recursion remain outside this admission. The generated typed methods and runtime `RecordingFrame`, Cell/Counter slots, ledger-8 VM and gas accounting remain the owners of execution. No IR schema, runtime ABI or primitive duplication is introduced.

### Acceptance and limits

Run fresh TypeScript captures and generated Rust native/recorded tests for a value above `u64::MAX` and for both maximum and small area inputs. Compare result, serialized state, four gas dimensions, exact ordered public VM tags, zero private outputs and Verify replay state/effects. The replay cost may differ from Gather execution cost; original TypeScript, native and recorded execution costs must agree. Prove, verify and apply one successful call per circuit with pinned ZKIR 2.1.0 and ledger-8. Refresh the two fixture snapshots; add a negative whitelist test, focused inventory/fixture checks, formatting and Clippy. No remote CI or push.

### Tracking

- Issue: https://github.com/MediaNoxLabs/compact/issues/220.
- Base: `01671999`; schema-12 test-only repair cherry-picked locally as `daa5f1b9`.
- Delivery: pending signed feature commit and root integration.

### Local validation

- Exact frontend and locally built Rust emitter report both target circuits as `recorded=true`, `observed_call=true`, `proof_required=true`.
- All 147 fixtures were freshly compiled; only `field-cast-uint128` and `widening-arith-oracle` generated Rust snapshots changed.
- TypeScript source captures were refreshed with per-query gas and ordered VM evidence. Rust state/gas/replay/proof checks are in progress.


### Completed local evidence (2026-10-05)

- A frozen `01671999` schema-12 Scheme frontend from `${HISTORICAL_NIX_STORE}/wym1z199zsk9l5mgg99830jx84lqzq6w-compactc` and the local AST renderer compile both original sources with strict `--rust-require-recording`. The checked inventory has 935 declarations, 309 proof-required exports, 32 unassessed, no source identity drift, and proof-ready APIs 246→248 (missing 63→61). Only `save` and `recordArea` change availability in this slice.
- The 147-fixture compiler sweep changed only the two targeted generated Rust snapshots. Both source fixture crates run their native and recorded tests. The two-source focused parity gate passed with 2/2 recorded APIs; receipt `${LOCAL_EVIDENCE}/compact-focused-adr118/receipt.json`.
- For `save((1 << 80) + 7)`, fresh TypeScript, native Rust and recorded Rust agree on the returned Field, serialized Cell state, private output count zero and all four execution gas dimensions. The two TypeScript queries total readTime 255000000 ps, computeTime 2483857950 ps, bytesWritten 58, bytesDeleted 36. The ordered VM is `push, push, ins, dup, idx, popeq`; Rust and TypeScript normalized full public transcripts match. Verify replay reaches the same state and effects. Its Verify-mode compute charge is lower than the original Gather read charge and is deliberately not equated with execution cost.
- For `recordArea(65535,65535)` and `recordArea(5,7)`, fresh TypeScript, native Rust and recorded Rust agree on the Counter state, zero private outputs, readTime 170000000 ps, computeTime 1323481916 ps, bytesWritten 36, bytesDeleted 36, and full ordered `idx, addi, ins` VM transcript. Verify replay reaches the same state and effects.
- Pinned ZKIR 2.1.0 produced `save` and `recordArea` prover/verifier keys and binary ZKIR. Both typed observed calls equal their direct recorded call prototypes; each proved, verified and applied through ledger-8 with the expected state and Cell/Counter value.
- A renderer unit guard permits transitive bounded arithmetic while rejecting a hash primitive and recursive helper. 91 renderer tests and six library tests pass. Targeted all-feature Clippy is clean. No remote CI or push.


### Delivery

Signed GPG + DCO conventional feature commit `7b1a67878f806b6e9b62c2ac524306159a7dd21a` is clean and ready to cherry-pick onto the schema-12 root. The preceding local test-fix commit `daa5f1b9` duplicates root `4c5b1d04` and is excluded from integration. Issue #220 is assigned to `rust-backend-v2`. The exact local 147-fixture sweep had zero stale or failed outputs. Focused gate receipt: `${LOCAL_EVIDENCE}/compact-focused-adr118/receipt.json`. Root integration and broad gate are pending.


### Integrated delivery update (2026-10-05)

Root cherry-picked the feature as signed conventional GPG+DCO commit `c6071253dbf6e35a75ca549bf2f7c0dcd29f4fe1`. The integration retained the adjacent `closed_pure_assert_call` behavior from ADR-0117 alongside this slice’s `closed_pure_unsigned_call`. Exact clean-head Nix package: `${HISTORICAL_NIX_STORE}/59wsqxxa054qxakl5gnmzda0w01ngzjz-compactc`; 147/147 generated fixtures and focused field/widening gate 2/2 pass at `${LOCAL_EVIDENCE}/compact-focused-c6071253/receipt.json`. Clean-head inventory is 251/316 proof-ready APIs, 65 known gaps and 25 unassessed. Broad full local gate remains in progress at `${LOCAL_EVIDENCE}/compact-full-c6071253`; its outcome must be recorded separately. No remote CI or push was run for this integration.
