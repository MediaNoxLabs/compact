---
id: RUST-ADR-0261
alias: ADR-0261
source_sha256: 6c6ef8f3643b42223ba7f56d67721b792b563279d283a6df053b56f93514ba9e
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0261 — Make Rust compatibility and runtime selection explicit

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted
parent: R030-16
```


## ADR0261 — Make Rust compatibility and runtime selection explicit

Read-only inspection at `012fbb44f8c4df3c3dc78ee97b209476e4541db8` on 2026-10-07. This is a vault-ready proposal, not an implemented change or publication decision. Repository source and vault untouched. Parent reports actual Rust 1.88 all-feature checks and 306 backend/macros/testkit tests; this research ran no additional builds and does not extend those results to other targets or unlocked future dependencies.

### Recommended decision

Keep milestone **0.3.0** as the engineering acceptance milestone. Do not mechanically set the compiler, language, npm runtime, Cargo packages, ABI and schemas to 0.3.0. Retain the existing numbers until a reviewed change actually crosses that specific compatibility boundary. Publish one machine-readable compatibility/source matrix, validate selected runtime roots against compiler-owned expectations before emission, and make mismatch diagnostics state required and selected identities. Do not silently upgrade or rewrite sources.

A candidate remains source-distributed: a matching bundled runtime/macro pair, or an explicitly validated shared source root. Registry manifests are an explicit consumer option, not proof that the required crates exist or were published. Tagging, registry publication, release uploads and their external acceptance remain separate authorized actions.

### Current independent version fields

| Boundary | Current value | Authority / code reference |
|---|---|---|
| Engineering milestone | 0.3.0|Vault `Milestone 0.3.0/backlog.md:273` (R030-16)|
| Installer/workspace package line | 0.5.1|`Cargo.toml:23` workspace package metadata; unrelated to contract runtime|
| Public Compact compiler | 0.31.133|`compiler/compiler-version.ss:23`; `flake.nix:266` repeats it|
| Compact language | 0.23.105|`compiler/language-version.ss:22`|
| TS runtime | 0.16.101|`runtime/package.json:3`; `compiler/runtime-version.ss:23` reads that package at frontend build|
| Rust backend/CLI Cargo package | 0.1.0|`tools/compact-rust-backend/Cargo.toml:18`; Nix CLI derivation `flake.nix:225`|
| Rust runtime and macros | Both 0.1.0|`runtime-rs/Cargo.toml:18`, `runtime-rs-macros/Cargo.toml:18`; runtime exact macro dependency `=0.1.0` at runtime manifest32|
| ContractLab package | 0.1.0, publish=false|`testkit-rs/Cargo.toml:18–26`; exact runtime version with local path|
| Generated consumer package | 0.1.0, edition2024, publish=false|`src/bin/compactc.rs:196–212`; no explicit `rust-version`|
| Rust runtime ABI | 50 |`runtime-rs/src/lib.rs:60`; emitter expectation `tools/compact-rust-backend/src/lib.rs:17`|
| Private frontend/backend IR | 20 |`compiler/rust-ir-passes.ss:2371`; `tools/compact-rust-backend/src/ir.rs:25`; renderer rejects any other value at `lib.rs:3083–3085`|
| Public capability report | 3 |`capabilities.rs:25`; schema 2 is internal unclassified draft, finalized to3 after authoritative frontend proof flags at61–108|
| Compiler artifact manifest | 1 |Observed retained generated `compiler/contract-manifest.json`; includes compiler/language/TS-runtime versions and file hashes|
| Package rehearsal manifest | 1 |`check_release_packages.py:93–112`; source commit/tree/dirty, package hashes, locks, toolchain and `registry_publication:false`|
| ContractLab snapshot format | 1 |`testkit-rs/src/snapshot.rs:25,34–75`; validates source/generated identities, runtime ABI, ledger and state mode; caller-supplied hashes are not authenticated provenance|
| Rust ledger baseline |ledger-8.0.3|`runtime-rs/src/lib.rs:62`, exact ledger/zk package graph at `runtime-rs/Cargo.toml:33–53`|
| Declared native Rust MSRV | 1.88, edition2024|runtime/macros/backend/testkit manifests each line20; measured check/test toolchain 1.88 is separate from primary toolchain 1.99|

Use source commit + content hashes + lock identities to identify unpublished0.1.0 snapshots. ABI 50 describes generated-source/runtime compatibility, not a stable binary ABI, ledger consensus version, package SemVer promise or proof readiness. Capability schema 3 describes the report shape; whether a circuit is available must still be read from the actual report and its proof flags.

### Actual distribution and feature behavior

1. `compactc` defaults to TypeScript; all current version-query flags delegate to Scheme. `--runtime-version` therefore means TS runtime, even when Rust targets are selected (`compactc.rs:620–649`; README417–420).
2. Default Rust output copies **both** runtime and macro Cargo manifests, README/LICENSE and complete source trees (`compactc.rs:341–350`), emits a relative path runtime dependency and exposes the `ledger-transaction` forwarding feature (`196–249`). This makes generated crates standalone source artifacts; the copied manifests retain original crate versions/MSRV/pinned dependencies.
3. `--rust-runtime-root` canonicalizes a caller-selected root and checks that each of two Cargo manifests exists (`253–265`). It then uses an absolute runtime path. It does not currently validate package identity, ABI, macro compatibility, ledger pins or MSRV.
4. Default bundle/registry source discovery prioritizes `COMPACT_RUST_RUNTIME_DIR`, then installed `share/compactc`, then build-time checkout fallback (`267–291`). Registry mode reads only the selected runtime package name and nonempty version (`294–309`), then emits an exact `=version` dependency. Its “matching compiler version” promise relies on supplied source correctness.
5. Nix packages copy runtime and macro sources beside the public CLI (`flake.nix:239–242,322–327`); Linux static-ELF packaging is a separate portability check. Nix CLI derivation `doCheck=false` is deliberate: compiler-backed acceptance is elsewhere (`235–238`), not evidence that tests were removed from the overall workflow.
6. Package rehearsal is local and explicitly not publication. It verifies unpacked runtime against a local macro patch and records that fact (`check_release_packages.py:163–182`). Testkit is not part of this two-package release rehearsal. Do not silently add it to distribution.

| Consumer profile | Permitted claim / dependencies | Not implied |
|---|---|---|
| Generated Rust default |Native primitives, actual onchain VM and upstream types; runtime default has no optional full ledger transaction package|Not a crypto-free/minimal-dependency runtime: circuits/proofs/zswap crates are still direct dependencies|
| `ledger-transaction` |Adds exact midnight-ledger 8.0.3 and rand0.8.5; available recorded/observed/preparation APIs follow actual capability rows|Not universal proof readiness, network availability, or compatibility with another ledger graph|
| ContractLab |Plain-ledger owned native/replay snapshots, typed trusted adapters and witness model; source/state boundaries explicitly checked|Not proof, network, durable secret storage or a sandbox against arbitrary transient callback mutations|
| Experimental WASM |Only the exact Node/browser subset, engine/target/import profile and artifact identities in ADR0256 receipt|Not a published supported target, WASI compatibility, full transaction runtime or browser proving promise|

### Concrete gaps from static code (not speculative failures)

#### A. Selected runtime source mismatch is diagnosed late

`shared_runtime_path` accepts two manifest files without checking their identity. `runtime_package_version` checks only runtime name/version, not compiler ABI expectation or the macro pair. Generated modules later emit bare const assertions (`lib.rs:3841,3891`). These protect numeric ABI equality but do not provide a concise expected/selected root and migration instruction. Same unpublished Cargo version can contain ABI 49 and ABI 50. A macro mismatch may instead surface as a missing derive/helper item before the bare assertion is useful.

**Bounded remedy:** one cohesive compiler-side compatibility module with compiled expected metadata and selected-root inspection, used by bundled/shared/registry modes. Validate package names, exact macro pairing, ABI and ledger identity plus declared MSRV. Fail before replacing output with a message such as “this compiler requires runtime ABI 50; selected root reports49; regenerate with its matching compiler or choose this compiler’s runtime root.” Keep the compile-time ABI assertion as defense against later dependency replacement. Selected metadata is a developer error check, not a security attestation against a malicious source tree.

Selected-root compatibility should use a small shipped metadata file whose contents are checked against source constants/manifests in package tests. Do not evaluate arbitrary Rust source or run user build scripts merely to discover an ABI. Compile the compiler's expected record into the CLI, validate the source package's record, and retain the generated ABI assertion against the actual linked runtime. Older roots lacking the record need an explicit diagnostic and migration instruction; do not pretend they were verified from a matching 0.1.0 label alone.

#### B. Generated consumer metadata omits MSRV

`crate_manifest` emits edition2024 and publish=false but no rust-version. Runtime and macro manifests declare1.88, so Cargo can still reject an older compiler through dependencies; the consumer manifest does not itself communicate the supported baseline. Actual1.88 evidence now exists, so add the explicit matching generated-package MSRV under an accepted metadata change. Do not infer that every historical generated artifact or every future unlocked dependency resolution has been tested.

#### C. No compact Rust-specific version query / generated compatibility summary

The CLI delegates existing runtime/version flags to Scheme, and the generated manifest currently reports TS runtime version plus hashes. Runtime ABI is buried in generated source, not a compact developer-facing compatibility object. Capability schema 3 does not report ABI, package versions, ledger or MSRV.

**Bounded remedy:** add a separate versioned Rust compatibility object/file (e.g. `compiler/rust-compatibility.json`) with schema 1, compiler/language/TS-runtime labels, Rust package versions, required ABI, IR/capability schema, ledger baseline, MSRV, selected distribution mode and runtime/macro content identities. Hash it in the existing contract manifest. A dedicated CLI JSON query may share the same module. Keeping this separate avoids gratuitously changing capability schema 3. Define unknown schema rejection explicitly; this file is not a public generic IR.

#### D. Release rehearsal has a hard-coded pair and incomplete compatibility metadata

`check_release_packages.py:80` requires macro version exactly `=0.1.0`. Correct today, but a later authorized package bump must update that literal separately or it will reject valid new pairs. Its manifest records package versions/locks but not ABI/IR/capability/MSRV fields. Derive the expected exact macro version from the authoritative source pair and record compatibility metadata, retaining the explicit local macro patch and no-publication marker. Do not broaden package set or add release actions as part of this fix.

#### E. Active prose still describes ABI 49

`runtime-rs/README.md:12` says current ABI 49. `tools/compact-rust-backend/README.md:414` likewise labels the current generated/runtime boundary ABI 49; earlier capability prose at131 calls it version 1. Implementation is ABI 50/capability 3. The milestone backlog at279 also preserves the older ABI 49 baseline. These are current-vs-history presentation gaps; retain historical ABI descriptions and receipts, append/update the current banner/matrix when the accepted metadata slice is delivered. Under the vault-first plan, do not rewrite historical ADR snapshots.

#### F. Adoption ledger/profile compatibility remains a distinct acceptance item

R030-16 explicitly calls out DID’s declared ledger-v8 8.1.0 versus Rust ledger 8.0.3 (`backlog.md:286`). The checked-in DID source manifest freezes two Compact files, not the entire original JS package/lock graph. Branch TS0.31.133/runtime 0.16.101 comparisons do not resolve that package-profile question or substitute for original-release TS0.31.1/runtime 0.16.0 comparison. Preserve separate profile rows, immutable source/import/lock hashes, and state which semantics/exports were actually exercised. Do not upgrade the Rust ledger graph to8.1 or ledger 9 merely to align a version label. No security or cross-instance experiment is part of this proposal.

### Compatibility and migration rules to accept

- **ABI:** increment when generated code requires a new runtime/macro contract or incompatible representation/behavior. Do not bump for test-only work, private refactoring or an additive consumer API unused by generated output. ABI mismatch requires a matching runtime selection or regeneration; it is not silently coerced.
- **IR:** private, exact frontend/backend schema agreement. New incompatible lowering representation requires an IR schema change and matched frontend/backend delivery, not a public migration parser. Keep stale IR failure before emission.
- **Capabilities:** preserve schema 3 unless its published format/semantics actually change; keep nonproof “not_applicable” distinct from recording gaps. New admitted domains are reported per actual export, not inferred from a version number.
- **Package SemVer:** unchanged0.1.0 for current unpublished source snapshots is not a promise that arbitrary0.1.0 trees interchange. Before any registry publication, choose an immutable release pair, rehearse it, pin exact versions and test an ordinary external registry consumer. Never republish different bytes under the same public version. A public breaking API must get an explicit pre-1.0 version decision and migration notes; not an automatic milestone-number mapping.
- **MSRV:**1.88 is the current declared native / independently tested minimum for the measured locked features/packages.1.99 remains the primary validation toolchain. Raising the minimum is an explicit compatibility decision with consumer tests and generated metadata changes; changing Rust build tooling alone does not change the runtime ABI.
- **Deprecation:** identify affected generated/public paths before replacing/removing APIs. Offer narrowly useful adapters/deprecations only where semantics remain exact; do not hide incompatible state/offer policies behind automatic conversions. Regenerate reviewed consumer fixtures and retain before/after failures with exact source identities.
- **Source baseline:** immutable revision + full relevant source/import hashes and lock identities. Keep user-selected DID v0.7.0 source frozen. “Latest” checks may report drift for review but must not replace it silently. Repeat drift check before RC, recording whether an update was accepted or deferred.

### Smallest implementable next slice / tests

Own a new backend compatibility-metadata module, surgical CLI manifest/root validation hooks, release-rehearsal metadata inspection, and focused tests. Runtime semantics, package versions, ABI 50, IR 20, capability 3 and publication configuration stay unchanged unless a concrete gap requires a new decision.

1. Valid bundled/shared roots with same ABI 50 emit the same semantic generated code and correct package metadata; paths/selection metadata may differ as expected.
2. Old ABI 49 vs new compiler 50 fails with explicit required/selected versions before output replacement. Old/new **compatible** ABI 50 consumer fixtures both compile with the validated root; this is distinct from accepting ABI 49.
3. Wrong runtime/macro package names, missing macro, differing exact macro version, ledger 9/mixed pinned metadata and malformed compatibility file refuse clearly. Keep explicit developer override selection visible; no implicit fallback to another root after mismatch.
4. Registry mode emits exact compiler-matching pair metadata; no local path leaks. Test generation locally; reserve actual registry-consumer acceptance for a separately published version.
5. Generated MSRV and runtime/macros/testkit declarations agree. Existing parent1.88 all-feature tests provide initial evidence; pin their exact receipt in the matrix. Confirm primary1.99 still passes relevant consumer checks.
6. Unknown compatibility schema and stale private IR fail clearly. Capability schema 2 drafts never masquerade as finalized schema 3; preserve current atomic proof-flag join tests.
7. Package rehearsal reflects source manifests rather than a hard-coded0.1.0 macro string, preserves archive verification and no-publication status, and records exact compatibility/source identity.

All changes above require the next accepted ADR/issue. No implementation, build, source edit, release/tag action or external mutation was performed for this research.

### Accepted implementation scope

Implement the bounded next slice above with compiler-owned metadata and source-record consistency tests. Milestone version, compiler/language labels, Rust package versions, ABI50, IR20, capability3 and ledger8.0.3 remain separate. This delivers metadata and developer diagnostics, not cryptographic authentication or release publication. Obsidian remains planning/documentation authority; record current prose corrections in vault now and publish consolidated developer docs at closeout. Avoid partial replacement of pre-existing generated output when preflight fails. No ABI change is expected because no new generated runtime symbol is introduced.

### Local delivery

`987b56b11be85ffa69a16aa24cbcb2e4dd4fb6ac`. 24focusedRusttests,5Python tests, strictClippy, actual two-package rehearsal, installed/bundled/shared/oldABI50consumer checks pass. ActualRust1.85 refusal confirms generated1.88declaration; earlier1.88qualification has its own scope. Root review reproduced inline-table panic and installed fallback; both corrected with regressions. New generated compiler/rust-compatibility.json fingerprints source and is hashed by artifact manifest. Package versions/ABI/IR/capabilityschema unchanged. Registry generation is not publication verification. ParentR030-16 remains open for profile and final release qualification. [ADR0261 — Runtime compatibility selection local receipt](references-0.3.0.md#note-027).
