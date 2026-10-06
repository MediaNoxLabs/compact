---
id: RUST-ADR-0182
alias: ADR-0182
title: "Record typed effectful return plans"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "ReturnPlan", "Cell"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: ccc8617eb6e78dd93878fe85c1cc6927184be3bfd7a100a678cad30610a0c349
---
# RUST-ADR-0182 — Record typed effectful return plans

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The typed recorder recursively lowers bounded effectful Field Cell returns, preserving prior effects, branch scope and a single condition evaluation. It is the separate recorded counterpart to ADR174, with explicit guards on unsupported compositions.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#287 closure](https://github.com/MediaNoxLabs/compact/issues/287#issuecomment-6017716481). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`8c6aa222`](https://github.com/MediaNoxLabs/compact/commit/8c6aa222511918af44292e96d4bef0ff9e7e8a92) · [`b433acec`](https://github.com/MediaNoxLabs/compact/commit/b433acec08f29c0d3e4571d6c25c8d642334396d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### History and problem
No prior ADR0182 or focused recording issue was found in the midnight vault/GitHub search. Preserve ADR0174/#278 native ReturnPlan and ADR0178/#282 scoped Field Cell recording history. Native effectful_return_oracle preserves pre-write value, condition timing, selected branch writes and witness calls; StateReturn::Effectful currently always refuses recording.

### Before / after
```compact
const before = state.read();
state.write(before + 1);
if (state.read() == 1) { state.write(disclose(next)); return before; }
else { unused.write(disclose(mark())); return before + 100; }
```
Before: correct native crate, recording unavailable. After: typed recording plan owns the complete ordered body and branch result.
```rust
let (frame, before) = state.record_read(frame)?;
let frame = state.record_write(frame, before + one)?;
let (frame, current) = state.record_read(frame)?;
let (frame, result) = if current == one {
    let frame = state.record_write(frame, next)?; (frame, before)
} else { /* selected witness and unused write */ (frame, before + hundred) };
```

### Emitter/runtime decision
Reuse typed_plan lexical bindings, typed values, witness metering and slot methods. Recursively lower ReturnPlan Value/Sequence/Let/Conditional; evaluate the condition once after prior effects, preserve branch-local scope, require exact matching return type and return frame/result together. Separate bounded Field-Cell effectful profile from existing ADR178 single-slot rootLet profile. Field params/results/literals/add/equality, typed Field Cell reads/writes and declared Field witnesses only; unsupported operations/types/path escapes and nonempty separate actions reject. Top-level Effectful recorder only admitted after full plan audit. ABI46/schema20 unchanged, no new runtime API.

### Evidence and bounds
Unchanged oracle source and independent fresh TS capture. Exact native/recorded values,state,effects,querysumgas,privateoutputs,VMprogram,replay. Both branches pinnedZKIR proofverification+ledgerapplication; negative scope/type/path/ownership guards. Preserve TSlastqueryaggregate discrepancy rather than hide it. Coracle/full-domain recording remains outside bounded profile. Targeted tests,freshness,Clippy,signedDCO localdelivery; no push/remoteCI.


### Implementation evidence
The original else-branch offset is an Unsigned local with maximum 100 followed by an explicit FieldCast. Preserve this exact type and checked small-unsigned conversion; do not relabel it as a Field literal. Recursive ReturnPlan lowering returns the selected frame and typed result; existing ADR178 admission remains separate. Independent fresh TS/native/recorded/replay checks pass both branches, with no witness in the chosen branch and exactly one in the other branch. Both proofs verified and ledger-applied using persistent ${LOCAL_EVIDENCE}/compact-adr182-proof artifacts and ${LOCAL_EVIDENCE}/compact-adr182-proof.log. This uses the shared unbalanced smoke policy and makes no funded-fee claim. Issue: https://github.com/MediaNoxLabs/compact/issues/287.


### Validation results
Backend: 9 library tests, 12 CLI tests, 148 renderer tests passed. Focused malformed-IR guards additionally reject missing/wrong Cell declarations and indices, parameter/result mismatch, branch/action local escape, witness arity/result mismatch, mixed branch return types, and unsupported action even in a statically untaken branch. Existing ADR178 root-Let admission tests passed unchanged. Source positive cohort confirms 1 proof-required export and strict recording admission; focused fixture freshness passes. ABI46 and schema20 unchanged.


### Signed delivery
Commit 4e9cc2c75199ffb8360f17acfb471b66de34e38e is conventional, GPG verified and DCO signed, with an explanatory body and ADR/issue/test references. Strict Clippy passed for backend, generated fixture and proof harness; formatting passed. Persistent proof artifacts: ${LOCAL_EVIDENCE}/compact-adr182-proof. Frozen compiler: ${LOCAL_EVIDENCE}/compact-adr182-compactc; combined Scheme ${HISTORICAL_NIX_STORE}/1d5x83y7dgg6abzv0fzvna893na3nfjj-compactc-scheme-local-slice/bin/compactc-scheme. No push or remote CI.


Exact-head focused receipt passed: ${LOCAL_EVIDENCE}/compact-focused-4e9cc2c7-effectful-return/receipt.json, 1 fixture and 1/1 recorded proof-required exports. Source cohort: ${LOCAL_EVIDENCE}/compact-adr182-source-scope.json.

### Current focused integration — 8c6aa222

Private schema20 / runtimeABI46 unchanged. Signed/DCO091bf091 (ADR182/#287) adds bounded typed effectful Field Cell return recording;8c6aa222 (ADR185/#289) adds typed read-only stateful assertion recording. Both preserve lexical scope, selected-branch behavior and witness/query order. Main GPG signatures verified.

- Backend9units+12CLI+149renderer tests pass; strict backend/proof-smoke Clippy and formatting pass.
- All165fixtures freshly checked:0updates/0failures (${LOCAL_EVIDENCE}/compact-integrated-adr185-refresh.log).
- Frozen five-source gate passes11/11recorded APIs: effectful returns, stateful assertions, rootLet, Counter comparison and welcome. Receipt ${LOCAL_EVIDENCE}/compact-focused-8c6aa222/receipt.json.
- Both effectful branches and both assertion APIs are proved, verified and ledger-applied on the integrated runtime under the shared unbalanced smoke policy. Logs ${LOCAL_EVIDENCE}/compact-integrated-adr182-proof.log and ${LOCAL_EVIDENCE}/compact-integrated-adr185-proof.log. The separate funded default-strict Set/Cell cases remain in the completed b433acec full checkpoint.
- Exact full-source inventory:208sources/729exports/187compiled;**337/360proof APIs available,23explicit gaps,369nonproof,0unassessed**;zero missing/unmatched rows and no baseline drift. Receipt ${LOCAL_EVIDENCE}/compact-8c6aa222-inventory.json. Remaining:7Kernel,7microDAO,4statefulStruct,4Coracle,1nativeZswap.
- The latest completed broad local gate is **b433acec**,349commands/165fixtures; these later two recording slices have focused integration evidence above.

Active ADR184 Kernel recording (ABI47 in isolation) and ADR186/#290 circuit-local witness eligibility. The latter fixes a reproduced composition bug outside existing corpus counts: adding an unrelated witness-using exported circuit currently removes recording from an unchanged Field rootLet/Counter profile. No broad readiness claim; no push/remote CI. User doc edit remains untouched.
