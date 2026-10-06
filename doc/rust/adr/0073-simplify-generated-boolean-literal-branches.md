---
id: RUST-ADR-0073
alias: ADR-0073
title: "Simplify generated boolean literal branches"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "generated-code", "ownership"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: de177a795b39780134ac3aa7206ca87cb3b9a07d2aed6feaa8441d5e7a7ceab0
---
# RUST-ADR-0073 — Simplify generated boolean literal branches

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept a narrow final-AST normalization for opposite literal Boolean branches with no branch statements/attributes. Preserve condition evaluation, grouping and recursive normalization. Identical-arm optimization belongs to ADR0075; this rule does not collapse arbitrary branches or hide effects.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#172 closure](https://github.com/MediaNoxLabs/compact/issues/172#issuecomment-6017522226). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`74b4e840`](https://github.com/MediaNoxLabs/compact/commit/74b4e840a7114a5638d047cbbded570b75ca1a2a). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 73
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/172
```

## Historical decision and amendments

### Problem

The typed pure and stateful AST paths both emit boolean conditionals such as `if value { false } else { true }`. Rust 1.99 Clippy reports `needless_bool` in the generated `boolean-logic`, `asset-registry-oracle`, and `passport-dogfood` crates. This makes `-D warnings` consumers fail and obscures a simple boolean operation in developer-facing source. ADR-0072's exact workspace probe recorded at least thirteen such diagnostics; the probe can stop early on other failing crates.

### Before and proposed after

```rust
// Before: generated pure or stateful value expression.
let inverted = if observed { false } else { true };
// Proposed: same single evaluation and Boolean result.
let inverted = !(observed);

// Before / after for the opposite arm order.
let unchanged = if observed { true } else { false };
let unchanged = observed;
```

### Decision and ownership

Run a narrow `syn::VisitMut` normalization over the completed generated `syn::File`, after all typed pure/stateful lowering. Match only an `if` expression with exactly one Boolean literal expression in each branch, no branch statements or attributes, and opposite literals. Replace `true/false` with the condition and `false/true` with its negation; recurse into children first. Do not optimize identical branches here because their condition may have effects; do not touch nonliteral branches, `if let`, or Unit statement control flow. This central AST pass covers both pure and stateful emitters without duplicating semantic rules. The typed IR remains the source of condition/result type validation; the pass changes only equivalent Rust syntax. No runtime, derive, public API, ledger-8/zk primitive, ABI 34 or private IR schema 8 change.

### Alternatives and risks

Separate simplification in every pure, recorded and stateful lowering path would drift. String replacement is unsafe. A broad Clippy allow would hide future source-quality defects. The AST matcher must preserve exactly one condition evaluation and the `?`/read order; it must leave expressions with branch work unchanged. Exact output style matters for generated-crate readability, not for ledger semantics.

### Verification and delivery

Create a focused MediaNoxLabs issue in `rust-backend-v2` before source edits. Probe both Boolean arm orders and controls with nonliteral branches or statement-bearing branches; inspect the final `syn` output. Refresh/check 137 fixtures and source delta. Run renderer tests, boolean-logic and relevant stateful TypeScript parity fixtures, and the exact Rust 1.99 workspace Clippy command to verify the `needless_bool` group disappears while other groups remain visible. Run proof only if VM operations or call input change. Record conventional signed/DCO commit, exact tests, source impact and open local/remote gates in the ADR, issue and milestone map. Keep branch local under the user's instruction.


### Tracking amendment — 2026-10-04

Focused [#172](https://github.com/MediaNoxLabs/compact/issues/172) was created and assigned to `rust-backend-v2` before AST edits.


### Local acceptance — 2026-10-04

A narrow `CompactBooleanLiterals` `syn::VisitMut` pass runs on the completed generated file after signature lint marking. It recursively unwraps an `if` expression with opposite literal-only Boolean branches into the condition or its negation, including a directly enclosing `!` (with parser parentheses/groups) so assertions render `if query.result` rather than `if !(!query.result)`. It skips `if let`, attributed expressions, identical arms, nonliteral arms and any branch with statements. One condition evaluation and nested `?`/query ordering are preserved. Pure `invert` changes `Ok(if value { false } else { true })` to `Ok(!(value))`; asset-registry assertions change `if !(if query.result { false } else { true })` to `if query.result`. No typed IR, runtime, derive, public API, ledger-8/zk primitive, ABI 34 or private schema 8 change.

The renderer probe covers both arm orders, a nonliteral control, an assertion-bearing branch and an enclosing assertion negation; all 64 renderer tests pass. The boolean-logic TypeScript short-circuit witness parity test passes, as do asset-registry chunked collection/counter parity and registration-gap tests. All 137 fixtures regenerate and remain fresh; six libraries changed, with 18 insertions and 70 deletions (net −52 generated lines). Formatting and scoped diff checks pass. Exact Rust 1.99 `cargo clippy --workspace --all-targets --all-features -- -D warnings` has no `needless_bool` or handwritten backend diagnostics. It still exits 101 on two generated stateful Unit bindings and eight passport complex condition blocks; early crate failure may hide later findings. No VM operations or call input changed, so proof was not repeated. Commit and signature evidence follow below; remote CI and push remain deferred.


### Commit and publication state — 2026-10-04

Conventional signed/DCO `74b4e840a7114a5638d047cbbded570b75ca1a2a` contains the AST normalizer, positive/negative renderer controls and six refreshed fixture libraries. `git verify-commit` reports a good signature from Yurii Shynbuiev. [#172](https://github.com/MediaNoxLabs/compact/issues/172) records the passed local checks and open remote CI. The only remaining tracked worktree change is the user-owned `doc/ledger-adt.mdx`; the branch and commit remain local and unpushed.
