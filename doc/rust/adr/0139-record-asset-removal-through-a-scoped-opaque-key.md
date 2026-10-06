---
id: RUST-ADR-0139
alias: ADR-0139
title: "Record asset removal through a scoped opaque key"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "opaque-string", "map", "asset-registry"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 5501fa827e8abec4d11de360de8510ca5361e7641c3f70fe7f4033653f3d1d0c
---
# RUST-ADR-0139 — Record asset removal through a scoped opaque key

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted scoped opaque-key binding only for the audited asset-removal authorization, membership, mutation and recordWrite sequence. Successful removal is proved from seeded registry state; missing/watched failures and key/order mutants preserve the boundary. Query execution aggregates and independent replay costs remain separately compared.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#241 closure](https://github.com/MediaNoxLabs/compact/issues/241#issuecomment-6017639951). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`05f54b60`](https://github.com/MediaNoxLabs/compact/commit/05f54b606b19e73f9181a1f38adc7e4fa06897db) · [`cc2d0668`](https://github.com/MediaNoxLabs/compact/commit/cc2d06689a382d60bfc5c5bb24e2d95521e92426) · [`d9fbd5d9`](https://github.com/MediaNoxLabs/compact/commit/d9fbd5d9b6e419303b84f417feeaa9ecdfacc19e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0139 — Record asset removal through a scoped opaque key
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The ledger-8 asset registry exports `removeRecord(recordId: Opaque<"string">)`. Schema-12 IR wraps its ordered assertions and Map/Set mutations in a `StateAction::Let` that binds `disclosedId` to the typed `recordId` parameter. The current Rust recorder rejects the outer Let at `actions[0]`, so the generated crate has only a native call and no proof-capable recorded or observed call. The six remaining asset registry proof gaps share this outer shape, but their nested operations have different risks.

### Before and after

```compact
export circuit removeRecord(recordId: Opaque<"string">): [] {
  const disclosedId = disclose(recordId);
  assertWritable();
  assert(records.member(disclosedId), "record does not exist");
  assert(!watchList.member(disclosedId), "record is still watched");
  records.remove(disclosedId);
  retiredKeys.insert(disclosedId);
  recordWrite();
}
```

Before, `ledger_contract::remove_record` works natively while `ledger_contract::recorded::remove_record` and its observed-call API are absent. After, a generated recorded call accepts the same typed opaque key, executes the six nested actions in order through typed ledger slots, and yields an ordered Verify program for proof. The Rust source should retain a scoped `OpaqueString` binding from the original parameter; no string conversion, field hashing, or eager flattening of nested actions is allowed.

### Decision and guards

Admit only a single `OpaqueString` Let binding sourced from a parameter whose declared type is also `OpaqueString`, then recursively validate nested actions. For this delivery, the public opaque-input gate admits only the exact asset-removal shape: `assertWritable`, record membership assertion, negated watch-list membership assertion, Map removal, retired-key Set insertion, and `recordWrite`, with a Unit result. Field names, physical indices, callee identity and key provenance must be checked against typed IR and ledger declarations. Other opaque Let shapes stay unavailable until separately proved.

No new runtime primitive is intended. Reuse ledger-8 Map/Set recording slots and existing `OpaqueString` Rust mapping. Schema 12 and runtime ABI remain unchanged.

### Acceptance

Capture fresh original TypeScript behavior and compare generated native/recorded Rust result, serialized state, all four gas dimensions, ordered VM operations, private effects, and independent Verify replay. Exercise success plus missing-record and watched-record negative guards. Compile and test the fresh generated crate. Prove and verify its observed call with pinned ZKIR 2.1.0, then ledger-8 validate/apply. Add a renderer positive/negative guard, exact-head capability report, and focused local gate. Record any limitation honestly. No push or remote CI.

### Delivery

Decision recorded before implementation. Tracking issue and signed local commit to follow.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/241 (rust-backend-v2). Created before implementation.

### Local verification (2026-10-05)

- Original ledger-8 TypeScript capture: `runtime-rs/tests/fixtures/asset-remove-record.json`, generated from `asset_registry_oracle.compact` with exact compiler package `${HISTORICAL_NIX_STORE}/28fvvy35kci7azn03y0nzp7j7g31mp71-compactc`. The removal emits 36 ordered VM operations across nine queries, one timestamp private output, and the expected missing-record and watched-record assertion failures.
- Generated native and recorded Rust removal match the original TypeScript serialized state, all four gas dimensions summed across the nine queries, ordered VM operations, private output, and private state. A separate flattened TypeScript replay matches Rust Verify replay state, effects and four replay gas dimensions. TypeScript `output.gasCost` reports its last query; the test intentionally compares the per-query sum for the complete call.
- The generated observed `removeRecord` call uses pinned ZKIR 2.1.0 prover/verifier and BZKIR artifacts. The proof was produced and verified, then a ledger-8 transaction validated and applied. The applied state no longer has the record, contains the retired key, and has revision/writeCount equal to two after the seeded insertion and removal.
- 110 renderer tests, all seven asset fixture tests, targeted all-target Clippy, and 147/147 generated fixture freshness passed. The negative renderer guard rejects reordered mutations and key provenance changes. The other five asset registry recording gaps remain gated.
- Conventional local commit `f2b6c533` is GPG-good and DCO signed. Issue #241 tracks delivery. Exact-head packaged compiler inventory and focused gate are running. No push or remote CI.

### Exact signed-head checkpoint

The frozen `${HISTORICAL_NIX_STORE}/s3s9mglf1y2gj1hwdvqb47n8b84hgpx2-compactc` package was built from signed `f2b6c533`. The exact-head focused gate passed with one fresh asset fixture, 5/10 proof-required asset calls recorded/observed, clean working tree, and receipt `${LOCAL_EVIDENCE}/compact-focused-f2b6c533-asset/receipt.json`. Compiler-attributed inventory `${LOCAL_EVIDENCE}/compact-f2b6c533-inventory.json` reports 278/316 proof-required APIs available, 38 known gaps, 25 unassessed exports, zero unmatched compiler circuits. This is +1 available API from the 05f54b60 base. The broader full gate will be run after branch integration.

### Integrated checkpoint (2026-10-05)

Signed GPG/DCO agent commit `f2b6c533` cherry-picked cleanly as `cc2d0668`. Combined root head passed 111 renderer tests, seven asset fixture tests, targeted all-target Clippy and formatting. Exact `${HISTORICAL_NIX_STORE}/7s8wnpi54phg2lff0hwl52m0ia45rd7i-compactc` compiler focused gate `${LOCAL_EVIDENCE}/compact-focused-cc2d0668/receipt.json` passed one fresh source, 5/10 asset calls recorded. Repository inventory `${LOCAL_EVIDENCE}/compact-cc2d0668-inventory.json`: 279/316 proof-required available, 37 known gaps, 25 unassessed, 0 unmatched. Rebuilt full asset proof artifacts from this integrated compiler; generated `removeRecord` observed call replayed, proved, verified, validated and applied in ledger-8. Broader full gate remains the `d9fbd5d9` checkpoint pending later integrations. No push or remote CI.
