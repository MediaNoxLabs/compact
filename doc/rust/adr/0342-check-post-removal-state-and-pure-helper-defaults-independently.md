---
id: RUST-ADR-0342
alias: ADR-0342
source_sha256: a21c7212dc5dd4f0edd057a5a9e13f6fb77d3b30a5297698f228ce4f440d195b
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0342 — Check post-removal state and pure-helper defaults independently

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for bounded test-only implementation. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0342 — Check post-removal state and pure-helper defaults independently

Date: 2026-10-07
Status: accepted for bounded test-only implementation
Parent: R03007 / #351

### Problem
The remaining export-family review found three assertion weaknesses. constructor-list-actions::drop_first compares two Rust states/replay effects but does not independently assert the expected post-removal contents. internal-pure-call and stateful-pure-call execute generated initialization, then overwrite stored before asserting its default. These are gaps in test strength, not observed production failures.

### Before / after
Keep existing native/recorded/replay comparisons. After drop_first, assert through ordinary typed state views for each actual result that items is empty, its length is zero and its head absent. The separate history List must retain length1 and Field4, as required by the source constructor. Before either pure-helper save call, independently assert stored equals Field0 in the freshly initialized state.

Existing pattern:
```rust
assert_eq!(native.context.query.state, recorded.execution.context.query.state);
```
Additional independent expectations use the generated PublicStateView and existing List view APIs; no handwritten VM decoding, copied implementation or new captures.

### Ownership and validation
Only the three existing integration-test files under constructor-list-actions, internal-pure-call and stateful-pure-call. No new packages/dependencies, source/generated/runtime/compiler changes or proof work. Preserve positive TS results and exact source/log links. Run these three packages with applicable features, strict scoped Clippy and formatting. Coordinate target leases; use the free normal target while the separate seeded-graph slice uses isolated targets. Report selected assertions and exact passed test names; do not call this a current proof rerun.

### Review boundaries
No consumer-safety/cross-instance/observation-trust investigation. Historical remaining-family review stays frozen; delivery adds explicit amendments. Parent semantic completeness and final candidate checks remain separate.

### Source evidence
examples/rust_backend/constructor_list_actions.compact creates items[Field1] and history[Field4], then drop_first removes the one item. Both pure helper sources declare default Field stored with no explicit constructor. Exact inspected source/test identities and the three findings are retained under /tmp/rust030-remaining-assertion-joins; complete helper closure is being verified before the report is frozen.


### Delivery — 2026-10-07

Signed commit `66dc6fd5`, six generated fixture tests and strict Clippy/formatting pass; three independent assertion gaps closed.

[Seeded compiler properties and generated state assertions — 2026-10-07](references-0.3.0.md#note-168) contains archive, scope and limits. Parent acceptance remains separate.
