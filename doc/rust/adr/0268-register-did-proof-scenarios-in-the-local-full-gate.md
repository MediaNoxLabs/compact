---
id: RUST-ADR-0268
alias: ADR-0268
source_sha256: 9ff3f8e80e26738308df1a4f9b1625fe3061a5e93385ac02adb767b5b4b2154b
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0268 — Register DID proof scenarios in the local full gate

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for implementation. Parent R030-19/#363, with evidence for R030-09/#353. No compiler/runtime admission or source-contract change is part of this decision. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0268 — Register DID proof scenarios in the local full gate

Status: accepted for implementation. Parent R030-19/#363, with evidence for R030-09/#353. No compiler/runtime admission or source-contract change is part of this decision.

### Concrete gap

`tools/compact-rust-backend/local_parity_gate.py:47` already inventories the original DID source and generated fixture; its full path at lines477–484 runs workspace Clippy and the existing general consumer/proof driver. The dedicated DID selectors added under ADR0259/0265 are not commands in that driver. A successful full gate therefore must not implicitly claim execution of the constructor-based DID proof lifecycles.

The current actual ADR0265 capabilities report has four available recorded APIs out of twelve proof-required exports: rotateControllerKey, recoverControllerKey, setAlsoKnownAs, deactivate. Eight remain explicitly unavailable. Twelve exported pure APIs and the broader native lifecycle have separate acceptance tests; key installation or compiler availability is not proof execution of any of them.

### Smallest component and ownership

Add a dedicated `tools/compact-rust-backend/did_proof_gate.py` with a small versioned scenario inventory (`did_proof_scenarios.json`, or an equivalently small typed constant). Add only one mandatory orchestration call from `local_parity_gate --full`, after ordinary generated-source freshness and package tests. Keep focused source mode unchanged. Do not enlarge check_compactc_target.py or duplicate its broad selector dispatch.

The module should expose one function accepting already frozen compiler/Scheme, target directory, persistent run directory and the parent command recorder; a small standalone CLI can support rerunning only this gate. It owns artifact preparation and receipt validation, not source lowering or runtime execution. Shared generic runner extraction is optional only if needed to avoid copying command/error logic; it is not a prerequisite refactor.

### Exact scenario inventory

| Scenario | Existing selector | Actual required key operations | Actual transaction sequence |
|---|---|---|---|
| Point lifecycle | `--did-point-lifecycle` | rotateControllerKey, recoverControllerKey, deactivate | original constructor → strict deployment → rotate → recover → deactivate |
| Alias lifecycle | `--did-alias-lifecycle` | preceding three + setAlsoKnownAs | original constructor → strict deployment → insert Unicode alias → remove Unicode alias |

The Alias harness currently installs all four verifier operations although it invokes only setAlsoKnownAs. That is intentional deployment identity and must be recorded, not described as four Alias proof calls. Both scenarios can share one freshly generated four-key artifact root. Two separate ledger states/private histories are created by the existing selectors.

The older `--did-deactivate` selector uses seeded contract state and is historical evidence; it does not substitute for the Point lifecycle. Retain its history, but do not make another redundant proof mandatory for this bounded gate.

Current pinned reference: zkir2.1.0 (`/nix/store/cb226jz3hv9s4j18xb1k2xh28yr5j65q-zkir-2.1.0/bin/zkir`), key rows1930/1930/1804 for rotate/recover/deactivate and1974 for setAlsoKnownAs, all k11. These are observed reference measurements, not universal hardcoded acceptance counts. Prior deactivate1780 vs1804 had identical ZKIR and incompletely retained historical tool identity; never attribute that difference to Rust changes.

### Artifact and execution contract

1. Before expensive work, verify source-manifest hashes for unchanged original DID and imported Schnorr source, runtime ABI/IR compatibility, required zkir availability, absolute readable MIDNIGHT_PP and upstream ledger/static funding fixture. Preserve explicit prerequisite errors.
2. Snapshot compiler, Scheme and actual resolved zkir executable with SHA256/version. Compile the original source with `--target rust --skip-zk` into the new persistent gate directory, supplying its real imported source closure. Do not compile all twelve keys or use `--rust-require-recording` globally: eight declared gaps remain valid.
3. Canonically format the fresh Rust and compare to the checked-in DID fixture used by the proof runner. Compare source/provenance, contract-info proof flags, capability rows and compatibility metadata. Require precisely the four selected APIs available; preserve a full twelve-row report so unsupported exports cannot disappear behind the selected list.
4. Run the existing proof-material preparation selector against the explicit cache. Generate only the four selected key pairs from their fresh ZKIR using the frozen zkir. Record key-generation commands, exit status, tool hash, input/output hashes and measured k. Verify expected files are nonempty; reuse only if every source/ZKIR/tool/key identity is explicitly validated, otherwise regenerate. Do not assume a developer's warm parameter cache; prepare k11 explicitly before a closed-network proof run if the existing keygen path fetched it.
5. Build proof-smoke once using Rust1.99, locked Cargo graph and caller's reusable target; preserve CARGO_INCREMENTAL=0 and existing profile policy. Freeze/copy that binary after build and hash runtime/proof harness/generated fixture/Cargo.lock inputs. Run both selectors against the same fresh artifact root. Retain every command/result in parent and child receipts; a nonzero selector fails the gate. No relaxed strictness or failed-case fallback.
6. Recheck all frozen executable, source closure, fixture, harness and key hashes at the end. The parent receipt embeds or hashes the child receipt. Missing, failed, skipped or stale scenarios mean failed/incomplete, never successful full acceptance.

A fresh output directory prevents accidental acceptance of stale files. Build and artifact caches are optimization only. Generated source hash equality is the crucial bridge between the frozen compiler's output and the fixture statically linked into proof-smoke.

### What successful selectors actually establish

`tools/compact-rust-proof-smoke/src/did_point_lifecycle.rs` already starts with the generated constructor, installs verifier operations via ContractDeploy, proves and applies default-strict deployment with separate real upstream Dust. Each call uses observed state fetched after actual application, compares native and recorded state/gas/private outputs, validates typed preparation against manual FAB binding, proves nonempty cryptography and rejects changed binding inputs, then applies default-strict and compares the applied public data to the captured TS lifecycle. Exact same-time replay must fail with ReplayProtectionViolation(IntentAlreadyExists), leaving ledger unchanged.

This is offline upstream ledger execution, not a network/indexer/finality claim. Source-prescribed stored constructor id remains zero; the actual distinct deployment address is recorded without altering source. No cross-instance authorization/security claim or stopped probe belongs in this gate. The gate must not describe four supported exports as full DID recording or use proof success to claim every argument/branch is covered. Native29-case Alias parity and full native lifecycle are separate test dimensions.

### Receipt and publication boundaries

Prefer structured per-scenario success records emitted by the existing harness, with scenario, ordered entrypoints, strictness, deployment-applied, call counts, changed-binding refusal, replay refusal, final-state digest and artifact identities. Any small addition should be limited to reporting existing checked conditions, not replacing assertions. A zero exit plus frozen scenario inventory is already a strong prerequisite; do not manufacture proof metadata by parsing vague stdout phrases.

Keep raw private proof/witness logs private. Publish concise statuses, counts, hashes, command provenance and public measurements only. The receipt distinguishes required key operations from actually proved call occurrences and compile-time capability availability from proof acceptance.

### Meaningful gate tests

Use an injected command runner and small temporary artifacts for orchestration tests: missing tool/funding prerequisite fails before builds; missing or unexpected selected capability refuses; fresh generated fixture mismatch fails; missing key/nonzero keygen refuses; Point succeeds but Alias fails means overall failure; required scenario cannot be omitted; changed source/tool/key after preparation refuses; child receipt cannot report success with missing scenario/commands. Preserve no source or existing output mutation on preflight failure.

Integration acceptance is one actual frozen-source run executing both existing selectors, with final source/tool hash checks and independent review of receipt. Existing package/native tests remain mandatory; no additional broad proof replay is needed merely to test receipt serialization.

### Coordination

ADR0265 owner confirms selectors and four-key root `/tmp/compact-adr265/did`; final compiler `/tmp/compact-adr265/bin/compactc-final`, Scheme `/tmp/compact-adr251/source-gate-final2/bin/compactc-scheme`. Those are research references, not paths hardcoded into repository gate. Parent should approve ADR/issue and decide the small structured receipt addition before implementation. No source edits have been made for this proposal.

### Root review and implementation bounds

Accept the dedicated runner plus one full-gate hook. Reuse existing command recording and executable snapshot helpers instead of copying their implementations. The authoritative scenario list is fixed in the runner and cannot be silently narrowed by an optional full-mode selector. A focused standalone invocation may select a named scenario if its receipt explicitly records that partial scope. Prefer making both scenarios mandatory initially to avoid unnecessary CLI surface.

The existing proof harness should emit a small structured success summary only after its current assertions pass. Include proof call counts, exact replay refusal checks and deployment application; the orchestrator verifies the expected scenario/call identities. Preserve failure receipts. Do not redesign proving or add a second material cache. Inspect existing material-preparation semantics and make its required paths explicit before choosing prerequisite validation details.

The remaining eight recorded-export gaps stay visible. This delivery does not accept the DID parent, the local pipeline parent or remote qualification. Follow ADR → issue → local implementation → isolated tests and one actual integrated proof run. New public documentation stays in the vault until closeout.

Issue: https://github.com/MediaNoxLabs/compact/issues/392

### 2026-10-07 — Delivered

Delivered locally at `6b5e98e8f1f7cbb121118b16a5bffebe0f5c6cf4` (conventional, good GPG signature and DCO). The full local gate now requires the Unit-composition source registry and a dedicated original DID proof gate. Both Point and Alias scenarios are mandatory: four fresh keys, five actual proved call occurrences, 12 declared capabilities with four recorded/eight explicit gaps. The real dedicated run passed 11 commands in 55.464 seconds; all five 3,296-byte proofs verified and applied under default strictness, refused changed input binding and returned exactly IntentAlreadyExists on same-time replay. Original constructor data was deployed; constructor execution was not proved.

22 orchestration tests, linked proof-consumer compilation and strict all-target/all-feature Clippy passed. Full-parent integration was exercised with simulated external commands; the dedicated gate used real proving. The broad historical full suite was not rerun.

The new material identity check first refused a historical checkout's spend.bzkir before executing any command. Selecting the explicitly locked registry ledger8.0.3/static passed the retained audited digest checks. This identifies drift in the historical fixture location; it does not establish that a previous functional proof was invalid.

Source/tool/material inventories include 450 source identities, 32 material hashes and seven Dust fixture hashes. No dependency, ABI, schema or runtime semantic change. Parent #363 remains open; accepted parents remain 3/20. [ADR0268 — Delivery receipt](references-0.3.0.md#note-037) and [ADR0268 — Actual strict DID gate receipt](references-0.3.0.md#note-036).
