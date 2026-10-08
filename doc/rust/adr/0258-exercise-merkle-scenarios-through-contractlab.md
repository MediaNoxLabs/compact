---
id: RUST-ADR-0258
alias: ADR-0258
source_sha256: 5378b12f8f3b399982d59ab9d937375229aba7bfab082609216fc4011cf4b53f
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0258 — Exercise Merkle scenarios through ContractLab

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for implementation, 2026-10-07. Parent #350. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR-0258 — Exercise Merkle scenarios through ContractLab

Status: accepted for implementation, 2026-10-07. Parent #350.

Accept the test-only follow-up below. Existing generated Merkle APIs and actual upstream VM remain authoritative; no new production abstraction or runtime change. Preserve exact operation order and explicitly document the one gather/verify result-marker normalization. Proof/network support remains separately scoped.

## R030-06 ContractLab acceptance audit and smallest follow-up

Snapshot: Compact checkout observed at 8490847f65137bd219dcf15e0772419c0fea7186 on 2026-10-07. Read-only proposal; no source edits or new ADR/issue yet. Parent issue #350 remains open.

### Checklist against actual implementation

| Acceptance dimension | Current evidence | Remaining bound |
| --- | --- | --- |
| Native and recorded/replay with explicit environment | `testkit-rs/src/lab.rs` exposes typed generated-call closures, `Environment` supplies address, block/clock, coin public key, cost model, per-query gas limit and fixture seed; `tests/generated_scenarios.rs` runs recorded Counter, witnessed Cell, collections and native-only branches. | Seed is a caller fixture provenance label, not an RNG; document as such. DID lifecycle currently invokes `lab.native` only, correctly no Rust recorder claim. |
| Witness journals and owned snapshots/forks | `WitnessScript` validates ordered typed answers/errors and keeps journal; `Snapshot` clones owned public/private data, validates snapshot version, source/generated hash declarations, runtime ABI, ledger version, state mode and environment on restore. Parallel owned forks and rollback tested. | Artifact SHA values are caller declarations, not origin authentication. External witness side effects outside cloned private state are not rolled back. No durable secret snapshot codec is promised. |
| Reports, gas and privacy | `CallReport` exposes typed output, before/after state, effects, private outputs, execution gas; `ReplayReport` exposes ordered verify ops and replay gas separately. Default Debug redacts payloads and private error details. Boundaries tests exercise mismatch/error precedence and reports. | Proof and network levels intentionally absent from this plain-ledger crate; keep explicit tier labels in consumer docs. |
| Existing demonstrations | Counter recorded and rollback/fork, witnessed Cell recorded with private alignment/TS program, Set/Map collection states, native conditional witnesses all in `testkit-rs/tests/generated_scenarios.rs`. DID source lifecycle in `tests-rust-backend/did-adoption/tests/lifecycle.rs` uses `ContractLab` with exact TS native state/effects/private/gas and failure checkpoint; independently replays captured TS VM programs, but `report.replay()` is correctly None. Passport closure is 89 pure circuits/zero proof-applicable; do not force it through stateful lab. | No Merkle ContractLab scenario. DID generated recording and proof/apply remain separate compiler/contract adoption work; testkit cannot label a native call as recorded. |

### Proposed bounded follow-up

Add **one testkit integration scenario** for existing generated `compact-rust-merkle-tree-oracle-fixture`, plus a dev-dependency in `testkit-rs/Cargo.toml`. Own only `testkit-rs/tests/merkle_scenarios.rs` and that Cargo dev-dependency/lock hunk; no production testkit API, backend, runtime, schema or generated output change.

Use `ledger_contract::{initial_state, append, known, reset_tree}` and `ledger_contract::recorded::{append, known, reset_tree}` from `tests-rust-backend/merkle-tree-oracle/lib.rs`, `types::MerkleTreeDigest`, and the pinned TS capture `runtime-rs/tests/fixtures/merkle-tree-oracle.json`. Constructor: `initial_state(ConstructorContext::new(()))`, `ContractLab::from_constructor(identity_for(original_source, generated_lib), Environment::new(default_address, default_block, fixed_seed), initial)`. A typed call example:

```rust
let before = lab.snapshot();
let append = lab.recorded(|c| recorded::append(c, BoundedUint::<255>::new(7)?))?;
let after_append = lab.snapshot();
let root = MerkleTreeDigest { field: ledger::merkle_tree_view_at_path(after_append.public_state().get_ref(), &[0]).unwrap().root().unwrap().0 };
let known = lab.recorded(|c| recorded::known(c, root.clone()))?;
assert_eq!(*known.output(), true);
let mut fork = lab.fork();
let reset = fork.recorded(recorded::reset_tree)?;
```

Acceptance cases:
1. Append 7: compare observed Merkle root/first_free with generated `PublicStateView`, and compare recorded state with an independently run native call from the same prestate; full ordered verify ops must equal `nativeQueries.append7.queries[0].program`; execution gas equals sum of captured query gas dimensions; replay gas separately equals upstream VM replay value. No private outputs.
2. Known current root=true and pre-append root=false, with full program comparison to the two captured queries. Existing known capture has one `popeq.result` gather-vs-verify marker difference; compare via the same explicit single-field normalization used by the fixture's own test, not by sorting/reordering operations. Typed outputs and state unchanged; reports remain recorded/replayed.
3. Fork after append; reset only fork, assert root/first_free return to initial, original lab remains appended. Restore original from initial snapshot and verify metadata guard still applies. Compare reset program/gas to existing capture. Optionally set zero query gas limit and assert failed reset does not commit fork state.
4. Re-run the scenario twice with the same fixture seed and assert semantic state, typed outputs, full programs and gas agree. Do not claim proof/randomness determinism.

This is a *demonstration* of the already-supported Merkle recorder through ContractLab, not new Merkle backend coverage; the fixture already directly proves TS/native/recorded/VM behavior. The follow-up closes that named R030-06 example gap. The fixture’s existing direct test retains the full tagged state-byte comparison to `afterAppend7`; the lab scenario reuses the captured VM program/gas without importing extra serialization helpers. It does not close the full parent issue: real DID recorded/proof/application needs separate source capability work; optional strict proof/network adapters require their own tier receipts, not a universal VM or an inferred testkit pass. Documentation should continue to distinguish Native, Recorded/replay, Proof/application and Network.

Focused validation: `cargo +1.99.0 test --offline -p midnight-compact-testkit --test merkle_scenarios`, existing testkit tests, strict Clippy on the testkit. No broad proof gate is needed for this test-only slice. Check generated fixture ABI against current runtime before test; if root regeneration is underway, use the frozen accepted source/fixture pair rather than editing generated output.

### Delivered locally

`c43e8fd3a5931b7ffed5ee6c7f73943ab0621cdf`; 17 integration tests and strict Clippy pass. [ADR0258 — ContractLab Merkle local receipt](references-0.3.0.md#note-025). Parent R030-06 remains open.
