# Design: fix-ternary-expression-codegen

## Context

Both backends consume the same `Ltypescript` IR. The TS backend prints conditional expressions directly (`typescript-passes.ss:2850-2855`); the rust backend's shared expression renderer `expr-rust` (`compiler/rust-passes-emit.ss:1855-2144`) has **no clause for `(if src c e1 e2)`**, so its `[else]` raises `expr-variant`, which guarded callers swallow into "no walker shape matched" body errors (`rust-passes-emit.ss:2868` pure, `:1651` impure, `:203` constructor). The walkability gate `expr-supported?` (`rust-passes-walker.ss:720`, `[else #f]` at `:906`) has the same missing arm, which is what rejects impure bodies before emission is even attempted. `ctor-expr-rust` (`rust-passes-walker.ss:397-399`) falls through to `expr-rust`, so it inherits the fix for free. Return-position ternaries already work because `prepare-for-typescript` lifts only *top-level statement-position* conditional expressions into statement `if`s, which the statement walkers handle.

`compiler/README-rust-passes.md` documents the architecture ("the walker and the emitters both have to understand the same IR shapes") and the landing recipe for a new construct.

## Goals / Non-Goals

**Goals:**
- One expression-level fix that closes the gap in every position and body route, not per-walker statement patches.
- Laziness preserved exactly (spec: `compact-reference-proto.mdx:2114` — only the selected branch is evaluated).
- Regression coverage strong enough that no future backend change can silently drop or eager-ize the construct.

**Non-Goals:**
- No frontend/typechecker change; no TS backend change; no new shared desugaring pass (none exists for this; introducing one would be a larger architectural move than the gap warrants).
- No change to language version or runtime crates — the lowering uses plain Rust `if` expressions and the existing `compact_assert!`/`seq`-guard machinery only (precedent: G1 fix `8018e02`, Toolchain 0.31.111).
- Not fixing the location-less `expr-variant` catch-all diagnostic generally (this change merely stops ternaries from reaching it).

## Decisions

1. **Fix at `expr-rust` + `expr-supported?`, not in the body walkers.** The 5 upstream sites reach the gap via 3 different callers (const RHS via `stmt-pure-body-rust`, assert arg via `cond-rust` → `ctor-expr-rust` fall-through, interior operand via `arith-operand-rust`). A statement-walker patch would fix some positions and leave others broken. The single shared clause covers all callers; `ctor-expr-rust` inherits it by fall-through.
   *Alternative rejected*: desugar ternaries to statement `if`s in a shared pass — no such pass exists for expression-position conditionals; return-position lifting is the only precedent and only handles top-level statements.

2. **Emit a Rust `if` expression `if <cond> { <e1> } else { <e2> }`** — lazy by Rust semantics, matching the spec. Branch-local `seq` guard blocks (underflow asserts) render via the existing `seq` clause and stay inside their branch. Condition rendering recurses through the same condition routing the existing clauses use, preserving circuit/witness id routing.
   *Alternative rejected*: eager both-branches + select (ZK-select style) — observably wrong: an untaken underflowing branch would abort valid executions (e.g. `c = 9` in `c > 15 ? c - 10 : c`).

3. **Fixture `examples/ternary_cond_fixture.compact`** rather than extending `guarded_assert_arith_fixture.compact`: keeps each byte-parity crate attributable to one construct family, and the executing test needs the dedicated laziness oracle (`c = 9` must not trip; `c = 20` must compute) plus impure/constructor variants that don't belong in the arithmetic fixture.

4. **Version 0.31.117, `### Fixed` CHANGELOG entry** — mirrors the G1 fix's shape (compiler-only bump; language 0.23.103 / runtime 0.16.100 unchanged).

## Risks / Trade-offs

- [An `if` clause changes `expr-rust` output for previously-working shapes?] → No: absence of the clause *is* the failure; nothing currently renders through it. Byte-parity regen of all existing fixtures must stay green (AGENT.md §5.2 stop rule) — any diff in unrelated fixtures is an unintended consequence and blocks landing.
- [Laziness bugs that byte-parity cannot see] → the executing test pins both directions (untaken-branch-no-trap, taken-branch-computes) per "byte parity locks the text; executing tests lock the meaning".
- [More unsupported constructs behind this one] → empirically bounded: a probe rewrote only the 5 ternary sites of the digital-passport contract to if/else equivalents and the *entire* contract (incl. staged VC core) then compiled clean under `--target rust` (3,509 lines, zero `unimplemented!`), so no further known gaps exist in the dogfood target.
- [Generated-code fmt drift] → emitted bytes must already be rustfmt-clean (`codegen_regression` does not post-format); mirror existing clause formatting exactly.

## Migration Plan

Single forward change; no data or artifact migration. Rollback = revert commit (fixtures regenerate identically from the prior compiler).

## Open Questions

None — fix location, lowering shape, and fixture scope are all evidence-grounded by the audits recorded in the session that produced this change.
