---
id: RUST-ADR-0238
alias: ADR-0238
title: "Native build tools for the static Linux CLI"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-ci"
topics: ["CI", "Linux", "static"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 002fa36c1b95d893f063a57ad444829875c82c5c1b67298823b49125c9263c58
---
# RUST-ADR-0238 — Native build tools for the static Linux CLI

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-ci. Linux CLI packaging selects the static toolchain graph and verifies the resulting ELF has no dynamic interpreter or needed libraries; Darwin and Scheme package selections remain separate. Evaluation alone was not a build pass, so the final remote acceptance is the relevant outcome.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#342 closure](https://github.com/MediaNoxLabs/compact/issues/342#issuecomment-6017811306). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c) · [`373b6aeb`](https://github.com/MediaNoxLabs/compact/commit/373b6aeb45df5664022fa6ef0c7b4933a5ab7529) · [`5671d65d`](https://github.com/MediaNoxLabs/compact/commit/5671d65d82c539653d812b8fdafa16033e61c814). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

<!-- compact-adr-final-acceptance:03c03a39a2d39c43c91143b9526ca1c2ffae706c:ADR-0238 -->
### Final remote acceptance — 2026-10-06

At published commit `03c03a39a2d39c43c91143b9526ca1c2ffae706c`, all 10 required remote workflows passed on the same revision; the root-reviewed acceptance record and closed rust-backend-v2 milestone cover this bounded decision. [Issue #342 closure record](https://github.com/MediaNoxLabs/compact/issues/342#issuecomment-6017811306) and the [milestone closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) document the verified scope.

[Final evidence bundle](references.md#private-note-06) · [Architecture successor](references.md#private-note-07)

Earlier status statements, observations, tests and limits below remain historical evidence. Final acceptance supersedes their pending wording and does not expand implementation scope.

### Historical decision and delivery record


Date: 2026-10-06
Status: accepted; implementation and final remote validation pending
Milestone: rust-backend-v2

### Problem and terminal evidence

At published373b6aeb, extracted workflow37422770576 exceeded180 minutes. Its annotation explicitly reports the maximum execution time of3h0m0s; Compile was cancelled and E2E skipped. Main compiler workflow37423089905 likewise ended cancelled after its180-minute Build compiler step, with all following compiler/native/package/E2E gates skipped. No passing Linux source test result is inferred.

Both logs identify `${HISTORICAL_NIX_STORE}/ivkfvd8j8yn1yx12f9vik84p7hh99f59-rustc-1.92.0.drv` as the last started derivation. Read-only evaluation proves the generated CLI depends on that exact derivation through its Rust wrapper. Its flags set both build and host to x86_64-unknown-linux-musl, with a musl-native LLVM/bootstrap dependency graph. This is an avoidable build-tool choice in `compact-rust-cli`, not an identified contract or runtime failure.

The earlier45→180-minute adjustment in ADR237 was insufficient. A larger budget alone would retain this avoidable bootstrap. This decision instead corrects package-set ownership while preserving static distribution.

### Decision and before/after

Use `pkgs.pkgsStatic` for only the Linux Rust CLI derivation, retaining ordinary `pkgs` on Darwin. The separate `compactc-binary-nixos` Scheme packaging selection remains unchanged. Keep the current180-minute job budgets and all gate commands.

Before:
```nix
linking-pkgs = if pkgs.lib.hasSuffix "linux" system then pkgs.pkgsMusl else pkgs;
```

After:
```nix
linking-pkgs = if pkgs.lib.hasSuffix "linux" system then pkgs.pkgsStatic else pkgs;
```

Evaluated x86_64-linux and aarch64-linux static sets use GNU/Linux build tools and a musl host target with `isStatic=true`. Their cross Rust compiler uses native GNU build/host and a musl target; target musl `crt-static` remains enabled. Plain `pkgsCross.musl64` and its AArch64 equivalent do not assert `isStatic=true`, so they are not selected.

Add a Linux-only post-fixup check of the actual generated ELF using native readelf. Both program-header and dynamic-section reads must succeed; reject PT_INTERP or DT_NEEDED. Permit static PIE to have a dynamic section without a shared-library dependency. This checks portability after stripping/fixups rather than inferring it from a package-set name.

### Emitter/runtime and consumer impact

No change to Scheme analysis, private IR20, Rust AST rendering, generated contract APIs, runtime ABI49, TypeScript semantics or dependency pins. The Linux CLI still targets static musl; Darwin keeps its existing derivation. Rust1.99.0 remains the validated consumer/CI toolchain. Nix's internal CLI compiler version is a separate build detail.

### Alternatives and limits

A360-minute budget is the smallest scheduling-only alternative, but would keep the proven unnecessary native-musl Rust bootstrap. Trusted Nix closure reuse could reduce repeated cold work; a cache/producer redesign is outside this narrow repair. No new external secrets or cache trust changes are required.

Static cross Rust and target-standard-library derivations still exist. Evaluation establishes graph ownership, not binary-cache availability, build duration or successful execution. Remote Linux builds and ELF checks must pass. No timing guarantee is claimed.

### Validation and delivery requirements

1. Evaluate both supported Linux architectures and Darwin; confirm the exact failed musl-native Rust/LLVM paths are absent from the static CLI graph and the Darwin derivation is unchanged.
2. Review the bounded flake diff and post-fixup failure behavior, including static PIE semantics.
3. Run a focused derivation/build check where available and retain actual output evidence; do not pretend evaluation is a Linux build.
4. Publish one conventional, GPG-signed, DCO commit and run all required workflows at that exact revision. Required compiler, proof, package and E2E commands remain unchanged.
5. Preserve the373 failures. Leave the still-running debug job intact until its useful result is available; dispatch the new debug validation afterward. Final closure requires all final-revision checks, not the historical debug result.

Evidence: [extracted timeout](https://github.com/MediaNoxLabs/compact/actions/runs/37422770576), [compiler timeout](https://github.com/MediaNoxLabs/compact/actions/runs/37423089905), `${LOCAL_EVIDENCE}/compact-musl-cli-recursive.json`, `${LOCAL_EVIDENCE}/compact-m2-remote-373b6aeb/extracted-compile-test.log`, `${LOCAL_EVIDENCE}/compact-m2-remote-373b6aeb/compiler-build-linux-terminal-receipt.json`. Sanitized evidence and final links will be preserved with the milestone closeout.


### Published delivery and focused checks

Issue[#342](https://github.com/MediaNoxLabs/compact/issues/342), conventional GPG/DCO commit[5671d65d](https://github.com/MediaNoxLabs/compact/commit/5671d65d82c539653d812b8fdafa16033e61c814), published on codex/rust-backend-ast. The initiative now has264 audited commits and241 milestone issues.

Actual patched graph review passes for both Linux architectures and omits the exact failed native-musl Rust/LLVM paths. Darwin CLI derivations are byte-identical to baseline: the Linux postFixup attribute is omitted entirely via optionalAttrs, not added as an empty attribute. Native readelf and grep paths are absolute, and inspection errors fail closed.

The exact extracted postFixup body was tested in an isolated cached Linux container with networking disabled, read-only root and a temporary filesystem. Real static/static-PIE ELF passed; dynamic interpreter, shared-library dependency without interpreter, malformed/missing ELF, and grep-error cases failed as required (7 expected outcomes). Only tool paths and architecture label were mapped to the container. This checks the guard; it is not a build of the final Nix Compact output.

Receipts: `${LOCAL_EVIDENCE}/compact-adr238-graph-review.json` SHA256b9348d1e80c9e44f92add8d5d31015dcdae417a060df32ac52692cd6b15c24b6; `${LOCAL_EVIDENCE}/compact-adr238-elf-guard-receipt.json` SHA25645a5ae360e6395249c10928c8d3d3c1d619a7778fe5461f48f42f0b36ed6c4a9; `${LOCAL_EVIDENCE}/compact-adr238-receipt.json`.

Replacement[Compiler Build37443664998](https://github.com/MediaNoxLabs/compact/actions/runs/37443664998) and[extracted37443676255](https://github.com/MediaNoxLabs/compact/actions/runs/37443676255) are active. Nine required workflows are dispatched; new debug validation is deferred until the old373 debug run yields a terminal result. All ten must eventually pass at5671d65d before closure.


#### Debug replacement sequencing update

After both completed373timeout logs and exact CLI derivation analysis established the native-musl build-tool cause, retaining the old still-compiling debug job no longer provided a reason to delay fixed validation. Its in-progress Compile contract state was captured before intentional concurrency supersession. This revises the earlier plan to wait for its natural terminal result. It must be described as superseded, not as a debug test failure or six-hour timeout.

The final[debug run37444224530](https://github.com/MediaNoxLabs/compact/actions/runs/37444224530) now targets5671d65d. All ten required workflows have been dispatched at the new published revision. Actual results and closure remain pending. Snapshot: `${LOCAL_EVIDENCE}/compact-m2-remote-5671d65d/superseded-373-debug-snapshot.json`.
