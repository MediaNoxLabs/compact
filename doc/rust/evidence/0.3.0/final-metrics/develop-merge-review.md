# Rust backend 0.3 develop PR integration review

- Base: `develop` at `7e38fc668b1a51ad3e565787b8d30ebd6542a1c8`.
- Head: `codex/rust-backend-ast` at `0d7610b2792d60fee5d3713d4eb0ede3c98cc82a`.
- Merge base: `c69a4793f34388dd765fb7ea0299bfc20f9b6a8b`.
- Distinct commits: develop **77**, AST **684**; 0.3 milestone delta **104**.
- Three-dot diff: 2636 files changed, 1275063 insertions(+), 2760 deletions(-).
- 0.3-only diff: 1306 files changed, 512142 insertions(+), 12310 deletions(-).
- `git merge-tree --write-tree`: exit **1**, **17** conflict paths. This command did not mutate the worktree/index.

## Integration concerns

### I1 — Divergent backend integration

Develop contains the text-emitter Rust backend and AST adds an independent structured emitter/runtime lineage. Seventeen merge conflicts include runtime and macro add/add paths. Selecting ours/theirs mechanically would discard a backend or silently mix incompatible abstractions.

### I2 — Version and dependency baseline

Develop compiler is 0.31.114, runtime/macros 0.16.100, flake ledger 8.0.2; AST compiler 0.31.133, runtime/macros 0.2.0 and exact ledger 8.0.3 Rust pins. Develop HEAD is ledger9-labeled but actual flake pins are ledger 8.0.2; do not describe the entire branch as ledger9.

### I3 — Qualified branch versus integration result

Milestone acceptance qualifies AST implementation f9a49666 and documentation closure 0d7610b2. It does not qualify a merge into develop. Resolve compiler passes, CLI dispatch, Cargo package identity, workflow and packaging as a separate integration slice with focused tests followed by necessary broader gates.

### I4 — Entire historical lineage in requested PR

The PR has 684 AST-only commits versus 77 develop-only commits from merge-base c69a4793, while milestone 0.3 has 104 commits from accepted milestone2 head 03c03a39. PR includes earlier Rust-AST milestones and upstream ledger8 lineage, not just 0.3.

### I5 — Documentation and evidence dominate raw diff

The three-dot diff includes generated fixtures, snapshots, qualification receipts and historical ADR collection. Insertions are not handwritten code or independent test-case counts.

## Conflicting paths

- `.github/workflows/compact-test.yml`
- `CHANGELOG.md`
- `Cargo.lock`
- `Cargo.toml`
- `compiler/compiler-version.ss`
- `compiler/passes.ss`
- `doc/ledger-adt.mdx`
- `flake.nix`
- `runtime-rs-macros/Cargo.toml`
- `runtime-rs-macros/README.md`
- `runtime-rs-macros/src/lib.rs`
- `runtime-rs/Cargo.toml`
- `runtime-rs/README.md`
- `runtime-rs/src/context.rs`
- `runtime-rs/src/lib.rs`
- `runtime-rs/tests/context.rs`
- `tests-e2e/src/resources/compiler_man_page.txt`

## Recommendation

Open draft PR to develop for review. Preserve qualified ledger8 AST branch and label unresolved integration conflicts; do not represent draft as merge-ready or rebase into divergent backend as routine cleanup.

This review does not claim a successful merge build or execution test. User 0.4 proposals and all working files were left untouched.
