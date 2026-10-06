---
id: RUST-ADR-NNNN
alias: ADR-NNNN
title: Decision title
date: YYYY-MM-DD
decision_status: proposed
topics: []
delivery_status: not-implemented
---
# RUST-ADR-NNNN — Decision title

## Problem

Describe the developer/protocol failure and smallest Compact/Rust reproducer.

## Before

```rust
// Current generated or runtime code, with a repository source/fixture link.
```

## Decision and after

```rust
// Proposed or delivered code; label illustrative examples explicitly.
```

## Alternatives and rationale

Explain the choices, tradeoffs and ledger-8 / typed-IR boundaries.

## Emitter and runtime ownership

Identify typed IR variants and spans, renderer functions, public/private generated APIs, runtime and derive/macro changes, reused ledger/zk primitives, validation ownership, ABI/schema effects and migration policy.

## Verification and limits

Record exact revisions and reproducible fixture/command references. Separate source availability, sampled TS/native/recorded behavior, state/FAB/ordered-VM/four-dimensional gas comparisons, replay, proof, ledger application and live observations. Distinguish default-strict from smoke policies. Include relevant malformed inputs, compile-fail cases, compatibility and measured source/build costs. Preserve failures and avoid extrapolating beyond the measured domain.

## Tracking and delivery

- Focused MediaNoxLabs/compact issue: URL, before/after and explicit acceptance criteria.
- Milestone: agreed name and URL; do not assign new work to closed rust-backend-v2.
- Local delivery: conventional GPG-signed/DCO commits, exact tests and unresolved criteria.
- Publication/remote validation/release: distinct states with exact revisions and evidence.
- Supersedes / superseded by: explicit linked ADRs, only when the decision changes.

## Dated amendments

Append changes in rationale, implementation or evidence; preserve the original decision history. Use relative links for repository records and public pinned URLs for external evidence. Label unpublished evidence explicitly; never publish machine paths, secrets or private wallet/proof material.
