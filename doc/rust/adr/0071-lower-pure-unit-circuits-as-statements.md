---
id: RUST-ADR-0071
alias: ADR-0071
title: "Lower pure Unit circuits as statements"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "generated-code", "ownership"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 3a76c18ea997c6f9e7a81ad6d02f79d9e4c6802e783d9c79b7462077a46d237d
---
# RUST-ADR-0071 — Lower pure Unit circuits as statements

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept statement-position lowering for typed pure Unit bodies, omitting only effect-free trailing Unit literals and returning Ok(()). Preserve assertions, calls, error propagation and order. Later matcher cleanup fixes compiler Clippy without altering generated semantics; other value-position lowering remains separate.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#170 closure](https://github.com/MediaNoxLabs/compact/issues/170#issuecomment-6017518667). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`2c05ecce`](https://github.com/MediaNoxLabs/compact/commit/2c05eccea7daa1feec91c9c10621f5e8f723fb7f) · [`a8e9d722`](https://github.com/MediaNoxLabs/compact/commit/a8e9d722368f8303f8a3730ec125600ab12d3ac4). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 71
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/170
```

## Historical decision and amendments

### Problem

Rust 1.99 workspace Clippy rejects generated pure Unit circuits (`unused_unit`, `unit_arg`, `no_effect`, `let_unit_value`). The typed IR records their result as `Type::Unit`, but the AST emitter currently treats every body as a value and emits `Ok(#body)`. A sequence of assertions ending in Unit therefore becomes `Ok({ assert block; (); })`; direct Unit or Unit-valued conditionals also create redundant expression statements. The `passport-dogfood` and `asset-registry-oracle` fixtures reproduce this.

### Before and proposed after

```rust
// Before: faithful behavior, poor Unit syntax.
pub fn check(x: Field) -> Result<(), CompactError> {
    Ok({ assert_condition(x)?; () })
}
// Proposed: execute the same fallible statements, then return Unit once.
pub fn check(x: Field) -> Result<(), CompactError> {
    assert_condition(x)?;
    Ok(())
}
```

### Decision and ownership

For a pure circuit whose typed result is `Type::Unit`, lower the body in statement position and finish with `Ok(())`. At typed `Expr::Sequence`, omit a final literal Unit value when it has no side effects. Preserve every preceding step in source order, including assertions, calls and `?`; keep value-position lowering for non-Unit results. Use `syn` nodes, not string surgery or a broad Clippy allow. The runtime, derive crate, public signature, ledger-8/zk primitive mapping, ABI 34 and private IR schema 8 remain unchanged. A Unit-valued fallible call is still evaluated exactly once.

### Alternatives and risks

Crate-wide lint suppression would hide generated defects. Removing every Unit expression by syntax alone could drop a fallible call or assertion. The emitter must distinguish the typed effect-bearing forms from a literal Unit and verify order. `if` condition formatting, redundant `Ok(?)` and other generated diagnostics may require a separate decision after this slice.

### Verification and delivery

Create a focused MediaNoxLabs issue in `rust-backend-v2` before source edits. Test pure Unit literal, assertion sequence, Unit call and Unit conditional; refresh 137 fixtures. Run renderer tests and focused Clippy on affected fixture crates before the workspace Clippy probe. Review source delta and proof only if operation semantics or call input changed. Record exact conventional GPG/DCO commit and local gate outcomes here, in the issue, and delivery map. Leave remote CI open until the locally complete backlog is published.


### Tracking amendment — 2026-10-04

Focused [#170](https://github.com/MediaNoxLabs/compact/issues/170) was created in `rust-backend-v2` before emitter edits.


### Local acceptance — 2026-10-04

The typed `unit_statements` renderer now emits pure Unit bodies and Unit-valued sequence steps in statement position. It omits literal Unit, emits assertions directly, preserves fallible Unit calls, preserves `let` scope, and folds a guarded single assertion into short-circuit `&&`. All changes are `syn` AST construction; runtime, derives, ledger/zk mapping, public signatures, ABI 34 and private schema 8 are unchanged. The before/after asset-registry source changes `Ok({ { if !condition { return Err(...) } }; (); })` to `if !condition { return Err(...) }; Ok(())`. The guarded form changes nested `if guard { if !condition { return Err(...) } }` to `if (guard) && (!condition) { return Err(...) }`, preserving evaluation order.

The focused renderer probe covers literal, sequence assertion, fallible Unit call, conditional and guarded assertion; all 62 renderer tests pass. The pure assertion fixture passes TypeScript success/failure order parity and the guarded nested arithmetic fixture passes TypeScript parity. All 137 fixtures regenerate and are fresh; eight libraries changed, with 2,355 insertions and 3,150 deletions (net −795 generated lines). Passport, asset-registry and guarded fixture assertion/call/error site counts are unchanged. Rust 1.99 focused Clippy no longer reports `unused_unit`, `unit_arg`, `unnecessary_operation`, or the newly exposed `collapsible_if` in the affected generated libraries. The focused Clippy command still exits 101 on distinct bool, `Ok(?)`, complex-condition, and stateful Unit-binding syntax; the full workspace gate and remote CI remain open. No VM operation or call encoding changed, so the prior proof gate is not repeated for this AST-only slice. Commit hash and signature verification follow below.


### Commit and publication state — 2026-10-04

Conventional signed/DCO `2c05eccea7daa1feec91c9c10621f5e8f723fb7f` contains the emitter, renderer probe and eight refreshed fixture libraries. `git verify-commit` reports a good signature from Yurii Shynbuiev. The only remaining tracked worktree change is the user-owned `doc/ledger-adt.mdx`; it was not staged. The branch and commit remain local and unpushed. [#170](https://github.com/MediaNoxLabs/compact/issues/170) records the checked focused acceptance and open full-workspace/remote gates.


### Backend Clippy follow-up — 2026-10-04

The exact workspace gate run during ADR-0072 found two `collapsible_if` diagnostics in ADR-0071's handwritten `unit_statements` matcher that the focused generated-crate gate did not include. The next local code commit combines those matcher conditions; generated behavior is unchanged. This corrects the narrower earlier statement that the backend source had already passed full Clippy. The post-repair exact workspace gate reports no backend errors.


ADR-0071 backend Clippy follow-up is included in signed/DCO `a8e9d722368f8303f8a3730ec125600ab12d3ac4` and documented in [#170](https://github.com/MediaNoxLabs/compact/issues/170#issuecomment-5980984662). The exact workspace Clippy rerun reports no backend source errors.
