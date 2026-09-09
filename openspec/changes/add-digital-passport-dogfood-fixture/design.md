# Design: add-digital-passport-dogfood-fixture

## Context

Precedent is split: the did-05 fixture (`09263a9`) vendored `.compact` source under `examples/` with relative imports preserved, `header_config.json` per-file exclusions, and full registration — the good pattern; the hand-imported `digital-passport` crate (`e1e963f`) had no source, no registration, and was removed as a gap. Both were dropped in `08decb1` to de-brand the corpus, with codegen coverage replaced by neutral fixtures. This change reintroduces third-party material **in a bounded enclave** for a different purpose (dogfooding real upstream code, not covering codegen paths), with the source-of-truth pattern. The upstream repo is a pnpm monorepo; its contract needs `core-compact-staging/` (15 files from the npm package's `dist/`, per its `stage-core-compact.mjs`) so the compiler's relative include `../core-compact-staging/credentials` resolves. Probe evidence: TS target compiles clean on 0.31.116; rust target compiles clean once ternaries are fixed (expected 3,509±-line `lib.rs`, zero `unimplemented!`).

## Goals / Non-Goals

**Goals:**
- Byte-faithful vendoring (upstream `src/` + staged core) with mechanical, documented refresh.
- The dogfood is gated exactly like a first-class fixture: fmt/clippy/compile/integration in CI, byte-parity locally per AGENT.md convention.
- The de-branding decision stays intact *outside* the enclave — ADR + AGENT.md make the supersession explicit and bounded.

**Non-Goals:**
- No porting of upstream's vitest suite, `smoke-consumer`, release tooling, or npm packaging (different purpose: our captures pin Rust↔TS parity, not upstream product behavior).
- No scheduled CI byte-parity or upstream-drift detection (explicitly dropped in planning; local AGENT.md §2.1/§3.3 discipline is the enforcement).
- No automatic re-sync of the pin; refresh is a human act recorded in PROVENANCE.
- No neutralization/rename pass now — verbatim is the point; a later neutralization remains possible and is recorded as such.

## Decisions

1. **`examples/dogfood/` subdirectory rather than flat `examples/` or top-level `dogfood/`.** Keeps `codegen_regression.rs`'s root-detection and `examples_dir.join(src_name)` path resolution untouched (nested FIXTURES rows work — did-05 row was `("did-05/contract/src/did.compact", "did-05")`), while partitioning branded material out of the neutral corpus. A top-level `dogfood/` would need gate surgery for cosmetic benefit; flat `examples/` re-mixes categories.
   *Alternatives rejected*: top-level `dogfood/` (gate change, no functional gain); flat (re-opens the 08decb1 tension pointlessly).

2. **Commit the staged core in-tree; refresh without pnpm.** The compiler consumes exactly the 15 `dist/` files; committing them makes the fixture hermetic and CI network-free. The staging script's npm-exports resolution is documented in PROVENANCE with a `curl $(npm view … dist.tarball) | tar xz` equivalent so refresh needs no pnpm install.
   *Alternative rejected*: CI fetch + staging script (network-dependent fixture; breaks byte-stability of the crate source).

3. **`excluded_directories` (one `dogfood` entry) over per-file `excluded_files`.** `add_headers.py` matches `excluded_files` by bare basename repo-wide — fragile across a 15-file tree and future refreshes; `excluded_directories` prunes by directory name at any depth and survives re-syncs. (Precedent: `third_party`, `node_modules` handled this way.)
   *Alternative rejected*: per-file entries (did-05 precedent, but designed for 2 distinctive filenames).

4. **Crate naming `compact-contract-digital-passport-credential`** (package name), dir `digital-passport-credential` — distinct from the removed `digital-passport` crate to avoid history confusion.

5. **CI additions kept minimal**: one clippy `-p` step (audit finding: clippy is an allowlist, dev-deps get `--cap-lints allow` — without this step the crate is never linted) and one codegen-only smoke line pair in `build-compiler.yml`'s existing smoke step (`nix develop .#compiler --command compactc …` ×2 targets). No cargo on the smoke lane (no Rust toolchain in that devshell); no schedule-guarded jobs (dropped in planning).

6. **Phase 2 captures as tasks within this change**, gated behind phase-1 landing: representative subset (helpers + one issuance/presentation/verification round-trip). Captures need TS-side Jubjub/Proof fixture construction — upstream's `testing/` utils show how; our `fixtures/capture-*.mjs` pattern shows the harness. If a capture proves disproportionately hard, record the blocker and shrink the subset — never weaken the helper captures (they pin the ternary laziness fix from the consumer side).

7. **Own patch bump 0.31.118** with full embed-site sweep — `changelog-check.yml` requires CHANGELOG + compiler-version.ss in every PR diff; fixture-only releases have precedent (0.31.106 did.compact support).

## Risks / Trade-offs

- [`.gitignore` silently truncates vendored files] (bare `dist`/`gen`/`out`/`artifacts` match at any depth) → post-vendor `git status --ignored` check is an explicit task; any colliding segment gets renamed and recorded in PROVENANCE.
- [PROVENANCE in crate `Cargo.toml` clobbered by regen] → PROVENANCE.md lives in the vendored source dir, never in generated output.
- [Upstream moves; pin goes stale] → accepted: refresh is explicit; PROVENANCE documents the procedure; the value is regression-gating today's contract, not tracking upstream HEAD.
- [Capture complexity (Jubjub fixtures) delays phase 2] → phase boundary isolates it; phase 1 alone already delivers compile+byte-parity gating.
- [Generated crate size (~3.5k lines) slows CI] → bounded: one more member in an already multi-crate workspace; dev-dep compile only.
- [Someone "cleans up" the enclave as accidental branding] → ADR + AGENT.md §1 record the decision and its bounds explicitly.

## Migration Plan

Cut `feature/add-digital-passport-dogfood-fixture` from `feature/fix-ternary-expression-codegen` (or `codegen-rust` after change 1 merges there); merge into `digital-passport-patch`. No migration of existing artifacts; the removed fixtures stay removed. Rollback = revert commit.

## Open Questions

None blocking. Capture-subset tuning (exactly which protocol round-trip to pin) is decided during phase 2 against upstream's `testing/credential-fixtures.ts` and can shrink/grow without touching the spec's representative-subset contract.
