---
id: RUST-ADR-0337
alias: ADR-0337
source_sha256: d274f16e80ab4d65eece8c5901b4af37fd2c695b5adfe92027cdf242f312e583
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0337 — Avoid writable executable copies in installation-layout tests

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for bounded implementation and Linux validation. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0337 — Avoid writable executable copies in installation-layout tests

Date: 2026-10-07
Status: accepted for bounded implementation and Linux validation
Parent: R03019 / #363, 0.3.0 CI stabilization

### Problem and evidence
At signed dbfd7dc2, Linux run37610878324 passes formatting and actual Rust1.88 MSRV lanes, but core tests fail before launching compactc in `broken_installed_root_does_not_fall_back_to_build_checkout`. `.output()` returns OS26 ExecutableFileBusy (Text file busy). The test's existing root assertions never run. Unique PID/counter paths exclude an obvious name collision. The test copies the compiler into a new executable inode immediately before spawning while other tests spawn concurrently. A concurrent child retaining a transient writable copy descriptor is plausible, not proven by the log. Artifact hashes and source/lock identities were verified separately.

### Before / after
```rust
let root = Root::new();
fs::copy(built_compiler, installed_executable).unwrap();
```
Proposed test-only setup:
```rust
let root = Root::new_in(built_compiler.parent().unwrap());
fs::hard_link(built_compiler, installed_executable).unwrap();
```
Use a unique temporary root adjacent to the built compiler so link creation is on the same filesystem. Preserve the actual installed/bin/compactc invocation and all existing assertions. Removing the test root removes the link, not the original executable. Keep other tests' existing temporary placement.

### Decision
Stage an immutable hard link instead of writing executable bytes. Do not retry compiler failures or replace executable-location checks with mocks. No runtime-root policy, compiler production code, workflow skipping, serialization or generic retry. No investigation of the stopped ADR0285 trust lane: scope is solely test-process launch mechanics.

### Validation
Review cleanup and same-filesystem placement; run the entire compatibility integration target with normal parallelism plus strict scoped Clippy and formatting. Confirm source binary bytes remain identical and original binary remains after cleanup. Push signed conventional DCO commit with the separately tested delivery batch and validate the bounded Linux workflow once. Local macOS passing alone does not establish Linux resolution. Preserve prior failure receipt and any new failure without masking it.

### Limits
Test execution requires a writable Cargo build directory; supported Cargo test environments already build there. A hard link aliases immutable executable bytes and must never be modified. No production behavior or dependency change. Remote evidence remains failed until a new successful run is verified.


## Linux launch repair and current coverage — 2026-10-07

### Signed delivery and remote result

Branch pushed through `31a02ebec210984238ae2135db5938f379340568`; four new conventional commits have verified GPG signatures and DCO trailers. ADR0337/#461 replaces only the compatibility test's copied compiler with an immutable hard link in a unique same-filesystem root. The original installed-path and output-preservation assertions are unchanged. Local21compatibility tests, direct binary/cleanup verification, strict scoped Clippy and format pass. A preliminary receipt assumption confused Cargo's same-byte binary restaging with a change; a direct already-built invocation separates and verifies inode/link cleanup. No failed test was retried.

[Linux run37614242081](https://github.com/MediaNoxLabs/compact/actions/runs/37614242081) **passes at exact31a02ebe**:701four-owner tests,0failed/ignored/filtered; formatting and strict all-target/all-feature Clippy; actual Rust1.88 backend/Jubjub and standalone checks. Installed-root regression explicitly passes. Artifact verifier checks source/event/tree, both lock hashes, committed workflow and all12artifact digests. Verification SHA256 `fc1685a0809e2432b9637ca92fb20eba521c7b82e86b99171585639714ad7c3c`.

This is bounded midpoint CI. It does not establish full source regeneration, generated consumers, proof/network or final candidate acceptance on Linux. Prior dbfd7dc2failed run is preserved; transient writer inheritance remains a plausible explanation, not a proven causal trace.

### Fresh local coverage

Fresh backend profiles and current objects only at31a02ebe:505backend package tests and198source/generated comparisons,0stale/0failed. Mapped production **22,262/24,116=92.31%**; changed since939b7aaa **8,715/9,163=95.11%**. No unmapped production files. Exclude explicit test modules and parsed inline test spans; no historical hits or old production mappings reused. Source/head/lock/tool identities remain stable. Runtime/macros/testkit retain their separately qualified historical percentages; this is no new branch/MC/DC/proof or generated execution measure.

### Evidence and follow-up

[ADR0337 — Linux launch repair and current coverage.zip](references-0.3.0.md#note-115): 67 hashed entries; SHA256 `bc07f54bfb52250dd3c5ae84326310ba41b48878754c9f7f48c2f03f82ff7e04`. Includes failed and repaired Linux receipts/logs, local test-process repair evidence, source bindings, current coverage export/profile data and signed patch. Large instrumented objects and raw profiles remain in referenced local archives; their identity manifests are retained.

Issues#458/#459/#460 and bounded compiler-review child#452 are closed with exact local scope; #461 now has its Linux validation. Parent acceptance remains11/19pending pipeline review and other mandatory gates. Existing17exception and4historical proof-route assertion joins are being reconciled without unnecessary proof reruns. StoppedADR0285 remains unaccepted.
