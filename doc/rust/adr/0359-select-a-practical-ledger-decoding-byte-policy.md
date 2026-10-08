---
id: RUST-ADR-0359
alias: ADR-0359
source_sha256: a34480e12a2f8da6d9f0ca812cd7c7b6f496ff81d99456907025a3f62ff79fdb
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0359 — Select a practical ledger decoding byte policy

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered locally and pushed; explicit aggregate containment deferral accepted by owner. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0359 — Select a practical ledger decoding byte policy and document inherited resource limits

Date:2026-10-08
Status: delivered locally and pushed; explicit aggregate containment deferral accepted by owner
Implementation: [#494](https://github.com/MediaNoxLabs/compact/issues/494)
Parents:R03017/#361, final qualification#364
Problem statement: [CoPS-001 — Ledger decoding resource limits](../../cops/0001-ledger-decoding-resource-limits.md)

### Problem

ADR0353 supplies correct caller-selected size admission but no reusable practical preset. The0.3.0 assurance text also overstates total resource protection available from the pinned upstream deserializer. The owner now explicitly accepts encoded-byte admission for0.3.0, defers total decoded heap/object/CPU containment, and asks for a generous usable maximum plus TS/mainstream comparison.

### Decision

Retain upstream codecs and explicit custom EncodedSizeLimit::new. Add a documented generous preset/default selected after measuring retained real ledger artifacts, with explicit override for larger legitimate artifacts. No ledger protocol maximum, aggregate memory/CPU guarantee or default retry is implied. Keep legacy decode behavior compatible. Record exact sizing evidence and comparison in CoPS-001; API: `EncodedSizeLimit::DEFAULT_MAX_BYTES = 64 * 1024 * 1024`, `Default`, and `max_bytes()`; `new(max_bytes)` remains the override. The preset is opt-in through the limited decoder APIs, not implicitly applied to legacy calls.

### Before / intended after

```rust
// Before: each caller must invent the byte-policy value.
let limit = EncodedSizeLimit::new(application_max_bytes);
```

```rust
// Intended convenience policy; custom application budgets remain available.
let limit = EncodedSizeLimit::default();
let key = decode_verifier_key_with_limit(bytes, limit)?;
```

### Ownership and alternatives

Runtime owns byte admission and delegation; upstream ledger serializer owns schema/decoding. Emitted Rust and IR/ABI do not need change. Applications own transport acquisition and deployment isolation. A global allocator hook, forked codec or process-isolation subsystem would broaden this release and is deferred by explicit owner decision. The CoPS series is cross-backend problem history; ADR records our implementation choice, and CoIPs retain their existing proposal role.

### Verification

Measure existing encoded state/verifier artifacts with hashes and scoped provenance. Reuse actual decoder integration tests, add default-policy positive behavior and focused policy boundary coverage where meaningful. Keep small deterministic inputs; avoid a stress campaign. Check source/docs parity assertions from pinned sources. Update the assurance criterion explicitly and preserve the earlier assumption/history. Additive byte policy must not silently cap existing legacy APIs.


### Practical sizing decision

Read-only retained-artifact census:43verifier samples (35unique), largest2119bytes;1262tagged ContractState samples (535unique), largest17450bytes from DID JWK final state.64MiB (67108864bytes) is approximately3846times the largest measured state and31670times the largest measured verifier. No evidence in this partial corpus requires a larger preset. This is observed compatibility headroom, not a protocol bound, guaranteed safe process memory footprint or universal state-growth limit. Large deployments retain explicit override and should measure their own artifacts. [Decoder preset sizing — 2026-10-08](references-0.3.0.md#note-150) retains identities and scope.


### Delivery and evidence

Signed conventionalGPG/DCO9ffd7880405ed5cfd62909ed4aac64a638c7fc02, pushed to origin. Seven decoder tests, scoped strictClippy and formatting pass. Independent sibling review found no actionable issue; root verified source/log/lock identities against the exact commit. [ADR0359 — Practical byte policy receipt](references-0.3.0.md#note-136). CoPS-001 pins TS and mainstreamledger8source and distinguishes missing guarantees from reproduced vulnerabilities. Deferred problem#495has no milestone. No fresh whole-component coverage percentage is claimed for this additive policy delta; finalcandidate qualification remains#364.
