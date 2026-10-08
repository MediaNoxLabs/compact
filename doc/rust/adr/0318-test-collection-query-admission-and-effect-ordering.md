---
id: RUST-ADR-0318
alias: ADR-0318
source_sha256: 3aabfe817e850e83053484c5463fe948d4249464e597f548dcdf38322e2b2d3a
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0318 — Test collection query admission and effect ordering

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered-bounded-test-slice. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: delivered-bounded-test-slice
date: 2026-10-07
parent: R030-07
milestone: "0.3.0"
```

## ADR0318 — Test collection query admission and effect ordering

### Problem

Current changed backend coverage is8587/9060=94.78% on the documented source-identical cohort. Uncovered stateful query branches include declaration kind/index errors, operand/result types and the ordering of witness evaluation relative to a ledger query. Test these semantic boundaries through the actual dispatcher, not a reimplementation or synthetic coverage-only call.

### Before / after

```text
Before: valid generated collection fixtures + incomplete dispatcher refusal/effect coverage
After:  table-driven matching valid/refused query cases
        + exact diagnostics and no premature query statements
        + typed witness evaluated once before query
```

Examples to preserve: a Map lookup must use its declared key type; a Set member operand may be a witness but cannot become a query before declaration/operand validation; historic and current Merkle roots cannot be interchanged; List head retains the exact Maybe<element> result; nested maps use the physical ledger path for emptiness.

### Ownership and implementation

Add a cohesive collection-query test module beneath stateful/expression/tests.rs. Reuse the existing Declarations/Observation dispatcher harness. Keep production emitter/runtime/IR/ABI and generated output unchanged. Assert exact domain errors and positive effects/types. Inspect generated Rust AST only for observable ordering/routing obligations, avoiding complete syntax snapshots. Tests do not claim runtime/proof validation; existing fixtures own those stages.

### Verification

Run the focused dispatcher tests, library/test strict Clippy and formatting. Then a fresh bounded instrumented backend-library cohort, preserving historical coverage objects/profiles before target reuse. Reconcile source-identical production hits on the frozen denominator; no raw-profile/count addition or fabricated95% claim. Keep semantic/generated export/effect obligations separate from line coverage. No broad fixture/proof rerun is needed for test-only edits unless a failure or mapping change justifies one.

Root owns test edits and the coverage-target lease. Delegated read-only review inventories R030-07 remaining semantic obligations. No stopped ADR0285 investigation. Preserve user-owned doc/ledger-adt.mdx. Use a signed conventional DCO commit after verification; no push or remote CI at9 accepted parents.

### Tracking

Parent #351. Milestone0.3.0. Scope is useful collection-query unit tests and bounded evidence, not automatic closure of R030-07.

Delivery issue: https://github.com/MediaNoxLabs/compact/issues/443


### 2026-10-07 — Collection query slice delivered at d7c6b1ac

Signed conventional/DCO commit `d7c6b1ac9416d165b97136f7e05ca176ebfaf987` adds six methods in a cohesive collection test module. Twenty focused dispatcher methods,197 fresh instrumented library tests,strict library/test/all-feature Clippy and formatting pass. Production emitter/runtime/IR/ABI and generated output are unchanged. Tests cover declaration/index/kind precedence, operand domains,List Maybe result,Merkle flavor,counter threshold and emitted witness/query order. Review strengthened the AST assertion to direct unconditional statements, exact member/lookup operation and the actual witness result as query operand. This is compiler emission evidence, not a new runtime/proof run.

Explicit Boolean hit union on source-identical production mappings adds11changed lines: **8598/9060=94.90%**, whole mapped backend22152/24020=92.22%. The95% floor is still unmet (nine more hits on this frozen denominator). New test source is explicitly excluded; no new production denominator lines or raw-profile/count addition. Runtime/macro/testkit figures retain their earlier cohort scope. No fresh full-workspace or197-render/proof qualification is claimed.

Archive [ADR0318 — Collection query tests and coverage.zip](references-0.3.0.md#note-100), SHA256 `ed88b2367f28e65a806e9322d20cad0e57b3ad4d89b12b1b1044224ecc9daf44`; final receipt SHA256 `3ab8e9225436dd05c986bcd79504cbeda8a1fa054e80aeba13b9ffbfe9d4597b`. Earlier compile/assertion failures are retained as development attempts, corrected before final gates. User-owned doc/ledger-adt.mdx is preserved. No push or remote CI. #443 closes the bounded test slice; #351 remains open. Parent acceptance stays9/19.

Parallel read-only reconciliation preserves344evidence rows and427matching source/provenance/result hashes. It joins37oracle rows,DID,current passport cases,reducer exports/constructors andgeneric Jubjub while separating runtime,replay,proof andnetwork dimensions. It identifies four public reducer controls without a direct runtime case in the inspected evidence: alias-Set insert_control/member_control/remove_control and nested-relation seed. Another122inherited case-ID mappings and13constructor joins need reconciliation; these are not established missing tests. [R03007 — Generated export and semantic evidence reconciliation.zip](references-0.3.0.md#note-160), SHA256 `0b6d8c926ec8bf2c3dea09ab7450a3032878ed855736ec264d64a62849eed077`. Source checkpoint for that review is3361d696; only the reviewed collection tests changed afterward.
