---
id: RUST-ADR-0262
alias: ADR-0262
source_sha256: b7c6813006ba5712f913e69a18a4539c560eb2c211baccc6c15097a904bb3a60
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0262 — Exercise adopted DID recording through ContractLab

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** not separately declared. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0262 — Exercise adopted DID recording through ContractLab

Read-only snapshot: Compact `c43e8fd3a5931b7ffed5ee6c7f73943ab0621cdf`, 2026-10-07. Source of acceptance: Obsidian `Milestone 0.3.0/backlog.md` R030-06 and MediaNoxLabs/compact issue #350 (same five checks; still OPEN). No code or vault edits in this audit.

### Precise checklist

| Issue criterion | Present evidence | Disposition |
| --- | --- | --- |
| Native + recorded/replay, explicit clock/address/gas/identity and deterministic fixtures | `testkit-rs/src/environment.rs` accepts address, block/clock, coin key, cost model, per-query gas limit and a fixture seed; `lab.rs` executes ordinary typed generated calls and sealed recorded calls using upstream VM. `WitnessScript` checks ordered typed answers/errors and journals calls. Counter/Cell/collection/Merkle tests exercise both levels. | Core implemented. `fixture_seed` is a caller provenance label, not a lab RNG. `WitnessScript` supplies deterministic values; do not promise random fixture derivation. |
| Owned snapshots/forks, validated restore, parallel isolation | `snapshot.rs` checks version, caller-declared source and generated artifact SHA, ABI, ledger version, plain-ledger state mode; `lab.rs` also compares environment. Testkit boundary and parallel-fork tests pass. | Implemented within Clone/owned-state contract. Hashes detect mismatched declarations, not authenticate origin. Shared interior mutability and external witness effects are outside rollback. |
| Typed report/state/effects/ordered ops/private alignment and separate costs | `CallReport` exposes typed output, before/after ChargedState, Effects, aligned private outputs via explicit accessor, query-summed `execution_gas`; `ReplayReport` exposes ordered Verify ops and its separate VM replay gas. Debug omits private values/error payloads. TS program comparisons in Cell/Merkle are exact except documented Gather-vs-Verify PopEq marker. | Implemented. Method name `execution_gas()` should be explained as query-summed gas; it is not per-query detail or replay gas. No API alias required unless issue owner insists on literal `query_gas` name. |
| Failure preserves checkpoint; no fabricated traces or external rollback | `native` and `recorded` commit only after validated success; boundary tests cover failed calls, replay mismatch, gas limit, redacted diagnostics. DID lifecycle includes late failure after a TS prefix write and verifies lab state remains at checkpoint. | Implemented. The failed-call partial TS trace is independently captured, not a fabricated Rust report. External witness `Rc<RefCell>` journal can observe attempted calls and is not rolled back; private state inside lab is. |
| Tier policy and demonstrations | Counter, witnessed Cell, Set/Map and Merkle now have `ContractLab` demonstrations; Merkle landed ADR0258 at `c43e8fd3`. DID has direct `ContractLab::native` lifecycle across success/failure and independent TS VM prefix replay; generated `recorded::deactivate` is directly tested with 12 TS/native/recorded rows in `tests-rust-backend/did-adoption/tests/composition.rs`, but not via `ContractLab`. Passport pinned closure contains 89 pure circuits, zero stateful, zero ledger fields/witnesses, so direct typed pure tests are the applicable route; no lab/proof call should be manufactured. | A *single real-contract recorded lab scenario* is the clearest remaining demonstration if parent requires adopted stateful contract at both levels. Strict proof/ledger and network remain optional, distinct tiers; `testkit-rs/src/lib.rs` explicitly disclaims them. No general VM/DSL should be added. |

### Prioritized finite follow-up

1. **After the DID recording owner freezes the generated `deactivate` path**, add one DID integration test in `tests-rust-backend/did-adoption/tests/` (DID owner, not the testkit crate): construct `ContractLab<u64>` from the pinned DID constructor/state and source/generated artifact hashes, perform one valid `lab.recorded(|ctx| did::recorded::deactivate(ctx, &witness, signature, expected_version))`, compare typed `()`, resulting state/effects/private output order, exact TS Verify program after only the documented Gather/Verify cached-result marker handling, query-summed execution gas versus captured query rows, and separate replay gas. Add one stale-version or late-timestamp failure with no committed public/private change, but assert external witness call journal separately. Reuse `tests-rust-backend/did-adoption/tests/composition.rs` / `runtime-rs/tests/fixtures/unit-composition.json` rows and witness helper; avoid duplicate captured values or new backend lowering. Confirm the selected success case's source state/call environment agrees with `ContractLab` constructor or explicitly derive the prestate through typed calls/snapshot; never inject a fabricated state. This is a narrow ADR-ready real-contract `ContractLab` consumer, contingent on the DID owner's final signed handoff. No new testkit API required.
2. **Public usage guide** under documentation work package R030-08/#352: move the tested standalone consumer recipe from the vault evidence `ContractLab standalone consumer —2026-10-07.md` into a concise `testkit-rs/README.md`, link it from `doc/rust/README.md` after current milestone docs are ready, and run its same clean external Cargo consumer. Include one typed Counter native/recorded snippet, `--rust-runtime-root` to avoid duplicate runtime identity, snapshot/failure example, explicit seed meaning, query-summed execution vs replay gas, redacted private access, trusted adapter boundary, and four tiers (Native, Recorded/replay, Proof/application, Network). This is docs and reproducibility, not another library API. The existing vault evidence passed on Rust1.88/1.99 but is not a repo-native guide.
3. **Parent disposition**: once (1) and docs classification are recorded, #350 can be considered for closure without a testkit proof adapter. If issue owner interprets optional proof/network adapter as required now, record a separate bounded API/acceptance ADR before implementation; current `ContractLab` intentionally excludes wallet/Zswap and does not claim strict proof, ledger application or live node authentication. It would be unsound to infer those from local replay.

No immediate runtime refactor is justified by #350. Testkit production responsibilities are separated into seven modest files (`lab.rs` ~202 lines; environment/snapshot/witness/report/errors each <105). Runtime `context.rs` ~1416 lines, `recording.rs` ~927, `transaction.rs` ~1011, but the proposed DID lab consumer only calls existing generated/runtime APIs. Splitting those modules under this parent would add risk without advancing a missing acceptance condition; use a separately measured domain refactor only when a concrete change crosses those boundaries.

### Reviewable before/after sketch for item 1

Before (current DID adoption test):
```rust
let native = did::deactivate(context(row), &witness, sig.clone(), version);
let recorded = did::recorded::deactivate(context(row), &witness, sig, version);
// Direct fixture validates TS/native/recorded; no owned lab checkpoint.
```

After (proposed additional consumer; exact case/prestate selected from pinned capture):
```rust
let before = lab.snapshot();
let report = lab.recorded(|ctx| did::recorded::deactivate(ctx, &witness, sig, version))?;
assert!(report.replay().is_some());
assert_eq!(report.before(), before.public_state());
assert_eq!(report.public_state(), expected_state);
assert_eq!(report.effects(), &expected_effects);
// A separate failing call must leave lab.snapshot() equal to before.
```

This exercises the testkit boundary with a real adopted stateful contract. It does not duplicate the DID compiler admission work or imply proof/network success.

### Decision

Accept one independent test-only DID ContractLab consumer in a new test module, using original constructor/native setup and frozen TS captures. Coordinate source ownership with ADR0259. No testkit/backend/runtime API change. Keep documentation drafts in Obsidian until milestone closeout. Required gate: exact real adopted recorded success and failure checkpoint retention, private witness journal distinction, native/TS comparison and Clippy. Optional proof/network tiers remain separate as originally approved.

### Local delivery

`0b297501922db17d623d8ac1a8589f0068fe8c11`; twofocusedtests pass again at frozen committedhead, strictClippy evidence retained. Original constructor reaches captured prestate exactly; success compares independentTS/native/recordedprogram/state/effects/private/gas, and late witness failure preserves owned labcheckpoint while externaljournal remains observable. [ADR0262 — Adopted DID ContractLab local receipt](references-0.3.0.md#note-028).
