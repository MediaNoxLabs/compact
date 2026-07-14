<!--
This file is part of Compact.
Copyright (C) 2026 Midnight Foundation
SPDX-License-Identifier: Apache-2.0
-->

# ADR 0002 — Gate the export-typedef promotion to the Rust target

- Status: Accepted
- Date: 2026-07-14
- Scope: `compiler/analysis-passes.ss`, `.github/workflows/*`
- Relates to: the M3.5-E2 struct/enum promotion (task "E2"), ADR 0001

## Context

The Rust H5-H7 emitter needs a `pub struct` / `pub enum` declaration for
every user struct/enum reachable from a public surface (exported circuit
args/returns, all witness/native args/returns, public-ledger field types),
including types that are only referenced transitively through other structs'
fields. To supply these, the M3.5-E2 change added a pass in
`analysis-passes.ss` that walks those surfaces and synthesises a
`export-typedef` for each referenced user type.

That synthesis ran **unconditionally**, and it mutates the shared
`Lexpanded` IR — the same IR the TypeScript backend and the whole
`compiler/test.ss` unit corpus consume. Its comment claimed this was "a
no-op for non-affected examples," but it is not: any program with a struct
in a signature now carries extra `export-typedef` forms. Running
`./compiler/go` surfaced **~64 failures** across `expand-modules-and-types`,
`infer-types`, `reject-recursive-circuits`, `track-witness-data`, and
`combine-ledger-declarations` — every `test-equal?` that pins a program's IR
shape saw the injected typedefs.

These failures were masked for the branch's life: the "Build and test
compiler" job died earlier on the vscode-extension `.snap` issue (see ADR
0001 / AGENT.md §4) and never reached the test step. Once that was fixed,
the corpus ran and went red. The failures are **not** the TS backend's or
upstream's — they are this fork's Rust-codegen change leaking into a shared
pass. The stop-gap was an `if: github.repository != 'yshyn-iohk/compact'`
skip on the "Run compiler tests" step (and, transitively, the coverage
steps that consume `./compiler/go`'s output). A fork-scoped skip is a smell:
it makes the fork's job green while the job still means nothing, and it
would mask any *future* real regression in those passes.

## Decision

Gate the promotion to the Rust target.

- The synthesis block is wrapped in `(when (emit-rust) …)`. `emit-rust` is
  the config parameter set to `#t` exactly when `--rust` is passed (see
  `compactc.ss`), and it is `#t` inside the `print-rust` test blocks
  (`parameterize ([emit-rust #t] …)`). So the Rust fixtures and the
  print-rust snapshots still get their promoted typedefs, while the
  TS-backend IR — and every non-Rust `compiler/test.ss` case — is byte-for-
  byte what it was before M3.5-E2.
- `analysis-passes.ss` now imports `(config-params)` (it previously did
  not, so `emit-rust` was unbound there — the profiled `./compiler/go` build
  caught this even though the whole-program `nix build` did not).
- With `./compiler/go` green on the fork, the `if:` guards on "Run compiler
  tests" and the two coverage-packaging steps in `build-compiler.yml` are
  removed. `compact-test.yml`'s trigger is scoped to `tools/compact/**` (its
  non-hermetic, release-cadence-driven tests are unrelated to the codegen),
  and the fixture fmt coverage that job's `pre-check` provided is absorbed
  into `rust-runtime-test.yml` (`cargo fmt --all --check`). No fork-scoped
  `if:` guards remain in the repo.

## Consequences

### Positive

- `./compiler/go` passes on the fork with **no golden edits** — the TS
  corpus asserts the same IR it always did.
- The Rust-codegen concern no longer perturbs the TS backend's IR: the two
  targets are decoupled at the pass level, which is the correct boundary.
- Every fork-scoped CI skip is gone; the compiler test corpus is a real gate
  again.

### Verification

- `./compiler/go` → `GO_EXIT=0`, 0 test failures, `print-rust` 1/1 (×2).
- `codegen_regression` → all fixtures byte-identical (promotion still runs
  under `--rust`, so the emitted crates are unchanged).
- `cargo fmt --all --check` under nix → clean (fixture crates included).

## Alternatives considered

- **Bless the 64 goldens.** Rejected: it would enshrine the leak (the TS IR
  carrying Rust-only typedefs), is a large mechanical change to assertions we
  don't own, and risks blessing an unintended regression among them.
- **Keep the fork-scoped skip.** Rejected: dishonest green; masks future
  regressions in those passes; see the `fix-ci` guidance on suppression.
