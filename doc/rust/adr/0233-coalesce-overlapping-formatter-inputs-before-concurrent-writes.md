---
id: RUST-ADR-0233
alias: ADR-0233
title: "Coalesce overlapping formatter inputs before concurrent writes"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["formatter", "concurrency", "paths"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: dc5bd0cb75256deaf13ecd6226b4bf8a56eb1676a04e1100210854b959d508bc
---
# RUST-ADR-0233 — Coalesce overlapping formatter inputs before concurrent writes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Formatter inputs are canonicalized and overlapping aliases grouped before concurrent writes, with one execution per canonical file and per-input reporting retained. Distinct hard links and external concurrent processes are outside this decision.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#337 closure](https://github.com/MediaNoxLabs/compact/issues/337#issuecomment-6017801853). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c) · [`bb86008c`](https://github.com/MediaNoxLabs/compact/commit/bb86008cfdaff8c1cd93af0a5c5071efbbbbb0d2). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

<!-- compact-adr-final-acceptance:03c03a39a2d39c43c91143b9526ca1c2ffae706c:ADR-0233 -->
### Final remote acceptance — 2026-10-06

At published commit `03c03a39a2d39c43c91143b9526ca1c2ffae706c`, all 10 required remote workflows passed on the same revision; the root-reviewed acceptance record and closed rust-backend-v2 milestone cover this bounded decision. [Issue #337 closure record](https://github.com/MediaNoxLabs/compact/issues/337#issuecomment-6017801853) and the [milestone closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) document the verified scope.

[Final evidence bundle](references.md#private-note-06) · [Architecture successor](references.md#private-note-07)

Earlier status statements, observations, tests and limits below remain historical evidence. Final acceptance supersedes their pending wording and does not expand implementation scope.

### Historical decision and delivery record


Status: accepted for implementation. Date: 2026-10-06.

### Problem

Remote installer job112116902692 at157d7033 failed the existing duplicate-file scenario:166/167 tests passed. Two concurrent format-compact processes targeted the same input/output path. One reported formatted and the other failed. ARM local scheduling had hidden this race. Preserve the original assertion that each supplied occurrence reports its formatting result.

### Before

```rust
for path in inputs {
    join_set.spawn(async move { format_file(&bin, check_mode, path).await });
}
```

### Decision and after

Resolve existing inputs to canonical paths, group aliases and directory overlaps before spawning, execute the formatter once per canonical file, and report that result for each original input path. Distinct files remain concurrent. Failed canonicalization retains the original path so existing failures remain failures. This covers repeated, relative, and symlink paths; distinct hard-link names and concurrent external processes are outside this bounded fix.

```rust
for paths in grouped_inputs {
    // One formatter process, reports for every supplied occurrence.
    spawn_format_group(paths);
}
```

### Emitter/runtime changes

None. This is the compact installer CLI formatter coordinator. Preserve format/check modes, errors, exit status, and duplicate verbose reporting. Add deterministic process-invocation/overlap regression coverage and rerun the real archived formatter scenarios, strict package Clippy, then remote Linux/macOS installer gates.

### Evidence

Initial failure: ${LOCAL_EVIDENCE}/compact-linux-installer-112116902692.log and https://github.com/MediaNoxLabs/compact/actions/runs/37416628750/job/112116902692 . Implementation and final validation pending. No gate is waived.


### Signed local delivery — 2026-10-06

Signed conventional GPG/DCO commit **54700aec056721a9dbe165d7cd84d5cc1e5df17f**, based on bb86008c, groups every expanded input by its existing canonical identity before any formatter is spawned. Each group executes using the canonical key; failed canonicalization retains the original path. The original paths are retained solely for per-occurrence reporting. Thus repeated, relative, symlink and directory/file overlaps format once and still emit both existing `formatted` reports. Distinct canonical files remain parallel. No compiler/runtime, dependency or old assertion changes.

Three deterministic Unix fake-formatter tests check exact process count across overlapping paths (symlink supplied first), a two-file start barrier proving distinct-file concurrency, repeated check-mode diff without mutation, and repeated formatter failure reporting. Existing real archived formatter tests run separately and unchanged. The exact signed-head command passes **34 tests**: format 8, overlap 3, scenarios_four 10 and scenarios_three 13. This includes real sc34. Strict compact all-target/all-feature Clippy, rustfmt and diff checks pass on Rust 1.99.0 / macOS ARM. Receipt `${LOCAL_EVIDENCE}/compact-adr233-delivery-receipt.json`, SHA-256 `5ae68778e70501920661f096e24cf29a6eecbdf6e448c98e9e6ad816de3a04f1`; command/log/source hashes are included. The root-loaned warm target is released after delivery.

The original Linux 166/167 failure remains at `${LOCAL_EVIDENCE}/compact-linux-installer-112116902692.log`; remote validation of the fix remains root-owned. Existing hard-link names, external writers and filesystem replacement during execution remain outside this bounded coordination guarantee. Initial candidate used the first original alias to execute; review refined it to the canonical key before this final signed-head verification. Earlier candidate hashes are superseded, not delivery heads.
