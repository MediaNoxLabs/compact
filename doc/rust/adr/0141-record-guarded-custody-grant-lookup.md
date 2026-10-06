---
id: RUST-ADR-0141
alias: ADR-0141
title: "Record guarded custody grant lookup"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "map", "authorization", "asset-registry"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: d7bd60ef7da865e3bdb9f19770f64d4db2ada39b4284419a9669db467c31321a
---
# RUST-ADR-0141 — Record guarded custody grant lookup

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted an ordered opaque-key membership assertion, typed CustodyGrant lookup and audited pure time guard. The proof starts from a seeded valid grant and leaves it unchanged; missing and future grants retain distinct failures. Later shared matcher refactoring extends implementation rather than explicitly superseding this decision.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#243 closure](https://github.com/MediaNoxLabs/compact/issues/243#issuecomment-6017643291). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`6e8f82f7`](https://github.com/MediaNoxLabs/compact/commit/6e8f82f73908050fcc53553ad500f009c1bd5bec) · [`957bd577`](https://github.com/MediaNoxLabs/compact/commit/957bd5777b91f3abc7883d5790d3fe4bbe9408c5) · [`f72e22ab`](https://github.com/MediaNoxLabs/compact/commit/f72e22abf7f779a7e48063671e28af3c931a1079). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0141 — Record guarded custody grant lookup
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original ledger-8 `asset_registry_oracle.compact` exports `assertGrantEffective(grantId: Opaque<"string">, asOf: Uint<64>)`. Schema-12 IR binds a disclosed opaque key, asserts membership in `custodyGrants`, binds a typed `CustodyGrant` from a Map lookup, then calls the pure `assertGrantNotFuture(grant, asOf)`. The current generated Rust crate exposes only a native call. The exact integrated compiler rejects the nested Map lookup Let at `actions[0].action.actions[1]`. This prevents the guard from producing an observed proof call even though the runtime already has a typed Map recording slot.

### Before and after

```compact
export circuit assertGrantEffective(grantId: Opaque<"string">, asOf: Uint<64>): [] {
  const disclosedId = disclose(grantId);
  assert(custodyGrants.member(disclosedId), "grant does not exist");
  const grant = custodyGrants.lookup(disclosedId);
  assertGrantNotFuture(grant, disclose(asOf));
}
```

Before, a Rust consumer can call the native method but cannot call `recorded::assertGrantEffective` or `Contract::recording().assertGrantEffective_call`. After, the recorded method retains the typed opaque key, records the membership check and `CustodyGrant` lookup through the ledger-8 Map slot, then evaluates the generated pure assertion on the decoded grant and typed Uint64 argument. A successful call emits ordered Verify reads and leaves state unchanged.

### Decision and guards

Admit only the closed two-parameter custody-grant shape with a Unit result: one `OpaqueString` key parameter, one `Uint<64>` time parameter; one exact Map membership assertion; one exact Map lookup of `CustodyGrant` through the same field, physical index and scoped key; and one call to `assertGrantNotFuture`. Inspect the complete pure helper body before lowering: one comparison of the `grantedAt: Uint<64>` member with the typed `asOf` parameter, one assertion and Unit result. Preserve the original assertion messages. Reject a wrong Map type, mismatched key, extra action, changed field, changed comparison, or effectful pure helper. Other opaque-key calls remain unavailable.

The emitter should call `ledger_slots::custodyGrants.record_lookup` and the generated pure Rust function. Runtime ownership stays with the existing ledger-8 Map slot and its alignment-aware composite decode. No new primitive, schema or runtime ABI is intended.

### Acceptance

Capture fresh original TypeScript and compare native Rust, recorded Rust, and independent Verify replay for result, serialized state, four gas dimensions, ordered VM, private effects, and assertion failures for missing and future grants. Add a positive and negative renderer guard and refresh the generated fixture. Prove/verify the observed call with pinned ZKIR 2.1.0 and ledger-8 validate/apply from a fixture state containing one valid grant. Run focused local checks and update the full gate expectation for this call. Publish a signed GPG+DCO conventional local commit and exact capability evidence; no push or remote CI.

### Delivery

Decision recorded before implementation. Tracking issue and signed commit to follow.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/243 (rust-backend-v2). Created before implementation.

### Generated Rust API example

Before this slice, only the native call compiles:

```rust
let checked = ledger_contract::assertGrantEffective(context, grant_id, as_of)?;
// ledger_contract::recorded::assertGrantEffective is unavailable.
```

After this slice, consumers can record and prove the same read-only guard:

```rust
let checked = ledger_contract::recorded::assertGrantEffective(
    context, grant_id.clone(), as_of,
)?;
let observed_call = contract.recording().assertGrantEffective_call(
    &observed, private_state, grant_id, as_of,
)?;
```

The emitted body records `custodyGrants.record_member` then `custodyGrants.record_lookup`, checks the original missing-grant assertion, and calls the generated `pure_circuits::assertGrantNotFuture` on the typed decoded `CustodyGrant`. Both public APIs use the same state and typed input; the observed form adds the ledger-8 proof envelope.

### Local verification (2026-10-05)

- Original TypeScript capture `runtime-rs/tests/fixtures/asset-grant-effective.json` was generated from the unmodified ledger-8 source. It contains two read queries and nine ordered VM operations. The call leaves the serialized contract state unchanged, emits no private transcript output, and preserves null private state. Missing and future grants fail with distinct original assertions.
- Generated native and recorded Rust match the TypeScript result, serialized state, two-query ordered VM, complete-call gas in all four dimensions (read 765000000, compute 2938255815, written/deleted 0), and private effects. Independent flattened Verify replay matches TypeScript replay state, effects, and four replay gas dimensions. TypeScript `output.gasCost` reports the last query, so the complete call comparison sums both query costs.
- Pinned ZKIR 2.1.0 generated the `assertGrantEffective` prover/verifier and BZKIR. The observed generated call was proved and verified, then validated and applied through ledger-8; the seeded CustodyGrant remained unchanged.
- Renderer positive and negative guards pass: a lookup using the unscoped key or a changed pure comparison stays unavailable. 113 renderer tests, nine asset fixture tests, targeted all-target Clippy, and 147/147 generated fixture freshness pass.
- Conventional local commit `4eb758af` is GPG-good and DCO signed. The focused source-built exact-commit gate passed one fresh asset fixture and 6/10 proof-required asset APIs recorded/observed; receipt `${LOCAL_EVIDENCE}/compact-focused-4eb758af-asset/receipt.json`. A frozen Nix package and broad integrated-head gate are deferred to the parent branch. No push or remote CI.

Source-built compiler inventory at signed `4eb758af`: `${LOCAL_EVIDENCE}/compact-4eb758af-source-inventory.json` reports 281/316 proof-required APIs available, 35 known gaps, 25 unassessed exports, and zero unmatched compiler circuits. This is one more available API than the integrated 957bd577 base. The parent will freeze its exact packaged compiler after cherry-pick.

### Parent integration

Integrated signed commit 6e8f82f7. Exact combined f72e22ab Nix package built successfully at ${HISTORICAL_NIX_STORE}/fpysnwqwsf5ghf4ixkdlxx50jkx29lzy-compactc. Package-backed repository inventory ${LOCAL_EVIDENCE}/compact-f72e22ab-inventory.json reports 284/317 proof-required available, 33 known gaps, 25 unassessed exports, zero unmatched compiler rows. All 116 renderer tests pass. The complete local gate is still running at this head; no remote CI or push.
