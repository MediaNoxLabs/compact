# Ledger8.1.3 compatibility and local qualification

## Current source profile

The native runtime uses official ledger8.1.3 revision
`b5d3e965a18e4cac17fac4f38446fc55c524439a`, runtime ABI52 and private Rust IR20.
Capability reports use schema3 and source-origin transport uses schema3.
The supported v2 cohort is ZKIR/transient-crypto2.1.1, base-crypto1.0.2,
serialization1.1.1, proofs0.7.1, circuits6.1.0, stdlib1.1.0 and curves0.2.0.
The runtime compatibility manifest and locked dependencies are authoritative.
Generated consumers declare Rust1.89 or newer. Earlier0.3.0 Rust1.88 receipts
retain their original graph; they do not establish this graph's minimum version.

## Evidence scopes

The original recording-effect candidate qualified selected real8.1.3 local
proof/application paths and isolated public consumers. Those retained receipts
belong to their original source/tool epochs, including535e4470/df7367fc. They are
not relabeled as a new execution of this extracted branch. The selected cold/warm
artifact experiments measured only Counter/Let and Jubjub fixtures; they do not
establish general compiler performance or specification coverage.

This focused core extraction preserves compiler/runtime semantics and the
ordinary proof gates. Its local validation and independent review are listed in
the PR. No complete new historical qualification, proof-qualified release, MSRV,
platform/network deployment or package publication follows from this document.
The original source-policy JSON preserves its dated receipt descriptions.

## Ordinary local tests

Use the repository toolchain and affected tests first. Actual circuit tests still
perform their normal key generation, proofs, verification, assertions and negative
controls. No external cache is required by the compiler or default test entry
points. An optional local artifact-cache adapter is a separate delivery slice.

The shared guarded-Jubjub compiler correction changes affected v2 circuit bytes;
regenerate matching prover/verifier artifacts and qualify them before replacing
any deployed verifier identity. ZKIR-v3 native/Keccak qualification remains separate
from v2 ledger proof/application support.

Historical source-law tooling and its authenticated predecessor receipts remain a
separate review/qualification slice. Full epic closure requires that qualification
and independent acceptance; a finite capability corpus is not full language or
formal semantic coverage.
