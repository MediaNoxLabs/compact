---
id: RUST-ADR-0231
alias: ADR-0231
title: "Dispatch existing extracted compiler and CodeQL checks on candidate branches"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-ci"
topics: ["CI", "extracted", "CodeQL"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: cb21b6c03c7d07ef9b7654e3307dd0db2e48fe4d2b49752e1a0d799c599db56c
---
# RUST-ADR-0231 — Dispatch existing extracted compiler and CodeQL checks on candidate branches

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-ci. Dispatch triggers were added to existing extracted-compiler and CodeQL workflows without altering their jobs or security behavior. Final exact-head remote pass covers those workflows; historical pending sections remain delivery chronology.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#335 closure](https://github.com/MediaNoxLabs/compact/issues/335#issuecomment-6017798092). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c) · [`157d7033`](https://github.com/MediaNoxLabs/compact/commit/157d703312d8b37a097379c9ae437f4e4694cf17) · [`eb72a5ab`](https://github.com/MediaNoxLabs/compact/commit/eb72a5ab6085bf9b8564cc80c98c973e04825c2d). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

<!-- compact-adr-final-acceptance:03c03a39a2d39c43c91143b9526ca1c2ffae706c:ADR-0231 -->
### Final remote acceptance — 2026-10-06

At published commit `03c03a39a2d39c43c91143b9526ca1c2ffae706c`, all 10 required remote workflows passed on the same revision; the root-reviewed acceptance record and closed rust-backend-v2 milestone cover this bounded decision. [Issue #335 closure record](https://github.com/MediaNoxLabs/compact/issues/335#issuecomment-6017798092) and the [milestone closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) document the verified scope.

[Final evidence bundle](references.md#private-note-06) · [Architecture successor](references.md#private-note-07)

Earlier status statements, observations, tests and limits below remain historical evidence. Final acceptance supersedes their pending wording and does not expand implementation scope.

### Historical decision and delivery record


Date: 2026-10-06
Status: Accepted for implementation
Milestone: rust-backend-v2

### Problem and observed evidence

The published candidate `157d703312d8b37a097379c9ae437f4e4694cf17` has seven functional workflow dispatches but no pull request. Comparison to exact ledger-8 ancestor `eb72a5ab6085bf9b8564cc80c98c973e04825c2d` shows compiler passes/utilities and TypeScript runtime cryptography/descriptor/Zswap changes. The ordinary E2E command explicitly excludes the extracted compiler corpus; CodeQL is an independent JavaScript/TypeScript analyzer. Neither workflow can currently be manually dispatched at the candidate revision. This is an observed remote coverage gap; earlier local gates did not establish that these two suites had passed.

Evidence: `${LOCAL_EVIDENCE}/compact-remote-acceptance-plan.md`, `${LOCAL_EVIDENCE}/compact-remote-material-gates.json`, `${LOCAL_EVIDENCE}/compact-m2-remote-157d7033/initial-runs.json`.

### Decision

Add empty `workflow_dispatch` triggers to `.github/workflows/compiler-extracted.yml` and `.github/workflows/codeql.yml`. Preserve every existing PR, push, schedule and workflow_run trigger, job, permission, assertion and checkout behavior. The parent will dispatch each existing workflow at the final integrated branch revision and record the actual head SHA and result. No pull request to an unrelated main baseline is needed solely for these triggers.

### Before and after

Before, CodeQL accepts main push/PR and scheduled runs; Compiler extracted accepts PRs to main and the historical release workflow_run. A candidate branch push invokes neither.

After, both retain those events and additionally accept:

    on:
      workflow_dispatch:
      ... existing events unchanged ...

Example authorized follow-up (not executed by this delivery):

    gh workflow run compiler-extracted.yml -R MediaNoxLabs/compact --ref codex/rust-backend-ast
    gh workflow run codeql.yml -R MediaNoxLabs/compact --ref codex/rust-backend-ast

The run receipt must match the final integrated SHA; dispatch success alone is not test success.

### Ownership and compatibility

Workflow-only, two added mapping entries. No emitter, runtime, domain model, generated API, Rust ABI/schema, dependencies, suite selection or publication changes. Existing default checkout on workflow_dispatch resolves the selected branch/ref. Parent owns integration, push and remote execution. This isolated delivery performs no push or dispatch.

### Verification plan

Use actionlint for both workflows. Verify the complete file contents equal the base with exactly one dispatch entry added beneath `on:` in each file. Verify original triggers and all jobs remain byte-identical otherwise. Check changed paths and required license conventions. Sign conventional commit with GPG and DCO. Remote extracted/CodeQL results remain pending until the parent runs them.

### Boundaries

This does not claim Agda, fuzzer, published registry compatibility, foreign platforms or scorecard passed. No specification/Agda source changed in this initiative; those separate scopes stay explicit. It does not alter final full local product-gate evidence or conceal the now-discovered uncovered checks.


### Signed local delivery — 2026-10-06

Issue: https://github.com/MediaNoxLabs/compact/issues/335 (rust-backend-v2). Signed commit `39e4017d7e478666d4d5fff514c61eaa4b041251` from `157d703312d8b37a097379c9ae437f4e4694cf17`; two workflow files, two inserted entries. Actionlint 1.7.12 passed, exact-content invariant retained every pre-existing byte, diff check passed, good GPG signature and DCO verified, worktree clean. Receipt: `${LOCAL_EVIDENCE}/compact-adr231-receipt.json`. Remote extracted/CodeQL acceptance remains pending; no push or dispatch performed by this delivery. Parent owns integration and exact-final-head runs.
