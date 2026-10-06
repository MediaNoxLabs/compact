---
id: RUST-ADR-0235
alias: ADR-0235
title: "Prepare locked dependencies for clean macOS Rust consumers"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-ci"
topics: ["CI", "macOS", "Cargo"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 11c37a56c06f25f4ed6e2667c8c8b89d9b4972c5a5029760a5c5b68e100720a3
---
# RUST-ADR-0235 — Prepare locked dependencies for clean macOS Rust consumers

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-ci. The macOS consumer workflow fetches locked dependencies before offline generated-crate compilation while retaining the gate commands. This is cache preparation, not proof that generated lockfiles equal the workspace lock.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#338 closure](https://github.com/MediaNoxLabs/compact/issues/338#issuecomment-6017804267). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c) · [`3f3fec01`](https://github.com/MediaNoxLabs/compact/commit/3f3fec0159c7e3479b1b48d8ba5b3fb89e130c44) · [`9a94248c`](https://github.com/MediaNoxLabs/compact/commit/9a94248cc0007866557dfb915048b995d13baf3c). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

<!-- compact-adr-final-acceptance:03c03a39a2d39c43c91143b9526ca1c2ffae706c:ADR-0235 -->
### Final remote acceptance — 2026-10-06

At published commit `03c03a39a2d39c43c91143b9526ca1c2ffae706c`, all 10 required remote workflows passed on the same revision; the root-reviewed acceptance record and closed rust-backend-v2 milestone cover this bounded decision. [Issue #338 closure record](https://github.com/MediaNoxLabs/compact/issues/338#issuecomment-6017804267) and the [milestone closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) document the verified scope.

[Final evidence bundle](references.md#private-note-06) · [Architecture successor](references.md#private-note-07)

Earlier status statements, observations, tests and limits below remain historical evidence. Final acceptance supersedes their pending wording and does not expand implementation scope.

### Historical decision and delivery record


Status: accepted for implementation. Date: 2026-10-06.

### Problem

Remote Apple Silicon compiler job112116555009 successfully builds compactc with Nix, but the generated consumer invokes Cargo with --offline before the host Cargo cache has midnight-base-crypto. Nix sandbox dependencies do not populate the host Cargo registry. Local acceptance used an already prepared cache and did not establish this clean-runner prerequisite.

### Before and after

Before: build compactc, then immediately run check_compactc_target.py --consumer.
After: cargo fetch --locked against the committed workspace lock, then build compactc and execute the same unchanged offline consumer gate.

```yaml
- name: Fetch locked generated-consumer dependencies
  run: cargo fetch --locked
```

This step belongs only to the macOS job, after the pinned Rust toolchain setup. The Linux job already performs its locked fetch and fixture resolver before the same gate. No compiler/emitter/runtime behavior changes. No offline test is removed or made online.

### Acceptance

Actionlint and an order/unchanged-command diff check locally, then the same macOS remote consumer gate on the final revision. Retain original failure: ${LOCAL_EVIDENCE}/compact-macos-compiler-112116555009.log and https://github.com/MediaNoxLabs/compact/actions/runs/37416625222/job/112116555009 . Final remote acceptance pending.

### Integrated delivery

Signed DCO commit `3f3fec01` is integrated in published candidate `9a94248cc0007866557dfb915048b995d13baf3c`. Actionlint1.7.12 and an exact diff/order invariant pass. Independent review found no blocker: the host locked fetch supplies the runtime dependency graph, including midnight-base-crypto1.0.0, before the offline consumer executes. This is cache preparation, not a claim that later generated Cargo lockfiles equal the workspace lock. Proof parameter setup remains separate. Final remote Compiler Build: https://github.com/MediaNoxLabs/compact/actions/runs/37418349719 (pending at this entry).
