---
id: RUST-ADR-0104
alias: ADR-0104
title: "Attribute top-level Counter and Tiny proof APIs"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["inventory", "original-contracts", "source-provenance"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e91a318f5bc8834975032ce02a1580ad581a8294212ef3fac0840a7b4a7db5b1
---
# RUST-ADR-0104 — Attribute top-level Counter and Tiny proof APIs

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted exact original Counter and Tiny source attribution using their compiled cohorts and byte-equivalent fixtures. Existing behavior evidence is retained, but the external npm Counter runtime E2E was not rerun in this slice; attribution is not a new general emitter capability.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#207 closure](https://github.com/MediaNoxLabs/compact/issues/207#issuecomment-6017581175). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`c1fe760c`](https://github.com/MediaNoxLabs/compact/commit/c1fe760c15802600e05339f462cd53e33906729c). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 104
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/207
```

## Historical decision and amendments

### Problem and evidence

The repository inventory scans original `examples/counter.compact` and `examples/tiny.compact` but compiles only the `examples/rust_backend` fixture copies and checked source cohorts. Their six exported circuit rows therefore have `proof_required: null`, despite exact-source Rust compilation succeeding. This is a source-coverage and evidence attribution gap. The original Counter source is compiled in `tests-e2e/src/tests/runtime.compatibility.e2e.test.ts`; Tiny has source-level TypeScript compiler tests and pinned `print-zkir` / `print-zkir-v3` expectations in `compiler/test.ss`.

### Before and after

Before, `examples/counter.compact:increment` has `proof_required: null`, `rust_recorded: null`, `rust_observed_call: null`. After exact-source compiler assessment it has `proof_required: true`, `rust_recorded: true`, `rust_observed_call: true`, and `compiler_metadata_source: examples/counter.compact`. Tiny `set`, `get`, and `clear` receive the same proof-required assessment; `public_key` is compiler-pure and proof-false, so its recording status is `not_applicable`. Source paths, signatures, and lexical baseline identities remain unchanged.

### Decision and boundaries

Create a checked top-level source cohort for Counter and Tiny with explicit TypeScript source references and exact `contract-info.json` proof flags. Compile both targets against the same pinned compiler, join metadata only to identical original source paths, and require capability proof flags to agree with contract-info. Existing generated fixtures from `examples/rust_backend` are useful behavior probes but do not establish original-source attribution. Exercise the generated original Counter crate through a local Rust consumer; assess Tiny generated crate compilation and its recorded API against its own source. Do not assert semantic parity merely from compiler acceptance or inventory status. No emitter, IR, macro, or runtime change is planned; the before/after generated API is already `recorded::increment/read_round` and `recorded::set/get/clear`.

### Remaining scope

`examples/election.compact` compiles for Rust but reports five unavailable proof calls; `examples/zerocash.compact` compiles but reports two unavailable proof calls. They remain unassessed by the checked inventory until a separate slice establishes their exact-source behavior and proof path.

### Validation

Run the checked TypeScript/Rust source cohort gate, inventory unit tests and an exact-source compiler-backed receipt. Expect unassessed exported circuits to decrease by six, known proof denominator to increase by five, nonproof by one, and available recorded proof calls by five, without source-identity drift. Verify a local generated Counter consumer and Tiny crate, plus rustfmt. Run locally only; no push or remote CI.

### Tracking

- MediaNoxLabs issue: pending.
- Delivery: proposed.

Issue created: https://github.com/MediaNoxLabs/compact/issues/207 in rust-backend-v2.

### Local delivery, 2026-10-05

Signed GPG/DCO conventional commit `25275af1` (base `c1fe760c`) adds the checked Counter/Tiny source cohort, optional exact formatted Rust-fixture comparison, provenance-preserving compiler inventory assessment, and a full local-gate step. Issue [#207](https://github.com/MediaNoxLabs/compact/issues/207) is in `rust-backend-v2`. Frozen compiler `${HISTORICAL_NIX_STORE}/s0pxq91fl2c966jnwrizw1inwgd3liyg-compactc` (SHA-256 `9600da4ce472c0e066e251484f2df5ead662d525d243b50d059bccbd9c3b941e`) compiles both original sources for TS and Rust. Both formatted original-source Rust files byte-match their checked fixtures. The original generated crates compile, and the existing fixture suites pass five Counter VM/replay tests and three Tiny TS/native/recorded/VM tests. All 18 inventory unit tests pass.

Exact pre-ADR-0101 full inventory receipt `${LOCAL_EVIDENCE}/adr104-full-inventory.json`: 191 sources, 931 declarations, 301 proof-required, 225 available, 76 missing, 343 nonproof, 37 unassessed, zero identity drift, zero unmatched compiler circuits or missing metadata. Counter contributes two proof-required available calls; Tiny contributes three plus one proof-false pure helper. These are original-source compiler assessment and generated-code equivalence; Tiny fixture tests establish executing parity for its matched code. Counter has VM/replay tests and a TypeScript compiler/runtime e2e reference, but this slice did not rerun the external npm runtime e2e. Election and Zerocash remain unassessed with seven known unavailable capability calls. Root integration and exact-head full gate remain. No push or remote CI.
