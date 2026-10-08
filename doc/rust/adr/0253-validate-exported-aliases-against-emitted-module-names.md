---
id: RUST-ADR-0253
alias: ADR-0253
source_sha256: bd9c2de18468f1c9269a9df9697620318d0afa03ae0a861713f44072b1dbafe8
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0253 — Validate exported aliases against emitted module names

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-implementation-in-progress. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-implementation-in-progress
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/378
```

## ADR-0253 — Validate exported aliases against emitted module names

### Problem
Real Compact source can export a new type named ledger_slots alongside ledger fields. The frontend succeeds, then the unedited Rust crate fails E0255 because both the root alias and generated module occupy the same namespace. This is distinct from hygienic representation support inside types::Name.

### Before / after
```compact
export new type ledger_slots = Field;
export ledger count: Counter;
```
Before: compiler success, consumer Rust namespace failure. After: a source-located typed alias-conflict diagnostic at generation, before publishing unusable output. When no ledger_slots module is emitted, the alias remains valid.

### Decision and ownership
Emitter owns generated module names and validates source alias reexports against actual occupied namespaces using normalized Rust names. Preserve public source names; no silent renaming. Reuse ConflictingTypeAlias with source provenance unless evidence requires a more specific diagnostic. No runtime, ABI or IR change. Existing blanket reservations are historical policy; this slice must not introduce a new unconditional ban where no collision exists.

### Alternatives and verification
Moving the public slot API would break consumers unnecessarily. Waiting for rustc loses source provenance. Test real source, conditional no-module control, normal aliases, normalized/raw name equivalents and CLI error location. Other support-name candidates need their own reproducer before widening scope. Parent safety/coverage outcomes stay open.

### Original reproducer and consumer evidence
```compact
import CompactStandardLibrary;
export new type ledger_slots = Field;
export ledger count: Counter;

```

```text
     Locking 324 packages to highest Rust 1.99.0 compatible versions
      Adding generic-array v0.14.7 (available: v0.14.9)
      Adding midnight-base-crypto v1.0.0 (available: v1.0.2)
      Adding midnight-circuits v6.0.0 (available: v6.3.0)
      Adding midnight-coin-structure v2.0.1 (available: v2.0.2)
      Adding midnight-curves v0.2.0 (available: v0.2.1)
      Adding midnight-ledger v8.0.3 (available: v8.1.2)
      Adding midnight-onchain-runtime v3.0.0 (available: v3.1.2)
      Adding midnight-onchain-state v3.0.0 (available: v3.0.2)
      Adding midnight-onchain-vm v3.0.0 (available: v3.1.2)
      Adding midnight-proofs v0.7.0 (available: v0.7.3)
      Adding midnight-serialize v1.0.0 (available: v1.1.1)
      Adding midnight-storage v2.0.1 (available: v2.0.3)
      Adding midnight-storage-core v1.1.0 (available: v1.2.1)
      Adding midnight-transient-crypto v2.0.1 (available: v2.1.1)
      Adding midnight-zk-stdlib v1.0.0 (available: v1.3.0)
      Adding midnight-zswap v8.0.3 (available: v8.1.3)
      Adding rand v0.8.5 (available: v0.8.8)
      Adding sha3 v0.10.8 (available: v0.10.9)
   Compiling zerocopy v0.8.60
    Checking zeroize v1.9.1
    Checking either v1.19.0
    Checking want v0.3.2
   Compiling midnight-compact-runtime-macros v0.1.0 (/tmp/rust030-alias-namespace/slots/contract/runtime-rs-macros)
    Checking hyper v1.12.0
    Checking rayon v1.12.0
    Checking itertools v0.14.0
    Checking generic-array v0.14.7
    Checking der v0.7.10
    Checking rustls-pki-types v1.15.1
    Checking blst v0.3.17
    Checking hyper-util v0.1.21
    Checking rustls-webpki v0.103.15
    Checking rustls-native-certs v0.8.4
    Checking crypto-common v0.1.7
    Checking block-buffer v0.10.4
    Checking crypto-bigint v0.5.5
    Checking spki v0.7.3
    Checking sec1 v0.7.3
    Checking rustls v0.23.45
    Checking digest v0.10.7
    Checking pkcs8 v0.10.2
    Checking sha2 v0.10.9
    Checking hmac v0.12.1
    Checking signature v2.2.0
    Checking crypto v0.5.1
    Checking sha3 v0.10.8
    Checking midnight-serialize v1.0.0
    Checking rfc6979 v0.4.0
    Checking ppv-lite86 v0.2.21
    Checking tokio-rustls v0.26.6
    Checking rand_chacha v0.3.1
    Checking rand v0.8.5
    Checking hyper-rustls v0.27.10
    Checking reqwest v0.12.28
    Checking group v0.13.0
    Checking fake v2.10.0
    Checking atomic-write-file v0.2.3
    Checking pairing v0.23.0
    Checking elliptic-curve v0.13.8
    Checking midnight-curves v0.2.0
    Checking ecdsa v0.16.9
    Checking k256 v0.13.4
    Checking midnight-proofs v0.7.0
    Checking midnight-base-crypto v1.0.0
    Checking midnight-storage-core v1.1.0
    Checking blake2b_halo2 v0.1.0
    Checking sha3-circuit v0.1.0
    Checking midnight-circuits v6.0.0
    Checking midnight-storage v2.0.1
    Checking midnight-zk-stdlib v1.0.0
    Checking midnight-transient-crypto v2.0.1
    Checking midnight-coin-structure v2.0.1
    Checking midnight-onchain-state v3.0.0
    Checking midnight-onchain-vm v3.0.0
    Checking midnight-onchain-runtime v3.0.0
    Checking midnight-zswap v8.0.3
    Checking midnight-compact-runtime v0.1.0 (/tmp/rust030-alias-namespace/slots/contract/runtime-rs)
    Checking compact-contract-slots v0.1.0 (/tmp/rust030-alias-namespace/slots/contract)
error[E0255]: the name `ledger_slots` is defined multiple times
  --> lib.rs:33:1
   |
25 | pub use types::ledger_slots;
   |         ------------------- previous import of the type `ledger_slots` here
...
33 | pub mod ledger_slots {
   | ^^^^^^^^^^^^^^^^^^^^ `ledger_slots` redefined here
   |
   = note: `ledger_slots` must be defined only once in the type namespace of this module
help: you can use `as` to change the binding name of the import
   |
25 | pub use types::ledger_slots as other_ledger_slots;
   |                             +++++++++++++++++++++

warning: unused import: `types::ledger_slots`
  --> lib.rs:25:9
   |
25 | pub use types::ledger_slots;
   |         ^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

For more information about this error, try `rustc --explain E0255`.
warning: `compact-contract-slots` (lib) generated 1 warning
error: could not compile `compact-contract-slots` (lib) due to 1 previous error; 1 warning emitted

```

### Local delivery — 2026-10-07

Delivered locally at `2c34c78ac47c0471e64c320a46afe5ada61768a9`: conditional namespace check,14focusedtests, strictClippy and standalone Cargo controls. GPG/DCO verified. [ADR0253 — Alias namespace local receipt](references-0.3.0.md#note-020).
