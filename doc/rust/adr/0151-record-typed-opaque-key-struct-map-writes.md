---
id: RUST-ADR-0151
alias: ADR-0151
title: "Record typed opaque-key struct Map writes"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "map", "asset-registry", "mutation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 9959d5c201ff99b9d5da2ba16ec98e55aeb6fa31d120b11b627bc2e7ded68dad
---
# RUST-ADR-0151 — Record typed opaque-key struct Map writes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the structural opaque-key/struct Map Insert and Update profile with audited writable/mutation guards, branch provenance and recordWrite continuation. Both branches are proved and failure guards match; changed key/value/order and unsupported helper effects remain refused.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#254 closure](https://github.com/MediaNoxLabs/compact/issues/254#issuecomment-6017660779). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
title: ADR-0151 — Record typed opaque-key struct Map writes
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The ledger-8 asset registry `setCustodyGrant` is proof-required but recorded Rust stops at nested `StateAction::Let`. It binds a disclosed `OpaqueString` key, a typed `CustodyGrant` struct, and a mutation enum, then performs writable and mutation guards, conditional Map remove or absence guard, Map insert, and `recordWrite`. A contract-name matcher would duplicate the source and make the emitter brittle.

### Before and after

Compact: `setCustodyGrant(grantId, grant, mutation)` accepts Insert or Update, rejects invalid mutation, checks writable, enforces presence or absence, writes a typed grant, and records the write. Before: `ledger_contract::setCustodyGrant(...)` is native only. After: `ledger_contract::recorded::setCustodyGrant(...)` and `contract.recording().setCustodyGrant_call(...)` expose the same typed transition and proof.

### Decision

Add a shared typed opaque-key/struct-value Map-write recognition path. Verify three scoped parameter aliases, exact declaration/index/key/value provenance, ordered writable and mutation guards, conditional update remove versus insert absence guard, typed Map insert and write counter continuation. Audit called guards and `recordWrite` transitively. Reject unrecognized actions, changed order, alternate key or value, unexpected branch, unsupported enum, and effectful helper bodies. Reuse ledger-8 Map, Cell and Counter slots; do not change runtime ABI or schema. Rename the now-shared guarded-map read locals inherited from ADR-0146.

### Acceptance

Capture fresh original TypeScript and compare native Rust, recorded Rust and independent Verify replay for Insert and Update, serialized state, ordered VM program, four gas dimensions, private effects, and missing, duplicate, invalid mutation, frozen/closed guards. Exercise a nonempty opaque key and composite grant. Add positive and negative renderer tests, refresh generated fixture, prove and verify the observed call with pinned ZKIR 2.1.0, then ledger-8 validate/apply. Run focused local checks and create one conventional signed GPG+DCO commit. No push or remote CI.

### Delivery

Decision recorded before implementation. Issue and commit to follow.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/254 (rust-backend-v2).

### Local delivery — 2026-10-05

Conventional GPG-signed and DCO commit `cf187beb54fb67e67af1d056281f3e50ee5c1a6a` implements the closed typed opaque-key/struct-value Map Insert and Update recorder and observed call. The original asset-registry TypeScript source was freshly compiled; its captured fixture exactly matches the checked-in oracle. Rust native, recorded, and independent Verify replay match original TypeScript serialized state, ordered VM operations, four gas dimensions, private state/effects/outputs, and witness calls for a nonempty key and composite grant. Missing Update, duplicate Insert, invalid mutation, closed, and frozen guards match TypeScript. Negative IR mutations reject changed key/value provenance and changed action order.

Pinned ZKIR 2.1.0 generated setCustodyGrant keys and circuits; both Insert and Update were proven, verified, validated, and applied through ledger 8 using the generated observed-call API. Proof artifacts: `${LOCAL_EVIDENCE}/compact-adr151-proof`. Local checks: four asset-registry integration tests, 118 renderer tests, strict Clippy, 148 fixture outputs with zero stale/failed, and exact-head focused parity gate 8/10 recorded (receipt `${LOCAL_EVIDENCE}/compact-focused-cf187beb/receipt.json`). The broad local gate now requires the capability, keys, and proof/apply selector. Runtime ABI and IR schema remain unchanged. Branch remains local; full parent-branch integration gate is pending.
