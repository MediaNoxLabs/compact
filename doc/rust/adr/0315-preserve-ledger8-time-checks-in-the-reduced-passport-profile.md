---
id: RUST-ADR-0315
alias: ADR-0315
source_sha256: 3f045de2a1a2d93921c4d4f98392f716ddcf5306013475b5fb3821743e28add9
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0315 — Preserve ledger8 time checks in the reduced Passport profile

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** verified-compatibility-classification. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.

**Later scope:** ADR0316 removes ACC adoption and supersedes the earlier #429 follow-up ownership below; no ACC follow-up is scheduled.

## Original decision and amendments

```yaml
status: verified-compatibility-classification
date: 2026-10-07
parent: R030-12
milestone: "0.3.0"
```

## ADR0315 — Preserve ledger8 time checks in the reduced Passport profile

### Problem

The owner requests removing every ledger8-incompatible feature from the Passport contract. The previous Rust diagnostic for blockTimeLessThan could be mistaken for a ledger9/10-only dependency. Pinned upstream/ledger-8 commit eb72a5ab6085bf9b8564cc80c98c973e04825c2d already implements blockTimeLessThan and blockTimeGreaterThan in compiler/midnight-ledger.ss lines515 and529. Deleting these would remove valid ledger8 grant-expiry and recovery-delay protections.

### Decision and before/after

Keep the reduced Jubjub-only source from ADR0314 as the ledger8 compatibility profile. Both foreign-curve arms remain excluded. Source inventory finds all retained native types, builtins, Kernel and ADT operations in the pinned ledger8 compiler/library. Successful TS generation is corroborating frontend evidence, not full runtime/proof acceptance.

```compact
// Before and after: preserve the ledger8 timing guards.
assert(g.scope.expires_at == (0 as Uint<64>)
       || kernel.blockTimeLessThan(g.scope.expires_at), "grant expired");
assert(!kernel.blockTimeGreaterThan(disclose(now_upper)), "now bound is in the past");
assert(!kernel.blockTimeLessThan(pending_not_before), "veto window still open");
```

The first example abbreviates the original diagnostic text; the retained actual source stays byte-identical. These are authorization/time semantics, not compatibility scaffolding. Classify the Rust refusal as missing backend lowering. Any later implementation must preserve context-bound VM reads, strict comparison, disclosed bounds and proof transcripts; a host-clock check or unchecked witness is not equivalent.

### Component ownership

The upstream ledger8 compiler owns Kernel semantics. The qualified native graph uses midnight-onchain-runtime3.0.0 with block-time context. Rust IR production currently refuses the time operation before emitter/runtime execution. Supporting it will require a separately reviewed emitter/runtime/recording change; this classification does not add a new IR/ABI shape or implementation.

### Verification plan

Preserve pinned upstream library snapshots/hashes and a retained-native inventory. Compile a two-circuit time-bound reducer with the current ledger8 TS target; execute strict less/greater comparisons at below/equal/above boundaries and extreme valid Uint64 bounds through the actual TS VM context. Reproduce the Rust refusal independently on that small source. Reconcile all results with the existing full reduced-source TS-generation/Rust-failure receipt. No expensive proof work, broad gates or source weakening is needed to classify the gap.

### Scope and limits

This completes the requested ledger8-compatibility cleanup/classification for the reduced source to the extent supported by frontend/library evidence: no further ledger9-only executable API has been identified. Maintain 45 circuit exports and25 ledger fields; no additional retained feature is removed. Full ACC Rust adoption remains unaccepted and its ADR0314 deferral is not converted into delivery by successful TS generation. The reduced source and remaining Rust gaps are preserved for a deliberate future adoption step. Generic Jubjub support remains in 0.3.0.

### Tracking

Related ADR0314/#439, deferred account #356/#429. Record this correction and evidence in the midnight vault. Keep protocol-version compatibility and Rust backend capability as separate fields in adoption reports.

Decision/verification issue: https://github.com/MediaNoxLabs/compact/issues/440


### 2026-10-07 — Ledger8 compatibility correction, ADR0315/#440

`blockTimeLessThan` and `blockTimeGreaterThan` already exist in pinned upstream/ledger-8 `eb72a5ab`. They are **Rust lowering gaps**, not identified ledger9/10-only APIs. Fourteen generated TS VM boundary cases pass for the minimal two-operation source; equality is false for both strict comparisons. Rust reproduces the missing time-operation lowering. No proof/Rust execution claim.

The owner requested removing ledger8-incompatible Passport functionality. Both foreign-curve arms are already excluded. The retained45-circuit/25-ledger-field native inventory resolves in pinned ledger8, and its full TS generation passes. No additional ledger9-only executable API was found. Keep the ledger8 grant-expiry and recovery-delay guards intact. Full ACC adoption remains unaccepted/deferred; no parent completion or implied Rust capability follows from these checks.

[ADR-0315 — Preserve ledger8 time checks in the reduced Passport profile](0315-preserve-ledger8-time-checks-in-the-reduced-passport-profile.md) and [ACC PR177 — Ledger8 compatibility and time-operation classification](references-0.3.0.md#note-006) record native owners, source locations and reproducible evidence. Archive [ADR0315 — Ledger8 time primitive compatibility.zip](references-0.3.0.md#note-099), SHA256 `4aa35bff786aba8d215caefc29fd8a5ab3cac30082d3da6ba1594f0d68b883d1`; capture SHA256 `fbe28488807cc0cea6204605ad60d56440d039b3bac453cb40a731a76378e8a6`. Deferred follow-up #429 owns any future time-query lowering/recording implementation.
