---
id: RUST-ADR-0236
alias: ADR-0236
title: "Select an actual Intel macOS runner in installer CI"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-ci"
topics: ["CI", "macOS", "runner"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 2c871515bf8fcdd700e25b6484250cd38641ccf70bdeb6c9439a9636fbdd4f5b
---
# RUST-ADR-0236 — Select an actual Intel macOS runner in installer CI

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-ci. Installer CI selects an actual Intel macOS runner alongside ARM and Ubuntu. The final remote artifact confirms the runner architecture; no Windows or Linux ARM platform claim follows.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#340 closure](https://github.com/MediaNoxLabs/compact/issues/340#issuecomment-6017807984). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c) · [`157d7033`](https://github.com/MediaNoxLabs/compact/commit/157d703312d8b37a097379c9ae437f4e4694cf17) · [`ad4fc7c1`](https://github.com/MediaNoxLabs/compact/commit/ad4fc7c180546fe3a3c46591b48db463b536777d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0236 — Select an actual Intel macOS runner in installer CI
status: accepted for implementation
milestone: rust-backend-v2
head_before: ad4fc7c180546fe3a3c46591b48db463b536777d
```

## Historical decision and amendments

<!-- compact-adr-final-acceptance:03c03a39a2d39c43c91143b9526ca1c2ffae706c:ADR-0236 -->
### Final remote acceptance — 2026-10-06

At published commit `03c03a39a2d39c43c91143b9526ca1c2ffae706c`, all 10 required remote workflows passed on the same revision; the root-reviewed acceptance record and closed rust-backend-v2 milestone cover this bounded decision. [Issue #340 closure record](https://github.com/MediaNoxLabs/compact/issues/340#issuecomment-6017807984) and the [milestone closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) document the verified scope.

[Final evidence bundle](references.md#private-note-06) · [Architecture successor](references.md#private-note-07)

Earlier status statements, observations, tests and limits below remain historical evidence. Final acceptance supersedes their pending wording and does not expand implementation scope.

### Historical decision and delivery record


### Problem

`.github/workflows/compact-test.yml` currently selects `ubuntu-latest`, `macos-15`, and `macos-latest` for the installer test matrix. The remote run at `157d7033` retained installer-provenance artifacts named `installer-fixture-provenance-macos-15-ARM64` and `installer-fixture-provenance-macos-latest-ARM64`; both macOS rows actually ran on ARM64. Thus the matrix did not test the x86_64 macOS archive/installer path despite having two macOS entries. GitHub's current hosted-runner reference lists `macos-15-intel` as Intel and both `macos-15` and `macos-latest` as arm64 for standard runners: https://docs.github.com/en/actions/reference/runners/github-hosted-runners (public/private runner tables, read 2026-10-06).

### Before / after

```yaml
# Before: two macOS ARM64 jobs in the observed remote run.
os: [ubuntu-latest, macos-15, macos-latest]
# After: retain Linux x64 and macOS ARM64; explicitly add macOS Intel.
os: [ubuntu-latest, macos-15-intel, macos-latest]
```

Make exactly the one-line runner-label change. Do not add a runtime architecture check: the job already names its provenance artifact using `${{ matrix.os }}-${{ runner.arch }}` and the retained receipts identify archive target and architecture. The next remote run must confirm the new job reports `runner.arch=X64` with its own receipt. No Windows or Linux ARM acceptance is inferred.

### Compiler, emitter and runtime impact

None. No compiler target, TypeScript/Rust emitter, generated crate, runtime ABI/schema, fixture, test logic or dependency change. This is only a workflow platform-selection correction.

### Acceptance and limits

- ADR and MediaNoxLabs `rust-backend-v2` issue precede code.
- Workflow diff contains only `macos-15` → `macos-15-intel`; `ubuntu-latest` and `macos-latest` remain.
- Local `actionlint` succeeds and workflow matrix parse reflects exactly the three intended labels.
- A signed conventional DCO commit is delivered without push from this isolated branch. Local lint validates syntax; only a future remote run can establish actual Intel runner execution and installer results.

### History

Created before implementation after the first remote provenance review. Source evidence: `${LOCAL_EVIDENCE}/compact-m2-remote-157d7033/installer-provenance/` contains both macOS `ARM64` artifact folders. This ADR intentionally separates the selection fix from a platform pass claim.


### Local delivery — 2026-10-06

Conventional GPG-verified/DCO commit `cf16db69a8042b64145a108ea4f7359145722fe3` from base `ad4fc7c1` replaces only the workflow runner label `macos-15` with `macos-15-intel`. Issue [#340](https://github.com/MediaNoxLabs/compact/issues/340) is assigned to `rust-backend-v2`. The final matrix is `ubuntu-latest`, `macos-15-intel`, `macos-latest`; workflow tests, fixture sources, compiler/runtime and dependencies are unchanged. Local `actionlint` 1.7.12, exact matrix assertion, diff check, GPG, DCO and clean-worktree checks passed. Receipt: `${LOCAL_EVIDENCE}/compact-adr236-receipt.json` SHA256 `599674f43c4638eab08b34476b75450425ff7a2c3864db6767ba2282563c6704`.

The prior remote run established `macos-15-ARM64` and `macos-latest-ARM64` only. GitHub's hosted-runner reference lists `macos-15-intel` as Intel. Actual Intel execution and installer result remain pending the next remote workflow; the local label/lint check does not establish platform acceptance. No push or remote CI was triggered by this isolated commit.
