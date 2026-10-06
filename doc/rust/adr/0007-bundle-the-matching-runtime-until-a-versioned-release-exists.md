---
id: RUST-ADR-0007
alias: ADR-0007
title: "Bundle the matching runtime until a versioned release exists"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-interim"
topics: ["distribution", "provenance", "validation", "runtime-packaging"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 0b3fd2c5c349a0143508838ad7947457c0107b0b45763ab210c5b248defafe42
---
# RUST-ADR-0007 — Bundle the matching runtime until a versioned release exists

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-interim. Retain the accepted interim bundled matching-runtime default, with explicit shared-source and later exact-registry modes. Local signed candidate/archive and multi-contract rehearsals do not establish crates.io publication. Preserve the later approved branch-and-Nix distribution scope separately from the historical release wish list; optional registry syntax did not supersede bundling.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#106 closure](https://github.com/MediaNoxLabs/compact/issues/106#issuecomment-6017407693). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`0eec27ef`](https://github.com/MediaNoxLabs/compact/commit/0eec27ef86d2da9c98c933e513ebcc8f2db224ff) · [`13b6aca1`](https://github.com/MediaNoxLabs/compact/commit/13b6aca1d09a3bbab79bb67553e48eeb6bb85e86) · [`16829c24`](https://github.com/MediaNoxLabs/compact/commit/16829c2427d7918755b8b5176b0ea9ffb0e0c3db) · [`785db0a8`](https://github.com/MediaNoxLabs/compact/commit/785db0a889b90413fd9a7c5b09709e6978f04e91) · [`85198166`](https://github.com/MediaNoxLabs/compact/commit/85198166909e09fb040fd5880c63a422826d3820) · [`8949e894`](https://github.com/MediaNoxLabs/compact/commit/8949e894c6c708b70e23a169de040f9af8fdb253) · [`99020541`](https://github.com/MediaNoxLabs/compact/commit/990205411c5d050961e88302670a3114846342f6) · [`99aaeb1f`](https://github.com/MediaNoxLabs/compact/commit/99aaeb1fd32f583f0943dbad71c8515aac5a0a90) · [`e75f13ba`](https://github.com/MediaNoxLabs/compact/commit/e75f13ba2fdc954a29114177c31994a63e3cbec2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 0007
status: accepted-interim
date: 2026-10-02
milestone: rust-backend-v2
issue: 106
```

## Historical decision and amendments

### Problem

Generated code and runtime ABI must move together. A Git pin to the M1 runtime lacked the new recording API, and a path to a developer workspace made generated contracts nonportable. Bundling a copy makes one contract build independently, but two generated crates each bundling the same runtime package conflict in one Cargo graph. The macro crate is not yet published, so ordinary runtime `cargo package` cannot resolve its version from crates.io.

### Before and after

```toml
# Before: a generated crate expected an external checkout or stale Git runtime.
midnight-compact-runtime = { git = "...", rev = "<M1 revision>" }

# Interim standalone output: matching sources are copied into contract/.
midnight-compact-runtime = { path = "runtime-rs" }
```

For several contracts, invoke each compile with `--rust-runtime-root /path/to/compact`, so their manifests point at the same canonical local runtime. The generated crate reexports that exact runtime as `contract_crate::runtime` and forwards `ledger-transaction` when needed.

### Decision and ownership

The compiler packages matching runtime and macro source by default and records generated files in the hashed manifest. `--rust-runtime-root` is an explicit local multi-contract mode. ABI 3 assertions and schema-6 rejection protect the generated/runtime boundary; compiler, runtime, and Midnight crate versions remain pinned and documented. Do not silently switch to an unpublished crates.io runtime.

The emitter owns the generated manifest; runtime and macro crates own package metadata and release order. Publish the macro crate first, then the runtime, then evaluate whether the backend CLI is separately published. Once a versioned runtime is available and a fresh consumer proves exact graph compatibility, revise this ADR and migrate the generated dependency.

### Evidence, consequences, and remaining work

Local commits `99020541`, `85198166`, `8949e894`, `99aaeb1f`. Standalone and combined external Cargo consumers pass; the combined graph uses one shared runtime path. The packaged `--consumer --proof` gate passes. The shared path must remain available, so it is not a portable release solution. Ordinary runtime `cargo package` awaits a published `midnight-compact-runtime-macros 0.1.0`. Clean remote CI, artifact provenance, and tagged release validation remain open.

### Tracking

- Issue: [M2 stable crates, CI, and release #106](https://github.com/MediaNoxLabs/compact/issues/106).
- Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Delivery: interim local package strategy; versioned distribution not delivered.


### Decision history

- 2026-10-02: Created ADR-0007 from the generated-crate research probes with status `accepted-interim`; linked MediaNoxLabs/compact#106 in `rust-backend-v2`. Local delivery evidence is recorded above. Future changes should append a dated amendment or link a superseding ADR.


### Amendment — 2026-10-02: rehearse both release archives locally

#### Problem and before/after

`cargo package -p midnight-compact-runtime-macros` verified the macro crate, but its archive lacked the Apache-2.0 license text. The runtime archive had the same omission. Ordinary `cargo package -p midnight-compact-runtime` cannot resolve its exact macro dependency until `midnight-compact-runtime-macros = 0.1.0` is published. These two failures made a local package check insufficient as release evidence.

```toml
# Before: the crate archive contained no license text.
include = ["Cargo.toml", "README.md", "src/**", "tests/**"]

# After: both crate manifests explicitly include a local LICENSE copy.
include = ["Cargo.toml", "LICENSE", "README.md", "src/**", "tests/**"]
```

The new `python3 tools/compact-rust-backend/check_release_packages.py` runs `cargo package` on the macro crate, then on the runtime with a temporary Cargo patch to the local macro source. It inspects both `.crate` archives for `Cargo.toml`, `LICENSE`, `README.md`, and `src/lib.rs`, checks that the runtime archive still declares the exact `=0.1.0` macro version without a path dependency, and compiles each unpacked package. This models release order without changing the published manifest.

#### Ownership, evidence, and limit

The emitter and generated Cargo manifest do not change in this slice; generated contracts still bundle the ABI-matched runtime by default. Runtime and macro package manifests own the archive contents. The release rehearsal owns the local package verification. The runtime README now describes the current Cell, Counter, Set, Map, and List recording surface and the macro-first release order.

The local rehearsal passed: macro archive 8 entries and runtime archive 218 entries, both including LICENSE; both unpacked packages compiled. The earlier ordinary runtime package attempt failed solely because the exact macro crate is not yet published. The temporary patch is only a rehearsal mechanism. A clean remote CI run, publishing the macro and runtime, an unpatched external consumer, artifact provenance, and a tagged candidate remain open. This does not change ADR-0007's accepted-interim status or close [#106](https://github.com/MediaNoxLabs/compact/issues/106) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


The `Compiler Build` workflow now schedules this rehearsal as a separate step with Python 3.13. The existing header gate initially found seven runtime source/test files with incomplete license headers and the new script without one; the required headers were completed. Local `add_headers.py --validate` reports 0 missing across 1,449 files, and `cargo fmt --all -- --check` passes. The remote workflow has not yet run on this local branch.


#### Local delivery record

Conventional GPG-signed/DCO commit `13b6aca1` implements this amendment. The exact package rehearsal, header validation, and format gates above passed; the worktree is clean and 34 commits ahead of the remote M1 branch. [Issue #106 delivery comment](https://github.com/MediaNoxLabs/compact/issues/106#issuecomment-5945400418) records the same evidence. This hash is local until a future branch push; no remote CI or published crate is implied.


### Amendment — 2026-10-04: clean signed ABI-28 candidate

#### Problem and before/after

Earlier archive manifests at the local ABI-28 commit reported `dirty: true` because the primary checkout contains a pre-existing user-owned `doc/ledger-adt.mdx` edit. That prevented the release script's `--candidate-tag` check from proving clean source provenance, even though package archives verified.

```text
Before: source.commit = 0eec27ef, source.dirty = true, candidate_tag = null
After:  source.commit = 0eec27ef, source.dirty = false,
        candidate_tag = rust-backend-v2-abi28-rc1
```

A managed clean worktree at exact signed/DCO commit `0eec27ef86d2da9c98c933e513ebcc8f2db224ff` was used without touching the primary checkout. Local annotated tag `rust-backend-v2-abi28-rc1` is GPG-signed, `git verify-tag` passes, and the tag points to that commit. From the clean checkout, `check_release_packages.py --candidate-tag rust-backend-v2-abi28-rc1 --manifest target/rust-runtime-release-abi28-clean-rc1.json` passed, then `--verify-manifest` reproduced it. Macro archive: 9 entries, 10,908 bytes, SHA-256 `c066456e5374ad5ac5f8c8a9e74b9caa8c7313378c8306bb460086602a23920b`. Runtime archive: 247 entries, 168,921 bytes, SHA-256 `0b30b3340975714a5e00a3253b11f3a3bb478fa1061f1628c4ae2eb34a4a14a6`. Both archive hashes exactly match the earlier rehearsal from the primary checkout. Clean `nix build .#compactc` returned the same `${HISTORICAL_NIX_STORE}/65mg76yq8qdlaw1ql883lavr6galazjs-compactc` package that passed the 93-call consumer/proof/ledger gate.

The target symlink in the clean worktree points to disposable shared build outputs and is Git-ignored; `git status --porcelain` remained empty during the candidate check. The tag and branch remain local/unpushed. The runtime package still requires the local macro patch for unpacked Cargo verification until the macro crate is published; neither crate was published. Same-head remote CI, unpatched registry consumer, backend distribution and wallet/node submission remain open. This amendment advances #106's provenance gate without changing the interim bundling decision or claiming production release.


### Proposed amendment — 2026-10-04: archive-only generated consumer rehearsal

#### Problem and before

The clean signed ABI-28 candidate has reproducible macro and runtime archives, but `check_release_packages.py` still passes a local `patch.crates-io.midnight-compact-runtime-macros.path` while verifying the runtime package. That proves the archives can compile with source assistance, not that a generated crate can resolve the archived runtime and its exact macro dependency as ordinary versioned packages. The generated contract also defaults to a bundled path:

```toml
midnight-compact-runtime = { path = "runtime-rs" }
```

#### Proposed after and ownership

Rehearse a separate generated Counter consumer using only the two packaged `.crate` archives and Cargo's directory source replacement. Vendor the pinned upstream crates, add the extracted macro/runtime archives with checksums, and change only the disposable generated contract copy to:

```toml
midnight-compact-runtime = { version = "=0.1.0" }
```

The external consumer depends only on that generated crate and runs a real circuit. The released archives' manifests must retain version-only macro dependency; no `[patch]` or local runtime path may appear in the final consumer graph. This is a release-test change. The AST emitter, typed IR, runtime and macro source, ledger/zk mapping, public API and ABI 28 remain unchanged. The generated default remains bundled until publication and compatibility policy are ready.

#### Evidence required and limits

Verify the exact candidate archive SHA-256 values, an offline Cargo consumer using archive bytes and upstream pinned sources, generated contract ABI assertion, negative test when either archive is absent or mismatched, and no source path patch in the final consumer. Record disk/build cost and exact command. This local source-replacement test cannot stand in for a public registry, an unpatched crates.io consumer, same-head remote CI, or backend distribution. Keep #106 open for those gates. The initial manual probe resolved both archives and ran a minimal ABI-28 program; full generated-crate evidence is pending.


### Delivery amendment — 2026-10-04: archive-only generated consumer

Local conventional GPG-verified/DCO commit `e75f13ba2fdc954a29114177c31994a63e3cbec2` adds `check_archive_consumer.py` and the compiler CI step. It starts from a fresh packaged `compactc --target rust --skip-zk` Counter output. In a disposable copy, it replaces the generated `midnight-compact-runtime = { path = "runtime-rs", package = "midnight-compact-runtime" }` dependency with `{ version = "=0.1.0", package = "midnight-compact-runtime" }`, removes both bundled source directories, and builds an external crate that directly depends only on the generated contract. Cargo's directory source replacement is populated from the exact macro/runtime `.crate` archive bytes plus pinned upstream crates; the final consumer has no `[patch]`. A real Counter increment/read test passes. Metadata and Cargo.lock confirm both Midnight release crates resolve as registry-style versioned packages from the archive source.

The direct local command and the workflow-shaped `nix develop .#compiler --command python3 ...` command passed. From the clean worktree at this exact commit, signed annotated tag `rust-backend-v2-abi28-rc2` passes `git verify-tag`, `check_release_packages.py --candidate-tag ... --manifest`, and `--verify-manifest` with `dirty: false`. Exact-head `nix build .#compactc --print-out-paths --no-link` produced `${HISTORICAL_NIX_STORE}/9637iphdcfrn3sf0aagbnl8pw5r7xwc5-compactc`; the archive-only generated consumer passed against that compiler and the clean rc2 manifest. Altered macro hash and missing runtime archive copies were rejected before vendor/build. The macro archive has 9 entries / 10,908 bytes / SHA-256 `5e1fc99780ff5953294c61be5fada8fe65db3f32e4d70ac8759652ea2c86949d`; runtime has 247 entries / 168,919 bytes / SHA-256 `a68f937749cdbe516cb030d7e9eb075554f212f4dbf92b30626b5968f0f1f1b0`. Both archive hashes differ from rc1 because Cargo's `.cargo_vcs_info.json` records the new commit, and each reproduced byte-for-byte on the rc2 rerun.

The CI step is committed but **remote CI has not run** because the M2 branch remains local. This local source-replacement check verifies archive dependency resolution; it is not crates.io publication or a public registry consumer. The original interim bundled default and ABI 28/schema 8 remain unchanged. The emitter, runtime, macro and ledger/zk primitives were not modified. Vendor preparation used about 903 MiB in the manual probe and the script removes its temporary source tree afterward. Package publication, default multi-contract distribution, current-head wallet admission and remote CI remain #106 exit gates. Issue and milestone delivery recorded separately.


### Proposed multi-contract archive consumer extension — 2026-10-04

#### Problem and before

The archive-only gate at `e75f13ba` proves one generated Counter crate can replace its bundled runtime path with a version-only package from the signed archives. A normal Rust application often combines separately generated contracts. Each default output bundles a distinct `midnight-compact-runtime 0.1.0` path package, so two such outputs cannot form one portable Cargo graph without `--rust-runtime-root` pointing at a local checkout. The single-contract archive check does not prove that the versioned distribution solves this.

```toml
# Two independently generated manifests, before a versioned release:
midnight-compact-runtime = { path = "runtime-rs", package = "midnight-compact-runtime" }
```

#### Proposed after and ownership

Extend the disposable release-test consumer to compile Counter and Boolean Cell from the same exact-head packaged `compactc`, rewrite both copies to `midnight-compact-runtime = { version = "=0.1.0", package = "midnight-compact-runtime" }`, remove both bundled runtime/macro trees, and depend on the two generated contract crates in one external Cargo project. Exercise both native circuits and a cross-crate type assignment through their `runtime` reexports, then assert Cargo metadata contains exactly one archived runtime and one archived macro package with registry-style source and lock entries. Retain the existing negative archive hash/missing-file gates.

Release tooling and its CI step own this change. Typed IR schema 8, AST emitter, generated public APIs, runtime, derive macro, ledger-8/zk pins and ABI 28 remain unchanged. Do not silently change the default generated manifest until a published runtime and compatibility policy exist. The precise acceptance boundary remains local Cargo directory source replacement; public registry publication, remote CI, production wallet and release distribution remain separate #106 work. Measure execution and source size only if they change materially; no generated-code optimization is claimed.

#### Verification

Use the clean signed candidate archive manifest and exact-head packaged compiler, run both generated contract tests and the singleton graph assertion offline, reject a malformed archive before build, and compare scoped diff/CI YAML. Record exact signed/DCO commit and limitations after delivery. Focused [#106](https://github.com/MediaNoxLabs/compact/issues/106) is already in rust-backend-v2.

### Delivery amendment — 2026-10-04: shared archive runtime across generated crates

Conventional GPG-verified, DCO-signed local commit `16829c2427d7918755b8b5176b0ea9ffb0e0c3db` extends `check_archive_consumer.py`, its CI step, and the release guide. The release test invokes one exact-head `compactc --target rust --skip-zk` binary independently for Counter and Boolean Cell, rewrites only the disposable generated Cargo manifests to the exact versioned runtime, removes both bundled copies, then builds one external crate with both generated contracts. Its tests increment/read Counter and write/read the Boolean Cell, and a cross-crate `ContractAddress` assignment establishes a shared runtime type. Cargo metadata asserts exactly one registry-style macro and runtime package and direct dependencies from both generated crates to the same runtime ID. Typed IR, emitter, macro, runtime and ABI 28 did not change.

The implementation passed from the previously signed clean rc2 source commit and its byte-matched archives: both generated-contract tests and singleton graph assertion; malformed macro hash and missing runtime archive were rejected before vendoring. Python syntax and scoped `git diff --check` passed. The first attempt to produce a new archive manifest on the dirty primary checkout hung during Cargo's runtime verification compile (twice, including with incremental compilation disabled), so it was interrupted rather than counted as evidence. A clean signed `rust-backend-v2-abi28-rc3` tag points at `16829c24`; its exact-head package verification is running separately. Keep #106 open until that gate, remote CI, crates.io publication, a public-registry consumer, and distribution policy are established. No push or publication occurred.

### Delivery verification amendment — 2026-10-04: clean rc3 candidate

The proposed multi-contract archive consumer extension is verified at local signed/DCO commit `16829c2427d7918755b8b5176b0ea9ffb0e0c3db`, clean GPG-signed annotated tag `rust-backend-v2-abi28-rc3`. `git verify-tag` and commit `%G? = G` passed; clean worktree status was empty. `check_release_packages.py --candidate-tag rust-backend-v2-abi28-rc3 --manifest target/rust-runtime-release-abi28-clean-rc3.json` passed with `dirty: false`, and `--verify-manifest` reproduced both archive hashes. Exact-head `nix build .#compactc --no-link --print-out-paths` produced `${HISTORICAL_NIX_STORE}/735qmnaq6vwiipm5l5q9mz0c5h8hhbpl-compactc` (CLI 0.31.133). `check_archive_consumer.py --manifest target/rust-runtime-release-abi28-clean-rc3.json --compiler ${HISTORICAL_NIX_STORE}/735qmnaq6vwiipm5l5q9mz0c5h8hhbpl-compactc/bin/compactc` passed the two generated contract tests, cross-crate type identity, and singleton archived runtime/macro graph. Corrupt macro hash and absent runtime archive copies were rejected before vendoring. Macro archive: 9 entries / 10,908 bytes / SHA-256 `074afd363e0377894d2fc3bd89ca05bfc6f3efcfd8a2f80ae4db9ce5126e075d`. Runtime archive: 247 entries / 168,914 bytes / SHA-256 `91b60c84f82b9b53a260a08bbf798497e180e9ceb308f7a5e395e58c95404ef5`. The hash change reflects the commit embedded by Cargo in `.cargo_vcs_info.json`.

The clean candidate verifier used `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0` after the shared default debug target's runtime `rustc` appeared stalled in two interrupted dirty-checkout attempts. The fresh-profile first pass compiled for 23m34s; the cached `--verify-manifest` runtime verification took 7.94s. This is a local source-replacement release check, not published crates or a public registry consumer. No branch/tag push, remote CI, or production distribution occurred. The generated default still bundles matching runtime sources; the version-only rewrite exists only in the disposable release test. Keep #106 open.

### Registry-mode amendment — 2026-10-04

ADR-0051 / #150 adds an **explicit** alternative to this interim bundled default: `compactc --target rust --rust-runtime-registry` emits the exact matching versioned runtime package dependency without copied sources. The clean signed rc4 candidate and two-contract archive-only consumer passed with no generated-file edits; the default bundled mode and shared source-root mode remain. This changes the compiler CLI/package selection and release test only, not ABI 28 or runtime semantics. The registry mode is not yet generally buildable from crates.io because the macro/runtime packages have not been published. This ADR's interim default remains justified until publication, public consumer and compatibility policy pass; see ADR-0051 for commands, hashes and size measurements.



### Exact-head archive-only generated consumer — 2026-10-04

In the same clean `785db0a889b90413fd9a7c5b09709e6978f04e91` checkout, `python3 tools/compact-rust-backend/check_archive_consumer.py --manifest target/rust-runtime-release-785db0a8.json --compiler ${HISTORICAL_NIX_STORE}/nsaiqjddnzwldv8q02nbdnvkwfn53z51-compactc/bin/compactc` exited 0: `archive-only Counter + Cell consumer passed with one shared runtime`. The gate verified the manifest's exact commit/tree and both archive hashes, installed the unpacked macro/runtime archives into an offline vendor directory, generated independent Counter and Boolean Cell crates with exact `=0.1.0` runtime dependencies and no copied runtime sources, checked their compiler manifest entries, ran both contract tests in a separate Cargo consumer, and inspected the dependency graph for one shared runtime. This is a local archive-only simulation, not public registry publication. The full 95-call proof/ledger and ledger-v8 handoff gates had already passed with the same Nix-built compiler; same-commit remote CI is still pending.
