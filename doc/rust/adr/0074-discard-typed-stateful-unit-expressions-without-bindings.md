---
id: RUST-ADR-0074
alias: ADR-0074
title: "Discard typed stateful Unit expressions without bindings"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "generated-code", "ownership"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b55d63d5921ba7fc80765d2008a6daa91b85aeddfc73c6cb589e12c009933642
---
# RUST-ADR-0074 — Discard typed stateful Unit expressions without bindings

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed statement discards for Unit expressions while omitting only literals or already-materialized simple paths/projections. Every other Unit expression must execute once in order, preserving assertions, witnesses, calls and errors. Non-Unit discard behavior and runtime semantics stay unchanged.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#173 closure](https://github.com/MediaNoxLabs/compact/issues/173#issuecomment-6017523850). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`7ec96608`](https://github.com/MediaNoxLabs/compact/commit/7ec9660885189cfdce24e32785c532ea50f9ea91). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 74
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/173
```

## Historical decision and amendments

### Problem

Constructor steps and stateful expression actions use `let _ = #rendered;` for every discarded result, even when the typed IR result is `Type::Unit`. Generated `let _ = __compact_call_3.result;`, `let _ = __compact_witness_20;` and `let _ = { if ... return Err(...) };` trigger Rust 1.99 Clippy `let_unit_value`. The exact workspace probe after ADR-0073 exposed constructor-stateful-call and asset-registry examples; mixed-width and zerocash examples were seen in earlier partial probes. A generated Rust crate should not require consumers to allow this lint.

### Before and proposed after

```rust
// Before: callee has already been invoked and metered.
let __compact_call_3 = ensureFirst(context, value)?;
context = __compact_call_3.context;
total_cost += __compact_call_3.gas_cost;
let _ = __compact_call_3.result; // result: ()
// Proposed: the Unit projection is omitted; preceding effects remain.
let __compact_call_3 = ensureFirst(context, value)?;
context = __compact_call_3.context;
total_cost += __compact_call_3.gas_cost;

// A Unit expression that itself performs a check must still execute.
assert_condition()?;
```

### Decision and ownership

Use the typed result from `render_state_expression` in constructor expressions, stateful expression actions and stateful `Expr::Sequence` steps. A shared AST statement-discard helper may omit only a Unit literal or a simple local path/field projection whose source has already been materialized in the ordered `statements` vector. It must execute any other Unit expression exactly once in statement position, preserving assertions, pure calls, witness calls and `?`. Non-Unit discards retain the existing behavior. Keep validation and effect ordering in the typed stateful/constructor emitter, not a text or broad final-file rewrite. No runtime, derive, ledger-8/zk primitive, public signature, ABI 34 or private IR schema 8 change.

### Alternatives and risks

Changing every `let _ =` to an expression statement could introduce `path_statements` or `unnecessary_operation` warnings and make no-effect statements visible. Removing every Unit value would drop fallible calls or assertions. The helper must distinguish already-materialized paths from effect-bearing expressions. If a block contains only an assertion, lower it to its statement while retaining local binding scope for more complex blocks.

### Verification and delivery

Create a focused MediaNoxLabs issue in `rust-backend-v2` before source edits. Renderer probes should cover a Unit stateful-call result, Unit witness result, Unit assertion, fallible Unit pure call and a non-Unit discard. Refresh/check 137 fixtures; compare generated source and ordered call/transcript/gas behavior in constructor-stateful-call, asset-registry and relevant witness/negative fixtures. Run focused Rust 1.99 Clippy first, exact workspace gate after the group clears; report later groups separately. Proof only if VM operation or call encoding changes. Record conventional signed/DCO commit, exact tests and open remote gates in ADR, issue and milestone map. Branch stays local.


### Tracking amendment — 2026-10-04

Focused [#173](https://github.com/MediaNoxLabs/compact/issues/173) was created and assigned to `rust-backend-v2` before emitter edits.


### Local acceptance — 2026-10-04

The shared `discard_expression` helper takes a rendered `syn::Expr` and its validated Compact `Type`. For Unit it omits `()` and a simple materialized local/witness/call-result path or field projection, executes an effect-bearing expression once in statement position, and unwraps a singleton `if` assertion block without changing scope. Non-Unit discards keep `let _ = value;`. Constructor expression steps, stateful expression actions and stateful sequence steps now pass the typed result into this helper. The stateful call, context update, gas cost and witness transcript remain in the ordered statement list before the discarded projection. The constructor-stateful-call fixture removes only `let _ = __compact_call_3.result;`; mixed-width retains its error-returning assertion directly; zerocash retains witness invocation, metering, private-state update and transcript append while omitting two Unit paths. No runtime, derive, ledger-8/zk primitive, public API, ABI 34 or private IR schema 8 change.

The focused helper unit test covers Unit literal, witness path, call-result field, fallible Unit call, Unit assertion and non-Unit discard. All 64 renderer tests pass. Constructor stateful-call state parity, mixed-width TypeScript boundary parity, and zerocash constructor/mint and spend with captured Merkle path TypeScript state parity all pass (four focused behavior tests). All 137 fixtures regenerate and are fresh; five libraries changed, with seven insertions and eighteen deletions (net −11 generated lines). Formatting and scoped diff checks pass. Exact Rust 1.99 `cargo clippy --workspace --all-targets --all-features -- -D warnings` no longer reports `let_unit_value`; it still exits 101 on eight passport complex condition blocks and two ternary identical branches, with possible later findings masked by early crate failures. No VM operation or call encoding changed, so the prior proof gate was not repeated. Commit/signature evidence follows; remote CI and push remain deferred.


### Commit and publication state — 2026-10-04

Conventional signed/DCO `7ec9660885189cfdce24e32785c532ea50f9ea91` contains the typed discard helper, constructor/stateful call-site changes, helper test and five refreshed fixture libraries. `git verify-commit` reports a good signature from Yurii Shynbuiev. [#173](https://github.com/MediaNoxLabs/compact/issues/173) records passed local acceptance and open remote CI. Only the user-owned `doc/ledger-adt.mdx` remains dirty; the branch and commit are local and unpushed.
