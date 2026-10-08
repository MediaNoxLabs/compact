---
id: RUST-ADR-0347
alias: ADR-0347
source_sha256: dafaf805904111b7b254552015656bdc8b9fcf822ccc7d4d1ef1248a5bd2ee4a
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0347 — Track versioned Compact specification conformance separately from fixtures

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered — bounded conformance infrastructure; broader conformance remains open. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0347 — Track versioned Compact specification conformance separately from fixtures

Date: 2026-10-08
Status: delivered — bounded conformance infrastructure; broader conformance remains open
Parents: R03007/#351 and R03017/#361
Source: codex/rust-backend-ast at f3526275; compiler0.31.133 / language0.23.105 / native ledger8.0.3 / private IR20 / ABI50

### Problem

The fixture/export census accounts for retained examples but cannot define complete Compact support. Native execution, recording and proof preparation accept different domains. Agda's constructor-to-typing-rule coverage is not Rust execution coverage. The checked-in formal Lsrc header names language0.16.103/compiler0.24.103, versus the current language0.23.105/compiler0.31.133. Source inspection finds grammar drift, unfinished operational definitions and a no-argument syntax generator invocation in formal CI. A passed historical formal check must not be promoted to current backend correctness.

### Decision and before/after

Before: fixture/export rows -> bounded assertion evidence.
After: pinned language requirement or grammar form -> semantic owner/frontend lowering -> Rust native / recording / proof-support classifications -> explicit evidence strength, limitations and follow-up.

The user approves a versioned requirements map and local drift check, with delegated grammar, backend mapping and formal research. Keep independent axes: normative conformance, TS differential behavior, corner cases and guarantees, native/recorded/replay/proof agreement, pinned ledger semantics and combinations of features. Support, observed behavior and formal guarantees are separate fields. Spec disagreements and missing formal definitions remain visible; TS is an oracle to compare, not automatic authority.

### Scope

Implement a reproducible bounded inventory from actual current compiler grammar and checked-in formal grammar, plus an explicit semantic-family/backend support map. Include meaningful local checks for stale/changed/unclassified inputs. Record how derived frontend forms relate to private Rust IR; do not require a one-to-one relation. Full specification alignment, a complete interpreter, whole compiler refinement proof and implementation of every missing primitive are later work. This slice cannot claim 100% semantic correctness or 100% native/recorded/proof support.

Use supported compiler tooling for live grammar extraction where available; do not silently treat regex or old JSON as the current compiler's full grammar. If full extraction is unavailable, fail explicitly or record a bounded source inventory with that limitation. Preserve existing generated fixtures and language semantics. No new library or language dependency upgrade without justification.

### Evidence and gates

Pin source/compiler/language/formal identities. Distinguish checked local inventory from historical execution and reviewer inference. Verify deterministic extraction/normalization, current baseline consistency and that an introduced unknown/changed requirement is detected. Semantic assertions must cite actual source owners and existing tests/receipts; unreviewed combinations are unknown, not covered. Existing formal debt remains visible even if a baseline drift check passes. Local testing first; no remote CI campaign or Agda installation required for this slice.

Planning, research, ADRs and progress logs remain in midnight; maintainable conformance tool/data may live in repository, while guides publish at closeout. No tests/builds on stopped ADR0285/#409 work. User-owned doc/ledger-adt.mdx is preserved. Child completion does not change 12/19 parent acceptance.

### Integration scope amendment — 2026-10-08

The full Python harness exposed a pre-existing stale fixture declaration snapshot. A separate exact-HEAD scratch reproduction at f3526275, using 459 byte-identical inputs, reproduces the same 19 additions and zero removals. Twelve vector-widen declarations and seven witness-effect declarations landed in earlier ADR0330/0334/0336 changes without updating parity_baseline.json. Reconcile only these identity rows using the existing writer and independently compare the exact additions. Keep every test assertion unchanged. This repairs fixture bookkeeping; it does not promote those declarations to language conformance or add semantic execution evidence. Retain the initial failed run and rerun the full Python harness.

### ADR0347/#475 conformance baseline delivered — 2026-10-08

Signed GPG/DCO [1aa60622](https://github.com/MediaNoxLabs/compact/commit/1aa606228502e131f0c901fe4cb932cc6051df7f) adds the versioned conformance map and source-bound checker. Actual extraction accounts for190 productions + 78 structural entries;86 reference headings map to30 families. Ten Boolean/conditional obligations separately track native, recording/observed APIs, proof applicability and historical execution. Undecomposed semantics and10 formal gaps remain visible; no 100% coverage/proof claim.

29 focused tests and144 full Python harness tests pass. Independent review rejected 8 mutations and verified the inherited fixture snapshot repair (exactly 19 additions, 0 removals). First failed suite and exact committed-source reproduction are retained. No Rust/TS/proof/Agda rerun or runtime change. User document preserved. Parent progress **12/19** unchanged; #351/#361 and final release remain open.

[ADR0347 — Versioned specification conformance baseline](references-0.3.0.md#note-124) contains the full family/seed tables, commands, research and next slice: complete Boolean truth tables, selected/skipped failures and source typing refusals, with recording/proof qualified separately. [Compact specification Agda and Rust-AST conformance](references-0.3.0.md#note-143) explains why current historical formal artifacts do not prove Rust correctness.

Archive [ADR0347 — Specification conformance baseline.zip](references-0.3.0.md#note-123), SHA256 `e68e4c3bf2205380b0e30152511672c55164c44a6dd08e09e83a70c8f5c2c019`, 573 byte-verified entries. Source/data/tool hashes, live extraction, initial/final logs, review reports and exact patch/commit retained.
