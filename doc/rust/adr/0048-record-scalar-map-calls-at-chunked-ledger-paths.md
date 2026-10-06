---
id: RUST-ADR-0048
alias: ADR-0048
title: "Record scalar Map calls at chunked ledger paths"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: a1c0c7ffbdd26b3c4fc23c771a397a9b56e250e8e9c8b848872d3787881523af
---
# RUST-ADR-0048 — Record scalar Map calls at chunked ledger paths

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept scalar Map recording at compiler-assigned chunked paths by removing only the root-depth restriction. Existing declaration, key/value and expression checks and canonical VM programs remain authoritative. This does not authorize complex nested Map values or unrestricted helper expressions.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#147 closure](https://github.com/MediaNoxLabs/compact/issues/147#issuecomment-6017478848). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`b9c66264`](https://github.com/MediaNoxLabs/compact/commit/b9c6626466ab8b45fdbfaa573dc916d7b508badd). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 48
status: accepted-partial
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/147
```

## Historical decision and amendments

### Problem and evidence

A real ledger-8 Compact contract with fifteen scalar declarations followed by `table: Map<Boolean, Field>` gives the Map physical path `[1,14]`. Packaged ABI-25 `compactc --target rust --skip-zk` accepts the source and emits `MapSlot<bool, Field>::new(&[1,14])` and native methods. It emits no recorded module or observed call builders. The AST recording emitter still checks `physical_path().len() == 1` for Map insert/default/remove/reset, member/lookup/size/isEmpty returns, and nested lookup/member expressions. The runtime `MapSlot`, `CircuitContext`, `RecordingFrame`, metered view and shared ledger-8 Map/Set VM builders already take complete paths. Suppression is therefore an emitter acceptance gap, not a missing primitive.

### Before and proposed after

Before:

```rust
assert_eq!(ledger_slots::table.path(), &[1, 14]);
let native = contract.put(context, true, Field::from(7))?;
// contract.recording.put(...) and put_call(...) are absent.
```

Proposed after:

```rust
let recorded = contract.recording.put(context, true, Field::from(7))?;
let call = contract.recording.put_call(&observed, (), true, Field::from(7))?
    .prepare(verifier, randomness)?;
```

### Decision proposal and alternatives

Permit complete scalar Map recorded bodies at a valid compiler-assigned chunked path by removing only the Map-specific root-depth checks. Keep declaration index, key/value type, supported-expression and return-shape validation. Continue to lower through typed `MapSlot<K,V>`, which owns the fixed key/value types and physical path. Reuse the existing Map insert/lookup builders and Set-derived member/remove/reset/size/emptiness programs; do not introduce a second VM sequence or hand-written raw-op generated code. A manual low-level frame workaround would expose VM plumbing to generated-crate users and leave observed calls absent. No new macro or FAB representation is justified.

The private IR schema already holds physical paths and should stay 8. Generated public methods expand, so bump the generated/runtime ABI from 25 to 26 if delivered. Root Map output and low-level `u8` callers remain compatible. This proposal does not broaden nested-Map value mutations; `MapNode` remains a structural marker.

### Emitter/runtime ownership and verification plan

`tools/compact-rust-backend/src/recorded.rs` owns eligibility and typed method emission. `tools/compact-rust-backend/src/lib.rs` owns ABI and slot declarations. `runtime-rs/src/slots.rs` and `runtime-rs/src/ledger/collections.rs` already own pathful Map behavior; amend only if TypeScript/ledger evidence reveals a semantic mismatch. Keep upstream ledger-8 `Op`, `QueryContext`, FAB and cost model authoritative.

Check in a sixteen-declaration source with a constructor-seeded Map and all scalar operations: insert, insertDefault, member, lookup, remove, size, isEmpty, reset, and an ordered multi-insert call. Capture TypeScript results, serialized state, input atoms/alignment, four-dimensional query gas and ordered public transcript shape. Compare Rust native, recorded and replayed results/state/effects/gas. Check a one-dependency generated consumer and wrong-typed key/value rejection. Run manual/generated exact pre-proof parity plus independent proof, verify, validate and apply for every call. Keep root Map and all existing fixtures green; report source-size and compile-time impact honestly. A fresh packaged compiler, release archive rehearsal, remote CI and registry/wallet gates are separately reported.

### History

- Source probe: ignored `target/chunked-map-probe.compact`; packaged ABI-25 compiler emits `MapSlot<bool, Field>` at `[1,14]` and native methods but no `recorded` module.
- Related: ADR-0029/0030 for typed Map keys, ADR-0046/0047 for chunked Set/List, parent #104 for acceptance and #105 for call delivery.
- Proposal only. Append a dated implementation and verification amendment with the signed/DCO commit; do not rewrite the original rationale.
### Implementation and verification amendment — 2026-10-04

Decision accepted for scalar Map calls at compiler-assigned chunked paths. Signed/DCO local commit `b9c6626466ab8b45fdbfaa573dc916d7b508badd` removes only Map-specific one-element path eligibility checks in `recorded.rs`; index, key/value type, supported expression and return-shape guards remain. Generated/runtime ABI is 26; private IR schema stays 8. No new runtime VM program, ledger primitive, FAB encoder or macro was needed: generated `MapSlot<bool, Field>::new(&[1, 14])` carries the compiler path into existing runtime and ledger-8 builders. Nested-Map value mutation remains out of scope.

The delivered developer-facing API is:

```rust
let recorded = contract.recording.put(context, false, Field::from(7_u64))?;
let prepared = contract.recording
    .put_call(&observed, (), false, Field::from(7_u64))?
    .prepare(verifier, randomness)?;
```

The checked-in sixteen-declaration `chunked_map.compact` covers constructor seed, put, ordered two-insert put_pair, insertDefault, member, lookup, remove, size, isEmpty and reset. TypeScript ledger-8 capture and Rust fixture match returned values, serialized contract state, ordered public transcript, input FAB atoms/alignment for multi-parameter calls, and all four per-query gas dimensions. Rust native, recorded, replayed and metered witness views agree. A one-dependency generated consumer passes all nine observed methods; wrong key and value types are rejected with E0308. The fixture also rejects a neighboring scalar path.

Fresh ABI-26 packaged `compactc` at `${HISTORICAL_NIX_STORE}/2bvkn2m1y366qhqa1avhn5k6w7k2l3bs-compactc` passed the complete generated consumer/manifest gate with **85 independently proven, verified, validated and applied calls**, including all nine chunked Map circuits. Manual and generated call bytes match exactly before proof: put 544, put_pair 630, put_default 559, has 500, get 524, remove_key 503, table_size 493, table_is_empty 530 and reset_table 497 bytes. The focused fixture, 57 renderer tests, all-features workspace check, fresh 136-fixture outputs and `cargo fmt --all -- --check` pass. Generated chunked Map source is 810 lines / 35,936 bytes; root Map is 708 lines / 30,406 bytes, but the chunked contract has fifteen additional declarations and a constructor, so this is not an optimization comparison. Compile-time impact was not measured.

The runtime macro and runtime crate archives package and verify with 9 and 245 entries respectively; a second run reproduced `target/rust-runtime-release-abi26.json` at commit `b9c66264`. The manifest reports dirty source solely because the pre-existing user-owned `doc/ledger-adt.mdx` change remains unstaged. The branch is local/unpushed. Remote CI, a clean signed release, registry publication, wallet/node submission and broader complex Map shapes remain open; issue #147 stays open until its external gates are met.
