---
id: RUST-ADR-0158
alias: ADR-0158
title: "Record authority guarded optional topic and enum phase advancement"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "authorization", "optional-cell", "enum"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 04a3f1847f1c3c0cbea844f41f6349f0dfd9bbf3e95bf4b1b73aab77e404aab9
---
# RUST-ADR-0158 — Record authority guarded optional topic and enum phase advancement

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted shared typed authority-prefix composition with an optional topic read and closed pure enum successor. Eight empty/Unicode behavior cases and all four Unicode transition proofs include final-to-final no-op accounting. Arbitrary helper effects and voting flows remain outside this slice.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#262 closure](https://github.com/MediaNoxLabs/compact/issues/262#issuecomment-6017673782). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`28b4db98`](https://github.com/MediaNoxLabs/compact/commit/28b4db98a8fb9224627edb5bba69846449bc9153). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
Original election advance executes natively but lacks recorded and observed-call APIs. It authorizes a Bytes32 secret/hash against authority, requires an optional topic, reads the enum state, computes its pure successor and writes it. Source ordering and private transcript must be preserved.

### Before / after
```rust
// Before: native only
ledger_contract::advance(context, &witnesses)?;
// After: proposed proof-ready facade
let generated = ledger_contract::Contract::from(witnesses);
generated.recording().advance(context)?;
generated.recording().advance_call(&observed, private)?;
```

### Decision
Share the typed authority prefix with ADR0155, then match a closed optional Boolean/string Cell projection and a pure enum successor with exact declaration/formal provenance. Do not match contract/field/type names. Keep witness → hash → authority read/assert → topic read/assert → enum read → successor → enum write order. Reuse ledger8 recording primitives; no schema/ABI changes.

### Required evidence
Independent TS/native/recorded/private/gas/VM/replay for setup→commit, commit→reveal, reveal→final, final→final. Empty but present topic accepted; absent topic and wrong authority rejected. Pinned proof verified/applied through ledger8, structural mutation rejection guards, fixture freshness and Clippy.

### Limits
No implicit support for arbitrary effectful successor helpers or optional projection shapes. Voting and add_voter remain separate.


### Delivery — 2026-10-05
Issue #262: https://github.com/MediaNoxLabs/compact/issues/262. Signed local `cfb60ee62007a016c122872870db4c79cc9e57f8`, GPG G and DCO, based on main `28b4db98`.

#### Emitter and runtime
`AuthorizedContinuation` holds validated authority prefix statements and remaining source actions. Both ADR0155 optional writes and ADR0158 enum advancement share it. Continuation checks exact optional Cell Boolean/string layout, member index/name, exact enum slot and same-slot write, pure internal successor signature, and recursively closed enum decision tree. Extra helper assertions, changed source order or bad indexes reject recording. Renaming all relevant circuit/helper/witness/field/type/member/variant names stays supported. Existing pure emitter renders enum semantics; ledger8 Cell/witness recording primitives are reused unchanged. No runtime, schema, ABI or frontend change.

#### Evidence
Fresh independent TS captures from schema13 frontend cover eight cases: empty and Unicode topic, each setup→commit, commit→reveal, reveal→final, final→final. Exact native/recorded state, 4-query/12-op VM sequence, four gas dimensions, witness atoms/alignment and private state, replay state/effects/gas agree. Empty but present topic passes; absent topic and wrong authority reject with TS messages. Final→final has zero bytes written/deleted and lower compute gas, proving the read/write no-op path is measured rather than assumed.

Pinned ZKIR 2.1.0 compiled advance at k=13 / 4192 rows. All four Unicode-topic transitions independently replayed/partitioned, verified, deployed and applied through ledger8. Typed observed-call preparation equals manual preparation for each. Artifacts `${LOCAL_EVIDENCE}/compact-adr158-rust`, selector `--election-advance`; integrated broad proof harness includes both advance and set_topic.

152 fixtures fresh; 128 renderer tests; 3 election tests; targeted backend/election/proof all-target/all-feature Clippy pass. Both original examples/election.compact and oracle now report 2/5 recorded, with advance and set_topic available. Root will update the original-source cohort manifest at integration to require advance positively; this isolated branch preserves its old manifest to avoid conflict with the newly introduced cohort assertion format.

#### Limits
add_voter and both voting calls remain separate recording slices. This slice admits only the closed typed optional-read / enum decision tree; arbitrary helper effects remain unavailable. No push or remote CI.


Focused exact-head gate passed at `cfb60ee6`: `${LOCAL_EVIDENCE}/compact-focused-cfb60ee6-election/receipt.json`, one fixture / 2 of 5 recorded, Rust 1.99.0.
