# CoPS002 local ProofLab review

2026-10-08. Read-only review of testkit-rs/src/proof.rs, proof_tests.rs, Cargo.toml, lib.rs and the narrow Cargo.lock addition. Checked against pinned midnight-zkir2.1.0 LocalProvingProvider and midnight-transient-crypto2.0.1 Resolver/ParamsProverProvider/ProvingProvider definitions. No builds, tests or source changes by this reviewer.

## Verdict

No outstanding actionable correctness/API finding in the bounded local-test slice.

The initial unconditional crate-level ProofLab rustdoc link could not resolve when the proof feature was disabled. Root changed it to plain code text; final source confirms the correction. The new no_run example calls official check/prove separately, identifies artifact/parameter prerequisites and seeded RNG's local-test scope. Its compilation receipt is owned by root's running documentation gate, not this source review.

## Ownership and behavior checked

- ProofLab owns resolver/parameters and returns the actual upstream LocalProvingProvider borrowing both and owning the supplied RNG. Bounds match the official types; upstream check borrows, prove consumes the provider, and split behavior remains upstream-owned. No parallel provider abstraction or private-product types introduced.
- verifier resolves once, separates missing material, resolver I/O and decoder failure, and delegates exact byte admission to the existing runtime helper. The size policy is applied after resolution, to verifier bytes only. Documentation correctly leaves acquisition limits with the resolver and does not imply a cap for prover/IR bytes or upstream prove/check.
- Default Display/Debug omit embedded provider diagnostics, while explicit Error::source preserves the cause. This is a narrow formatter property, not a blanket secrecy promise for error chains.
- Feature proof is opt-in; dependency versions match the existing ledger8 closure. Lock diff adds references to existing packages; it does not introduce the issuer's alternate dependency graph. No issuer source appears in the reviewed slice.

## Evidence scope

The four tests exercise one-shot resolution categories and source diagnostics, real serializable verifier exact/one-under admission, an actual upstream IR assertion check with one positive/one refusal, missing/malformed IR refusal and failed prove calls with deliberately invalid key material. The assertion IR is constructed directly in the test; it is not evidence of a newly compiled Compact contract. Counting zero parameter requests establishes those selected early-refusal paths, not all upstream failure orders.

No successful proof is produced or cryptographically verified by these tests. Empty key material deliberately causes prove to fail; manually checking NoParameters reports its refusal only. The no_run example documents and compile-checks successful-path syntax, but does not execute proving. This is appropriate evidence for a reusable local composition object and helper, not ledger/network acceptance, product integration or a production nonfunctional qualification.

Root reports full testkit plus proof-feature tests passing and owns remaining docs/Clippy receipts. This independent review inspected code only and does not independently certify those runs. No further feature, provider framework or audit campaign is required by this review.
