---
id: RUST-ADR-0051
alias: ADR-0051
title: "Select the matching versioned runtime at code generation"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler-cli", "diagnostics", "runtime-packaging"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 4d5929d5f70f1c74e1304522a3561aee03782d0e7b9533fb4bceaae3ee66bd97
---
# RUST-ADR-0051 — Select the matching versioned runtime at code generation

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept explicit exact-version registry-mode generation as an alternative to bundled and shared-source modes, with early mode validation and no generated edits required by the archive rehearsal. Macro/runtime registry publication was not delivered by this decision; package-copy reduction is not a compile/runtime speedup.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#150 closure](https://github.com/MediaNoxLabs/compact/issues/150#issuecomment-6017484356). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`6b65a491`](https://github.com/MediaNoxLabs/compact/commit/6b65a4915d6984665925971463195dbc4dd795a2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 51
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/150
```

## Historical decision and amendments

### Problem and evidence

A generated Rust contract defaults to a bundled local `runtime-rs` path dependency. A second generated contract has another runtime source path, and an application cannot use the signed runtime archives as versioned Cargo packages without editing each generated `Cargo.toml` and removing copied runtime trees. The clean rc3 archive consumer passed only after doing those disposable edits. This is a compiler packaging boundary, not a missing ledger-8 primitive or AST renderer feature. It violates the milestone exit expectation that generated output is usable without hand editing.

### Before and proposed after

Before, even when the caller intends to consume a released runtime:

```sh
compactc --target rust counter.compact out-counter
compactc --target rust cell_boolean.compact out-cell
# The application must edit both generated manifests to share a versioned runtime.
```

```toml
midnight-compact-runtime = { path = "runtime-rs", package = "midnight-compact-runtime" }
```

Proposed explicit mode:

```sh
compactc --target rust --rust-runtime-registry counter.compact out-counter
compactc --target rust --rust-runtime-registry cell_boolean.compact out-cell
```

```toml
midnight-compact-runtime = { version = "=0.1.0", package = "midnight-compact-runtime" }
```

The version must come from the compiler-distributed matching `runtime-rs/Cargo.toml`, not a caller-supplied loose range. The output contains no bundled runtime/macro source trees. The `ledger-transaction` feature and generated ABI assertion remain as before. This opt-in mode is useful for the existing exact archive source-replacement gate and, after publication, an ordinary registry consumer. It does not claim crates.io availability today.

### Decision proposal and alternatives

Add `--rust-runtime-registry` as a third, explicit dependency source: bundled by default, shared source via `--rust-runtime-root`, or exact versioned registry package. Reject registry plus root, duplicate registry flags, and registry mode without Rust target before invoking Scheme. Discover and validate the matching runtime manifest before running the frontend, then emit a Cargo manifest with an exact version and no local runtime copies. Keep the Rust target's ZKIR/proving behavior unchanged. A caller-supplied arbitrary version would allow a compiler/runtime mismatch and is deferred until compatibility metadata can make that safe. An implicit registry default would fail fresh builds while the runtime is unpublished.

### Ownership and compatibility

`tools/compact-rust-backend/src/bin/compactc.rs` owns target option parsing, runtime package metadata, manifest construction and source inclusion. `check_archive_consumer.py` owns the archive-only integration gate and should stop rewriting generated files. Typed schema-8 IR, `syn`/ `quote` renderer, generated API, runtime, proc macro, ledger-8/zk mapping and ABI 28 remain unchanged. The generated compile-time ABI assertion still rejects an incompatible published package. The exact Cargo version pins package identity; it is not a proof that the package is published or semantically compatible.

### Verification and limits

Run CLI selection/diagnostic tests; emit Counter and Boolean Cell independently from an exact-head packaged `compactc` with registry mode; assert each untouched manifest has the same exact package version and neither generated output includes `runtime-rs` or `runtime-rs-macros`; use the signed rc3 macro/runtime archives as Cargo directory source replacement to build one external consumer with both contracts and exactly one runtime/macro package; run both circuits and cross-crate type identity; retain wrong-hash/missing-archive rejection. Check output manifest hashes and no Rust-output leftovers after invalid options. Test bundled/shared-root regression. Record source/build cost only if it changes materially. Public registry publication, remote CI, release compatibility policy and production wallet admission remain separate milestone gates.

### History

- 2026-10-04: proposal after clean rc3 archive consumer proved versioned packages work, but only through disposable hand edits. Delivery evidence and issue link pending.

### Focused issue — 2026-10-04

[MediaNoxLabs/compact#150](https://github.com/MediaNoxLabs/compact/issues/150) tracks this decision in `rust-backend-v2`; parent #106 retains package publication and remote CI. The proposal was recorded before implementation.

### Accepted local implementation and verification — 2026-10-04

Conventional GPG-verified/DCO commit `6b65a4915d6984665925971463195dbc4dd795a2` implements the explicit `--rust-runtime-registry` mode in `compactc`. The CLI reads the compiler-distributed `runtime-rs/Cargo.toml` package name/version before Scheme runs, emits `midnight-compact-runtime = { version = "=0.1.0", package = "midnight-compact-runtime" }`, and leaves out copied runtime/macro source. The two existing modes remain: bundled default and `--rust-runtime-root` shared path. TS-only use, duplicate flag, flag value, and conflicting shared-root flag are rejected before output. The archive consumer now invokes the flag for Counter and Boolean Cell; it no longer edits generated manifests or deletes source trees. It checks the untouched dependency, absent copies, Cargo output-manifest hash/size, two executing circuits, a cross-crate runtime type assignment and one registry-style macro/runtime package in Cargo metadata. No IR schema, AST renderer, generated Rust circuit API, runtime, macro, ledger-8/zk primitive or ABI 28 changed.

Verification: seven focused `cargo test -p compact-rust-backend --bin compactc --offline` tests passed. Scoped `cargo fmt --check`, Python AST parse and `git diff --check` passed. Packaged `nix build .#compactc --no-link --print-out-paths` at the clean commit produced `${HISTORICAL_NIX_STORE}/72hq82g98kvdz0mffsx7s38nc3pjd4cc-compactc` (CLI 0.31.133). The clean signed annotated `rust-backend-v2-abi28-rc4` tag points at that commit; `git verify-tag`, GPG commit verification and clean worktree status passed. `check_release_packages.py --candidate-tag rust-backend-v2-abi28-rc4 --manifest target/rust-runtime-release-abi28-clean-rc4.json` and `--verify-manifest` passed with `dirty: false`, source tree `b1ad399b0e474ab4aa730fa7f57b345db7afde92`. Macro archive: 9 entries / 10,910 bytes / SHA-256 `819d094771b5d769b3cf1f60ad8f15563516170037fd02093991f7831c549faa`. Runtime archive: 247 entries / 168,915 bytes / SHA-256 `14f6ac3c644299d0a3dfe1db66dcdf8c62099824336e4e275fcf31c5649660b9`. The archives' changed hashes reflect Cargo's embedded source commit, not a runtime source change.

Exact-head `check_archive_consumer.py --manifest target/rust-runtime-release-abi28-clean-rc4.json --compiler ${HISTORICAL_NIX_STORE}/72hq82g98kvdz0mffsx7s38nc3pjd4cc-compactc/bin/compactc` passed against those archive bytes with no generated output modification. Bad macro hash and missing runtime archive were rejected before vendoring. The same packaged compiler in `nix develop .#compiler` compiled Counter in registry mode **without** `--skip-zk`; its compiler, contract, ZKIR and key directories, `increment.prover`/`increment.verifier`, exact Cargo dependency, absent source copies, and contract file hashes/sizes in `contract-manifest.json` were checked. Bundled and shared-root Counter modes still emit their expected paths; invalid registry combinations create no output.

For Counter, the generated contract directory is 3 files / 11,879 source bytes in registry mode versus 23 files / 257,869 bytes when runtime sources are bundled (20 KiB versus 304 KiB allocated on this filesystem). This is a package-copy reduction, not a Rust source-code or runtime performance optimization; compile-time impact was not measured. The first clean rc4 package verification took 4m07s in the runtime crate on this host, while `--verify-manifest` reused compiled dependencies. No branch/tag push, public registry package, unpatched crates.io consumer, remote CI or compatibility policy beyond the exact package pin and ABI assertion is claimed. Keep #150 and parent #106 open for those release gates.
