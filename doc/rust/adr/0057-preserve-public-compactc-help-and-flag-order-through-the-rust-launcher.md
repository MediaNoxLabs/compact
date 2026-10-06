---
id: RUST-ADR-0057
alias: ADR-0057
title: "Preserve public compactc help and flag order through the Rust launcher"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler-cli", "diagnostics"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 6d9442fd995bf8e7c893148f9dddccdd6a32078a02affe080a19d517a1e94f87
---
# RUST-ADR-0057 — Preserve public compactc help and flag order through the Rust launcher

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept launcher/frontend changes that preserve public compactc naming, help and flag precedence while keeping Scheme internals private. Existing public-format expectations and later proof/handoff checks retain exact revisions. Do not read an older skipped/interrupted workspace run as a completed current gate.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#156 closure](https://github.com/MediaNoxLabs/compact/issues/156#issuecomment-6017495043). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`09c52a7d`](https://github.com/MediaNoxLabs/compact/commit/09c52a7dfd0f122313b57e1134aefe852d38a2e8) · [`785db0a8`](https://github.com/MediaNoxLabs/compact/commit/785db0a889b90413fd9a7c5b09709e6978f04e91) · [`f155ae78`](https://github.com/MediaNoxLabs/compact/commit/f155ae7834bfcc9acec31648019f8e4eef45f7e2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 57
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/156
```

## Historical decision and amendments

### Problem and reproduction

The exact committed `09c52a7d` compiler passes `compiler/go`, source maps, and 477 of 497 active end-to-end cases, but five `compiler.smoke.e2e.test.ts` cases fail deterministically. The new first-class Rust `compactc` launcher delegates public `--help` and error usage to its internal `compactc-scheme` sibling. Chez therefore prints the implementation binary name and internal `--emit-rust-ir`/`--skip-ts` flags. The launcher also checks for `--help` anywhere before version flags, reversing the established first-flag behavior for `--version --help`. The old manual-page fixture lacks public Rust options. Nine separate composable-contract hook timeouts occurred during local macOS cold startup and require separate diagnosis; they are not explained by this decision.

### Before and after public CLI

Before:

```text
$ compactc --help
Usage: compactc-scheme <flag> ... <source-pathname> <target-directory-pathname>
  --emit-rust-ir ...
  --skip-ts ...

$ compactc --version --help
Usage: compactc-scheme ...
```

After:

```text
$ compactc --help
Usage: compactc <flag> ... <source-pathname> <target-directory-pathname>
  --target <ts|rust> ...
  --rust-require-recording ...
  --rust-runtime-root <path> ...
  --rust-runtime-registry ...

$ compactc --version --help
0.31.133
$ compactc --help --version
Usage: compactc ...
```

The public help should describe the target and runtime selection. The Scheme IR and TypeScript suppression switches remain supported for the internal launcher but do not belong in the public help. Normal TypeScript compilation keeps its established output and errors.

### Ownership and alternatives

The Rust CLI launcher owns public flag selection and precedence; the Compact Scheme entry point owns compiler parsing/help; the end-to-end manual-page resource owns the exact public help expectation. Use a public command name only in `compactc.ss`'s command-line parsing and usage display, leaving `format-compact` and `fixup-compact` unchanged. Retain Scheme compiler internals without displaying them to ordinary users. Do not merely relax the smoke tests: the `compactc-scheme` name and precedence regression are real public API defects. Avoid changing shared `program-common` naming semantics for other tools. Typed IR, Rust emitter, generated source, runtime, derive macros, ledger-8/zk mapping, ABI 28, private schema 8, and capability-report schema 1 do not change.

### Acceptance and risks

- Public `compactc` help, no-argument/unknown usage, and version/help ordering match established behavior and include documented Rust options.
- Internal Scheme flags remain accepted by the launcher for Rust generation but are absent from public help; direct Scheme compiler behavior may remain implementation-specific except for its usage name.
- Update the exact end-to-end manual-page fixture and focused CLI tests. Verify packaged Nix `compactc` smoke cases and full `compiler/go` after Scheme source changes, then the complete CI suite on the published branch.
- Preserve the distinction between deterministic help failures and the local macOS cold-start hook timeouts; do not claim the latter fixed by this ADR.

### History

- 2026-10-04: proposed from exact-commit end-to-end run: 477 pass, 14 fail, 6 skip. Five smoke failures demonstrate public help/name/precedence drift. Issue to be created before implementation.


- 2026-10-04: focused [MediaNoxLabs/compact#156](https://github.com/MediaNoxLabs/compact/issues/156) created and assigned to rust-backend-v2 before implementation; register updated. The issue distinguishes five deterministic smoke failures from nine local cold-start hook timeouts.



### Local cold-start distinction — 2026-10-04

The 9 composable-contract failures in the first full macOS E2E run were `beforeEach` hook timeouts at 10 seconds while many Chez processes were concurrently starting from a fresh Nix archive; a sampled Chez child remained at dyld startup while its Rust launcher waited on the child. Without code changes, a warm focused replay of unchanged `compiler.composable.direct.e2e.test.ts` with the same exact `09c52a7d` Nix compiler passed 17 tests, 4 skipped, in 3.84 seconds. This supports a local cold-start/concurrency explanation, but it does not substitute for the complete remote CI run. The five help/flag-order smoke failures are deterministic and are addressed separately by ADR-0057/#156.



### Local delivery — 2026-10-04

Conventional good-GPG/DCO commit `785db0a889b90413fd9a7c5b09709e6978f04e91` implements the decision in four files only: `compiler/compactc.ss` prints the public `compactc` name and hides internal handoff flags from help, `tools/compact-rust-backend/src/bin/compactc.rs` chooses the first public query flag in order, and the E2E manual-page resource/helper now expect the public target options and command name. No typed IR, emitter, generated crate, runtime, macro, ABI 28, schema 8, dependency pin, version, or contract artifact format changed. The user-owned `doc/ledger-adt.mdx` remains unstaged.

Focused launcher tests pass 8/8, `cargo fmt --all -- --check` and scoped `git diff --check` pass. A disposable WIP source snapshot with these four edits passed `nix build .#compactc`, including its Scheme compiler and source-map check phase, producing `${HISTORICAL_NIX_STORE}/nsaiqjddnzwldv8q02nbdnvkwfn53z51-compactc`. A pristine `git archive` of the **committed** `785db0a8` independently passes the 1,520-file header validator and resolves `nix build .#compactc` to the same store path. The packaged binary matches the exact public manual-page fixture, returns 0.31.133 for `--version --help`, returns help for `--help --version`, and prints public `compactc` usage for no args and unknown flags (five direct checks). The same exact committed source and package pass the five formerly failing E2E smoke cases, with the other 15 cases filtered out. Packaged `--skip-zk` compilation of `examples/tiny.compact` produced the expected TypeScript output and `--target rust --skip-zk` produced a Rust Cargo crate, private IR, capabilities and ZKIR. The full E2E suite and same-commit remote CI have not been rerun after this commit; the separate clean Rust workspace suite remains running at earlier `f155ae78`.



### Exact-commit full E2E gate — 2026-10-04

A pristine `git archive` of signed/DCO `785db0a889b90413fd9a7c5b09709e6978f04e91`, using its exact Nix-built `${HISTORICAL_NIX_STORE}/nsaiqjddnzwldv8q02nbdnvkwfn53z51-compactc` and lockfile-identical E2E dependencies, passed the full `tests-e2e` `yarn test` run under `nix develop .#compiler`: **50 test files passed, 491 tests passed, 6 skipped, 0 failed** (100.17 seconds wall clock; Vitest 70.04 seconds). This supersedes the earlier note that the full E2E suite had not been rerun. The five previously deterministic CLI smoke failures and nine cold-start timeout failures did not recur. This is local exact-commit evidence, not the required same-commit remote `Compiler Build` workflow. The clean Rust workspace suite remains in flight at the older `f155ae78` head; its result cannot be attributed to this commit.


### Exact-head packaged generated-consumer gate — 2026-10-04

A pristine `git archive` of signed/DCO `785db0a889b90413fd9a7c5b09709e6978f04e91` passed `COMPACTC=${HISTORICAL_NIX_STORE}/nsaiqjddnzwldv8q02nbdnvkwfn53z51-compactc/bin/compactc python3 tools/compact-rust-backend/check_compactc_target.py --consumer` with exit 0 and `compactc target boundary and manifest: passed`. The script checks TS/Rust/combined/legacy target outputs, private IR schema 8 and source positions, capability report, manifest, atomic output/symlink handling, and five separate Cargo consumer paths: standard, shared runtime, witnessed, Merkle witnessed, and List shapes. This is the exact CI macOS consumer command's local analogue using the Nix-built package from the same committed source. It does not include `--proof`; earlier clean rc7 proof/ledger evidence is at `f155ae78`, and same-commit remote CI remains open. The full Rust workspace suite still runs at earlier `f155ae78`.


### Exact-head packaged proof and ledger gate — 2026-10-04

A fresh pristine `git archive` of signed/DCO `785db0a889b90413fd9a7c5b09709e6978f04e91` passed the complete local `nix develop .#compiler --command env COMPACTC=${HISTORICAL_NIX_STORE}/nsaiqjddnzwldv8q02nbdnvkwfn53z51-compactc/bin/compactc ... python3 tools/compact-rust-backend/check_compactc_target.py --consumer --proof` gate with exit 0 and `compactc target boundary and manifest: passed`. The command used the same Nix-built compiler from the exact commit; its separate Cargo consumers, ZKIR/prover/verifier artifacts, recorded replay, proof verification, ledger validation and application all completed. The smoke script and proof program are byte-unchanged since `f155ae78`, where the gate's **93 call cases** were enumerated; this is exact-head revalidation of that 93-case program, including observed Counter and Field subtraction/multiplication calls. A reused Cargo target cache reduced build work but did not alter source or emitted output. This invocation did not set `COMPACT_RUST_WALLET_HANDOFF` or `COMPACT_RUST_DEPLOY_HANDOFF`; the CI JavaScript wallet-handoff check is a separate pending gate. Same-commit remote `Compiler Build` CI remains open.


### Proof case-count correction — 2026-10-04

The preceding exact-head note's **93** count was the pre-arithmetic ABI-28 baseline. Signed/DCO `f155ae78` added `subtract_amount` and `multiply_amount`, so its program and committed `785db0a8` run contain **95** recorded/proven/validated/applied call cases. A second direct run of the exact-head compiled proof smoke binary against a preserved copy of its generated artifacts exited 0; `${LOCAL_EVIDENCE}/compact-proof-wallet-785db0a8.log` contains 95 `generated ... trace replayed and partitioned` lines and 95 `deployment and proven call validated and applied` lines. This correction supersedes only the case count in the preceding paragraph; its gate outcome remains valid. The replay emitted a 3.3 KiB call handoff and 1.7 KiB deploy handoff, SHA-256 `1aefb8bf0c0d96d1a0c2d9f36727b43fbdc400d76d0895c396ec7464787c0bac` and `30e0dd810e2363fd6ca37265f47f44c67d99d574bd02e051f9e0f4b77e731924`. JavaScript ledger-v8 handoff validation is pending.


### Exact-head ledger-v8 wallet handoff — 2026-10-04

The second exact-head `785db0a8` proof-smoke replay with `COMPACT_RUST_DEPLOY_HANDOFF` and `COMPACT_RUST_WALLET_HANDOFF` exited 0 and produced 1,765 deploy bytes and 3,372 call bytes. The committed-source `check_wallet_handoff.mjs` decoded both with pinned `@midnight-ntwrk/ledger-v8@8.0.3`, checked one deploy and one proven call in segment 1, matching address `85e623ca9ada2b6379b5ce1c929467474d7690c9b38e1bec78177bf92b589a2b`, and exact deserialize→serialize byte equality. The installed package's lockfile matches the exact archive's `wallet-handoff/package-lock.json`; the checker also validates the package name/version. The command exited 0: `ledger-v8 8.0.3 decoded 1765 deployment and 3372 call bytes ...`. The preserved log independently counts 95 replayed and ledger-applied call cases. This is a local binary handoff check, not a live wallet/node submission or same-commit remote CI result.
