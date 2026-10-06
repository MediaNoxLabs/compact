---
id: RUST-ADR-0237
alias: ADR-0237
title: "Allow the extracted compiler cold-build budget"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-ci"
topics: ["CI", "extracted", "timeout"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: ebf5390d101aefd8f3d8361d52b80efd32546964005a5591839a937a69a8da7a
---
# RUST-ADR-0237 — Allow the extracted compiler cold-build budget

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-ci. Extracted-compiler cold-build job budget was raised from 45 to 180 minutes without changing its build and test commands. The prior timeout remains historical failure evidence; the later Linux static graph fix was still required.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#341 closure](https://github.com/MediaNoxLabs/compact/issues/341#issuecomment-6017809773). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c) · [`373b6aeb`](https://github.com/MediaNoxLabs/compact/commit/373b6aeb45df5664022fa6ef0c7b4933a5ab7529) · [`9a94248c`](https://github.com/MediaNoxLabs/compact/commit/9a94248cc0007866557dfb915048b995d13baf3c). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

<!-- compact-adr-final-acceptance:03c03a39a2d39c43c91143b9526ca1c2ffae706c:ADR-0237 -->
### Final remote acceptance — 2026-10-06

At published commit `03c03a39a2d39c43c91143b9526ca1c2ffae706c`, all 10 required remote workflows passed on the same revision; the root-reviewed acceptance record and closed rust-backend-v2 milestone cover this bounded decision. [Issue #341 closure record](https://github.com/MediaNoxLabs/compact/issues/341#issuecomment-6017809773) and the [milestone closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) document the verified scope.

[Final evidence bundle](references.md#private-note-06) · [Architecture successor](references.md#private-note-07)

Earlier status statements, observations, tests and limits below remain historical evidence. Final acceptance supersedes their pending wording and does not expand implementation scope.

### Historical decision and delivery record


Date: 2026-10-06
Status: accepted; implementation and replacement remote validation pending
Milestone: rust-backend-v2

### Problem and evidence

The exact published candidate `9a94248cc0007866557dfb915048b995d13baf3c` ran Compiler extracted workflow37418371075. Job112122334042 ran from05:25:53 to06:11:17UTC; GitHub explicitly annotated: “The job has exceeded the maximum execution time of 45m0s.” Compile was cancelled and Test E2E skipped. This is an actual job-budget failure, not a passing compiler test or an actor cancellation.

The sanitized Compile log shows progress through LLVM21, zlib, git and a last logged start of the ZKIRv3 binary build (05:55:38), with no compiler/test error before cancellation. Its SHA256 is `b0f90737a5e8b5c9be1c06a8a56aca8d2410fae5c4aa3160cbf144d86ef0b49b`. The cache push token is absent, so no private-cache write is assumed. No credentials are needed for the proposed fix.

The45-minute setting predates Rust in the2025 initial import. Both Linux workflows invoke the same default `nix build` graph. The main Compiler Build workflow already documents90+minutes for an uncached build and grants180minutes. The extracted workflow additionally needs dependency installation and its extracted E2E suite within its total job budget.

### Decision and before/after

Align only the extracted build job timeout to180minutes. Keep its build graph, test commands, failure semantics and artifact uploads intact.

Before:
```yaml
jobs:
  build:
    timeout-minutes: 45
```

After:
```yaml
jobs:
  build:
    # Match the main compiler job: an uncached default Nix build can take 90+ minutes.
    timeout-minutes: 180
```

### Emitter/runtime and consumer impact

None. The Scheme frontend, private IR20, Rust AST emitter, generated APIs, runtime ABI49, TypeScript runtime and dependency pins are unchanged. This is a CI execution budget change. Longer execution permits the existing gate to complete; it does not establish correctness on its own.

### Validation and delivery

Required focused checks: actionlint and an exact one-setting/one-comment diff confirming both `nix build` and `yarn test:extracted` remain. Commit must be conventional, GPG-signed and DCO-signed. The final published revision must pass the full required remote suite, including the extracted E2E tests, before milestone closure. Keep the failed45-minute run and skipped tests in the history.

The remaining9a compiler/debug jobs should finish sufficiently to reveal independent failures before replacing the candidate; do not discard their useful diagnostics merely to restart a timer.

Evidence: [failed run](https://github.com/MediaNoxLabs/compact/actions/runs/37418371075), `${LOCAL_EVIDENCE}/compact-extracted-timeout-assessment.md`, `${LOCAL_EVIDENCE}/compact-m2-remote-9a94248c/extracted-check-annotations.json`, sanitized `extracted-compile.log`.

### Published delivery

Issue: [#341](https://github.com/MediaNoxLabs/compact/issues/341). Conventional GPG/DCO commit [373b6aeb](https://github.com/MediaNoxLabs/compact/commit/373b6aeb45df5664022fa6ef0c7b4933a5ab7529) is published. Actionlint passes. Exact byte-diff verification confirms only the timeout setting and explanatory comment changed; the original final blank line and all build/test/artifact commands are preserved. The262 previously verified commits plus this immediate signed successor give263 initiative commits.

Replacement [extracted run37422770576](https://github.com/MediaNoxLabs/compact/actions/runs/37422770576) has started with180minutes. No successful extracted E2E result or milestone closure is yet claimed. The current240 issues remain open until their required final checks pass.
