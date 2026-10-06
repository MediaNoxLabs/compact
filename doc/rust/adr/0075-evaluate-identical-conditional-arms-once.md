---
id: RUST-ADR-0075
alias: ADR-0075
title: "Evaluate identical conditional arms once"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "generated-code", "ownership"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 1a0638b5ece10c104354eb4e7d04b258c8efa7aa73559fd6bc85aea7bf64d4db
---
# RUST-ADR-0075 — Evaluate identical conditional arms once

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept identical-arm detection at the typed IR boundary while evaluating the condition once before the shared arm. Preserve stateful witness/query effects and failure order even when the result arms match. This is not permission to drop condition evaluation or infer equality from rendered text.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#174 closure](https://github.com/MediaNoxLabs/compact/issues/174#issuecomment-6017525570). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`95bb28ae`](https://github.com/MediaNoxLabs/compact/commit/95bb28aec17f85b581928bf7502631b04cb6a1ba). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 75
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/174
```

## Historical decision and amendments

### Problem

Compact may express `if condition { value } else { value }`, including arms whose lowering allocates different Rust temporary names. Rust 1.99 Clippy reports `if_same_then_else` in the generated ternary oracle: one pure same-literal Uint expression and one constructor assertion with equal Compact comparison arms but `__compact_value_9`/`__compact_value_10` temporary names. The final Rust AST cannot reliably recognize semantically identical arms after name allocation. Removing the conditional without evaluating its condition could drop a witness, ledger read, fallible cast or assertion.

### Before and proposed after

```rust
// Before: identical generated values and duplicate arm lowering.
let result = if check()? { value } else { value };
// Proposed: evaluate the condition exactly once, then one arm.
let _ = check()?;
let result = value;
```

### Decision and ownership

At the typed `Expr::If` boundary in pure and stateful emitters, compare the two Compact IR arms before rendering. When `then == otherwise`, validate and evaluate the Boolean condition once in source order, then lower the shared arm once. For stateful lowering, append the condition expression to the ordered statement list before rendering the arm so witness/query side effects and errors remain; retain the existing effects and gas accounting. For pure value lowering, return an AST block with condition evaluation followed by one rendered arm. Preserve the ordinary two-branch lowering when arms differ, and preserve Type::Unit behavior. No final-file token equality or broad lint suppression. Runtime, derive, public API, ledger-8/zk primitives, ABI 34 and private IR schema 8 stay unchanged.

### Alternatives and risks

A final `syn` comparison misses alpha-equivalent branches with different temporary names. Dropping the condition risks losing effects. Rendering both arms and discarding one can consume extra temporaries or duplicate validation. The IR equality is exact and deliberately conservative; merely equivalent but structurally different branches remain outside this decision. `let _ = condition` must not trigger a Unit lint because validation requires Boolean.

### Verification and delivery

Create a focused MediaNoxLabs issue in `rust-backend-v2` before source edits. Probe same-arm pure and stateful conditionals with a fallible or witness condition, plus different-arm controls; ensure one condition and one arm execution. Refresh/check 137 fixtures and review source delta. Run ternary TypeScript parity, renderer tests and exact Rust 1.99 workspace Clippy; report later diagnostics. Run proof only if VM operation or aligned call input changes. Record conventional signed/DCO commit, exact tests and remaining local/remote gates in ADR, issue and milestone map. Keep branch local.


### Tracking amendment — 2026-10-04

Focused [#174](https://github.com/MediaNoxLabs/compact/issues/174) was created and assigned to `rust-backend-v2` before emitter edits.


### Local acceptance — 2026-10-04

Pure value, pure Unit-statement and stateful `Expr::If` lowering now compare the two typed Compact IR arms before allocating Rust temporaries. For exact equality, they validate the Boolean condition, retain its ordered fallible/witness/query evaluation when needed, and lower the shared arm once. Effect-free Boolean literals and already validated Boolean parameters need no generated `let _ =` statement; other conditions emit one discard statement after any effect statements. Different arms retain their two-branch lowering. The ternary fixture changes `if c { Uint(3000000000) } else { Uint(3000000000) }` into one Uint value, and a constructor assertion now uses one comparison temporary rather than two. Later temporary indices shift deterministically. No runtime, derive, public API, ledger-8/zk primitive, ABI 34 or private IR schema 8 change.

The renderer probe covers a fallible pure condition, effect-free Boolean parameter, different-arm control and stateful witness condition with transcript append; all 65 renderer tests pass. Two ternary fixture tests pass, including constructor/stateful TypeScript byte parity and pure arm/subtraction behavior. All 137 fixtures regenerate and are fresh; only the ternary library changed, with 24 insertions and 32 deletions (net −8 generated lines). Formatting and scoped diff checks pass. Exact Rust 1.99 `cargo clippy --workspace --all-targets --all-features -- -D warnings` has no `if_same_then_else` or handwritten backend diagnostics; it still exits 101 on eight passport complex condition blocks, with later findings potentially masked. No VM operation or aligned call input changed, so proof was not repeated. Commit and signature evidence follow; remote CI and push stay deferred.


### Commit and publication state — 2026-10-04

Conventional signed/DCO `95bb28aec17f85b581928bf7502631b04cb6a1ba` contains typed pure/stateful shared-arm lowering, the witness/fallible-condition renderer probe and the refreshed ternary fixture. `git verify-commit` reports a good signature from Yurii Shynbuiev. [#174](https://github.com/MediaNoxLabs/compact/issues/174) records local acceptance and open remote CI. Only the user-owned `doc/ledger-adt.mdx` remains dirty; the branch and commit are local and unpushed.
