---
id: RUST-ADR-0189
alias: ADR-0189
title: "Relocatable Rust compiler distribution"
date: not-recorded
publication_date: 2026-10-07
decision_status: "accepted-distribution"
topics: ["packaging", "portable", "compiler"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e29e47c10486f56b6db7ad62506c767897290edf839cc3707aa6171be97e9d11
---
# RUST-ADR-0189 — Relocatable Rust compiler distribution

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-distribution. The relocatable binary carries runtime sources/macros, bundled tools and licenses and resolves them through extracted paths/symlinks. Local macOS ARM portable and offline-consumer checks passed; other platforms and final remote acceptance are evidenced separately.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#293 closure](https://github.com/MediaNoxLabs/compact/issues/293#issuecomment-6017726315). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`0c8e7293`](https://github.com/MediaNoxLabs/compact/commit/0c8e72935a77120a6f6f9ff4a06b7bff775d2b6e) · [`0f294223`](https://github.com/MediaNoxLabs/compact/commit/0f29422306a4c83ea18572afdd6eccbe7c5f5c05) · [`ee912835`](https://github.com/MediaNoxLabs/compact/commit/ee912835169d314851b99fc92c3e543ffaa2fd2e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
type: adr
status: accepted
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The normal Nix compactc package bundles Rust runtime sources, but compactc-binary-nixos and the portable compactc-binary derivation omit them. Runtime lookup then falls back to a vanished build checkout. The release workflow uses zip --junk-paths over a shallow glob, which cannot preserve runtime Cargo/source directories. Its shell launcher also assumes an unquoted direct executable path, whereas the compact installer creates symlinks.

### Before

```sh
nix build .#compactc-binary
zip --junk-paths compactc.zip result/**/**
# extracted compactc --target rust ... cannot find matching runtime sources
```

### Decision / after

Carry share/compactc/runtime-rs and runtime-rs-macros through both Nix distribution stages. Keep existing top-level archive executable names (compact installer and release tests expect them), but recursively preserve share/compactc. Resolve runtime sources next to an extracted executable as well as under its installation prefix. Make the launcher resolve symlinks and quote paths before adding local zkir helpers to PATH. Use a reusable archive builder from the release workflow, including optional notes, with collision checks and executable modes preserved.

```text
compactc.zip
  compactc, compactc.bin, compactc-scheme, format-compact, fixup-compact
  zkir, zkir-v3
  share/compactc/runtime-rs/{Cargo.toml,src/...}
  share/compactc/runtime-rs-macros/{Cargo.toml,src/...}
```

```sh
python3 tools/compact-rust-backend/build_compiler_archive.py --package result --output compactc.zip
unzip compactc.zip -d '${LOCAL_EVIDENCE}/relocated compiler'
'${LOCAL_EVIDENCE}/relocated compiler/compactc' --target rust --skip-zk contract.compact output
```

### Emitter/runtime changes

No generated Rust AST, runtime semantics, schema or ABI change. Only compiler runtime-source discovery and release packaging/launcher change. Existing explicit runtime override remains authoritative.

### Verification

Build the actual portable Nix distribution locally, create and extract its archive outside the checkout, unset runtime/Scheme overrides, run default TypeScript and strict Rust generation from an unrelated working directory, check generated Cargo offline and exercise installer-style symlinks and paths with spaces. Check archive required files and preserved modes; reject name collisions/missing inputs. Report platform limits honestly: local macOS ARM validation does not establish Linux/Intel portability. No remote CI or publishing.

### Status

Proposed before implementation. Issue and delivery evidence follow.


Issue: https://github.com/MediaNoxLabs/compact/issues/293 (rust-backend-v2). Implementation is local; portable Nix build and extracted archive verification are in progress.


### Implementation refinements discovered during local acceptance

- Real Nix output uses timestamps before the ZIP epoch; the archiver now clamps them to 1980 and has a regression test. The first archive attempt failed before publication; its log is retained at ${LOCAL_EVIDENCE}/compact-adr189-extracted-first.log.
- Runtime generation copied Cargo.toml, README and src but omitted crate LICENSE files. The same runtime source copy now includes LICENSE. Full extracted-vs-generated comparison will verify the expected bundle.
- The Nix CLI derivation now includes its crate manifests, README and src explicitly, so editing oracle captures or local Python harnesses no longer recompiles the release CLI.
- Five archive unit tests, twelve CLI tests, launcher symlink/spaced-argument probe, Clippy and formatting passed. A real extracted first package already compiles TypeScript and strict Rust with overrides unset; bundled ZKIR creates both counter verifier/prover keys. Final clean b324a960 artifact acceptance is still running.
- Intermediate rebuild ${LOCAL_EVIDENCE}/compact-adr189-clean-nix.log was stopped after the refinements; it is not release evidence. Final build log: ${LOCAL_EVIDENCE}/compact-adr189-final-nix.log.


### Accepted local delivery — fa8f8c7c

Clean signed/DCO commit fa8f8c7c builds the actual compactc-binary portable Nix derivation. The extracted macOS ARM archive passes default TypeScript generation, strict Rust generation, bundled ZKIR prover/verifier-key generation, exact runtime/source/license copying, and offline Cargo compilation. All run from an unrelated directory, with runtime/Scheme overrides unset, through paths containing spaces and a relative installer-style symlink. No cache clearing was needed after the timestamp fix.

Receipt: ${LOCAL_EVIDENCE}/compact-adr189-fresh-source-extracted/receipt.json. Archive: ${LOCAL_EVIDENCE}/compact-adr189-fresh-source-extracted/compactc.zip (SHA256 0383b59c5aed15c4ce8ffcd42d70c5a9186a8c797fc38668f320ba1c682c832f). Nix package: ${HISTORICAL_NIX_STORE}/i9i1hd4zj1c5fiw4k5cr9zlrnlxdf67p-compactc-binary-dist. Build log: ${LOCAL_EVIDENCE}/compact-adr189-fresh-source-nix.log. Five archive tests, thirteen CLI tests, strict CLI Clippy and formatting pass.

The original copied epoch timestamps reproduced a real stale Cargo runtime ABI mismatch on the shared warm target. Merely cleaning the target would hide it. Fresh byte writes fix it; a unit regression verifies new timestamps and exact bytes, and ${LOCAL_EVIDENCE}/compact-adr189-cargo-timestamps.log confirms rebuilding without cache deletion. Original failure ${LOCAL_EVIDENCE}/compact-adr189-extracted-final.log remains evidence.

Platform boundary: local aarch64-darwin validation only. Linux/Intel distribution checks and a final combined ABI48 release artifact remain separate. This isolated package uses ABI47/schema20 at its clean source commit; no claim that it includes later integrated188/190 behavior. No push, publish or remote CI.


### Corrective portability audit

The preceding execution checks passed on a machine with Nix installed, but `otool -L` exposed a retained Nix-store libiconv dependency in compactc.bin. The archive is therefore not yet a portable release candidate; its receipt has been narrowed to execution_checks_passed_linkage_gap_found. Extending the existing Darwin libiconv rewrite to the Rust command fixes the identified path. Linux Rust CLI builds must also use pkgsMusl to match portable Scheme linkage; actual Linux execution remains a separate gate. New local build/verification is in progress. Evidence: ${LOCAL_EVIDENCE}/compact-adr189-dynamic-libraries.log.


### Final portable linkage verification — 15b9829d

The corrective clean macOS ARM package now passes its Nix installCheckPhase: all six native executables must have no Nix-store dynamic library dependency. The extracted archive repeats the complete default TypeScript, strict Rust, bundled ZKIR key generation, exact runtime/source/license, installer-symlink/spaced-path and offline Cargo checks successfully. Receipt ${LOCAL_EVIDENCE}/compact-adr189-portable-final/receipt.json includes dynamic_libraries_no_nix=true. Executable dependency audit is preserved beside it. Build log ${LOCAL_EVIDENCE}/compact-adr189-linkage-guard-nix.log.

Main integration commits: 0c8e7293 (packaging/source timestamp fixes), 0f294223 (portable linkage and build guard). Linux now selects pkgsMusl, but no Linux/Intel execution is claimed. This isolated package is still ABI47/schema20. A new clean combined ABI48 package build at ee912835 is running in ${LOCAL_EVIDENCE}/compact-abi48-portable-nix.log alongside the main full local gate.

### Combined ABI48 portable validation — ee912835

The clean combined package now passes on aarch64-darwin, independently of the earlier isolated ABI47 package. Receipt `${LOCAL_EVIDENCE}/compact-abi48-portable-final/receipt.json`; archive `${LOCAL_EVIDENCE}/compact-abi48-portable-final/compactc.zip`; SHA256 `dfd11c0d4eb04bc5fbb01c195398854e39d522d57af32c992d3a263f753c26d4`. All six native executables pass the no-Nix dynamic-library check. Relocation with spaces, relative installer symlink, unrelated working directory, unset runtime/Scheme overrides, default TS and strict Rust generation, bundled ZKIR keys, exact runtime source copy and offline generated Cargo check pass. All 167 fixture captures match this packaged compiler; `${LOCAL_EVIDENCE}/compact-abi48-portable-fixtures.log`.

The full local suite's first 352 commands passed, then its final proof stage failed because the parent shell omitted ZKIR from PATH. Original failure retained. Only the final stage is resumed against identical source/compiler hashes with pinned packaged helpers; no full-pass claim yet. Signed follow-up `600d2d56` checks both helpers before full-gate work starts and is waiting outside the frozen main checkout. No push, publication or remote CI. Linux/Intel execution remains unverified.
