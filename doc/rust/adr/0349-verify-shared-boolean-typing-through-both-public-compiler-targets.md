---
id: RUST-ADR-0349
alias: ADR-0349
source_sha256: 02ddee3640dfd3f8b09036569addb5c71d258172be7b4e2db305cd0ff9c15903
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0349 — Verify shared Boolean typing through both public compiler targets

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered within the owner-approved conformance time box. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0349 — Verify shared Boolean typing through both public compiler targets

Date: 2026-10-08
Status: delivered within the owner-approved conformance time box
Parents: #351 / #361; ADR0347 requirements B01/B04/B05
Hard shared deadline: 2026-10-07 22:24:57 UTC (06:24:57 Asia/Makassar)

### Problem and decision

Private Rust IR admission does not demonstrate source-level operand typing. Logical operators lower to conditional forms before backend emission, so valid and invalid source programs must be tested through both public targets. Finish a finite source typing matrix, not an open-ended formalization campaign.

Before: the map identifies source-negative logical operand controls as unjoined.
After: valid Boolean controls compile for TS and Rust; non-Boolean left/right AND/OR operands, NOT operand and conditional guard are rejected with the expected type diagnostic and source location. Include literal short-circuit forms with an invalid RHS to establish static checking despite dynamic skipping. Both branches of a conditional remain statically checked even when the guard is literal.

### Ownership

Extend the existing compiler rejection gate with one cohesive bounded helper and source cases, plus a focused invocation using the same helper. Avoid a new testing framework. Preserve all existing checks. Qualify current source identity of the reused compiler/Scheme artifacts; rebuild only where actual production-source drift requires it. Generated outputs must be absent on rejection, and positives must produce the target artifact. Expected failure is a type diagnostic, never any nonzero status.

No compiler/runtime/dependency change is anticipated. Record discovered behavior disagreements before changing semantics. Tests may establish shared frontend enforcement, not a guarantee uniquely provided by Rust. No runtime, recording, proof, arbitrary type-universe or formal guarantee. The stopped #409 lane remains excluded.

### Acceptance

Finite cases pass on both targets with paired valid controls and exact diagnostic classes/locations. A deliberate wrong diagnostic expectation or accepted-program negative case must be detected. Existing Python harness remains passing; no broad proof/build campaign. Retain source/tool/command/log hashes and source-bound evidence joins in midnight. Independent review checks spec alignment and case sensitivity. Owner time cap takes precedence over unfinished optional conformance work; record and defer overflow.

### ADR0349/#477 delivered — 2026-10-08

Signed GPG/DCO [3455f2e9](https://github.com/MediaNoxLabs/compact/commit/3455f2e952c019cddd847a874b55f4a16f1ffb73) extends the existing rejection gate with shared Boolean typing. Each target compiles eight valid expressions and rejects twelve invalid forms with exact diagnostic, source location and absent output. The **26-call matrix passes**. Real compiler challenges detect wrong expected diagnostics and a valid program mislabeled negative. Independent review accepted the finite scope.

Reused source-qualified compiler/Scheme pair; no rebuild. Current Scheme source unchanged; only qualified test-only backend deltas since the earlier artifact receipt. Full Python harness **144 passed** before map update; focused **29 passed** and baseline check after update. B01/B04/B05 now link shared frontend evidence, separately from ADR0348 native execution. General type combinations, runtime, recording, proof and formal guarantees remain unqualified. No source-semantic change or stopped #409 work. User document preserved; parent count **12/19** remains unchanged.

Archive [ADR0349 — Shared Boolean source typing.zip](references-0.3.0.md#note-126), SHA256 `e171eac73ac35910f03e41ffc736709193e4f5aaa90150f677ab6a500dded538`, 288 byte-verified entries. Complete commands, binary/source hashes, source inputs, generated control outputs, sensitivity results, reviews and patch retained.
