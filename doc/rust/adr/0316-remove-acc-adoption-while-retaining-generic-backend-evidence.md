---
id: RUST-ADR-0316
alias: ADR-0316
source_sha256: b2102c20850f3e042db787435b92565e0fd7e446f27775b3fcc6602985284610
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0316 — Remove ACC adoption while retaining generic backend evidence

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-scope-removal. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.

**Scope removal:** ACC adoption was removed, not delivered; generic Jubjub evidence remains separate.

## Original decision and amendments

```yaml
status: accepted-scope-removal
date: 2026-10-07
parent: R030-12
milestone: "0.3.0"
```

## ADR0316 — Remove ACC adoption while retaining generic backend evidence

### Problem and owner condition

The owner authorizes dropping Passport ACC entirely if its witnesses, structures and ledger types add nothing unique. Root and an independent read-only subagent compared the pinned reduced45-circuit source with current generated fixtures/runtime owners. No unique language primitive, witness mechanism or ledger representation was identified. Its value is application composition and invariants, not a new backend domain type.

### Construct inventory and evidence

| ACC construct | Existing generic mechanism and representative tests |
|---|---|
| One held_coin(Bytes<32>) -> QualifiedShieldedCoinInfo witness, four call sites | Composite generated witnesses; native-zswap-intents-oracle tests coin/recipient witness outputs and ordering; shielded-send-oracle covers qualified coin inputs and native/recorded/VM behavior |
| GrantScope nested in GrantRecord | Plain Boolean/Uint/Bytes record fields; asset_registry_oracle nests Provenance in AssetRecord stored in Map and has insert/read/update/refusal tests |
| 25 ledger fields:20 cells,4 maps,1 set | Existing Cell/Map/Set mechanisms; asset registry has20 fields and chunked-ledger-oracle tests serialization beyond16 slots |
| Map<Uint<64>, Bytes<192>> inbox | Generic FixedBytes<const N: usize> and CellValue; opaque application data does not require an ACC-specific runtime type |
| Jubjub points/scalar arithmetic | Delivered jubjub-scalar-cell native/recorded/VM and strict proof regression gate remains maintained |

Source: PR177 b9e1357c21855c0d5680299faa7ba66a673cc401, isolated account.compact173 (witness),249/264(records),179–330(ledger fields). Runtime owners include primitives.rs FixedBytes and ledger.rs CellValue. This is a source/test inventory; no new test execution is claimed by this review.

The exact held_coin signature, Bytes192 map,25-field combination and full authorization/expiry/custody sequence have not been accepted through a full ACC run. Those are additional composition coverage and application semantics. Existing fixture categories do not prove ACC itself works.

### Before / after

```text
Before: keep a full ACC adoption follow-up
        + qualify every retained export, auth/custody sequence and proof
After:  remove ACC adoption from the initiative
        + retain independent generic regression fixtures/proofs
        + track ledger8 block-time lowering as a standalone backend gap
```

```compact
// Existing generic categories; no new ACC-only primitive needed:
witness held_coin(color: Bytes<32>): QualifiedShieldedCoinInfo;
export ledger inbox: Map<Uint<64>, Bytes<192>>;
export ledger grants: Map<Bytes<32>, GrantRecord>;
```

### Decision and implementation ownership

Remove full Passport ACC adoption from the initiative, not merely defer it. Close #356 and #429 as not planned. No replacement ACC milestone, follow-up, automatic run or full-contract acceptance gate. Preserve historical source/patches/receipts so the decision and prior work remain explainable. Do not delete or rename generic Jubjub code or archived evidence merely because identifiers contain ACC provenance.

Retain supported Jubjub primitives, typed recording, unit/integration tests, independently captured oracle and seven strict scalar-cell proof cases. Their runtime/emitter ownership is generic and useful independently of ACC. This scope change needs no production code, IR/ABI, dependency or protocol edit.

Keep blockTimeLessThan/GreaterThan as a separately tracked generic ledger8 Rust lowering gap, using ADR0315's minimal two-circuit source and14 actual TS VM boundary cases. Do not call that gap fixed or ledger9-only. The exact qualified-coin witness/Bytes192/state mixture can be chosen later as small regression coverage if risk warrants it; full ACC is not needed for that decision.

### Milestone and acceptance consequences

Original20-parent history:8 accepted,1 removed(ACC),11 open. Effective required scope remains19; removal is not delivery. CI stabilization still starts at ten accepted original parent outcomes. R030-20 needs the published removal disposition, not ACC support. Existing generated-source coverage, DID/digital-passport adoption, compatibility, documentation, resource and audit requirements remain. Neither parent closure nor this decision inflates completion.

### Alternatives and limits

Keeping ACC as a required real-world integration would add composition confidence but requires substantial application qualification beyond the identified generic gaps. The owner chose the conditional scope reduction; the construct inventory satisfies that condition. Do not generalize this finding into production acceptance of ACC or universal Rust coverage.

### Tracking

https://github.com/MediaNoxLabs/compact/issues/441

Generic time gap: https://github.com/MediaNoxLabs/compact/issues/442


### 2026-10-07 — ACC removed from initiative after construct review (ADR0316)

The owner condition is satisfied: no unique ACC witness mechanism, struct construct or ledger type was identified. One typed coin witness, two nested records and25 fields(20cells,4maps,1set) reuse existing backend/runtime categories. Exact ACC combinations and application invariants are not claimed tested. [ADR-0316 — Remove ACC adoption while retaining generic backend evidence](0316-remove-acc-adoption-while-retaining-generic-backend-evidence.md) removes adoption entirely; #356/#429 close as not planned, with no replacement ACC follow-up.

Retain delivered generic Jubjub regression/proof gates and immutable historical evidence. The ledger8 time-lowering gap remains independently open at https://github.com/MediaNoxLabs/compact/issues/442, unscheduled and outside any milestone. It is not an ACC restart. Scope/removal decision: https://github.com/MediaNoxLabs/compact/issues/441. Original20 inventory:8 accepted,1 removed,11 open; required19; no completion increase; CI threshold still ten accepted original parents.
