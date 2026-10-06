---
id: RUST-ADR-0136
alias: ADR-0136
title: "Record typed literal Merkle index bindings"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merkle", "lexical-scope"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 3a405de43c1880b950a8aa51a1b55dae66ff5f3322219b48a1f31fc46dd8172f
---
# RUST-ADR-0136 — Record typed literal Merkle index bindings

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted scoped exact Uint64 literal bindings composed with existing indexed Merkle recording, without a source-name bypass or new primitive. Original replacement parity/proof and the later integrated full local gate close this bounded decision; their inventory and gate cohorts remain distinct historical counts.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#238 closure](https://github.com/MediaNoxLabs/compact/issues/238#issuecomment-6017635007). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`05f54b60`](https://github.com/MediaNoxLabs/compact/commit/05f54b606b19e73f9181a1f38adc7e4fa06897db) · [`967eee59`](https://github.com/MediaNoxLabs/compact/commit/967eee596f0fe732911c6de1e90210c571dd0543) · [`ca1798a6`](https://github.com/MediaNoxLabs/compact/commit/ca1798a6cc4e52da17050c43ca5e91e1c42f572b) · [`d97b7c48`](https://github.com/MediaNoxLabs/compact/commit/d97b7c4865e4887bd758188c6f20be548b1c1129) · [`d9fbd5d9`](https://github.com/MediaNoxLabs/compact/commit/d9fbd5d9b6e419303b84f417feeaa9ecdfacc19e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0136 — Record typed literal Merkle index bindings
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original `merkle_path_verify.compact` exports `replace(value: Uint<8>)` as a proof call. Its schema-12 action is one `StateAction::Let`: bind `tmp_10: Uint<18446744073709551615> = 0`, then `MerkleInsertIndex(t, value, tmp_10)`. The Rust recorder already supports a typed indexed Merkle insert through the ledger-8 `MerkleSlot`, but rejects the enclosing pure literal binding. The exact packaged 967eee59 compiler marks only this source call unavailable at `actions[0]`; `append` and `verify` are available.

### Before and after

Compact source:

```compact
export circuit replace(value: Uint<8>): [] {
  t.insertIndex(disclose(value), 0);
}
```

Before, the generated crate has the native `replace` method but no `recorded::replace` or typed observed-call API. After, the recorded method retains the compiler-typed Uint64 position and calls the existing slot operation:

```rust
let position: runtime::BoundedUint<18446744073709551615> =
    runtime::BoundedUint::new(0)?;
let frame = ledger_slots::t.record_insert_index(frame, value, position)?;
```

The actual generated code may name the local differently; the type, value, ordered VM operation and proof observation are contractual.

### Decision

Extend the compositional `StateAction::Let` recorder to admit an exact, typed `UnsignedLiteral` binding at `Uint<18446744073709551615>`, using the existing `cell_source` type check and a scoped local. The nested action must still pass ordinary typed recording validation. Do not special-case the Merkle source name or bypass slot validation. Reject mismatched bounds, extra effects, wrong leaf types and unsupported nested operations.

No new runtime primitive is needed. Schema 12 and runtime ABI 37 remain unchanged. The existing ledger-8 indexed Merkle slot remains the sole VM operation.

### Evidence required before acceptance

Capture original TypeScript and compare generated native/recorded Rust result, serialized state, four gas dimensions, ordered VM, private effects and Verify replay. Prove, verify and ledger-8 validate/apply the generated typed observed call with pinned ZKIR 2.1.0. Add a renderer negative guard, fresh generated fixture, exact-source capability scope, and focused local gate. A full exact-head gate may follow later integrations. The expected inventory gain is one proof-required available API; verify the actual compiler-attributed inventory rather than assuming it.

### Delivery

Proposed before code. Issue and signed local commit to be linked after creation. No push or remote CI.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/238 (rust-backend-v2). Created before implementation.

### Local verification (2026-10-05)

- Original TypeScript capture: `runtime-rs/tests/fixtures/merkle-literal-index.json`, produced by `capture-merkle-literal-index.mjs` from `merkle_path_verify.compact` through the baseline ledger-8 TypeScript compiler. The replacement capture has 18 ordered VM operations, no private outputs, gas read 935000000 / compute 2277002266 / bytes written 436 / deleted 436.
- Generated native and recorded Rust replacement match the TypeScript serialized state and gas. Recorded VM program equals all 18 TypeScript operations. Replay returns the same state, effects and gas. Three fixture tests and the positive/negative renderer test pass.
- Full Rust target compilation produced `replace` prover/verifier and ZKIR/BZKIR. The existing Merkle smoke path proves both `verify` outcomes and the new recorded `replace` call with pinned ZKIR 2.1.0; ledger-8 validation and apply pass, with the applied state equal to native replacement.
- Fresh compiler capability output reports `append`, `replace`, and `verify` all recorded and observed. No runtime ABI change. Exact integrated-head package and focused gate follow after branch integration.


### Signed delivery and integrated checkpoint

Feature commit `d97b7c48` and Clippy cleanup `05f54b60` are conventional, GPG-good and DCO signed. Issue #238 tracks the slice. The combined head `05f54b60` includes ADR-0134 and ADR-0135 integrations too; exact `${HISTORICAL_NIX_STORE}/28fvvy35kci7azn03y0nzp7j7g31mp71-compactc` compiler inventory reports 277/316 proof-required APIs recorded and observed, 39 known gaps, 25 unassessed exports, 0 unmatched compiler circuits. Focused three-source gate at `ca1798a6` passed 3 fresh fixtures and 7/7 recorded calls; an exact `05f54b60` full local gate is running. Targeted Clippy passes at `05f54b60`. No push or remote CI.


Delivery evidence was posted to [issue #238](https://github.com/MediaNoxLabs/compact/issues/238#issuecomment-5989577910). The broad local gate remains in progress; do not treat this as a production-ready milestone closure.


Full integrated local gate passed at signed head `d9fbd5d9`: `${LOCAL_EVIDENCE}/compact-full-d9fbd5d9/receipt.json`, 147 fresh fixtures plus workspace, packaged consumer and proof/ledger checks. The selected gate cohort is 267/296 proof-eligible recorded; repository-wide inventory is 277/316. This closes ADR-0136 local acceptance, while milestone 2 retains known and unassessed backlog.
