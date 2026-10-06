---
id: RUST-ADR-0096
alias: ADR-0096
title: "Record scalar pure Field calls with typed arguments"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "pure-calls", "witnesses", "control-flow"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e980073ef15e85270013b1a39a0e9cdf53d61557a015ae4498e33143f983a033
---
# RUST-ADR-0096 — Record scalar pure Field calls with typed arguments

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted typed Field arguments to audited scalar pure callees plus a narrow Uint2 conditional Field argument; the same argument support explicitly also enables two witness wrappers. Pure-body effect restrictions remain. Historical macOS proof evidence requires a dedicated 64MiB test thread and does not establish reduced runtime stack use.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#198 closure](https://github.com/MediaNoxLabs/compact/issues/198#issuecomment-6017565923). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`aabf9241`](https://github.com/MediaNoxLabs/compact/commit/aabf924127553afa156ebbe7e24e6e4a1ced92dd). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 96
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/198
```

## Historical decision and amendments

### Problem and measured sources

At the local schema-11 `aabf9241` baseline, nine proof-required exported circuits have `Expr::Call` as their first recorded-lowering gap. ADR-0091 already permits zero-argument pure Field calls only when the full transitive pure body is scalar arithmetic; it deliberately excludes hashes and other primitives pending gas/VM parity. That guard is sound for pure `increment(Field): Field` and `idf(Field): Field`, but the current `StateAction::Let` lowering requires `arguments.is_empty()`. Therefore `internal_pure_call.save(Field)` is unavailable at `actions[0].bindings[0].value` despite its scalar call and typed Field Cell write. `ternary_cond_oracle.walkerCallPure(Boolean)` and `streamCallPure()` have the same pure callee, `idf(Field)`, with a typed argument `Coerce(FieldCast(If(Boolean, Uint<2> literal 1/2)), Field)`; stream first reads `flag` from a Boolean Cell. All three are proof-required, native-Rust executable, and have no recorded/observed API. The hash-bearing `pureBodyVec`, tuple/vector bridge and `pureFromImpure` gaps, and the stateful `impureConst` gap are distinct.

### Before and after Rust code

Before, direct execution invokes generated pure code and writes a Cell, while the recorded module has no corresponding method:

```rust
let value: runtime::Field = crate::pure_circuits::increment(input)?;
let step = crate::ledger_slots::stored.write(context, value)?;
```

After, the AST recorder binds each declared Field argument once in source order, invokes only a transitive scalar pure callee, then uses the typed ledger slot:

```rust
let frame = runtime::recording::RecordingFrame::new(context);
let argument: runtime::Field = input;
let value: runtime::Field = crate::pure_circuits::increment(argument)?;
let frame = crate::ledger_slots::stored.record_write(frame, value)?;
Ok(frame.finish(()))
```

For `walkerCallPure`/`streamCallPure`, the narrow `Uint<2>` conditional cast becomes `if condition { Field::from(1) } else { Field::from(2) }` before `idf(argument)`. In stream, `flag.record_read(frame)?` must precede the pure call and Field Cell write. The developer-facing generated crate gains `recorded::{save,walkerCallPure,streamCallPure}` and corresponding observed-call methods when each whole action/return shape validates.

### Decision and boundaries

Generalize the existing ADR-0091 pure Field Let path from zero arguments to declared Field formals, retaining `closed_pure_field_call` on the entire transitive callee body. Require matching arity/results, evaluate every typed Field argument exactly once with the ordered `field_expression` lowering, and bind it to a Field local before invoking the existing `pure_circuits` method. Add a bounded `field_expression` case for `FieldCast(If(Boolean, Uint<2> literal, Uint<2> literal))`; validate each unsigned literal and its declared maximum, and evaluate the Boolean condition through the existing recorder so Cell reads retain VM/gas order. Reject unsupported argument shapes with the structured capability gap. Do not relax the scalar-body guard for transientHash, persistentCommit, native witness, or ledger reads in pure callees. Do not claim `stateful_pure_call.save` until its additional result `StateReturn::Expression(Call)` is handled separately.

This changes only the AST emitter; it reuses generated scalar pure functions, `RecordingFrame`, and typed ledger-8 Cell slots. No IR node/schema, public runtime ABI, or new runtime primitive is needed. `midnight-zk` is used only by the pinned proof gate.

### Validation plan

Freeze the compiler and schema-11 Scheme/ZKIR binaries and source revision. Regenerate only affected fixtures. Require the five exact gains listed below with no losses. Compare TypeScript/native/recorded full ledger bytes, four gas dimensions, ordered public VM transcript, private state and FAB, and both conditional branches. Verify stream's read precedes write, and caller input is evaluated once. Prove/verify/apply `save(Field::from(7))`, `streamCallPure`, and `streamCallWitness` against pinned ledger-8 and ZKIR 2.1.0. Run focused renderer/generated fixture tests, Clippy, freshness/inventory, and root's combined full local gate after integration. No push or remote CI.

### Tracking

- MediaNoxLabs issue: https://github.com/MediaNoxLabs/compact/issues/198 (rust-backend-v2).
- Branch: `codex/adr94-witness-call-let`, stacked after signed/DCO `c95d77be78779f9970a123737d7410a978349769`.
- Local commit/evidence: pending.


### Same conditional argument in witness calls

The bounded `FieldCast(If(Boolean, Uint<2> literal))` helper is also reached by two proof-required `Expr::WitnessCall` arguments in `ternary_cond_oracle`: `witnessArg` and `streamCallWitness`. Both were unavailable at the argument expression, and both now gain recorded/observed methods without weakening the scalar pure-callee guard. This is an acceptance extension for the same typed conditional argument, not a new witness mechanism: existing `RecordingFrame::try_witness_metered` still owns private transition, FAB output, witness read gas, and order. The TypeScript capture and Rust tests must check both branches, argument values 1/2, one witness invocation, private state 7→8, FAB atoms/alignment, and ordered VM read/write. The proof gate additionally covers `streamCallWitness`. The five exact gains are `internal_pure_call.save`, `ternary_cond_oracle.walkerCallPure`, `.streamCallPure`, `.witnessArg`, and `.streamCallWitness`.

### Proof stack and local gate

The first direct proof run on macOS's default approximately 8 MiB main-thread stack overflowed during ledger transaction checking. The identical three proof cases passed when run with `ulimit -s 65520` (KiB; the host's maximum allowed limit, approximately 64 MiB). The production local gate now invokes only the `--pure-field-arguments` proof smoke on a dedicated 64 MiB Rust thread. This is test-harness stack allocation and changes no compiler, generated contract, runtime, proof, or ledger semantics. The final validation must run under the default shell limit to establish that the narrow thread configuration works; failure is a gate failure, not a skipped proof.

Focused local command, using the pinned schema-11 compiler and ZKIR on `PATH`:

```sh
COMPACTC=/path/to/pinned-schema11-compactc COMPACTC_SCHEME=${LOCAL_EVIDENCE}/adr89-schema11-scheme-root/compactc-scheme CARGO_TARGET_DIR=/path/to/isolated-target python3 tools/compact-rust-backend/check_compactc_target.py --proof
```


### Local delivery, 2026-10-05
Signed GPG/DCO commit 825b59d9f4c9d1e8d4277630953bb824acb2bbaa stacks on ADR-0094 c95d77be. Five proof-required APIs gain recorded and observed calls across two sources, with no loss in the 23-call focused inventory (6 to 11 available). The TypeScript captures, generated Rust native/recorded paths and ledger bytes match for the covered branches, four gas dimensions, ordered public VM, private transition 7 to 8, and FAB. Renderer: 75 passed; generated fixtures: 2 plus 4 passed; targeted Clippy, formatting, Python syntax, source-output byte comparison and manifest hashes passed. Under the default macOS shell stack limit, the dedicated 64 MiB proof thread replayed, proved, verified, validated, and applied save(7), streamCallPure, and streamCallWitness using pinned ZKIR 2.1.0. Full combined local gate remains with root after integration. No push or remote CI.
