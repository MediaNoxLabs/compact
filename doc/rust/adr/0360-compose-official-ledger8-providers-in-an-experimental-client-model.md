---
id: RUST-ADR-0360
alias: ADR-0360
source_sha256: 633ce0341b238c6e9524a9b477f770ec2318b243ef00a76a7ab734d0b67a7f0d
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0360 — Compose official ledger8 providers in an experimental client model

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered as a signed local testkit slice; experimental, not a production SDK. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0360 — Compose official ledger8 providers in an experimental client model

Date: 2026-10-08
Status: delivered as a signed local testkit slice; experimental, not a production SDK
Problem statement: [CoPS-002 — Rust contract client and provider boundaries](../../cops/0002-rust-contract-client-and-provider-boundaries.md)
Parent: https://github.com/MediaNoxLabs/compact/issues/496

### Problem

Application orchestration around generated Rust is scattered among smoke tools and examples. Consumers need clear acquisition, decoding and proof error boundaries without a second ledger domain model. The owner explicitly authorized investment, then confirmed reuse of official Resolver, ParamsProverProvider and ProvingProvider and official domains.

### Decision (refined after the owner's test-runner clarification)

Extend the existing midnight-compact-testkit with an opt-in proof feature and a small ProofLab<Resolver, ParamsProverProvider> object. It owns caller-supplied official providers and returns the official midnight_zkir::LocalProvingProvider for caller-supplied RNG. Consumers call the official ProvingProvider::check/prove/split directly. A verifier helper separates absent material, provider I/O and verifier decoding errors. No replacement provider traits or ledger domain DTOs.

ContractLab already runs generated contracts natively or with recorded VM replay. Reuse it rather than starting another client crate. ProofLab supplies the complementary local proof test setup. A single all-purpose run result would blur execution, replay, checking and proving, so keep these operations explicit.

Detailed external product research stays in Research/Midnight Rust integration references. No product code, private DTOs or dependency graphs enter Compact. All dependencies are existing pinned official ledger8/zk crates.

### Before / after

```rust
// Before: repeated provider assembly in individual test runners.
let provider = LocalProvingProvider { rng, resolver: &resolver, params: &params };
```

```rust
// After: reusable fixture owns the providers; canonical provider remains visible.
let proofs = ProofLab::new(resolver, params);
let provider = proofs.provider(rng);
let skips = provider.check(&preimage).await?;
let proof = provider.prove(&preimage, None).await?;
// ContractLab continues to own generated native / recorded test execution.
```

### Emitter / runtime changes

None. Add optional testkit dependencies only, feature-gated ledger-transaction and proof support. No new state reader, transport layer, deadline framework or production non-functional test campaign. Error Display is stable and source causes remain available for deliberate diagnostics. No promises of object safety, Send futures, automatic provider cloning or retries.

### Alternatives

Rejected for this slice: a separate client-model crate, a new state-source trait, universal async provider interface, and new bounded-reader machinery. The user clarified that a convenient reusable test runner is sufficient. Existing std/upstream traits and ContractLab already cover those concerns for local tests.

### Limits

Encoded byte admission does not bound total decoded heap or CPU, authenticate observations or impose I/O deadlines. Read adapters own transport, cancellation and timeout policy. Official Resolver returns an already allocated material bundle: any verifier admission here is after provider acquisition and is not a total artifact budget. Preimage checks are not proof generation, proof verification, submission or finality. No successful fake proof is evidence.

### Acceptance

Small offline tests of the reusable object with official Resolver and ParamsProverProvider implementations, real LocalProvingProvider check behavior on a tiny assertion IR, absent/malformed material, provider error causes and explicit proof failure. No successful fake proof or network acceptance claims. Run existing ContractLab tests as regression coverage, scoped strict Clippy and formatting, and independent source review. Full production integration remains the broader CoPS-002 follow-up; this adds no new production acceptance gate to 0.3.0.

### Local delivery evidence

Implementation issue [#497](https://github.com/MediaNoxLabs/compact/issues/497), milestone rust-backend-v0.3.0. Four new proof tests; proof-enabled testkit totals 25 unit/integration tests, plus 2 doc tests (one new compile-only example). Default feature run: 21 unit/integration tests plus 1 doc test. Scoped strict Clippy, rustfmt and diff checks passed. Independent agent source review: no outstanding findings; corrected one default-feature rustdoc link. No new registry packages or version changes; only five existing dependency edges added to the testkit lock entry. No successful proof generation is claimed for this slice.

### 2026-10-08 — ADR0360 local test runner delivery

User clarified a reusable local test object is sufficient. Delivered ProofLab under the existing testkit's optional proof feature; native/generated scenarios remain in ContractLab. Uses official Resolver, ParamsProverProvider and LocalProvingProvider/ProvingProvider directly. No copied product domains, new provider traits, production adapter framework or new package versions.

Signed conventional GPG/DCO commit `17433b40a14da83c5c500d92420149c28f755b88`. Proof feature: 25 unit/integration tests plus 2 doctests; default: 21 plus 1. Scoped strict Clippy, formatting, diff checks and independent source review passed. New proof tests exercise genuine assertion checking and refused proof attempts, not successful proving. #497 implements the narrow first slice of #496; wider client integration remains open. ADRs/planning stay vault-first. Current parent acceptance remains 16/19; #352, #364 and #358 still require closure work.

Receipt: [ADR0360 — Official provider test fixture receipt](references-0.3.0.md#note-137). Archive SHA256 `da389ca2e6447aa1ef61eb1f8a6aa558299a81172c863d1c69b15801f5ba009f`. Detailed product references are in the separate Research/Midnight Rust integration references folder.
