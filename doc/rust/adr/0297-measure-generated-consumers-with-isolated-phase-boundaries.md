---
id: RUST-ADR-0297
alias: ADR-0297
source_sha256: b47f6a58c8c07fbf3d5b0627e77b1898807d55d0b9f0aab59aef6a8ee9e4e180
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0297 — Measure generated consumers with isolated phase boundaries

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** locally-delivered-measurement-baseline. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: locally-delivered-measurement-baseline
date: 2026-10-07
parent: R030-18
issue: https://github.com/MediaNoxLabs/compact/issues/421
milestone: "0.3.0"
```

## ADR0297 — Measure generated consumers with isolated phase boundaries

### Problem and decision

Existing generation/proof logs and small abstraction probes do not provide a controlled current application performance checkpoint. Mixing compiler time, startup, context construction, recorded execution, replay, proving, heap allocation and peak RSS would conceal what is being measured.

Root approved the bounded scratch implementation described in [R03018 — Bounded performance harness proposal — 2026-10-07](references-0.3.0.md#note-166) (proposal SHA25659064553caed892dc7d863fa5bc19b50225b079b37c30a1d6ffadae7026f82f4). First deliverable: runnable shared driver, typed Counter/DID/passport adapters, correctness controls, receipt validation and a small bounded trial that catches measurement bugs. This is **not final milestone performance acceptance**. Final measurements wait for ADR0295 relation, ADR0291 limits/API and ADR0293 release versions.

Freeze source at signed40047fb86cac845c5706ab404bea444d6ac71762. Work only in scratch, using copied source/runtime/fixtures and copied matching compiler binaries. Do not mutate live compiler/runtime/fixtures. Use a leased existing warm target, no shared cache cleaning or giant new target.

### Before / after examples

Before, timing an entire command could include setup, testkit replay and JSON formatting:

```text
time cargo test did_alias_case  # compilation + test initialization + replay + assertions
```

After, typed inputs are prepared and independently checked outside the operation window:

```rust
let inputs = prepare_fixed_batch()?;  // frozen prestate, private state, witnesses
let mut outputs = Vec::with_capacity(inputs.len());
let started = Instant::now();
for input in inputs { outputs.push(run_native(black_box(input))?); }
let elapsed = started.elapsed();
check_and_drop(outputs)?;             // outside the measured window
```

This illustrates the boundary, not a promise of zero loop overhead. A no-op control and fixed preallocated output storage expose that overhead. Input consumption/drop semantics, reset and disposal are recorded explicitly. Native and recorded forms perform different work and are not labeled interchangeable cost implementations.

### Selected cases and phases

Three consumers, four operation rows: Counter.increment; original DIDv0.7.0 setAlsoKnownAs/insert-unicode from its existing alias capture; passport firstNameCommitment and digitalPassportClaimRoot with existing oracle values. Counter and DID have native/recorded modes; passport recorded/proof is explicitly not applicable.

Separate generation (`--target rust --skip-zk`), warm generated-library rebuild, startup-only process, native call and recorded call windows. Replay/observed preparation/proof/ledger validation remain excluded from runtime operation windows. Report warm dev-profile baseline only. Cargo JSON must prove owned-library rebuilt and dependencies fresh; no-op or cold-dependency samples refuse. No cold-build claim.

Optional allocation profiling uses cached DHAT0.3.3 only if its complete graph is available offline. It is a standalone scratch feature, not a runtime dependency or custom unsafe allocator. Separate allocation counts/bytes from uninstrumented timings; setup/reporting excluded from profiler window. Known allocation and zero-work controls verify instrumentation. Missing metrics are unavailable with reason, not zero. Process RSS is separately collected with explicit OS units and process/child boundaries; no per-call RSS division or peak subtraction.

### Sampling and conformance

Functional controls precede clocks and run after batches: existing independent oracle result/state/effects/private/witness comparisons. Reset each invocation to identical prestate; do not grow DID state or accumulate witness logs. Retain all attempted samples, raw order, tool/source/lock/feature/profile hashes, cache evidence, clocks and result checksums. First small trial uses one excluded setup pair and a small fixed measured pair count; the eventual baseline uses five measured pairs. Same-candidate A/A variance is not improvement. Any native/recorded pair is labeled different-work overhead evidence. No TS speed claim.

Fail closed on missing/duplicate/reordered cases, wrong modes/identities, bad output checks, cold/no-op builds, failed commands, source drift, unsupported units or missing required observations. Unit tests cover receipt and Cargo classification boundaries; no full parity/proof suite rerun solely for the harness. Frozen source copies permit live development elsewhere without contaminating samples.

### Acceptance and ownership

This ADR permits scratch harness implementation and bounded diagnostic trial only. Root reviews before product integration; no own commit, CI, push, tag, registry publication or production pin promotion. Planner/progress stays in Obsidian. Any requirement to rebuild a large dependency graph or add unavailable instrumentation is reported before expanding work. Negative measurements and missing metrics are valid findings, not performance improvements.

### Scratch diagnostic delivery — 2026-10-07

The runnable frozen40047 harness passed its corrected two-pair/batch4 diagnostic: 58 measured phase rows across three consumers/four operations, normal/heap separation, active/zero allocation controls, six gate unit methods and strict Clippy. A first trial exposed inconsistent per-process warmup; the accepted corrected window checks one same-mode warmup outside timing/profile for every case. No live code integration or final performance claim.

[ADR0297 — Frozen performance harness diagnostic](references-0.3.0.md#note-082) records all boundaries and missing final criteria. Root requires ambient environment/build flags/jobs capture, generated-to-measured-fixture formatted-byte equality, documented batch calibration, five pairs and the final relation/resource/version candidate before R03018 qualification. Delivery receipt SHA256 `372bcee23dba9de756df272874e16e57dfea3078c90562d53740c24c4546a89b`; archive [ADR0297 — Frozen performance harness diagnostic.zip](references-0.3.0.md#note-083) SHA256 `d68fb80f7aba12df168a6011782e8dcab276cb2443976ce68c94d8f272439dda`. The archive retains the pre-delivery decision text; this later addendum is historical progress.

### Root implementation follow-up — measurement identity checks

The diagnostic driver validates samples against expected rows supplied inside the same receipt. Before final qualification, derive the expected rows independently from bounded pair/batch settings and the fixed reviewed case/phase inventory, so removing both a sample and its declared expectation cannot pass. Also automate the root's generated/build/probe source-identity comparison: resolve the probe dependency's actual library path, normalize only through pinned Rust 2024 rustfmt, and compare exact resulting bytes. Hash the module and formatter in measurement receipts. These are scratch evidence-harness changes within ADR0297/#421, not emitter/runtime changes. Add focused negative controls for missing/reordered/duplicate rows and mismatched measured source. No new performance result until final candidate freeze and five-pair calibrated execution.


### 2026-10-07 — Bounded coverage and performance harness follow-ups

ADR0307 instrumented resource cohort passes23/23 at ec10f324. Boolean line-hit union with the earlier source-identical623-test+196-source corpus covers8331/8869 changed backend lines (93.93%), six additional lines. This is explicitly a mixed cohort with different package selection, not a fresh whole-tree run. The meaningful payload tests mostly strengthen assertions on already executed scanner lines. All58 earlier objects and all unchanged production hashes were verified; previous reports remain intact. Archive [ADR0307 — Bounded resource coverage.zip](references-0.3.0.md#note-090), SHA256 `95e35e6c61cf4936a4cf827b7f8dae1a03223cb8e0ded7611936ea27d3b7c485`. The proposed95% floor and coverage parent remain open.

ADR0297 harness now binds exact Cargo package/manifest/src_path, actual normal/heap preparation artifacts/features/locks, complete environment/config identity and isolated rustfmt normalization.26 unit tests, syntax checks and real hostile rustfmt-config control pass. Preparation-success unit cases use simulated Cargo; actual builds and calibrated five-pair measurements remain pending final frozen source. No speed claim. Archive [ADR0297 — Checked preparation and measurement identity.zip](references-0.3.0.md#note-081), SHA256 `54c18eee90e20e052c21a0317f290538d13d1dad4b11af7bad2de8ae553b9bd2`.


### 2026-10-07 — Actual ADR0297 performance baseline accepted within its scope

The immutable signed dce3c9e9 cohort passes genuine normal/heap preparation,54calibration commands, one excluded setup pair and five measured A/A pairs:136measured rows across Counter, originalDID and passport. All source/tool/feature/environment/build-freshness/correctness checks pass. Batch16 and short passport-window precision are explicit. Separate six DHAT windows retain counts/bytes/peak/end; no instrumented-latency or per-call RSS claim. Nine exact historical instrumentation identities supplement the current scratch graph; product dependencies are unchanged.

Root corrected an agent interpretation error: the approved greater-than10% trigger is a median regression versus a matching previous candidate, not the full range of this same-candidate A/A run. All13groups have reported variation; no comparative regression verdict applies. The original report/receipt remains immutable, and a separate hashed clarification supersedes its invented full-range requirement. A quiet-host repeat is not required to accept this scoped baseline. No speedup, TScomparison, stable latency ceiling or p99 claim. Missing absolute per-sample timestamps, power-state telemetry and separate result digests are disclosed; actual output checks and expected-source hashes are retained.

[Performance baseline at dce3c9e9](references-0.3.0.md#note-158) publishes the corrected interpretation and complete metrics. Archive [ADR0297 — Actual dce3c9e9 performance baseline.zip](references-0.3.0.md#note-080), SHA256 `9607fa57cdb2481241736a611cc79da2e71ea7e3649deb751bc4e906daa97441`;1709files with verified manifest, including all raw attempted rows. Issue#421 completes the harness/baseline slice. Parent#362 remains open for consolidated compiler/proof-resource ceilings and activatedACC feasibility disposition; overall acceptance remains8/20.
