---
id: RUST-ADR-0294
alias: ADR-0294
source_sha256: ad92e884063a6d089e976aa7279bf71decfeee3eaabfc4c209df0aca2fd80e09
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0294 — Maintain executable and strictly proved DID primitive reducers

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-test-only-qualification. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-test-only-qualification
date: 2026-10-07
parent: R030-11
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/418
```

## ADR0294 — Maintain executable and strictly proved DID primitive reducers

### Problem and before/after

[R03011 — Maintained DID reducer acceptance inventory — 2026-10-07](references-0.3.0.md#note-162) identified a precise acceptance gap. ADR0259/0265/0269/0278/0283 retain real minimal Compact sources and schema20 renderer/admission guards, plus independently executed and strictly proved original DID scenarios. The exact reduced sources are not maintained end-to-end generated execution/oracle/default-strict proof fixtures. Renderer JSON tests do not execute emitted Rust; full original DID proofs do not prove a different reduced source.

Before: maintained `.compact` → frozen schema20 JSON → typed renderer guard, with separately accepted full DID behavior/proofs.

After: maintained unchanged reduced `.compact` → current matched compiler → registered generated crate + independently captured TS cases → native/recorded/replay comparison → real proof/default-strict ledger apply, binding refusal and replay refusal. Source/generated/oracle/proof/material identities are bound and missing rows fail. Existing malformed-IR and full original regression tests remain.

Root approved this test-only qualification2026-10-07. Source fix issues remain closed; this decision does not revoke their accepted original DID evidence. Any concrete compiler bug found requires a separate proposal before a fix.

### Selected source and semantic leaf mapping

Paths below are beneath `tools/compact-rust-backend/tests/`. Reuse bytes unmodified where possible. Register generated crates against these maintained paths rather than copying untracked variants. Any consolidation requires a reviewed leaf-by-leaf equivalence decision before changing source.

| Slice / source | Selected meaningful exports | Semantic leaves and behavior |
|---|---|---|
| First: `point-composition/point_digest.compact` | `update`, `pure_projection_only`, `point_write_only`, `scalar_control` | Point X/Y through pure hash helper, authorize witness argument/order, Boolean active guard, Point Cell write, already-working Point/control paths; authorization failure and inactive-before-witness failure |
| First: `point-composition/point_guard.compact` | `check` | Ordered controller/recovery guard helpers, lazy Field inequality from Point coordinates; distinct-X success, equal-controller/equal-recovery refusals, full successful query order |
| First: `set-composition/set_string.compact` | `mutate`; existing insert/remove/member controls retain actual capability classification | Declared Enum mutation guard, Unicode/empty String Set member/insert/remove, Insert→Remove Counter bookkeeping, duplicate/missing/Undefined failure precedence; do not broaden unsupported standalone controls |
| Next alias completeness: `set-composition/opaque_digest.compact` | `run` | String+Enum product transient hash passed into Boolean authorization witness, followed by Boolean write; typed argument/hash and refusal order |
| Next alias completeness: `set-composition/pure_unit_guard.compact` | `run`, `local_control`, `transitive_control` | Pure Unit Enum predicate helper, local binding/transitive helper audit, accepted mutation and Undefined guard failure before write |
| Later Service: `map-composition/service_mutation.compact` | `set_service`, `remove_service` | Flat three-String product Map Insert/Update/remove, membership/duplicate/missing/Undefined order and Counter helper |
| Later Point Map: `map-point-composition/point_nested.compact` | `upsert`, `remove` | Flat String/Point product mutation, separately declared nested product key-only membership through action-free Boolean helper, lazy membership and exact typed slot/value ordering |
| Later nested JWK: `map-nested-product/nested-enum.compact` | `upsert`, `remove` | Nested named String/Enum product, enum pure assertion, branch-driven Map Insert/Update/remove, Counter |
| Later enum inequality: `map-nested-product/nested-enum-wide.compact` | `put`, `drop` | Wider/reordered nested product and actual declared Enum `!=` guard, insertion/removal; prevents inferring inequality from a reducer that only uses `==` |

Existing `point_store`, renamed flat products, empty/mixed products and malformed IR remain structural controls. This ADR does not require turning every negative or already-covered arity variant into a new proof contract. Frontend metadata determines each selected export's proof applicability; any pure/diagnostic/nonproof row records legitimate refusal/not-applicable status rather than manufacturing a proof.

### Before/after engineer-facing test usage

Before, a focused renderer test can assert only availability:

```rust
let rendered = render_with_capabilities(&contract)?;
assert!(rendered.capabilities.circuits.iter().any(|c| c.name == "mutate" && c.recorded));
```

After, the generated crate's actual native and recorded methods execute from equivalent constructor-derived state, then replay compares public VM/state/effects; a separate proof selector verifies/default-strictly applies its recorded observed call. Actual signatures are taken from freshly generated source. Test helpers may share transport/comparison plumbing but may not replace contract semantics with a new evaluator. Keep the source operation and case identity visible in every result.

### Ownership and execution sequence

1. Write accepted ADR and milestone issue before implementation. Freeze/hash selected source files and preserve the linked original regression receipts. Work in scratch first.
2. Wait for ADR0288's finalized frozen compiler/renderer and exact Scheme/runtime pair. Source/ABI/IR/native ledger remain as qualified; no live compiler/runtime edits. Inspect output/capability once to select cases, preserve unexpected refusal/error rather than papering it over.
3. Coordinate directly with ADR0288 before root Cargo manifest, proof-smoke main/Cargo, fixture freshness registration or other shared files. Its in-flight digest reducer remains separate. Avoid a new target; acquire an existing warm target lease before Cargo. Keep disk and source freeze identities in receipts.
4. Deliver bounded family slices: Point+Set first; remaining alias leaves then Service, PointMap and nestedJWK. Root reviews each maintainable source/test patch and final receipt before committing. No blanket one-off proof runner covering unreviewed operations.
5. Bind existing original DID scenario proof receipts separately. Do not rerun the whole original proof gate solely for a harness addition. If a new compiler fix becomes necessary, stop that path, report the reduced bug and get root's separate reviewed decision.

### Required evidence for each positive slice

- Maintained source registration and deterministic source/generated freshness; source file hashes and canonical generated crate/oracle paths included.
- Independent generated TS capture with exact compiler/runtime/module identities. Use matched frozen branch TS for snippet semantics; original-release DID oracle receipts remain separate and are not relabeled as snippet evidence.
- Native vs TS and recorded vs TS on typed result, full public state/effects, ordered program/query gas and witness inputs/outputs where present. Replay same public program from same prestate and compare result. Do not claim inaccessible failure context/gas; validate observable failure and committed rollback/prefix truthfully.
- Meaningful success, refusal and ordering cases as mapped above. Use canonical subgroup Points and lossless Field/byte/string transport; no invented impossible coordinate combinations.
- Real proof-applicable selected snippets: actual constructor-derived state/default initialization, verifier keys installed at legitimate deployment, explicit funded fees, nonempty proof verification, default strict whole ledger validation/application. Changed binding/public input refuses; same-time replay fails without state mutation. Constructor data deployment does not imply constructor execution proof.
- Isolated finite proof/source/material/ledger/operation identities. Do not use legacy shared helpers that disable balancing. No witness/RNG/seed export needed for public receipts.
- Per-family exact expected row inventory; failed, missing, duplicate or incomplete rows fail closed. Tests for the aggregator must exercise missing/failing evidence. Existing source/renderer negative tests remain intact.
- Focused tests and strict Clippy for changed test harness; broaden only for a concrete risk/gate. Preserve first failure logs. No new key generation for unchanged existing original circuits; snippet keys are expected new artifacts and remain bounded to selected operations.

### Domain boundaries and unchanged policy

No runtime/compiler semantic changes, dependency upgrade, ABI50/IR20/capability3 change, generated API redesign, source-name dispatch or stopped security-lane work. No ledger8.1 runtime promotion: native snippet proofs use existing8.0.3 ledger/zk graph. Source-fix issues remain accepted; R03011 closes only after its maintained evidence requirements are actually met. No CI, push, registry publication, tag/upload or own commit; root owns integration and release.

### Frozen selected source identities before implementation

```json
{
  "point-composition/point_digest.compact": "a25583542fa4663943c5bec61144911c2014df7019e44cae0d071b97198f994d",
  "point-composition/point_guard.compact": "fa46c08ef5a54c8fe987be9bc1ffa40f223f663630d5bc73f73050a7ecacaf55",
  "set-composition/set_string.compact": "04694fd4192ac63d14afacb5fb478b073dbcb977eaae262d0b4d3c6740f6aa98",
  "set-composition/opaque_digest.compact": "a37c43c298b2f72e5cc28cf94c56ab4eaa4e16757d577acdf023be5f81f7a3c7",
  "set-composition/pure_unit_guard.compact": "411bb785786295d6a6fbd83f8d97b6a791a0b1c57f78792629853dcf311d3e91",
  "map-composition/service_mutation.compact": "976cf5dc624bf853175b032590925e34fcd9e93c82e660f61ebe96419a3540d9",
  "map-point-composition/point_nested.compact": "d270124dd6e2eb619bfe5e4a86ed1e8be8d54a13c7321178481fe83f9b7bfa56",
  "map-nested-product/nested-enum.compact": "9be89300f16afe87853910611bf5e8d84f702d0756b1158b2d3749fa342550aa",
  "map-nested-product/nested-enum-wide.compact": "fa3d21da3db95d995d88dacb9f0d282bd2a0d74e8f44d201e9998368ef766a1c"
}
```

### Delivery — 2026-10-07

Root accepted and signed commit `40047fb86cac845c5706ab404bea444d6ac71762` (DCO and GPG verified). All nine unchanged selected sources now have maintained generated crates and independent TS/native/recorded/replay coverage: 55 behavior cases in nine behavior tests, nine provenance tests, 21 default-strict proof calls across 15 keys. Five evidence-gate unit methods and scoped strict Clippy pass. Constructor data was deployed and checked; constructor execution remains unproved. No compiler/runtime semantics or dependency graph change. Relation/R03011 parent acceptance remains separate.

See [ADR0294 — Nine maintained DID primitive reducers](references-0.3.0.md#note-077) for the exact matrix, measured key shapes, limits and reproduction. Delivery receipt `/tmp/rust030-adr294/delivery-receipt.json` SHA256 `366d1c7c5d02158a607415c09597c27bdd527dc1ff1f7c607a4032baca7edd76`; durable archive [ADR0294 — Nine maintained DID reducers.zip](references-0.3.0.md#note-078) SHA256 `b3e77704b8a08450fd88323aeb9a063b965778d02d3c828ef41e1bce37e71772`. The archived `accepted-adr.md` preserves the decision text as it stood before this delivery addendum; its hash is intentionally distinct from this later signed-history note.
