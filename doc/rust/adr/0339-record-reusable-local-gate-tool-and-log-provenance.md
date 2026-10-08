---
id: RUST-ADR-0339
alias: ADR-0339
source_sha256: ba8eba0d7dae557beea11414c38ceb0dee17a555f28469a9bdd9c75fac2ce172
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0339 — Record reusable local-gate tool and log provenance

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for receipt-only implementation. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0339 — Record reusable local-gate tool and log provenance

Date: 2026-10-07
Status: accepted for receipt-only implementation
Parent: R03019 / #363

### Problem
local_parity_gate.py seals source/compiler/lock identities and requested Rust version, but its reusable receipt omits observed cargo/rustc versions, relevant build overrides/config identities and command-log digests. Ad hoc historical sidecars sometimes supply these facts; no maintained wrapper makes them mandatory. CI records richer provenance. A passed receipt alone cannot resolve how two local gates differed.

### Before / after
Existing command evidence records argv/status/log path. Extend it with SHA256 of the completed log on both success and failure. Where stdout is a separately retained artifact, hash that output too.
```json
{"command": ["cargo", "+1.99.0", "test"], "exit_code": 0, "log": "tests.log", "log_sha256": "..."}
```
At gate start, collect actual cargo/rustc version identities under the chosen toolchain/environment and the relevant command selection/features. Record a strict allowlist of non-secret build settings and config-file identities. Do not dump the environment or Cargo credentials. Distinguish observed version/environment/config inputs from fully resolved compiler flags; environment overrides and Cargo configuration can interact.

### Decision
Keep this change limited to receipt production and its focused tests. No new factory, dependency, flags, target selection, skips, retry, or compilation behavior. Keep historical receipts immutable and use a compatible additive receipt extension unless the current schema demands an explicit bump. Failed command receipts must retain their failure and hashes, never be recast as success.

### Validation
Regression tests cover observed tool identity, absence/presence of selected profile/flag overrides, deterministic config hashes, exclusion of unrelated secret-like values, and success/failure log/output hashing. Use fake local command tools where appropriate so tests are independent of remote services. Execute one small existing real focused gate to prove the enhanced receipt works with the actual compiler/toolchain and unchanged selected tests. Avoid full parity/proof reruns for receipt-only code. Review log hashes against retained bytes.

### Scope and limits
This strengthens reproducibility metadata; it does not authenticate arbitrary tools/config, establish bit-reproducible builds or imply full resolved rustc flag capture. Parent R03019 requires explicit final review; R03020 owns later full candidate qualification. No stopped ADR0285 boundary work.

### Evidence
/tmp/rust030-r03019-acceptance/REPORT.md SHA25655d6e9ef5e7b0492da382247a13138d65b1a01baf525758a6737691d14fe8b14. Exact31a02ebe bounded Linux core+MSRV passed; this separate local harness metadata gap was found during pipeline acceptance review.


### Root review: retain and check end-of-run identities

The first patch records configuration/locks only at startup. The pipeline review explicitly identified lack of later lock-drift detection, so add an end-of-run snapshot of the same selected build inputs and require lock/config identities to match before reporting a successful gate. Preserve both snapshots on mismatch and fail with a concise provenance diagnostic, without rerunning commands or changing test selection. Add deterministic lock/config drift tests. This is an evidence-validity check, not a claim to resolve every compiler flag or an authenticated source guarantee. Repeat only the tiny focused gate after the metadata refinement. Preserve its first successful run as the original candidate receipt.


## Independent value assertions and local gate receipts — 2026-10-07

Three reviewed conventional GPG/DCO commits:

| Slice | Commit | Verified outcome |
|---|---|---|
| ADR0338/#462 | c921622dac4a7bed9afc5ce44ebf8e514ba714ac | Nine concrete assertion weaknesses resolved: independent wide Field values, exact inserted composite keys/sum78 and cardinality1, source-defined initial defaults before writes.8tests across4packages, strictClippy/format pass. |
| ADR0340/#464 | c8406a4dd74d3bee1cfd77b9337c14891621f7ab | Nine chunked read outputs independently anchored;25helper calls supply typed expected values.3package tests, strictClippy/format pass. ExactADR0335negative guards and all previous TS/state/replay checks retained. |
| ADR0339/#463 | 22ef5f23e115c3ad070ea1c8cd060a1b4fcafee8 | Reusable gate observes tool/config/environment inputs, hashes logs/stdout on success/failure, and retains end snapshots/refuses lock/config drift.19focused Python tests and3real generated behavior tests pass. |

Root reviewed all diffs and source/receipt bindings. The only dependency change is one fixture dev-edge to already-locked num-bigint; existing registry packages and versions are unchanged. No Compact/generated/compiler Rust/runtime source changes. The pure-family report identified9weaker assertions, not9production failures; the collection review identified9read-return weaknesses, not missing execution. The initial ADR0339candidate lacked end-of-run identity checks; root required the amendment, preserving both candidates and logs. Its197→198fixture-count correction belongs to previously deliveredADR0334registration.

### Evidence and scope

[ADR0338-0340 — Expected values and reproducible local receipts.zip](references-0.3.0.md#note-116) contains 113 hashed entries; SHA256 `4dbcc810683ade8bcc6c27a4a80bda21910177b585bec1d94749434b36201bf0`. Includes frozen pure/collection reviews,17exception joins and4historical proof-route joins, both receipt implementations and local execution evidence, exact patches and commit bindings. Large binary paths/hashes remain separately identified.

The17exceptions have qualified historical behavior joins; no missing tests were demonstrated. Four proof-runner routes remain historical, with explicitly qualified source/runtime differences and one older log lacking contemporaneous digest. Pure167rowreview:156bounded joins,9weaknesses(nowfixed),2exceptionsresolvedseparately. Collection185rowreview: concrete invocation/assertion routes and9return-valueweaknesses(nowfixed). Remaining family review is in progress; these counts are not new test counts or full semantic assurance.

Latest fresh coverage remains the exact31a02ebe production cohort: changed8,715/9,163=95.11%;198fresh renders. Subsequent changes here are test/receipt code. Exact31a02ebe bounded Linux CI passed; later branch run is separate. No new proof/network or branch-coverage claim. StoppedADR0285remainsunaccepted.
