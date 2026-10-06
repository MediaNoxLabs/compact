---
id: RUST-ADR-0072
alias: ADR-0072
title: "Return fallible pure tail expressions directly"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "generated-code", "ownership"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 04c368e30acd2d05314d10dc058414cf6ef330194e9a6c23d7a664d59298eed9
---
# RUST-ADR-0072 — Return fallible pure tail expressions directly

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept direct Result tails only for validated pure-circuit tails whose rendered AST is a Try expression. Keep nested question-mark operators, public types and other returns unchanged; Unit handling belongs to ADR0071. This is structural AST normalization, not textual rewriting.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#171 closure](https://github.com/MediaNoxLabs/compact/issues/171#issuecomment-6017520216). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`a8e9d722`](https://github.com/MediaNoxLabs/compact/commit/a8e9d722368f8303f8a3730ec125600ab12d3ac4). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 72
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/171
```

## Historical decision and amendments

### Problem

A pure circuit returning a value currently wraps every typed body in `Ok(#body)`. When the final `syn` expression is fallible, `expression_with_calls` emits a trailing `?`; the generated function therefore returns `Ok(crate::pure_circuits::callee(x)?)` or `Ok(runtime::cast_unsigned::<A, B>(x)?)`. Rust 1.99 Clippy rejects the redundant `Ok`/`?` pair in passport, ternary, mixed-width and Merkle fixtures. The source also hides the fact that the callee already returns the function's `Result` type.

### Before and proposed after

```rust
// Before
pub fn result(x: Field) -> Result<Field, CompactError> {
    Ok(crate::pure_circuits::callee(x)?)
}
// Proposed
pub fn result(x: Field) -> Result<Field, CompactError> {
    crate::pure_circuits::callee(x)
}
```

### Decision and ownership

At the pure circuit tail only, inspect the final typed `syn::Expr`. If it is `syn::Expr::Try`, emit the operand directly as the function's `Result` tail. Otherwise retain `Ok(body)`; ADR-0071 owns Unit statement position. This is structural AST use, not text replacement. The existing `expression_with_calls` validation guarantees Compact result type and `?` is only emitted for `Result<T, runtime::CompactError>` paths. Do not alter nested `?`, statement calls, result types or source parameter names. Runtime, derive macros, ledger-8/zk primitives, ABI 34 and private IR schema 8 remain unchanged.

### Alternatives and risks

A blanket Clippy attribute would hide unnecessary wrappers. Rewriting all nested `?` could change control flow or error boundaries. The tail-only rule is narrow and preserves the exact error type; tests should cover direct pure call and fallible unsigned cast plus a non-fallible control.

### Verification and delivery

Create a focused MediaNoxLabs issue assigned to `rust-backend-v2` before code. Add renderer probes for direct fallible tails and a non-fallible control; refresh and compare all 137 fixtures. Run the cached renderer suite and focused Rust 1.99 Clippy on affected generated crates, reporting other lint groups separately. Run behavioral fixture tests where the tail changes observable results; proof only if VM operations or call encoding change. Record exact conventional signed/DCO commit, tests, source delta and remaining full-workspace/remote gates in the ADR, issue and milestone map. Branch remains local under the user's local-first direction.


### Tracking amendment — 2026-10-04

Focused [#171](https://github.com/MediaNoxLabs/compact/issues/171) was created and assigned to `rust-backend-v2` before emitter edits.


### Local acceptance — 2026-10-04

The emitter now inspects the final typed `syn::Expr` for non-Unit pure circuits. For a `syn::Expr::Try`, it returns the operand `Result<T, CompactError>` directly; every other value tail stays inside `Ok(body)`. This preserves nested fallible evaluations, source parameter order and the exact error type. The internal pure-call fixture changes `Ok(increment(increment(value)?)?)` to `increment(increment(value)?)`; checked unsigned casts follow the same rule. ADR-0071 remains the Unit-body owner. This slice also folds three nested conditions in the handwritten `unit_statements` matcher after the exact full gate exposed `collapsible_if` under `-D warnings`; no generated bytes change from that mechanical source repair. Runtime, derives, ledger-8/zk primitives, ABI 34 and private IR schema 8 remain unchanged.

The focused renderer probe covers direct pure call, checked unsigned cast and non-fallible control; all 63 renderer tests pass. The internal pure-call fixture passes its callable/non-export test, and the ternary fixture passes two tests including constructor/stateful TypeScript byte parity and pure arm/subtraction behavior. All 137 fixtures are fresh; 15 libraries changed, with 109 insertions and 126 deletions (net −17 generated lines). Rust 1.99 exact `cargo clippy --workspace --all-targets --all-features -- -D warnings` no longer reports `needless_question_mark` or backend Clippy errors. It remains red on generated bool literals, complex condition blocks and a stateful Unit binding, with possible later findings hidden by early crate failures. Formatting and scoped diff checks pass. No VM operation or call encoding changed, so the prior proof gate was not repeated. Commit and signature evidence follow below; no remote CI or push is claimed.


### Commit and publication state — 2026-10-04

Conventional signed/DCO `a8e9d722368f8303f8a3730ec125600ab12d3ac4` contains the direct fallible tail rule, renderer probe, fifteen refreshed fixture libraries and the ADR-0071 matcher lint repair. `git verify-commit` reports a good signature from Yurii Shynbuiev. [#171](https://github.com/MediaNoxLabs/compact/issues/171) records passed focused acceptance and open remote CI. Only the user-owned `doc/ledger-adt.mdx` remains dirty in the worktree; the branch and commit are local and unpushed.
