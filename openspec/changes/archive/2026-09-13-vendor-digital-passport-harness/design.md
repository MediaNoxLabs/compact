## Context

PR #70 vendored the upstream package whole, then trimmed it (`7adfb04`) to the compact-only subset that the compiler's include graph actually resolves: 6 `src/**.compact` files + 15 staged `core-compact-staging/**.compact` files, 0 `.ts`. Its synced OpenSpec spec already stated the rule and the rationale ("no gate compiles, executes, or reads [the TypeScript]; compiler input is `.compact` only"). The sibling `add-digital-passport-dogfood-fixture` change has since been reshaped to match: it no longer claims the whole `src/` tree and explicitly defers the vendored set to this change ("`vendor-digital-passport-harness` owns the vendored set"), so no supersession is required here.

The compiler consumes `core-compact-staging/` because the entry's relative `include ../core-compact-staging/credentials` must resolve; upstream regenerates that tree as an untracked npm artifact, so it is committed here to keep the fixture hermetic and CI network-free.

Probe evidence (recorded in the PR): on the pre-fix compiler the TS target compiles the contract clean; the rust target fails at the ternary sites and at mixed-width comparison operands.

## Goals / Non-Goals

**Goals:**
- Byte-faithful, auditable vendoring of the compact-only subset, with a mechanical refresh path documented in `PROVENANCE.md`.
- The current Rust-codegen gap encoded as executable expected-refusals that the fix changes flip, so the oracle drives the fixes rather than trailing them.
- TS reference behaviour captured before any Rust change.

**Non-Goals:**
- No porting of upstream's vitest suite, `smoke-consumer`, release tooling, or npm packaging.
- No compiler change (this change must be green with the compiler as-is).
- No ADR/AGENT.md enclave policy or CI lint wiring (those belong to `add-digital-passport-dogfood-fixture`).

## Decisions

1. **Vendor the `.compact` subset, not the whole `src/`.** The TypeScript half is a different ecosystem and no gate reads it; committing it would make the fixture larger and create a false impression that it is exercised. The rule is stated as a requirement and checked by a completeness/no-`.ts` scenario.
2. **Commit `core-compact-staging/` in-tree; refresh without pnpm.** The compiler consumes exactly the 15 `dist/` files; committing them keeps the fixture hermetic. The npm-tarball staging procedure is documented in `PROVENANCE.md`.
3. **Oracle probes as `rejection_corpus` entries asserting current refusals.** The corpus already pins refusal kinds; adding the contract's real sites there makes the gap attributable per site and lets each fix change convert its own probes. A separate whole-entry expected-failure gate records the end-to-end state; it lives in `rejection_corpus.rs` as a `rust_backend_*` test that compiles the vendored entry in place by repo-relative path (a `compile_repo_relative` helper mirroring `codegen_regression.rs`), because the inline harness's temp-dir `compile()` cannot resolve the entry's `include ../core-compact-staging/...`.
4. **Captures authored now, executing Rust test deferred.** The capture script only needs the TS target, which compiles today; the Rust parity test can only run once the contract compiles, so it lands with the packaging change.

## Risks / Trade-offs

- [`.gitignore` silently truncates vendored files] (bare `dist`/`gen`/`out` patterns match at any depth) → a `git status --ignored` / `git ls-files` completeness task is explicit.
- [Expected-refusal probes rot into permanent acceptances if nobody flips them] → each probe names the fix change that must flip it, and the change that owns the flip is listed in the tasks.
- [Capture complexity (Jubjub fixtures) delays the change] → the helper captures are mandatory, the protocol round-trip may shrink; never weaken the helper captures.

## Migration Plan

Cut `feature/vendor-digital-passport-harness` from `codegen-rust` (after `compiler-backed-ci-gate`). No migration of existing artifacts. Rollback = revert commit.
