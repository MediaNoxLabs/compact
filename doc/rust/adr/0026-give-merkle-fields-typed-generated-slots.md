---
id: RUST-ADR-0026
alias: ADR-0026
title: "Give Merkle fields typed generated slots"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts", "merkle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 79261f899f5ad0de8879e7c79c9224022c300ada8576ed8b879cf8aeb3dc1d36
---
# RUST-ADR-0026 — Give Merkle fields typed generated slots

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed plain/historic Merkle slot declarations carrying path, leaf type and depth. Later native gas and complete selected VM-program comparisons refine the initial evidence. Slot readability and native parity do not alone establish recording, proof coverage or every Merkle mutation.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#126 closure](https://github.com/MediaNoxLabs/compact/issues/126#issuecomment-6017442473). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`2cbf2ca5`](https://github.com/MediaNoxLabs/compact/commit/2cbf2ca57af043898929f0dd61bef49956762da7) · [`70564c42`](https://github.com/MediaNoxLabs/compact/commit/70564c424b154ddf51fa5bda7ab748ae3a7b74dd) · [`cfa6e46c`](https://github.com/MediaNoxLabs/compact/commit/cfa6e46cbd9246d49ccf49bc6edc4c05bed1f2cb) · [`d6576650`](https://github.com/MediaNoxLabs/compact/commit/d6576650ab8be8e47509f06d9476a15cb25dce9e) · [`ef9ed3b6`](https://github.com/MediaNoxLabs/compact/commit/ef9ed3b696d7b7c56208c9cf4876bb0478abd1a9) · [`fba11faf`](https://github.com/MediaNoxLabs/compact/commit/fba11fafeb3222772a0702a211548db829a0ae77). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 26
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: 126
```

## Historical decision and amendments

### Problem

The ledger-8 IR knows each plain or historic Merkle field's leaf type, physical path and depth. Generated native Rust still repeats numeric paths and depth at every operation. A developer viewing the generated crate cannot find a named descriptor for a Merkle declaration, unlike Cell, Counter, Set, Map and List. This also leaves path/type relationships dispersed across emitted statements.

### Before and proposed after

```rust
// Current generated native body.
let step = context.merkle_insert(0, __compact_param_0)?;
let read_step = context.merkle_is_full(0, 3)?;
```

```rust
// Proposed public generated declaration and native body.
pub const tree: runtime::slots::MerkleSlot<runtime::BoundedUint<255>, 3, false> =
    runtime::slots::MerkleSlot::new(&[0]);
let step = crate::ledger_slots::tree.insert(context, __compact_param_0)?;
let read_step = crate::ledger_slots::tree.is_full(context)?;
```

The const generic depth and historic flag are compile-time facts. A historic-only `reset_history` method is available only for `MerkleSlot<T, DEPTH, true>`. `insert` consumes the declared `T`; hash insertion consumes `FixedBytes<32>`; root checks accept the validated Compact digest type. The runtime forwards to the existing ledger-8 `CircuitContext` methods and does not reimplement Merkle semantics.

### Emitter and runtime ownership

`tools/compact-rust-backend/src/lib.rs` emits one `ledger_slots` declaration from the typed IR. `stateful.rs` uses the name for native Merkle writes and reads while retaining declaration and argument type checks. `runtime-rs/src/slots.rs` owns the typed forwarding methods. No new VM operation, private IR schema change, proof artifact or macro is proposed. A public runtime/generated crate API grows, so bump the generated/runtime ABI from 11 to 12 and run the package compatibility gate. Recorded Merkle entry points remain unsupported until their full trace can be emitted; this ADR does not change exposure.

### Alternatives and tradeoffs

Keeping numeric calls requires less runtime surface but hides the compiler's type/path/depth facts. A macro for Merkle statements would obscure generated code and diagnostics. Separate plain and historic slot structs would expose the distinction in names but duplicate the common forwarding API; one const flag keeps the common semantics in one type while a specialized impl restricts historic-only operations. Public slot constructors remain a typed convenience, not an access-control boundary (ADR-0002).

### Acceptance and delivery

Use compiler-backed plain and historic Merkle fixtures, renderer tests and an external consumer calling the slot. Check wrong leaf type at compile time. Compare generated native results, private state/FAB order, four-dimensional gas and ledger-8 VM behavior against unchanged TypeScript captures. Run fixture freshness, oracle/rejection, runtime/backend/workspace, packaged consumer/proof/application and clean-source release-manifest gates. Record source size and any generated API break. Keep remote CI, publication and wallet/node submission as explicit milestone gates.

Tracking: parent [#108](https://github.com/MediaNoxLabs/compact/issues/108) and rust-backend-v2. Implementation and evidence will be appended; this proposal claims no delivered code.



### Delivery amendment — 2026-10-03

Local conventional GPG-signed/DCO commit `ef9ed3b696d7b7c56208c9cf4876bb0478abd1a9` delivers the native typed slot slice on `codex/rust-backend-ast`. The generated plain-tree fixture now contains:

```rust
pub const t: runtime::slots::MerkleSlot<runtime::BoundedUint<255>, 3u8, false> =
    runtime::slots::MerkleSlot::new(&[0u8]);
let step = crate::ledger_slots::t.insert(context, __compact_param_0)?;
let read_step = crate::ledger_slots::t.is_full(context)?;
```

The historic fixture uses `MerkleSlot<BoundedUint<255>, 3u8, true>` and calls `reset_history`. `stateful.rs` routes native insert, indexed/default/hash insert, reset, fullness and root checks through these slots. `runtime-rs/src/slots.rs` forwards to the existing `CircuitContext`/ledger-8 methods; no VM instruction implementation or recorded Merkle exposure changed. The compiler IR remains schema 8. The new public slot surface bumps generated/runtime ABI 11→12, with compile-time assertions in all generated crates. `MerkleSlot::check_root` accepts any runtime `CellValue`; generated native calls still validate the exact Compact `MerkleTreeDigest` shape before emission. Public constructors remain unsealed.

**Evidence:** 54 backend renderer tests and four CLI tests; full runtime suite; four focused plain/historic Merkle fixture packages matching pinned TypeScript state bytes; all-target workspace check; 132 fresh compiler fixtures; 37 pinned source inventory; two rejected unsupported programs with source diagnostics. A separate one-dependency consumer compiled a typed `insert`, rejected `bool` instead of `BoundedUint<255>` with E0308, and rejected plain-tree `reset_history` with E0599. The packaged `compactc --target rust --consumer --proof` gate replayed, partitioned, proved, verified, validated and applied its 52 existing recorded call cases on ledger 8. It is a regression gate: Merkle circuits remain native, so this is not a claim that Merkle calls are proved. `cargo fmt --check` and staged diff checks passed. Clean-source package rehearsal produced and verified `target/rust-runtime-abi12-clean.json`, bound to commit `ef9ed3b6`, tree `2536d273`, `dirty=false`, with 8 macro and 229 runtime archive entries. The unrelated `doc/ledger-adt.mdx` edit was preserved byte for byte and excluded from the commit.

**Source-size impact:** Merkle tree fixture 279→290 lines, historic insert 277→289, historic default 93→99; unchanged Merkle multi-path 181→181, tiny 540→540 and passport 4,532→4,532. Election grows 1,123→1,128 and zerocash 770→774 because their Merkle declarations gain named slots. This is a readability/type-boundary change, not a generated-size optimization.

**Limits:** Existing Merkle TypeScript captures pin state/results but not a four-dimensional gas or ordered VM transcript for native Merkle calls. Forwarding to the identical context methods preserves the implementation path, but independent TypeScript gas/transcript parity remains an open acceptance item. No remote CI, branch publication, registry release or wallet/node submission is claimed. Focused [#126](https://github.com/MediaNoxLabs/compact/issues/126) stays open for those gates; parent [#108](https://github.com/MediaNoxLabs/compact/issues/108) retains other typed-slot work.



### Native Merkle gas parity amendment — 2026-10-03

Follow-up local conventional GPG-signed/DCO commit `70564c424b154ddf51fa5bda7ab748ae3a7b74dd` closes the independent **four-dimensional gas** part of the previous limit. It does not change the emitter, runtime, public slot API, ABI 12 or private IR schema 8. The ledger-8 TypeScript capture programs now intercept each `QueryContext.query`, retain ordered operation tags and per-query costs, and label initial `isFull`, initial `checkRoot`, first insert for both plain and historic trees, plus historic `forget_history`. The captures reproduce every earlier state/result oracle field byte for byte; they add only `nativeQueries`. Rust fixture tests sum the captured query costs and compare `readTime`, `computeTime`, `bytesWritten` and `bytesDeleted` with the generated native circuit result. Both focused tests pass. The TypeScript wrapper's reported cost is checked against its final query; these selected calls each currently make one query.

Before this amendment the Merkle fixture asserted serialized state/results but no gas:

```rust
let after_append7 = append(known_init.context, bounded::<255>(7)).unwrap();
assert_state(after_append7.context.query.state.get_ref(), &oracle, "afterAppend7", 1);
```

After it also checks the ledger-8 query sum for every gas dimension:

```rust
let after_append7 = append(known_init.context, bounded::<255>(7)).unwrap();
assert_native_query_gas("append7", &after_append7.gas_cost, &oracle);
assert_state(after_append7.context.query.state.get_ref(), &oracle, "afterAppend7", 1);
```

`tools/compact-rust-backend/oracles/{merkle_tree,hmt_insert}_capture.mjs` owns the TypeScript query capture; the generated fixture tests own Rust comparison. No new runtime method or macro is needed. Focused plain/historic tests, formatting/staged diff checks and a clean-source ABI-12 archive manifest pass. The refreshed manifest binds `target/rust-runtime-abi12-clean.json` to source commit `70564c42`, with `dirty=false`; the unrelated documentation edit was restored byte for byte. Ordered TypeScript operation tags are retained in the fixture for later transcript analysis, but no independent Rust native operation-order comparison is claimed. Recorded Merkle proof, branch publication, remote CI, release and wallet/node submission remain open.



### Packaged-consumer delivery amendment — 2026-10-03

Local conventional GPG-signed/DCO commit `cfa6e46cbd9246d49ccf49bc6edc4c05bed1f2cb` turns the typed Merkle consumer probe into a permanent packaged gate. The earlier emitter/runtime delivery is unchanged: ABI 12, private IR schema 8, `MerkleSlot<T, DEPTH, HISTORIC>`, and ledger-8 forwarding remain as described above. This commit changes only `tools/compact-rust-backend/check_compactc_target.py`.

Before, the shared-runtime consumer compiled Cell, Counter, Set, Map and List crates while the Merkle slot was checked in a separate one-off Cargo consumer. After, the packaged gate generates both plain and historic Merkle crates, compiles them into the same one-dependency shared-runtime graph, and calls their typed APIs:

```rust
let merkle_insert = compact_contract_merkle_tree_oracle::ledger_slots::t
    .insert(merkle_context, BoundedUint::<255>::new(7).unwrap()).unwrap();
let merkle_full = compact_contract_merkle_tree_oracle::ledger_slots::t
    .is_full(merkle_insert.context).unwrap();
let historic_insert = compact_contract_hmt_insert_oracle::ledger_slots::t
    .insert(historic_context, BoundedUint::<255>::new(7).unwrap()).unwrap();
let _reset = compact_contract_hmt_insert_oracle::ledger_slots::t
    .reset_history(historic_insert.context).unwrap();
```

The same gate rejects `t.insert(context, true)` with Rust E0308 (`BoundedUint<255>` expected) and `plain_t.reset_history(context)` with E0599 (method exists only for `MerkleSlot<T, DEPTH, true>`). These are external consumer compile-time boundaries, not new Merkle VM semantics. No runtime or macro code changed; no generated fixture output changed.

**Evidence:** the direct packaged `--consumer` gate passed its positive and negative checks. The complete `nix develop .#compiler --command ... check_compactc_target.py --consumer --proof` gate passed, including target/manifest checks and all 52 existing offline ledger-8 replay, partition, proof, verification, validation and application cases. The 52 cases are regression coverage for previously supported recorded circuits; no Merkle recorded proof is claimed. Python syntax and scoped diff checks passed. Clean-source ABI-12 package write/verify passed for commit `cfa6e46c`, tree `65fe5e01`, `dirty=false`, with 8 macro and 229 runtime archive entries. The unrelated `doc/ledger-adt.mdx` edit was restored byte for byte and excluded from the commit.

**Remaining acceptance:** independent Rust native operation-order comparison with the TypeScript Merkle query tags, recorded Merkle proof and wider semantics, branch publication, remote CI, registry release and wallet/node submission. Focused [#126](https://github.com/MediaNoxLabs/compact/issues/126) remains open in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).



### Native Merkle operation-order amendment — 2026-10-03

Local conventional GPG-signed/DCO commit `fba11fafeb3222772a0702a211548db829a0ae77` closes the **ordered operation-tag** comparison left by the ABI-12 slot and gas amendments. The existing ledger-8 TypeScript captures contain `nativeQueries[*].queries[*].opTags`, but the Rust fixture tests previously checked only result/state and four-dimensional gas. Before this change, the runtime built a VM array inside each query method and passed it directly to `QueryContext::query`:

```rust
let program = [Op::Dup { n: 0 }, Op::Idx { /* ... */ }, /* ... */];
let result = context.query(&program, gas_limit, cost_model)?;
```

After the change, private builders produce the exact `Op` sequence that production passes to `query`, and a runtime unit test serializes those same ledger-8 operations to compare their ordered tags with the independent TypeScript captures:

```rust
let program = check_root_program::<T, D>(path.into(), root, MerkleHistory::CurrentOnly);
let result = context.query(&program, gas_limit, cost_model)?;
// Test: tags(&check_root_program::<_, DefaultDB>(...)) == oracle["nativeQueries"]["knownAtInit"]["queries"][0]["opTags"]
```

The comparison covers seven programs: plain and historic initial `isFull`, initial `checkRoot`, first insert, and historic `forget_history`. `is_full_program`, `check_root_program`, `merkle_insert_hashed_program`, and `historic_reset_history_program` are private runtime builders used by the real query path. Plain/historic root checking now shares one branch-aware builder; it retains the respective `Root`/`Eq` and `Member` programs. `runtime-rs` adds a test-only direct `serde` dependency to inspect ledger `Op` serialization. The emitter, generated crate, slots, macros, public runtime API, ABI 12 and private IR schema 8 are unchanged.

**Evidence:** the new captured-tag unit test passes; the compiler-backed plain and historic Merkle fixture integration tests still match their TypeScript results, serialized state and four-dimensional query gas. `cargo check -p midnight-compact-runtime --lib` passed. A clean-source ABI-12 archive manifest wrote and verified for `fba11faf`, tree `28b87cee`, `dirty=false`, with 8 macro and 229 runtime archive entries; the unrelated documentation edit was restored byte for byte. The earlier 52-call packaged proof gate passed at `d6576650` and was not rerun for this private Merkle builder refactor, since those existing recorded cases contain no Merkle proof.

**Limits:** tag order does not compare operand payloads, path keys, cache flags or full Rust/TypeScript serialized VM instructions. The functional/state/gas fixtures cover observable effects for these calls, but a full operand-level transcript comparison and recorded Merkle proof remain open. No branch publication, remote CI, registry release or wallet/node submission is claimed. Focused [#126](https://github.com/MediaNoxLabs/compact/issues/126) stays open in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).



### Complete selected native VM programs — 2026-10-03

Local conventional GPG-signed/DCO commit `2cbf2ca57af043898929f0dd61bef49956762da7` strengthens the preceding tag-only amendment for the **same seven selected native calls**. The ledger-8 TypeScript capture previously retained only the operation names:

```js
opTags: args[0].map((op) => typeof op === 'string' ? op : Object.keys(op)[0]),
```

It now also retains each complete normalized `QueryContext.query` program. A shared capture helper converts typed byte arrays to numeric arrays, BigInts to decimal strings and `undefined` result placeholders to `null`, matching ledger-8 Rust `Op` serde shape. The runtime unit test compares serialized production-builder operations with the TypeScript capture in order:

```js
program: normalizeQueryProgram(args[0]),
```

```rust
let actual = serialized_program(program); // serde_json::to_value for each ledger-8 Op
assert_eq!(tags(&actual), expected_tags(oracle, call));
assert_eq!(actual.as_slice(), oracle["nativeQueries"][call]["queries"][0]["program"].as_array().unwrap());
```

This compares opcode, path key bytes and alignments, cached/push-path/storage flags, immediate/count operands, pushed cells and the first inserted leaf hash for plain/historic `isFull`, `checkRoot`, first insert and historic `forget_history`. The query program is the same private builder output used by production `QueryContext::query`; no copied implementation is asserted. The TypeScript capture scripts own independent ledger-8 input; `runtime-rs/src/ledger/merkle.rs` owns comparison. No emitter, generated crate, macro, public runtime API, ABI 12 or private IR schema 8 change is made by this follow-up.

**Evidence:** re-running the ledger-8 TypeScript capture for both oracle contracts reproduced every pre-existing JSON field exactly before adding `program`. The Rust complete-program unit test passes for all seven calls. Both plain/historic functional/state/four-dimensional gas fixture tests had passed after the production-builder extraction in `fba11faf`; this follow-up changes only tests/captures/documentation. All three JavaScript modules pass `node --check`; the 37 pinned oracle source inventory reports zero failures; scoped format/diff checks pass. Clean-source ABI-12 archive write/verify passes for `2cbf2ca5`, tree `6e4bae57`, `dirty=false`, with 8 macro and 229 runtime entries. The unrelated documentation edit was excluded and restored byte for byte. The 52-call packaged proof regression last passed at `d6576650`; no Merkle recorded proof is claimed.

**Limits:** these seven captured native calls have complete VM-program parity, but indexed/default/hash insertion, later-state root/fullness queries, reset-to-default and other Merkle paths are not covered by this exact program comparison. Recorded Merkle trace/proof, branch publication, remote CI, registry release and wallet/node submission remain open. This amendment supersedes the previous statement that operand-level comparison for the selected calls was missing; it preserves the earlier tag-only checkpoint as history. Focused [#126](https://github.com/MediaNoxLabs/compact/issues/126) remains open in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
