---
id: RUST-ADR-0314
alias: ADR-0314
source_sha256: ec0430c299ef2a11f14228d5f7f4b5cc26f6ae077fccc9c016725df8af85120b
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0314 — Restrict ACC authorization to Jubjub and bound adoption work

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-and-applied-acc-deferred. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.

**Superseded scope:** ADR0316 removes ACC adoption from the initiative. The earlier deferral below remains historical.

## Original decision and amendments

```yaml
status: accepted-and-applied-acc-deferred
date: 2026-10-07
parent: R030-12
milestone: "0.3.0"
```

## ADR0314 — Restrict ACC authorization to Jubjub and bound adoption work

### Problem and owner decision

The owner now excludes secp256k1 as well as P256 from this ledger8 branch and authorizes deferring Passport ACC from 0.3.0 if needed. ADR0313's retained k256 obligation is superseded. Preserve research history while eliminating both foreign-curve implementation/adoption work from the active backlog. Existing Jubjub work is useful independently of ACC.

### Decision

Target an explicit Jubjub-only ACC source variant first. Remove all 35 k256 circuit exports and four private k256 helpers from the already P256-free isolated source. Retain the 35 Jubjub exports, ten shared exports, constructor, witness, all 25 ledger fields and shared structs/hash commitments. Remove only complete declarations selected by exact names, using token-aware balanced braces rather than line spans guessed across comments or strings. Retained executable declarations must compare byte-for-byte with the prior scalar-adapted source. Do not change challenge/signature, nonce/epoch, recovery or custody semantics to get a compile result.

```compact
// Before: distinct foreign-curve and Jubjub authorization entry points.
export circuit withdraw_unshielded_with_k256(/* original signature */): [] { /* original */ }
export circuit withdraw_unshielded_with_jubjub(/* original signature */): [] { /* original */ }

// After: only the original Jubjub entry point and shared custody remain.
export circuit withdraw_unshielded_with_jubjub(/* unchanged signature */): [] { /* unchanged */ }
```

These schematic examples explain the deletion boundary, not executable substitutes for authorization. Retain exact original source, all removed names, a compatibility patch and test receipts.

### Branch and runtime ownership

Audit tracked production source, manifests, generated fixtures and new history for foreign-curve work. Scratch P256/k1 research never established production support. Remove any branch-owned ACC foreign-curve implementation if found. Do not confuse DID key-type metadata/serialization with an ECDSA verifier: preserving required DID v0.7.0 behavior and its source-pinned oracle remains an existing milestone requirement. Likewise do not delete the k256 package from Cargo.lock by hand if the qualified upstream ledger8 graph requires it. Record these boundaries explicitly; the supported Compact ACC authorization target is Jubjub only, not a claim of a dependency graph containing no curve names.

Compiler/runtime/ABI/IR stay unchanged unless the audit finds actual removable implementation. Historical upstream meeting notes and immutable evidence are not active feature support.

### Bounded check and fallback

Run only local, bounded --skip-zk TS and Rust compile probes for the reduced source first. Keep actual first-layer diagnostics; inventory generated capability reports if available. No foreign-curve backport, large proof/model run or dependency fork. If meaningful retained ACC gaps still require a substantial separate adoption effort, use the owner's fallback: defer full ACC adoption beyond 0.3.0, preserve a tracked follow-up and all useful Jubjub reducers. Record the decision as deferred rather than delivered, adjust effective required scope without increasing accepted parent count, and remove ACC-only closure obligations from resource/coverage/audit gates. All other milestone requirements remain.

### Validation and consequences

The isolated source must retain exactly 45 circuit exports and 25 ledger fields; all 39 removed k256 declarations must be inventoried. Verify no executable Secp256k1/Secp256r1/P256/WebAuthn/k256 references remain. Original source and user edits must remain unchanged. Source generation, executable Rust, recording, VM replay and strict proofs remain separate evidence levels. Jubjub-only scope cannot claim parity with the original three-arm ACC.

### Tracking

Supersedes ADR0313/#438's retained k256 requirement; parent #356 and source inventory #429. Write progress and disposition to midnight before repository publication at milestone closeout.

Decision/delivery issue: https://github.com/MediaNoxLabs/compact/issues/439


### 2026-10-07 — Jubjub only; full ACC deferred by authorized fallback (ADR0314/#439)

The owner also excludes secp256k1 and permits removing full ACC adoption from 0.3.0. [ADR-0314 — Restrict ACC authorization to Jubjub and bound adoption work](0314-restrict-acc-authorization-to-jubjub-and-bound-adoption-work.md) supersedes ADR0313's retained k256 obligation. The isolated Jubjub-only variant removes 35 k256 exports and four helpers, retains 45 circuit exports and all 25 ledger fields, and preserves retained declarations byte-for-byte. TS generation passes; Rust refuses `Kernel operation blockTimeLessThan`. Full retained-account behavior/recording/proof qualification remains substantial and unaccepted. Use the authorized fallback: defer full ACC beyond 0.3.0; #356/#429 remain open, outside the milestone. No further ACC/foreign-curve work is scheduled for 0.3.0.

Branch audit finds no Compact P256/k1 implementation to delete. Preserve DID JWK metadata, immutable upstream history and k256 required by pinned native ledger8 crates. Preserve delivered generic Jubjub functionality and its maintained proof gate. Remove ACC-only adoption/resource/coverage/audit closure obligations; all other gates remain. Frozen original inventory: 8 accepted, 1 deferred, 11 open; effective required outcomes 19. Deferral adds zero completed outcomes; midpoint CI remains ten accepted original parents.

[ACC PR177 — Jubjub-only variant and milestone deferral](references-0.3.0.md#note-004) contains the boundary and future entry criteria. Receipt SHA256 `5885463ffc4f2d4b0955119c2979423f5494519ff2136f0b3e8cdabc2db92631`; archive [ADR0314 — Jubjub-only ACC probe and deferral.zip](references-0.3.0.md#note-098), SHA256 `0e37c82f44da56d354c7c93ae48ae78d13ce49fcd5a3ece8369c18d572f265a8`. No production changes, signing/commit, push or remote CI.
