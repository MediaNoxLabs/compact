---
id: RUST-ADR-0196
alias: ADR-0196
title: "Preserve native terminal lexical return scope"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler", "lexical-scope", "return"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 9a95081ef2771dbf1b3c90a7255768576c5ed3d17e8ad9d689eb5c2b17267aa8
---
# RUST-ADR-0196 — Preserve native terminal lexical return scope

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Native terminal Sequence/Let continuation retains lexical bindings without leaking earlier siblings or branch locals. It changes native lowering only; recorded return support remains a separate decision.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#301 closure](https://github.com/MediaNoxLabs/compact/issues/301#issuecomment-6017739398). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`15999efb`](https://github.com/MediaNoxLabs/compact/commit/15999efb7df1a3138720897433fa0aada62beac7) · [`ee912835`](https://github.com/MediaNoxLabs/compact/commit/ee912835169d314851b99fc92c3e543ffaa2fd2e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem and evidence
A two-binding Compact circuit compiles for TypeScript but native Rust fails with `unknown parameter "after"`. Review evidence is `${LOCAL_EVIDENCE}/compact-composition-review/root_two_bindings.compact`, its TS/Rust logs, and `results.json`; immutable compiler `${LOCAL_EVIDENCE}/compact-adr193-compactc`, schema20/ABI48. A single-binding version compiles. The identical two-binding action body returning an independent Field parameter also compiles and records.

```compact
export ledger x: Field;
export circuit run(): Field {
  const before = x.read();
  const after = before + 1;
  x = after;
  return after;
}
```

### Root cause
The frontend `stateful-return-ir` extracts a flat Parameter(after), while `stateful-body-ir` retains outer Let(before) → Sequence → inner Let(after) → CellWrite. Native `render_stateful_circuit` captures return_parameters only for a pointer-identical last top-level Let. It restores the inner scope before rendering the extracted return. This is a native scope bug; unsupported recording combinations are a separate review.

### Before / after
Before, native emission fails before a generated API is available. Intended generated semantics after the fix:
```rust
let before = /* exactly one typed Cell read */;
let after = runtime::field_add(before, /* one */);
let context = /* exactly one typed Cell write of after */;
let result = after;
```
This is an explanatory sketch, not a new runtime API. Existing syn AST generation, canonical slot operations and CircuitResult remain authoritative.

### Decision and ownership
Parent approved an explicit return-continuation flag in native `stateful.rs`: begin at the final top-level action, propagate only to a Sequence final child and a Let body, and capture the lexical bindings on that terminal chain. Never propagate into If branches, earlier siblings or independent ReturnPlan/action-helper boundaries. Preserve outer bindings and legitimate shadowing. Do not hoist, recompute, discard or reorder effects. Do not expose malformed out-of-scope locals merely because an identifier matches.

No Scheme, schema, ABI or runtime changes are expected. A frontend-wide ReturnPlan migration is unnecessary for this bounded fix. No recorded profile broadening: new source capability metadata must remain honest.

### Validation plan
Use same-tail two/three bindings, an independent parameter return control, legal source shadowing when available, witness/query evaluation-once cases, and earlier-sibling/branch-local/unknown-name escape negatives. Capture independent TS outputs, public state/effects, query gas, witness arguments/private-output order, successes and meaningful failure prefixes; compare native generated crate. Add renderer/source regression and focused fixture freshness, tests and strict Clippy. Proof applicability and unavailable recording remain explicit; no proof claim for a native-only repair. Finish with conventional GPG+DCO local commit and exact receipt.

### Coordination
ADR0195 owns typed_plan.rs/recorded.rs and may proceed in parallel. ADR0196 owns stateful.rs and independent tests/fixture/gates. ADR0194 microDAO advance remains funded_coin_finish's next slice. No remote CI or push.


### Implementation and focused evidence
Issue: https://github.com/MediaNoxLabs/compact/issues/301. Native continuation propagation is implemented without Scheme/runtime/schema/ABI or recorded-profile edits. Renderer negatives retain earlier-sibling, branch-local and independent ReturnPlan scope boundaries; terminal nested shadowing and outer bindings remain valid. An old terminal Sequence negative now asserts native acceptance with recording still refused, matching the repaired scope contract.

New unchanged `terminal_lexical_return_oracle.compact` has five proof-required exports: two/three bindings, independent echo, ordered witness/assert/write, and a nested helper. Native emission succeeds for all; only existing echo records (1/5). The predecessor193 compiler rejects this exact source at unknown parameter after.

Fresh independent TS capture and native tests pass13 scenarios at initial values0 and4, zero witness delta, failure after witness/before write, and zero-gas failure before witness. Exact result/FAB, serialized public state/effects, private state/output order and summed query gas match. TS wrapper gas is retained separately in the JSON; no aggregate wrapper equality or post-error context is invented. All success paths have one read and one write; observed adds exactly one witness between them. No recording/proof claim for the four newly admitted native methods.

Backend14 unit+13CLI+153renderer tests and targeted strictClippy pass. Existing root-Let and Effectful-return consumer suites pass. Full fixture freshness and final signed receipt pending.

### Local delivery

Signed GPG+DCO commit `2b3f3d050767090cc292323f6fddf624a16b87d0` (`codex/adr196-native-terminal-scope`), based on signed ADR193. No push or remote CI.

Native return-continuation flag propagates only along terminal Sequence/Let scopes, stopping at branch, sibling and independent ReturnPlan boundaries. No recording profile/runtime/ABI/schema changes. Four previously rejected native methods now compile; new fixture remains honestly 1/5 recorded.

Validation:
- 13 independent TS/native cases: two/three bindings, independent return, nested helper, exactly-once witness, assertion-before-write and gas-before-witness failures; results, serialized state/effects, private state/outputs and summed-query gas match.
- 14 backend unit + 13 CLI + 153 renderer tests; 34 Python tests.
- Three focused consumer suites and strict Clippy.
- All 170 generated fixtures fresh, zero unrelated generated changes.
- Exact new source fails predecessor193 with `unknown parameter "after"`.
- Clean exact-head receipt `${LOCAL_EVIDENCE}/compact-focused-adr196-native-terminal-scope-clean/receipt.json`: passed 3 fixtures, 3/7 proof-required recorded across the new fixture and existing root-Let/Effectful controls. Four recording gaps retained, no new proof claim.

The first receipt exposed an omitted declaration baseline; the signed commit now contains exactly six circuit plus one witness identity. Final clean receipt and Python inventory checks pass. ADR0196 and register updated in midnight.


### Integrated native lexical return fix — 15999efb

ADR196/[#301](https://github.com/MediaNoxLabs/compact/issues/301) is integrated as GPG/DCO-verified `15999efb`. Native return binding scope now follows only the terminal Sequence/Let continuation, stopping at branches, earlier siblings and independent ReturnPlan blocks. Schema20/ABI48 unchanged; no recording admission or runtime policy change.

- 13 independently captured TS/native cases cover two/three bindings, independent parameter returns, nested helper results, ordered witnesses, assertion and gas failures.
- Combined main checks pass: all **170 fixtures fresh**, backend14 library +13 CLI +153 renderer tests, targeted strict Clippy, and three affected generated crate suites. `${LOCAL_EVIDENCE}/compact-focused-15999efb/receipt.json` has3/7 proof-required APIs available; the new fixture retains1/5 recorded and four explicit recording gaps.
- Exact inventory `${LOCAL_EVIDENCE}/compact-15999efb-inventory.json`: **356/368 proof-required APIs available,12 explicit gaps,zero unassessed exports within the checked corpus**;211 sources/737 exports/190 compiled roots/369 nonproof, zero baseline drift or missing/unmatched rows. Adding the new regression source exposes four recording gaps; no previously available API is lost.
- Integration evidence: `${LOCAL_EVIDENCE}/compact-15999efb-integration-receipt.json`. Native-only scope means no new proof claim; broader proof and portable archive evidence remains explicitly at ee912835.

Active: ADR195 receiveShielded parity/proof, then ADR194 microDAO advance; independent ADR197/#300 TS coin descriptor correction. Repaired receive admission was reviewed for hidden reads in unused bindings/arguments and pure helper purity/types/cycles. Direct pinned TS runtime b16 probe accepts2^64/u128MAX and leaves narrow commitments unchanged; this is a candidate runtime fix, not yet integrated. Composition research remains linked in the production leftovers. No push, publication or remote CI. User documentation edit preserved.
