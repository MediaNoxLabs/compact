---
id: RUST-ADR-0100
alias: ADR-0100
title: "Record scalar pure Field calls returned after state actions"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "pure-calls", "return-values"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 8ac5e75ab3fa15b4e8d94ac3f6127a388354d29d2dfdcd12d7f152ff4e116ef1
---
# RUST-ADR-0100 — Record scalar pure Field calls returned after state actions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted an audited scalar pure Field return call after state actions, preserving the source's second call rather than reusing an earlier value. The bounded save case has TS/native/recorded/proof evidence, with a dedicated 64MiB proof thread in the historical harness. Other return/body domains are not implied.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#203 closure](https://github.com/MediaNoxLabs/compact/issues/203#issuecomment-6017574349). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

### Original source metadata

```yaml
adr: 100
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/203
```

## Historical decision and amendments

### Problem

At the schema-11 ADR-0096 baseline, `stateful_pure_call.save(Field): Field` is proof-required and native Rust works, but the recorded/observed API is unavailable at `StateReturn::Expression`. Its action already records `stored = disclose(square(value))` through a typed Cell slot. The final result independently calls `square(value)` again. Existing Field expression return lowering accepts no-action circuits only. Blindly accepting every return call would admit hash or other pure bodies before their VM/gas behavior is established.

### Before and after generated Rust

Before, only direct execution is available:

```rust
let saved = ledger_contract::save(context, Field::from(7))?;
assert_eq!(saved.result, Field::from(49));
```

After, the generated crate exposes the recorded and observed call while preserving the source evaluation order:

```rust
let recorded = ledger_contract::recorded::save(context, Field::from(7))?;
assert_eq!(recorded.result, Field::from(49));
let call = contract.recording().save_call(&observed, private_state, Field::from(7))?;
```

The recorder emits a typed `square(argument)?` after the recorded Cell write, then finishes the same frame with that Field result. It does not reuse the earlier action value, because the source calls the pure function twice.

### Decision and scope

Add a `StateReturn::Expression { value: Expr::Call }` arm only for a Field-returning exported circuit with prior state actions, a matching pure Field callee, matching arity, Field-only formal arguments, and the ADR-0091 transitive `closed_pure_field_call` scalar guard. Lower each argument with the existing ordered Field expression recorder, bind once, and invoke the already generated pure method. Unsupported argument/body shapes retain a structured recording gap. Existing `RecordingFrame` and typed ledger slots own VM, gas, state and private transitions; no IR/schema/runtime ABI or ledger primitive changes.

### Acceptance

Compile exact `examples/rust_backend/stateful_pure_call.compact` with pinned schema-11 Scheme and local ZKIR 2.1.0. The single `save` capability must become proof-required recorded/observed with no other capability loss. Compare TypeScript/native/recorded return 49, full state bytes, all four gas dimensions, ordered VM write, unchanged private state and FAB. Prove, verify, validate and apply the emitted `save(7)` call against ledger-8. Run focused renderer, generated fixture, Clippy, manifest/freshness and inventory checks. Root runs the combined exact-head local gate after integration. No push or remote CI.

### Tracking

- MediaNoxLabs issue: https://github.com/MediaNoxLabs/compact/issues/203 (rust-backend-v2).
- Branch: `codex/adr94-witness-call-let`, stacked after signed/DCO ADR-0096 `825b59d9f4c9d1e8d4277630953bb824acb2bbaa`.
- Local commit: signed GPG/DCO `d9ee19992604728560dec6b59c57d9ed96ca59f1`.


### Local delivery, 2026-10-05
Signed GPG/DCO commit d9ee19992604728560dec6b59c57d9ed96ca59f1 stacks on ADR-0096 825b59d9. The exact stateful_pure_call source capability changes from one to two of two proof-required recorded/observed APIs: only save gains, read_stored remains available, and exported pure square remains nonproof. The full 191-source inventory reports 214/296 proof-required available, 82 missing, and no unmatched compiler circuits. The unchanged source returns Field 49 after writing Field 49; TypeScript/native/recorded return, full serialized ledger state, four gas dimensions, ordered VM write, private state and FAB agree. Pinned schema-11 Scheme and ZKIR 2.1.0 emitted keys and ZKIR; direct recorded and observed calls matched before proof, and save(7) proved, verified, validated and applied on ledger-8 under the default shell stack with a dedicated 64 MiB proof thread. Renderer 75/75, generated fixture 2/2, targeted Clippy, format, manifest, generated-output byte comparison, and all 139 fixture freshness checks passed. Root combined exact-head local gate remains after integration; no push or remote CI.
