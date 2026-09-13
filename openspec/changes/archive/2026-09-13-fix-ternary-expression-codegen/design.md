## Context

Both backends consume the same `Ltypescript` IR. The TS backend prints conditional expressions directly (`typescript-passes.ss:2850-2855`); the Rust backend's `expr-rust` has no `(if src c e1 e2)` clause, so its `[else]` raises `expr-variant`, which guarded callers swallow into "no walker shape matched" body errors (pure `rust-passes-emit.ss:2868`, impure `:1651`, constructor `:203`). The walkability gate `expr-supported?` (`rust-passes-walker.ss:720`, `[else #f]` at `:906`) has the same missing arm. Return-position ternaries already work because `prepare-for-typescript` lifts only top-level statement-position conditionals into statement `if`s, which the statement walkers handle.

The crucial difference from PR #70: the typechecker already wraps each ternary arm to the join type (`analysis-passes.ss:2589-2603`), and `type-directed-expression-coercion` makes the renderer consume that wrapper. So this change must not add any literal/width logic of its own — the `if` clause simply obtains the expected type from its context and renders each arm through `expr-rust-typed`.

`compiler/README-rust-passes.md` documents the architecture ("the walker and the emitters both have to understand the same IR shapes") and the landing recipe.

## Goals / Non-Goals

**Goals:**
- One expression-level clause that closes the gap in every position and body route, not per-walker statement patches.
- Type/width correctness for arms delegated entirely to `type-directed-expression-coercion`.
- Laziness preserved exactly (spec: `compact-reference-proto.mdx:2114` — only the selected branch is evaluated).
- Coverage strong enough that no future backend change can silently drop or eager-ize the construct: a probe per reachable matrix cell.

**Non-Goals:**
- No frontend/typechecker/TS-backend change; no new shared desugaring pass.
- No change to language version or runtime crates (plain Rust `if` expressions).
- No new coercions: any arm type/width handling belongs to the prerequisite change; this change adds none.

## Decisions

1. **One clause in `expr-rust`, arms via `expr-rust-typed`.** The 5 upstream sites reach the gap via three callers (const RHS, assert arg, arithmetic operand); a statement-walker patch would fix some positions and miss others. `ctor-expr-rust` and streaming inherit by fall-through. Rejected: a shared desugaring pass to statement `if`s (none exists for expression-position conditionals; larger architectural move).
2. **Rust `if` expression, never eager select.** Lazy by Rust semantics, matching the spec; an untaken underflowing branch cannot abort a valid execution. Rejected: eager both-branches + select (observably wrong).
3. **Explicit coverage matrix in `tasks.md`.** Enumerated as a table so a missing cell is visible; this is the direct antidote to PR #70's fixture-by-accident coverage.
4. **State-byte parity for ledger writes.** The `route × position` cell that writes to a ledger gets a serialized-byte comparison, since decoded-value tests hid the 1-vs-8-byte divergence in PR #70.
5. **Version 0.31.118**, `### Fixed` entry, language/runtime unchanged.

## Risks / Trade-offs

- [Arms rendered at the wrong expected type] → inherited from the prerequisite change and pinned by the matrix; this change adds no arm logic to diverge.
- [Laziness bugs byte-parity cannot see] → executing tests pin both directions (untaken-branch-no-trap, taken-branch-computes).
- [Streaming route admits a shape it cannot render] → `expr-supported?` and the streaming renderer both route arms through the typed entry; a dedicated streaming matrix row covers it, and the no-sentinel guard from the prerequisite change refuses rather than splicing.
- [Format drift] → emitted bytes must be rustfmt-clean; mirror existing clause formatting.

## Migration Plan

Cut from `codegen-rust` after `type-directed-expression-coercion`. Rollback = revert commit; fixtures regenerate from the prior compiler. The oracle probes revert to REJECTION.
