---
id: RUST-ADR-0013
alias: ADR-0013
title: "Bind runtime archives to a release manifest"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["distribution", "provenance", "validation", "runtime-packaging"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 088d87705f6a413158db4f3aa60463b67f489840df3ba9ddd70c950ceb365799
---
# RUST-ADR-0013 — Bind runtime archives to a release manifest

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept hash-bound runtime archive manifests and verification, with optional clean signed-tag candidate checks separate from ordinary possibly dirty rehearsals. Subsequent archive and multi-contract receipts advance provenance, not registry distribution or cross-platform reproducible bytes. Preserve exact source/tree/lock/archive identities and the later branch-only milestone distribution amendment.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#114 closure](https://github.com/MediaNoxLabs/compact/issues/114#issuecomment-6017421423). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`16829c24`](https://github.com/MediaNoxLabs/compact/commit/16829c2427d7918755b8b5176b0ea9ffb0e0c3db) · [`2f0f3841`](https://github.com/MediaNoxLabs/compact/commit/2f0f3841e0c4bc8a8c1d2705c26692b3137d5177) · [`6b65a491`](https://github.com/MediaNoxLabs/compact/commit/6b65a4915d6984665925971463195dbc4dd795a2) · [`785db0a8`](https://github.com/MediaNoxLabs/compact/commit/785db0a889b90413fd9a7c5b09709e6978f04e91) · [`c1a0fc89`](https://github.com/MediaNoxLabs/compact/commit/c1a0fc890468e1497219c74ad002926cbb457cf6) · [`e75f13ba`](https://github.com/MediaNoxLabs/compact/commit/e75f13ba2fdc954a29114177c31994a63e3cbec2) · [`f155ae78`](https://github.com/MediaNoxLabs/compact/commit/f155ae7834bfcc9acec31648019f8e4eef45f7e2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 0013
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/114
```

## Historical decision and amendments

### Problem

`check_release_packages.py` packages and compiles the two runtime crates, but the result is only a log. Reviewers cannot match a release candidate to the exact Git revision, dependency locks and `.crate` bytes that passed the rehearsal. The runtime still depends on a local macro patch until the exact macro version is published, so a passing rehearsal is not a publication claim.

### Before

```sh
python3 tools/compact-rust-backend/check_release_packages.py
# checked two local archives; no durable byte-level record
```

### Decision and after

Extend the existing rehearsal with a deterministic JSON manifest containing the source commit and tree state, candidate tag when requested, package names/versions, archive SHA-256 and sizes, Rust/Cargo toolchain versions, and hashes of the repository and isolated backend Cargo locks plus `flake.lock`. An ordinary local rehearsal may be dirty and must say so. Candidate mode must require a clean checkout at the named signed Git tag. A verifier must repackage and compare the recorded package bytes and source inputs. CI should retain the manifest and package archives together.

```sh
python3 tools/compact-rust-backend/check_release_packages.py --manifest target/rust-runtime-release.json
python3 tools/compact-rust-backend/check_release_packages.py --verify-manifest target/rust-runtime-release.json
# For a future tagged candidate:
python3 tools/compact-rust-backend/check_release_packages.py --candidate-tag rust-backend-v2-rc1 --manifest target/rust-runtime-release.json
```

### Emitter and runtime ownership

The Rust emitter, IR, generated crate API, runtime source code, proc macros and ABI do not change. This is a release-gate change around the existing paired `0.1.0` macro/runtime archives. The runtime's exact macro version remains the published dependency contract; local package verification still uses the explicit Cargo patch and the manifest must disclose that. The generated contract stays bundled until an independently published runtime is validated.

### Alternatives and rationale

A log or a commit hash alone cannot identify the archive bytes. A GitHub artifact alone is downloadable but does not state its input or dependency graph. A manifest with archive hashes and lock hashes is readable, can be checked locally, and can travel with the archives. It is not a supply-chain attestation or proof of crates.io publication; those gates remain separate.

### Verification and risks

Check that repeated package runs have stable hashes, manifest verification detects a changed archive or lock input, and candidate mode rejects dirty, missing, lightweight or unsigned tags. Run the existing package rehearsal and repository header/format gates. Rust/Cargo version strings are diagnostic inputs; cross-platform package hash reproducibility still needs a remote runner. Record local results and limitations in a dated amendment.

### Tracking and delivery

- Focused MediaNoxLabs issue: [#114](https://github.com/MediaNoxLabs/compact/issues/114), under parent [#106](https://github.com/MediaNoxLabs/compact/issues/106).
- Milestone: rust-backend-v2.
- Local commit: pending.
- Delivery state: proposed.

### Amendments

Append dated evidence without rewriting the problem or rationale.


### Delivery amendment — 2026-10-02: verifiable local archives

**Before/after workflow.** Local conventional GPG-signed/DCO commit `c1a0fc890468e1497219c74ad002926cbb457cf6` extends the existing release rehearsal, README and compiler CI. Before, the gate only printed that each unpacked `.crate` compiled. After, it writes a stable JSON record and verifies a fresh package run against it:

```sh
python3 tools/compact-rust-backend/check_release_packages.py --manifest target/rust-runtime-release.json
python3 tools/compact-rust-backend/check_release_packages.py --verify-manifest target/rust-runtime-release.json
```

The record names commit, Git tree and dirty state; optional candidate tag; both package names, versions, archive SHA-256, byte lengths and entry counts; Cargo/Rust versions; and SHA-256 for workspace Cargo.lock, isolated backend Cargo.lock and flake.lock. It explicitly discloses the local macro patch and that registry publication is false. CI runs write plus verify and uploads the manifest beside the two archives. A release candidate additionally requires a clean checkout, a signed annotated tag and that tag at HEAD. The script does not modify the emitter, generated code, proc macros, runtime ABI or Rust runtime behavior. The packaged runtime manifest still declares the exact macro version with no path.

**Evidence.** Two ordinary package rehearsals produced identical archive hashes (`b8fddd16…` for macros, `a3a2ee01…` for runtime). A saved manifest verified against a fresh package run. A deliberately changed macro hash failed at `manifest.packages[0].sha256`. Candidate write and verify passed on a temporary local GPG-signed annotated tag at the clean commit; the tag was deleted afterward. Dirty worktree, lightweight tag and unsigned annotated tag were rejected before packaging. `cargo fmt --all -- --check`, staged diff check, header validation (0 missing across 1,461 files), Ruby YAML parse and `git verify-commit HEAD` passed. The local source commit has a DCO sign-off. No M2 branch push or remote CI run is claimed.

**Limits.** The local manifest under ignored `target/` is a rehearsal artifact; CI will retain its own archives and manifest after the branch is published. A signed tag and matching hashes do not establish registry publication, unpatched consumer resolution, cross-platform byte reproducibility, a real `rust-backend-v2` release candidate, or wallet/node integration. Those gates remain open on [#114](https://github.com/MediaNoxLabs/compact/issues/114) and [#106](https://github.com/MediaNoxLabs/compact/issues/106). This amendment supersedes the proposal's pending-delivery placeholders while preserving its rationale.


### Clean rc2 provenance amendment — 2026-10-04

The signed/DCO release-test commit `e75f13ba2fdc954a29114177c31994a63e3cbec2` has a new local GPG-signed annotated tag `rust-backend-v2-abi28-rc2` rather than moving rc1. A clean checkout wrote and re-verified `target/rust-runtime-release-abi28-clean-rc2.json` with `dirty: false`, exact commit/tree/locks and archive SHA-256 values. The exact-head Nix compiler package is `${HISTORICAL_NIX_STORE}/9637iphdcfrn3sf0aagbnl8pw5r7xwc5-compactc`, and the archived macro/runtime packages support a generated external Counter consumer through version-only Cargo dependencies in an isolated vendor source. See ADR-0007 / #106 for the implementation and negative archive checks. `.cargo_vcs_info.json` embeds rc2's commit, so the two archive hashes differ from rc1 while each is reproducible at rc2. Tag and branch remain local; remote CI, registry publication and distribution are not claimed.

### Candidate verification amendment — 2026-10-04: rc3

The conventional GPG/DCO commit `16829c2427d7918755b8b5176b0ea9ffb0e0c3db` is tagged locally with verified GPG-signed annotated `rust-backend-v2-abi28-rc3`. Its clean manifest `target/rust-runtime-release-abi28-clean-rc3.json` records tree `77b552560ca299dd79dca60da2369e3143c5917f`, `dirty: false`, macro SHA-256 `074afd363e0377894d2fc3bd89ca05bfc6f3efcfd8a2f80ae4db9ce5126e075d` (10,908 bytes), and runtime SHA-256 `91b60c84f82b9b53a260a08bbf798497e180e9ceb308f7a5e395e58c95404ef5` (168,914 bytes). `--candidate-tag ... --manifest` and `--verify-manifest` both passed. The matching Nix `compactc` path is `${HISTORICAL_NIX_STORE}/735qmnaq6vwiipm5l5q9mz0c5h8hhbpl-compactc`; an archive-only Counter+Boolean Cell consumer passed against this exact compiler and manifest with a shared runtime. The archives differ from rc2 because Cargo embeds the source commit. The milestone release remains local; registry publication, public consumer and remote CI are open under #106/#114.

### Candidate verification amendment — 2026-10-04: rc4 registry mode

The signed/DCO commit `6b65a4915d6984665925971463195dbc4dd795a2` has local GPG-signed clean tag `rust-backend-v2-abi28-rc4`. The manifest `target/rust-runtime-release-abi28-clean-rc4.json` records tree `b1ad399b0e474ab4aa730fa7f57b345db7afde92` and `dirty: false`; `--candidate-tag ... --manifest` and `--verify-manifest` both passed. Macro `.crate` SHA-256 `819d094771b5d769b3cf1f60ad8f15563516170037fd02093991f7831c549faa` (10,910 bytes), runtime `14f6ac3c644299d0a3dfe1db66dcdf8c62099824336e4e275fcf31c5649660b9` (168,915 bytes). Exact-head Nix `compactc` `${HISTORICAL_NIX_STORE}/72hq82g98kvdz0mffsx7s38nc3pjd4cc-compactc` generated untouched version-only Counter and Boolean Cell crates that passed the archive-only shared-runtime consumer. ADR-0051 / #150 owns the new option and its before/after design. Publication, public consumer and remote CI remain open in #106/#114/#150.



### Exact-head clean runtime archive rehearsal — 2026-10-04

A separate clean managed checkout at signed/DCO `785db0a889b90413fd9a7c5b09709e6978f04e91` passed `check_release_packages.py --manifest target/rust-runtime-release-785db0a8.json` and a second `--verify-manifest` packaging run. Manifest schema 1 binds commit `785db0a8`, source tree `3b81e28e61c186b47482da408545679acb1c981c`, `dirty: false`, dependency locks and toolchain. The macro archive has 9 entries and SHA-256 `84c50d14d66d83751174ea6612344c7d3d80d2e63741581ad87832fa082dd5b5`; runtime has 247 entries and SHA-256 `5e812c54acf183de163de1af7c2e2243466d8146fd4a402f251a2b85f3154f09`. No new tag or registry publication is claimed; the amended rust-backend-v2 exit does not require either. The generated-only consumer from these archives has been started but is not yet reported as passed. The older full workspace test at `f155ae78` and same-commit remote CI remain open.


### Exact-head archive-only generated consumer — 2026-10-04

In the same clean `785db0a889b90413fd9a7c5b09709e6978f04e91` checkout, `python3 tools/compact-rust-backend/check_archive_consumer.py --manifest target/rust-runtime-release-785db0a8.json --compiler ${HISTORICAL_NIX_STORE}/nsaiqjddnzwldv8q02nbdnvkwfn53z51-compactc/bin/compactc` exited 0: `archive-only Counter + Cell consumer passed with one shared runtime`. The gate verified the manifest's exact commit/tree and both archive hashes, installed the unpacked macro/runtime archives into an offline vendor directory, generated independent Counter and Boolean Cell crates with exact `=0.1.0` runtime dependencies and no copied runtime sources, checked their compiler manifest entries, ran both contract tests in a separate Cargo consumer, and inspected the dependency graph for one shared runtime. This is a local archive-only simulation, not public registry publication. The full 95-call proof/ledger and ledger-v8 handoff gates had already passed with the same Nix-built compiler; same-commit remote CI is still pending.

### Exact M2 head replay — 2026-10-04, `2f0f3841`

A clean detached checkout at signed/DCO `2f0f3841e0c4bc8a8c1d2705c26692b3137d5177` packaged and independently verified manifest `target/rust-runtime-release-2f0f3841.json`. It binds source tree `506d69a070d89d877189cebd1d44eea67b072f5d`, `dirty: false`, dependency locks and toolchain. The macro archive has 9 entries, SHA-256 `9ca5dcb4dd2b6ab14a1e0243253f8069b8908ff7cf2859633abf0d42bfd08276`; runtime has 247 entries, SHA-256 `f6083e636932c7747034ce9422a97117ea25857bee4880072920ccc9394f96d0`. Using exact Nix `${HISTORICAL_NIX_STORE}/nsmhgkw6lbysbxipvdfy11m17q3f6yk4-compactc`, `check_archive_consumer.py` passed two independently generated Counter/Boolean Cell crates from the archives with one shared runtime. The earlier 95-call proof/ledger gate at this exact commit also passed. This is local package/consumer provenance; neither public registry publication nor same-commit remote CI is claimed. Current rust-backend-v2 scope requires the pinned branch and complete remote CI, not registry publication.
