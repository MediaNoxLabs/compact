---
id: RUST-ADR-0186
alias: ADR-0186
title: "Circuit-local witness eligibility"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "witness", "eligibility"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: c9671c496f43ea5ca8a2ff3a64a59b3f7e6a05cb134fa8b2c097c7cde0549ac9
---
# RUST-ADR-0186 — Circuit-local witness eligibility

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Recording eligibility follows actual lowered WitnessCall use across branches rather than unrelated contract declarations. Zero-witness profile guards remain, and hidden or untaken witness nodes still reject.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#290 closure](https://github.com/MediaNoxLabs/compact/issues/290#issuecomment-6017721200). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1dbf0023`](https://github.com/MediaNoxLabs/compact/commit/1dbf0023284a95937e604b4f528f5279b794fe6f) · [`3fa3b0ba`](https://github.com/MediaNoxLabs/compact/commit/3fa3b0ba87400f9ebe0f31264637dac4e8ff16b6) · [`b433acec`](https://github.com/MediaNoxLabs/compact/commit/b433acec08f29c0d3e4571d6c25c8d642334396d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
type: adr
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

Recording eligibility for an unchanged root Field Cell circuit depends on unrelated exported circuits. At b433acec (schema20/runtimeABI46), `root_let_action_return_oracle.step` has recorded/observed APIs. Adding the declarations below leaves `step` semantically unchanged but makes both APIs unavailable:

```compact
witness unrelated_value(): Field;
export circuit unrelated(): Field {
  return disclose(unrelated_value());
}
```

An unused witness alone is eliminated by the frontend and does not reproduce the problem. A separate exported circuit retains the witness in the contract-wide declaration map, triggering `witnesses.is_empty()` in the typed recording admission. The Counter comparison entry uses a similar global condition and must be audited.

### Decision / scope

ADR0186 in the midnight vault: eligibility must depend on actual witness use in the lowered circuit plan. Preserve existing no-witness profile boundaries; do not silently admit witness-bearing Field Cell or Counter compositions. Reuse the typed planner and runtime without a new ABI/schema unless implementation evidence requires it.

### Before / after

Before, adding `unrelated()` removes `contract.recording.step_call(...)` from an otherwise unchanged consumer.

After, `step` retains the same recorded/observed API and semantics; `unrelated` remains native/nonproof according to authoritative compiler metadata. Actual witness use in profiles not designed for it must continue to be rejected.

### Acceptance

- Source-backed regression for unrelated retained witnesses alongside Field root-Let and Counter comparison circuits.
- Strict recording compilation respects proof applicability: unrelated native/nonproof circuits do not invalidate supported proof APIs.
- Native/recorded state, result, transcript and query gas unchanged for the original calls.
- Negative tests prove actual unsupported witness use still fails; malformed scope/type checks stay intact.
- Compiler metadata/capability joins and generated-crate Cargo checks, targeted tests/Clippy, exact local receipt.
- Conventional GPG+DCO commit; no push or remote CI.

Reproduced artifacts: `${LOCAL_EVIDENCE}/compact-unrelated-witness-root-let.compact` and `${LOCAL_EVIDENCE}/compact-unrelated-witness-root-let/contract`. Linked prior deliveries: #282, #275; in progress typed assertion #289. This closes a compositional capability defect; it is not a claim of complete language parity.

### Implementation direction

Track actual witness calls while lowering the complete typed plan, including audited branch bodies. Remove the contract-wide declaration-map emptiness checks only where equivalent zero-usage admission can be enforced after lowering. Do not erase witnesses or alter the circuit source to force eligibility. This protects consumer APIs against adding an unrelated exported circuit.

### Delivery

Pending implementation and local evidence.


### Implementation evidence
Plan now counts actual lowered WitnessCall nodes, including both audited conditional branches. Field root-Let and Counter comparison profiles require zero such uses after lowering; unrelated declarations no longer veto either profile. Runtime/schema/ABI unchanged. Regression composes each unchanged source with a retained unrelated witness/export in temporary generated crates, using the supported shared runtime root option. Strict compiler proof metadata joins remain authoritative; direct generated-consumer comparisons check original/composed native and recorded state/results/programs/gas. Existing source fixtures remain unchanged, so no redundant declaration-denominator entry is added. Focused tests separately reject actual Field-write witness use, Counter threshold witness use and witness use in an untaken branch.


### Regression results
Previous ADR185 compiler rejects the composed Field source on step (captured ${LOCAL_EVIDENCE}/compact-adr186-pre-fix.log). New source gate --witness-composition preserves one Field-root and four Counter proof APIs while the retained unrelated export stays proof-not-applicable. Both temporary generated consumers Cargo-test and compare original/composed native/recorded results, state, VM operations, gas and replay; Counter comparison values/short-circuit branches/assertion failure covered. Source hashes logged in ${LOCAL_EVIDENCE}/compact-adr186-source.log. Backend 10 library +12 CLI +149 renderer tests pass; direct renderer negatives confirm actual Field/Counter/untaken-branch witness use is still rejected. Both original checked-in fixtures remain fresh and unchanged.


### Signed delivery receipt
Commit ebfa999d is conventional, GPG verified and DCO signed with explanatory body and ADR/issue/test references. Strict Clippy/formatting passed. Exact-head local receipt ${LOCAL_EVIDENCE}/compact-focused-ebfa999d-witness-composition/receipt.json: 2 unchanged fixtures, 5/5 recorded proof-required APIs. Source composition tests separately verify retained unrelated witnesses and authoritative proof-not-applicable metadata. Source regression log ${LOCAL_EVIDENCE}/compact-adr186-source.log, previous-compiler rejection ${LOCAL_EVIDENCE}/compact-adr186-pre-fix.log. Frozen compiler ${LOCAL_EVIDENCE}/compact-adr186-compactc and combined Scheme1d5. No new runtime, schema, ABI or fixture inventory entry; no push or remote CI.


### Main integration checkpoint

Signed 1dbf0023, verified together with ABI 47 at 3fa3b0ba. Source composition checks pass for unchanged Field root-Let and Counter circuits alongside another exported witness-using circuit. Unsupported actual witness use, including untaken branches, remains rejected. ${LOCAL_EVIDENCE}/compact-integrated-abi47-composition.log.
