---
id: RUST-ADR-0185
alias: ADR-0185
title: "Record typed stateful assertion expressions"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "assertion", "expression"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: d0464e9fcd25050bc72e02689ef7df3ae8148e80af0c1dc771e0365d7340467f
---
# RUST-ADR-0185 — Record typed stateful assertion expressions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Typed Unit, Sequence and Assert evaluation records two bounded read-only stateful assertion exports. Sequence steps require Unit and assertions fail before their result expression; the profile does not admit arbitrary stateful effects.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#289 closure](https://github.com/MediaNoxLabs/compact/issues/289#issuecomment-6017719656). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`8c6aa222`](https://github.com/MediaNoxLabs/compact/commit/8c6aa222511918af44292e96d4bef0ff9e7e8a92) · [`b433acec`](https://github.com/MediaNoxLabs/compact/commit/b433acec08f29c0d3e4571d6c25c8d642334396d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem and history
ADR0183 admitted native nested assertions but both stateful_assert_oracle exports still lack recording. ADR0162/0171 already provide typed witnesses, scoped short-circuit Boolean expressions and canonical Counter queries; ADR0182 added typed return control flow. Reuse these mechanisms without broadening unrelated admission profiles.

### Before / after
```compact
const gate = disclose(next_gate(1));
assert(disclose(selected) && gate && open.read() && disclose(next_gate(2)) && low.lessThan(high) && disclose(next_gate(3)), "nested stateful guard");
return high.read();
```
Before: native-only call and explicit recording gap. After:
```rust
let (frame, gate) = frame.try_witness_metered(/* typed witness */)?;
let (frame, accepted) = if selected && gate { /* selected queries and witnesses */ } else { (frame, false) };
if !accepted { return Err(CompactError::AssertionFailed(message)); }
let (frame, result) = high.record_read(frame)?;
```

### Decision / emitter and runtime
Add typed Unit, Sequence and Assert expression evaluation to Plan. Sequence steps must evaluate to Unit; Assert requires Boolean and fails before evaluating the result. A separate bounded read-only admission profile accepts Boolean parameters, Unit/Uint64 results, typed Boolean witnesses with Uint8 parameters, Boolean Cell reads, Counter reads/lessThan, scoped bindings and short-circuit Boolean If. Audit all branches and declared slot/signature types. No writes, general local calls, Kernel or coin operations. Reuse canonical runtime frame/query methods; no runtime changes, schema20/ABI46 unchanged.

### Validation and boundaries
Preserve unchanged source and all twelve independent TS cases, including each short-circuit failure and zero-gas ordering. Successful native/recorded calls compare exact VM/private/state/effects/querysum and replay gas. Failure calls compare assertion-versus-gas rejection and exact observable witness prefixes; Result::Err does not expose consumed context/gas, so do not claim inaccessible failed Rust gas parity. Prove/verify/ledger-apply both public APIs using pinned keys and shared unbalanced smoke policy. Add malformed type/scope/witness/query and unsupported effect guards. No original microDAO/Coracle behavioral closure claim, no funded fee claim, no remote CI or push.

### Evidence update
Issue https://github.com/MediaNoxLabs/compact/issues/289 (rust-backend-v2). Both recorded APIs pass pinned proof verification and ledger application; artifacts retained at ${LOCAL_EVIDENCE}/compact-adr185-proof and log ${LOCAL_EVIDENCE}/compact-adr185-proof.log. Twelve original TS cases retained, plus an independent per-query budget case that permits two reads but rejects costlier Counter.lessThan: native and recorded witness prefix is [1,2], with no witness3 or final read. Limits are per query in both runtimes; cumulative gas remains a separate reported measure. Successful calls compare complete verifying VM operations, private outputs, state/effects, query-summed gas and replay gas. Backend 9 library, 12 CLI and 149 renderer tests pass, including Unit sequence steps, Boolean assertion typing, wrong Cell/Counter declarations, wrong witness arity/result, lexical escape and extra ledger write refusal. Original microDAO still compiles and Cargo-checks; all seven proof-required recording gaps remain. ABI46/schema20 unchanged.


### Signed delivery
Signed conventional GPG+DCO commit 5ed14945. Strict Clippy passed for backend, fixture and proof harness. Exact-head focused receipt ${LOCAL_EVIDENCE}/compact-focused-5ed14945-stateful-assert/receipt.json passed: 1 fixture, 2/2 proof-required recorded exports. Frozen compiler ${LOCAL_EVIDENCE}/compact-adr185-compactc pairs with combined Scheme ${HISTORICAL_NIX_STORE}/1d5x83y7dgg6abzv0fzvna893na3nfjj-compactc-scheme-local-slice/bin/compactc-scheme. Proof artifacts/log retained as above. No push/remote CI.

### Current focused integration — 8c6aa222

Private schema20 / runtimeABI46 unchanged. Signed/DCO091bf091 (ADR182/#287) adds bounded typed effectful Field Cell return recording;8c6aa222 (ADR185/#289) adds typed read-only stateful assertion recording. Both preserve lexical scope, selected-branch behavior and witness/query order. Main GPG signatures verified.

- Backend9units+12CLI+149renderer tests pass; strict backend/proof-smoke Clippy and formatting pass.
- All165fixtures freshly checked:0updates/0failures (${LOCAL_EVIDENCE}/compact-integrated-adr185-refresh.log).
- Frozen five-source gate passes11/11recorded APIs: effectful returns, stateful assertions, rootLet, Counter comparison and welcome. Receipt ${LOCAL_EVIDENCE}/compact-focused-8c6aa222/receipt.json.
- Both effectful branches and both assertion APIs are proved, verified and ledger-applied on the integrated runtime under the shared unbalanced smoke policy. Logs ${LOCAL_EVIDENCE}/compact-integrated-adr182-proof.log and ${LOCAL_EVIDENCE}/compact-integrated-adr185-proof.log. The separate funded default-strict Set/Cell cases remain in the completed b433acec full checkpoint.
- Exact full-source inventory:208sources/729exports/187compiled;**337/360proof APIs available,23explicit gaps,369nonproof,0unassessed**;zero missing/unmatched rows and no baseline drift. Receipt ${LOCAL_EVIDENCE}/compact-8c6aa222-inventory.json. Remaining:7Kernel,7microDAO,4statefulStruct,4Coracle,1nativeZswap.
- The latest completed broad local gate is **b433acec**,349commands/165fixtures; these later two recording slices have focused integration evidence above.

Active ADR184 Kernel recording (ABI47 in isolation) and ADR186/#290 circuit-local witness eligibility. The latter fixes a reproduced composition bug outside existing corpus counts: adding an unrelated witness-using exported circuit currently removes recording from an unchanged Field rootLet/Counter profile. No broad readiness claim; no push/remote CI. User doc edit remains untouched.
