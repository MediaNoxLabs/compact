---
id: RUST-ADR-0146
alias: ADR-0146
title: "Record guarded opaque-key struct Map reads"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "map", "opaque-string", "typed-helpers"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 96e9b3acc843fe53385468b6b9eb8c0ebcbc5530cf790921274b2bf6e5f37b64
---
# RUST-ADR-0146 — Record guarded opaque-key struct Map reads

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the shared typed membership-guarded struct Map-read mechanism with full closed pure-body audit, preserving the earlier custody-grant path. Nonempty opaque notes demonstrably decode by alignment atoms and pass the proof flow, correcting the prior erroneous comment; this is not unrestricted opaque decoding or helper admission.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#249 closure](https://github.com/MediaNoxLabs/compact/issues/249#issuecomment-6017652868). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4360d651`](https://github.com/MediaNoxLabs/compact/commit/4360d65180e3626a27e3128bc2610200f359f575). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0146 — Record guarded opaque-key struct Map reads
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The ledger-8 asset registry exports `assertStoredRecordFresh(recordId, policy, currentTime)` as a proof-required call. Schema-12 IR binds an `OpaqueString` key, checks membership in `records`, binds an `AssetRecord` from that same Map, then calls the pure `assertRecordFreshEnough` assertion. Generated Rust has only a native call because the recorder rejects the nested struct Map lookup Let at `actions[0].action.actions[1]`. ADR-0141 added the same pipeline for `CustodyGrant` with a contract-name-specific matcher; copying that matcher would make the emitter harder to maintain.

### Before and after

Compact source:

```compact
export circuit assertStoredRecordFresh(recordId: Opaque<"string">, policy: FreshnessPolicy, currentTime: Uint<64>): [] {
  const disclosedId = disclose(recordId);
  assert(records.member(disclosedId), "record does not exist");
  const record = records.lookup(disclosedId);
  assertRecordFreshEnough(policy, record, disclose(currentTime));
}
```

Before, only the native Rust call is available:

```rust
ledger_contract::assertStoredRecordFresh(context, id, policy, now)?;
// ledger_contract::recorded::assertStoredRecordFresh is unavailable.
```

After, the generated crate also provides the typed recorded and observed proof calls:

```rust
let recorded = ledger_contract::recorded::assertStoredRecordFresh(context, id.clone(), policy.clone(), now)?;
let call = contract.recording().assertStoredRecordFresh_call(&observed, private_state, id, policy, now)?;
```

The emitter records Map membership and typed `AssetRecord` lookup through the ledger-8 slot, then invokes the generated pure Rust guard. A successful read changes no state.

### Decision and guards

Refactor ADR-0141 into a shared typed guarded Map-read mechanism. Admit only one scoped `OpaqueString` parameter alias; an ordered membership assertion and struct Map lookup on the same declared field, physical index and key; and one pure Unit assertion call. Match every pure argument to either the looked-up typed struct or an explicitly typed public parameter. Audit the complete pure AST with a small closed whitelist of scalar parameters, struct projections, comparisons, bounded subtraction, local bindings, `if`, assertions and Unit. Reject calls, witness reads, ledger reads, hashes, extra actions, changed key provenance, mismatched Map types, or effectful pure bodies. Preserve the original assertion messages and operation order.

The runtime keeps ownership of encoding and decoding through the existing ledger-8 Map slot. The source comment claims a nonempty `AssetRecord.note` fails composite decoding, but it points to a nonexistent decoder. The actual `CompactCellValue` derive slices by alignment atoms and gives `OpaqueString` one Compress atom. Probe both empty and nonempty notes before declaring operation parity; correct the comment if the nonempty path succeeds, or document and guard the real failure if it does not. This decision does not change schema 12 or ABI 37.

### Acceptance

Capture fresh original TypeScript; compare native Rust, recorded Rust and independent Verify replay across result, serialized state, ordered VM, all four gas dimensions, private effects, and missing/future/stale record guards. Probe nonempty note explicitly. Add renderer positive/negative guards and verify ADR-0141 grant behavior remains available. Refresh the generated fixture. Prove and verify the observed call with pinned ZKIR 2.1.0, then ledger-8 validate/apply from a seeded record state. Update the full gate expectation, run focused local checks, and publish one conventional GPG+DCO signed local commit. No push or remote CI.

### Delivery

Decision and [MediaNoxLabs/compact#249](https://github.com/MediaNoxLabs/compact/issues/249) were recorded before implementation. The shared matcher replaces the custody-grant-specific branch. It accepts the original grant and the stored record freshness call by typed provenance, complete pure-body audit, and operation order. Negative IR mutations of lookup key and extra pure work remain unavailable. The generated fixture includes the new recorded and observed APIs.

The nonempty-note probe succeeded. The generated `CompactCellValue` decoder consumes alignment atoms, so the `AssetRecord.note` value `"nonempty note"` is preserved in native Map lookup and the recorded proof flow. The old source comment named a nonexistent decoder and asserted the opposite; it has been corrected. The fresh TypeScript capture exercises the same nonempty note and compares native, recorded and Verify replay state, ordered VM operations, four gas dimensions, private effects, and missing/future/stale guards.

Local evidence: 117 renderer tests and 9 asset-registry tests passed; all 148 generated fixture outputs match; targeted backend, fixture, and proof-smoke Clippy passed with warnings denied. Pinned ZKIR 2.1.0 proof output `${LOCAL_EVIDENCE}/asset-adr146-proof` successfully proved, verified, validated, and applied the observed `assertStoredRecordFresh` call through ledger 8. Exact-head focused receipt `${LOCAL_EVIDENCE}/compact-focused-2349b83d-asset/receipt.json` passed with 7/10 asset operations recorded and a clean working tree. The local conventional DCO commit is `2349b83d7511af0ea2b94a966c347cb00f6b4056`; `git verify-commit` reports a good GPG signature. No push or remote CI.



### Main integration evidence — 2026-10-05

Main4360d651 passed the affected consumer/proof/ledger phase at ${LOCAL_EVIDENCE}/compact-consumer-4360d651/receipt.json, including this integrated slice. This closes the phase interrupted by the earlier stack overflow/harness TypeError. It does not claim a whole-workspace rerun at4360. Both Merkle fixtures additionally pass16/16 recorded at ${LOCAL_EVIDENCE}/compact-focused-4360d651/receipt.json; main renderer122 passes.
