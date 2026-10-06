---
id: RUST-ADR-0052
alias: ADR-0052
title: "Publish Rust compiler output only after complete generation"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler-cli", "diagnostics", "output-publication"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 8adb42123bea695cc1bd2eb576eb3e5347b6fbe119d60f1568f1c9948dd2ad5d
---
# RUST-ADR-0052 — Publish Rust compiler output only after complete generation

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept staged complete Rust/joint output publication that protects existing output on handled failures. ADR0077 later adds locking and crash-recovery hardening without replacing the original decision. Preserve the limits on concurrent old tools, recovery timing and filesystem/power-loss atomicity.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#151 closure](https://github.com/MediaNoxLabs/compact/issues/151#issuecomment-6017486273). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`e4984cb1`](https://github.com/MediaNoxLabs/compact/commit/e4984cb1c34e6af0c9481aea291d9c771fbd632f) · [`fa2e4490`](https://github.com/MediaNoxLabs/compact/commit/fa2e4490b810e6274c5b362cea29e63ca71318d0). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 52
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/151
```

## Historical decision and amendments

### Problem and reproduced evidence

The packaged ABI-28 `compactc --target rust --skip-zk` runs the Scheme frontend directly in the requested output directory, then renders `contract/lib.rs`, copies runtime sources, writes `Cargo.toml`, and refreshes `compiler/contract-manifest.json`. If the runtime source lookup fails after rendering, the command exits 1 but leaves `contract/lib.rs`, `contract/compact-rust-ir.json`, and compiler metadata without a Cargo manifest. This is a partially generated library at the path a consumer may trust. The same sequence can overwrite pieces of a previously valid output before a later failure. It weakens #104's rejection boundary and #103's buildable-artifact contract.

Reproducer, using a packaged `compactc` and a missing source override:

```sh
COMPACT_RUST_RUNTIME_DIR=/missing compactc --target rust --skip-zk counter.compact out
# exit 1; out/contract/lib.rs exists, out/contract/Cargo.toml does not
```

### Before and proposed after

Before, `run()` passes the final path to Scheme and writes each artifact into it as it goes:

```rust
Command::new(compiler).args(&args).status()?;
fs::write(output.join("contract/lib.rs"), source_code)?;
copy_runtime_sources(&output.join("contract"))?;
refresh_manifest(&output)?;
```

After, create a unique sibling staging directory on the same filesystem, pass that path to Scheme, complete all Rust files and the refreshed output manifest there, then rename the complete directory into place:

```rust
let staging = StagedOutput::new(&output)?;
compile_and_render(staging.path())?;
refresh_manifest(staging.path())?;
staging.publish(&output)?;
```

On a failed compile, remove only the tool-owned staging directory. A previously existing output must remain byte-for-byte unchanged. For an existing output, move it to a unique sibling backup only after staging is complete; restore it if publication fails, and remove the backup after a successful replacement. Refuse a file or symlink as the output root rather than moving it unexpectedly. TypeScript-only invocations retain the Scheme command's existing behavior; a joint `--target ts --target rust` invocation stages the entire output together.

### Ownership and alternatives

`tools/compact-rust-backend/src/bin/compactc.rs` owns the output transaction. The Scheme frontend, typed schema-8 IR, AST emitter, generated API, runtime, macro, ledger-8/zk primitives and ABI 28 do not change. A simple preflight for the runtime directory would fix the repro but leave render errors, copy errors and manifest failures able to publish partial output. Writing files in place and deleting on error would destroy a previously valid output. A sibling stage plus rename provides a clear success boundary with ordinary filesystem operations. The two-rename replacement of an existing directory is not crash-atomic on every filesystem; the design preserves the previous output for handled errors and documents that limit.

### Verification and risks

Add a packaged CLI regression that reproduces the invalid-runtime failure with no final output and no leaked stage, plus a valid output followed by the same failure whose full file tree and hashes remain unchanged. Test successful recompilation into an existing output and a joint TypeScript/Rust target. Retain source-located unsupported-source probes and check no partial `lib.rs`/`Cargo.toml`. Run packaged compiler/consumer and a complete ZKIR/key artifact with refreshed manifest, then verify the release archive/consumer gates are unaffected. Avoid following an output symlink or deleting user-owned files outside the requested output. Record exact conventional GPG/DCO commit, tests, output behavior and residual crash/concurrency limits in this ADR, its focused issue and the milestone map. No public API or ABI bump is expected.

### History

- 2026-10-04: proposal after packaged rc4 direct repro. The temporary output showed `lib.rs=true`, `Cargo.toml=false`, private IR and compiler manifest present after exit 1. Delivery pending.

### Focused issue — 2026-10-04

[MediaNoxLabs/compact#151](https://github.com/MediaNoxLabs/compact/issues/151) tracks this decision in `rust-backend-v2`, with parent #103/#104. The proposal and repro were recorded before implementation.


### Delivery amendment — 2026-10-04

Local conventional GPG-signed/DCO commit `fa2e4490b810e6274c5b362cea29e63ca71318d0` implements the staged-output design. `compactc` now gives Scheme a unique sibling directory, renders/copies the Rust crate and refreshes the compiler manifest there, then publishes only after those steps succeed. A failed generation removes its stage. A failed rebuild preserves every byte of an existing output. A successful rebuild replaces it and removes the temporary backup. File and symlink output roots are rejected. This applies to Rust-only and joint TypeScript/Rust targets; no emitter, runtime, schema-8 IR, generated API or ABI-28 change was needed.

The packaged CLI regression passed: missing runtime source leaves no fresh output or stage, preserves an existing tree and hashes on failed rebuild, successful replacement removes a sentinel, and file/symlink roots fail safely. Seven `compactc` unit tests, scoped formatting and Python syntax checks passed. A clean detached checkout at this exact commit produced GPG-verified `rust-backend-v2-abi28-rc5`, reproducible 9-entry macro and 247-entry runtime archives, and `target/rust-runtime-release-abi28-clean-rc5.json` with `dirty: false`. Macro SHA-256: `32cc04a62b1d945ec035fa2b93053310cec244c85f244856374510ce5b225d72`; runtime SHA-256: `8c8e86f97b4943e9e12ab1191fe52536ae60b7fd1d71518be99618138b2fe232`. Exact-head Nix `compactc`: `${HISTORICAL_NIX_STORE}/j98pm7f45991af6p3pknzyxiirnjvqlz-compactc`. Release manifest generation and re-verification, untouched Counter+Boolean Cell archive-only external consumer, invalid archive rejection, and the full packaged `--consumer --proof` gate passed. The latter generated ZKIR/keys and replayed, proved, independently verified, ledger-validated and applied the current offline cases; Counter wallet/deploy handoff files were emitted.

The two-rename replacement is recoverable for handled errors but cannot promise crash atomicity or safe concurrent writers on every filesystem. The branch and rc5 tag remain local. Same-head remote CI, registry publication and production wallet policy remain open; [#151](https://github.com/MediaNoxLabs/compact/issues/151) remains open for those release gates.


### Superseding hardening amendment — 2026-10-05

[ADR-0077 — Serialize Rust output publication and recover interrupted replacement](0077-serialize-rust-output-publication-and-recover-interrupted-replacement.md)/[#176](https://github.com/MediaNoxLabs/compact/issues/176) adds a cross-process output lock and next-invocation recovery for the two-rename crash window at signed/DCO `e4984cb1`. ADR-0052 remains the original staged-publication decision; its handled-error and complete-output guarantees still apply. The new hardening narrows its concurrency/recovery limit without claiming power-loss atomicity or deleting stale user-visible backups.
