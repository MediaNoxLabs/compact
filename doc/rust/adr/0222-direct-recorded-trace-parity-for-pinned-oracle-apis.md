---
id: RUST-ADR-0222
alias: ADR-0222
title: "Direct recorded trace parity for pinned oracle APIs"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["recording", "trace", "oracle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 34c0222c754f845f04191426a85681cc182b5e916e3505b8605864b443a50008
---
# RUST-ADR-0222 — Direct recorded trace parity for pinned oracle APIs

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. Nine direct cases compare full public Verify programs, private outputs, gas, state and replay for Map, nested Map, witness and Set APIs. They add no new proof/ledger claim, and Set membership-present is outside the empty-source checks.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#325 closure](https://github.com/MediaNoxLabs/compact/issues/325#issuecomment-6017781065). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`15074e81`](https://github.com/MediaNoxLabs/compact/commit/15074e81a283dda1e05518ec0df8e804fe0a9387) · [`412f2099`](https://github.com/MediaNoxLabs/compact/commit/412f20990fdadfa6c58b0ab6f5777ff87e95b300). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: accepted bounded test-only delivery, 2026-10-06. Milestone rust-backend-v2.

### Problem

The manual37-source review found that map.put, nested_map.ping and witnesses.pull have direct native TypeScript result/state checks but lack direct exported-circuit complete recorded trace/gas/replay comparisons. Set.check tests recording for7 but only native behavior for8. Slot-level tests do not establish exported recording behavior.

### Before / after

Before: `put(context, 7, 9)` matches a captured state, while the complete `recorded::put` program is not checked there. `pull` checks one private witness output only on the native path.

After: execute the same unedited original exports through native and recorded generated APIs, comparing independent TypeScript full public program, exact private aligned outputs, summed query gas, final state and upstream replay. Exercise insert/replace/distinct-key Map paths, repeat nested ping, witness42 and zero, Set check7/check8. Chain contexts where the TS case is sequential. Results remain unit; no proof/ledger application claim.

### Changes

New independent capture script and JSON, focused Rust tests for four existing packages, reviewed evidence links. Compact source, generated crate, emitter, runtime, schema20 and ABI49 unchanged. Reuse common test-only trace comparisons where doing so reduces duplicated assertions; keep each export invocation explicit. Node capture checks pinned corrected runtime and original source/generated hashes. Coordinate durable37-source matrix with ADR220 owner.

### Validation

Four fixture package tests and strict Clippy; four fresh unchanged fixtures; independent capture repeated byte-for-byte; source provenance check. No full gate per test-only slice. Final frozen-head full acceptance incorporates the suite. No remote CI/push.


### ADR222 delivered and ADR220 integrated — 412f2099 (2026-10-06)

Signed/DCO15074e81 delivers ADR222/#325:9 direct independently captured TS/native/recorded cases over Map insert/replace/distinct, nested-map ping/repeat, witness pull42/0, Set check7/check8. All10 package tests, strict4packageClippy,37source-links,4freshunchangedfixtures and4/4requiredAPI focused gate pass. Full public Verify program/private aligned outputs/query-gas sums/state/replay compared; independent review found no actionable issue. Receipt:${LOCAL_EVIDENCE}/compact-adr222-delivery-receipt.json and ${LOCAL_EVIDENCE}/compact-15074e81-integration-receipt.json. No new proof/ledger claim; Set membership-present is not claimed from its empty-source checks.

Signed/DCO412f2099 integrates ADR220 sourcea57e7ee8:30direct pure cases and independent AssetRegistry close success/repeat refusal. Its source receipt records29tests,Clippy,freshness/capture checks:${LOCAL_EVIDENCE}/compact-adr220-delivery-receipt.json. The durable37-source203-row review matrix is now in the repository; its hashes need the reviewed ADR221/222 deltas before the combined root matrix gate. Do not treat this pending root integration as already passed. Source availability remains386/386 atd1411fe5; latestfull/portable3404c30c. No remoteCI/push.
