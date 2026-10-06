---
id: RUST-ADR-0221
alias: ADR-0221
title: "Exercise sampled oracle branches and nonempty collections"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["behavior-matrix", "branch", "collection"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b72ae95f19ccc49a526ac0d42b486379baa03ae0db91ef5dbc0e012a9d573f20
---
# RUST-ADR-0221 — Exercise sampled oracle branches and nonempty collections

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. Seven additional branch and nonempty-collection rows compare independent TypeScript with native/recorded Rust. The signed follow-up replaces initial shape-only VM checks with complete ordered public-program payload comparisons for affected stateful rows; the earlier capture receipt is historical.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#326 closure](https://github.com/MediaNoxLabs/compact/issues/326#issuecomment-6017782778). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

## Historical decision and amendments

Status: proposed before implementation, 2026-10-06. Milestone: rust-backend-v2. Source and generated code are unchanged. This is a test-only decision.

### Problem

The reviewed 37-source behavior matrix (review head 31136879; ${LOCAL_EVIDENCE}/compact-37-reviewed-behavior-report.md) found direct invocations but sampled paths for four existing fixtures. chunked_ledger.ping and struct_collision.runWrapBeta only use true; ternary streamCompareEq and streamStructMember only observe false; set_size check_set_empty and check_map_empty only observe empty collections. These are branch and data-state gaps, not missing export admission. ADR220 owns call_arg and AssetRegistry; this ADR leaves its reviewed matrix file untouched.

### Decision and before/after examples

Before: assert ping(true), runWrapBeta(true), and stream calls with the default false flag. After: capture and assert ping(false), runWrapBeta(false), and each stream call with flag=true in a separately seeded original state. For the collection circuits, seed one Field key in the Set and one key/value in the Map, then call the unchanged exports and assert both written Boolean flags are false. The original circuits contain no Compact assert on nonempty collections; expected assertion behavior means test assertions on false flags and no runtime error.

```rust
// Existing sampled branch:
assert_eq!(ping(true)?, ts_true);
// New original-export boundary:
assert_eq!(ping(false)?, ts_false);
let seeded = ledger_slots::s.insert(context, Field::from(42))?.context;
let seeded = ledger_slots::m.insert(seeded, Field::from(7), Field::from(9))?.context;
let checked = recorded::check_set_empty(seeded)?;
assert_eq!(ledger_slots::flag_set.inspect(checked.execution.context.query.state.get_ref())?, false);
```

The TypeScript side seeds equivalent public state with the pinned ledger-8 runtime, captures exact before/after state, result, query gas and public transcript. Rust compares native and recorded results, state/effects, exact ordered public program and replay where the export is stateful; pure exports compare direct independent TS results. Retain original inputs, source/generated/runtime provenance and any difference between reported last-query gas and full query sum. Set/map seeding is setup, not a claim that the original contract exports insertion circuits.

### Emitter and runtime impact

None. Reuse existing generated crates, runtime SetSlot/MapSlot operations and recording APIs. No schema, ABI, primitive mapping, gas rule or offer policy change. A genuine parity failure is a separate implementation slice with its own ADR and issue.

### Acceptance

Six new case rows across four source fixtures: chunked ping(false), struct collision runWrapBeta(false), ternary streamCompareEq(true), ternary streamStructMember(true), nonempty Set check, nonempty Map check. Pin corrected-TS capture hashes and direct Rust assertions. For the four stateful rows compare native, recorded and replay, exact VM operation order, query gas dimensions, effects, initial/final state and zero private outputs; for two pure rows compare direct results. Run focused fixture tests, formatting and strict Clippy locally. No proof is needed for this test-only slice; no remote CI or push. Send exact case IDs and capture/test hashes to ADR220 matrix owner without editing the shared reviewed matrix.


### Scope amendment before call-argument work (2026-10-06)

ADR220 owner requested adding the remaining sampled call_arg.impureInIfArm(false) branch to this test-only slice. This brings acceptance to seven rows across five original sources. The constructor sets flag=true; capture a separately seeded flag=false state, then require the untaken storeVec arm to make no write or private output. Compare exact original TypeScript result/state/effects/public program/query gas with native, recorded and replay Rust. The existing true branch remains covered by its historical capture. ADR220 continues to own the shared reviewed matrix; this ADR contributes its exact row evidence without editing that file. No emitter or runtime change is authorized by this amendment.

### Local delivery receipt (2026-10-06)

Signed conventional DCO commit: 6a65902fc6b99b75462df044c5a94d125d69ab53 (good GPG signature; clean isolated checkout). Seven rows across five unchanged original sources, including call_arg.impureInIfArm(false). Corrected TS runtime 0.16.101, Scheme compiler SHA256 41441b9b25459793fd55be7fa7e0e61b64c10070cab3a801243c9b3d909dd0d4. Five captures regenerate byte-identically and pin source/generated/runtime/compiler hashes. Native and recorded Rust assertions cover all stateful rows; replay, exact ordered public VM, gas, effects, before/after state and zero private outputs pass. Five focused fixture packages: 27 tests pass; strict Clippy, fmt and node syntax pass on signed head. No proof claim. Receipt with per-case hashes: ${LOCAL_EVIDENCE}/compact-adr221-signed-receipt.json. ADR220 matrix owner received exact seven-row delta; shared matrix unchanged in this branch.

### Full public-program parity follow-up (2026-10-06)

Review found that the first signed delivery compared ordered VM operation shapes rather than complete payloads. Signed DCO follow-up 38702dd942d8da209c86d9a56a3478389a3071bb adds full publicTranscript to three stateful captures (five rows), serializing TypeScript Uint8Array values as JSON arrays and comparing complete ordered operations directly with serde_json(recorded.public.verify_ops()). No mismatch or semantic normalization was needed. Signed-head five packages/27 tests, strict Clippy, fmt and byte-identical three capture reruns pass. Refreshed row hashes and provenance: ${LOCAL_EVIDENCE}/compact-adr221-full-program-receipt.json. The first receipt remains historical and is superseded for these three stateful captures.
