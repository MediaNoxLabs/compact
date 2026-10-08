---
id: RUST-ADR-0317
alias: ADR-0317
source_sha256: b98cde53bfaae210dfd85bcca869b91504b2dc5439255d4eca5e59328e56ae04
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0317 — Accept the scoped performance and resource baseline

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-local-parent. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-local-parent
date: 2026-10-07
parent: R030-18
milestone: "0.3.0"
```

## ADR0317 — Accept the scoped performance and resource baseline

### Problem

R030-18 remained open while full ACC resource feasibility was an active requirement. ADR0316 removes ACC adoption from the initiative. Reconcile the delivered baseline and remaining scope without rerunning expensive measurements that would not answer a new question.

### Decision

Locally accept R030-18/#362. Root and a delegated read-only review reconciled the existing receipts, source drift, compiler limits and resource inventories. This is baseline acceptance, not a final-release performance guarantee or the milestone's external coding-agent audit.

### Before / after

```text
Before: measured dce3 baseline + calibrated compiler limits + proof inventory
        waiting on full ACC resource qualification
After:  same immutable measurements accepted for retained scope
        ACC obligation removed by ADR0316
        future matching candidate regression >10% median requires investigation
```

No emitter/runtime/domain/API code change is needed. Production source and package inputs remain unchanged from measured dce3c9e9 through3361d696: the diff comprises tests and one cfg(test) module declaration. No new benchmark result is inferred from current HEAD.

### Acceptance evidence

- Five paired same-candidate A/A observations,136 measured rows: generation,warm generated builds,startup,native/recorded execution and separate heap windows. Exact source,lock,toolchain,profile,features,host and actual Cargo preparation/freshness evidence retained.
- Root reverified all10 performance receipt files,all4 resource receipt files and both archived evidence hashes. A read-only subagent independently checked the same evidence plus six model-fill records and binary-IR equality.
- Published compiler limits from ADR0291/0307 and ordinary-worker/frame regression evidence remain valid; applied across bounded ingress/traversal/render work, not all surrounding tools.
- Resource inventory separates original DID exports,reducers,generic Jubjub and historical Counter evidence. Six retained-IR model-only measurements fill missing rows/k without generating keys/proofs. Existing proof/key sizes are linked with their provenance.
- Proposed >10% median regression trigger applies only to a matching previous candidate. It is not a same-candidate full-range/median threshold. The historical addendum correcting that interpretation remains authoritative.
- Cold dependency builds are explicitly unmeasured; warm builds are qualified. Tradeoffs and rejected abstractions remain documented. No extra quiet-host run is required merely to make variance look smaller.

### Limits retained

This baseline does not promise stable latency ceilings,p99,release throughput,TS speed comparisons or controlled prover latency. A/A spread remains descriptive. Per-sample absolute timestamps,power telemetry and separate result digests were not all captured. The actual operation checks did pass. Historical Counter key provenance is weaker and read_round proof qualification is not established. Model-only rows are not newly generated proofs. Compiler limits do not universally bound Scheme,rustc,generated output,proving or runtime memory. Final frozen-revision host/security/release qualification remains separate.

### Evidence and tracking

Parent https://github.com/MediaNoxLabs/compact/issues/362.

- [Performance baseline at dce3c9e9](references-0.3.0.md#note-158) and its clarification addendum.
- [Compiler and proof resource baseline](references-0.3.0.md#note-145) and model-fill addendum.
- [R03018 — Performance and resource acceptance inventory — 2026-10-07](references-0.3.0.md#note-167).
- [ADR-0316 — Remove ACC adoption while retaining generic backend evidence](0316-remove-acc-adoption-while-retaining-generic-backend-evidence.md).

Source checkpoints: measured dce3c9e90c1cfd5b391c44355ea9f86e862bf317; reviewed current3361d696f359a4c86be2256d8e0613f70a4a26aa. Acceptance raises completed original parents from8 to9. Required scope19; one removed ACC outcome remains recorded separately. CI stabilization still waits for ten accepted original parents.
