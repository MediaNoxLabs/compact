---
id: RUST-ADR-0076
alias: ADR-0076
title: "Lift typed block conditions before generated if expressions"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "generated-code", "ownership"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: d3f74adef4ebc3bde962947d2515c0497ca2cc23f8a2d4474217df26b94d59fa
---
# RUST-ADR-0076 — Lift typed block conditions before generated if expressions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept lifting typed block conditions into collision-safe scoped bindings in pure If/assertion lowering. Preserve lexical scope, Boolean typing, once-only evaluation and error order. Keep the separate parity-harness lint repair and bounded fixture evidence distinct from the emitter change.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#175 closure](https://github.com/MediaNoxLabs/compact/issues/175#issuecomment-6017527114). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`246b3494`](https://github.com/MediaNoxLabs/compact/commit/246b349440e503d20fedf1437571b80739467279) · [`73fe1f6c`](https://github.com/MediaNoxLabs/compact/commit/73fe1f6ceab2bbb3e85ab29e9e677fff4b7d92e7). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 76
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/175
```

## Historical decision and amendments

### Problem

Compact's typed `Expr::Let` becomes a Rust expression block containing local declarations. When used as the condition of a generated pure `if`, the emitter currently writes `if { let local = ...; predicate } { ... }` (or nests that `if` inside an assertion). Rust 1.99 Clippy reports eight `blocks_in_conditions` errors in the passport-dogfood fixture, including date quotient and birthday calculations. The generated source is hard to read because fallible casts and local bindings sit inside the condition header. The exact workspace Clippy gate after ADR-0075 reports this as its current remaining family; later crate failures may expose further findings.

### Before and proposed after

```rust
// Before
let adjusted = if {
    let month = date.month;
    month <= two
} { previous_year()? } else { date.year };
// Proposed: evaluate the typed Boolean once, then use a named condition.
let adjusted = {
    let __compact_condition = {
        let month = date.month;
        month <= two
    };
    if __compact_condition { previous_year()? } else { date.year }
};
```

### Decision and ownership

At the pure `Expr::If`/assertion AST boundary, recognize a rendered Boolean `syn::Expr::Block` containing local statements and lift it into a scoped `let` before the `if`. The typed IR already verifies Boolean condition and branch result types. Choose a generated condition name that cannot shadow current Compact parameters/local bindings, and keep the named binding inside the expression's own block. Apply the same rule to Unit statement-position control flow and assertions so nested forms are consistent. Evaluate condition exactly once, before either branch, preserving nested `?`, error order and local scope. Do not transform a block with no local statement, or arbitrary Rust source by text replacement. Stateful query/witness lowering is outside this slice unless the same diagnostic appears there. Runtime, derives, public API, ledger-8/zk primitives, ABI 34 and private IR schema 8 stay unchanged.

### Alternatives and risks

A generated-module Clippy allow would hide opaque condition code. Rewriting `Expr::Let` itself would affect all expression contexts. A final-file visitor would need to allocate names across nested scopes without typed parameter context. The pure boundary has both the Boolean type and current parameter map. The lift must preserve `return Err` and `?` from the original condition block; tests should include false and true branches, fallible conditions, nested assertions and a simple-condition control.

### Verification and delivery

Create a focused MediaNoxLabs issue in `rust-backend-v2` before source edits. Add renderer probes for a Let-block condition, nested assertion and simple control. Refresh/check all 137 fixtures and inspect the passport source delta. Run passport behavior/TypeScript parity tests, 65+ renderer tests and the exact Rust 1.99 workspace Clippy gate; report any newly exposed diagnostics. Proof only if VM operations or aligned call input change. Record conventional signed/DCO commit, exact tests and open local/remote gates in ADR, issue and milestone map. Keep branch local.


### Tracking amendment — 2026-10-04

Focused [#175](https://github.com/MediaNoxLabs/compact/issues/175) was created and assigned to `rust-backend-v2` before AST edits.


### Local acceptance — 2026-10-04

ADR-0076/[#175](https://github.com/MediaNoxLabs/compact/issues/175) is accepted locally. The typed AST emitter lifts a Boolean `Expr::Let` block to one scoped `__compact_condition: bool` binding before generated `if` and assertion statements, with collision-safe names and one evaluation. Simple conditions keep their shape. Four fixture libraries changed (asset registry, guarded arithmetic, mixed width and passport); the substantial passport reformat is nested Rust block layout, with no runtime, ABI 34, schema 8, ledger or ZK primitive change. No VM operation or aligned call input changed, so the proof gate was not repeated.

Verification: 66/66 renderer tests; 137/137 fixture outputs fresh; nine focused parity/behavior tests across the four affected crates; exact `cargo +1.99.0 clippy --workspace --all-targets --all-features -- -D warnings`; formatting and scoped diff checks. The all-target Clippy pass exposed pre-existing parity harness lints, repaired in the separate test-only commit. Conventional DCO/GPG commits: emitter `73fe1f6ceab2bbb3e85ab29e9e677fff4b7d92e7`, harness `246b349440e503d20fedf1437571b80739467279`; signatures verified. Branch remains local/unpushed; remote CI is deferred under the milestone plan.
