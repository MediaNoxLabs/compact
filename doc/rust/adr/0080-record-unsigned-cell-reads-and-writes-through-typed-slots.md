---
id: RUST-ADR-0080
alias: ADR-0080
title: "Record unsigned Cell reads and writes through typed slots"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 4396d721f9e34b912bfa5197489ca7412381aa615879e45e1e07b75adb1dba6f
---
# RUST-ADR-0080 — Record unsigned Cell reads and writes through typed slots

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed unsigned Cell recording for the validated narrow/wide carriers, literals/defaults and witness bindings through existing checked representations. Preserve declaration/type/path identity and exclusion of unsupported arithmetic/branches. Its170/333 capability baseline is historical availability evidence, not current coverage or universal unsigned semantics.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#180 closure](https://github.com/MediaNoxLabs/compact/issues/180#issuecomment-6017535344). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`866129dc`](https://github.com/MediaNoxLabs/compact/commit/866129dc4d16e9412e278747b76dd96939ab6435). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 80
status: accepted-local
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/180
```

## Historical decision and amendments

### Problem and measured scope

At ABI 35 the local 137-source fixture corpus has 160/333 exported circuits with recorded and observed-call APIs and 173 without them. The typed schema-8 IR already models unsigned ledger `Cell` writes and reads. Native generated Rust executes those operations using `CellSlot<BoundedUint<MAX>>` or `CellSlot<WideUint<HIGH, LOW>>`, yet the recording emitter rejects `Type::Unsigned` in `StateAction::CellWrite` and `StateReturn::CellRead`. Its `cell_source` omits generic unsigned literals, and its `StateAction::Let` witness binding accepts Boolean/Field/Bytes but not Unsigned. These are separate emitter gates over an already typed runtime path, not a need for another VM implementation.

Fresh IR probes identify six affected exported circuits across four existing oracle contracts: `bounded_uint_oracle.set_small` (parameter write, `Uint<0..100>` lowered to maximum 99), `uints_oracle.set_byte` (parameter write, maximum 255), `cross_circuit_oracle.reset` (unsigned zero literal write), `cross_circuit_oracle.reset_and_set` (Unit callee plus parameter write), and `wide_uint_oracle.writeWide`/`readWide` (`Uint<248>` witness write and direct Cell read). These are candidate gains, not a capability claim until the emitted methods compile and pass the gates. ADR-0079/#178 will produce typed missing-reason counts; reconcile this probe list against those counts before implementation.

### Developer-facing before and after

Before, native use is possible but these contracts omit proving methods:

```rust
let next = ledger_contract::set_small(context, BoundedUint::<99>::new(99)?)?;
let wide = ledger_contract::readWide(next.context)?;
// No ledger_contract::recorded::set_small or typed set_small_call.
```

After, the same declared slots provide recorded execution and typed call preparation:

```rust
let recorded = ledger_contract::recorded::set_small(context, BoundedUint::<99>::new(99)?)?;
let call = ledger_contract::recorded::Contract::default()
    .set_small_call(&observed, (), BoundedUint::<99>::new(99)?)?;
let wide = wide_contract::recorded::readWide(context)?;
```

The generated body remains a typed Rust operation, such as:

```rust
let frame = runtime::recording::RecordingFrame::new(context);
let frame = crate::ledger_slots::small.record_write(frame, __compact_param_0)?;
Ok(frame.finish(()))
```

For `reset_and_set`, expand `reset()` into the *same* `RecordingFrame`, then write the parameter in source order; do not open a nested recording session or coalesce two writes. The wide witness path should use the existing metered `TryWitnesses` bridge and bind the `WideUint` value before calling `wide.record_write(frame, value)`.

### Decision and ownership

Extend the AST recorder's `CellWrite` and direct `CellRead` eligibility to `Type::Unsigned` while checking the declaration kind, exact Compact type and physical index. Use the existing `cell_source`/checked `expression_with_calls` path for unsigned literals and `Default`, rather than constructing numeric text or duplicating range validation. Permit typed unsigned witness `Let` bindings, preserving witness private outputs and any metered ledger reads. Keep unsupported nested arithmetic or expressions explicitly unavailable until a separate typed expression slice exists; do not silently fall back to native-only execution when claiming a recorded API.

No runtime method or primitive is added. `CellSlot<T: CellValue>::record_write/read` already delegates to `RecordingFrame::write_cell/read_cell`; the frame reuses `ledger::cell_write_program` and the actual gathered FAB read value for Verify transcript replay. `BoundedUint<MAX>` covers declared widths through 128 bits with exact byte alignment; `WideUint<HIGH, LOW>` covers supported 129–248-bit values under the pinned field modulus. The source `Uint<0..100>` fixture lowers to maximum 99, so its generated type must remain `BoundedUint<99>`. Pinned `midnight-ledger` 8.0.3 and matching `midnight-zk` crates own VM, FAB, proof and serialization semantics. Generated code emits no VM opcode lists. No derive, macro or new lightweight DSL is warranted for a slot call.

Private IR schema stays 8 and runtime ABI stays 35 if only emitter eligibility changes. ADR-0079 may independently increment the capability-report schema; retain its reason model when a circuit becomes supported. Recheck ABI if implementation unexpectedly adds a public runtime API.

### Acceptance and risk

- First, create a focused `MediaNoxLabs/compact` issue in milestone `rust-backend-v2`, linked to this ADR and parent [#152](https://github.com/MediaNoxLabs/compact/issues/152); reconcile the six probe circuits with ADR-0079's measured reasons before code.
- Renderer tests cover parameter, literal/default, witnessed wide write, direct read, Unit callee write order, wrong declared type/index, and a still-unsupported unsigned expression. Capability booleans/reasons change only where complete recorded methods exist.
- Extend existing TypeScript oracle captures where necessary to compare native/recorded result, serialized state, four-dimensional total gas, ordered public Verify operations, private witness outputs, and failures at bounds. For two writes in `reset_and_set`, compare per-query costs or their sum: TypeScript's wrapper may report only the final query's gas.
- Check generated native/recorded/replayed effects and state, and a separate Cargo consumer for `BoundedUint<99>`, `BoundedUint<255>`, `BoundedUint<u64::MAX>` and `WideUint` typing. Add source-to-proof-to-ledger application cases for representative small, wide, read and sequential writes where compatible ZKIR and keys are available. A state-only TypeScript fixture is insufficient for a proof claim.
- Use focused renderer and generated-crate tests while editing; then refresh/check all 137 fixtures and run the pinned packaged compiler/consumer/proof gate because generated recording/proof behavior changes. Run exact Rust 1.99 workspace Clippy and Nix packaging at the integration checkpoint. Record gate durations and exact commit. Conventional GPG-signed/DCO local commit; no push or remote CI under the user's current policy.

This ADR covers direct unsigned Cell read/write and witness-fed write only. Arithmetic, nested unsigned expressions, other ledger ADTs, and full TypeScript language parity remain separate work. A measured gain of up to six corpus circuits must not be presented as full production readiness.

### Tracking

- Accelerator: [Milestone 2 — Full TS parity delivery accelerator](references.md#private-note-10).
- Capability reason work: [ADR-0079 — Explain missing Rust recording capabilities from typed lowering](0079-explain-missing-rust-recording-capabilities-from-typed-lowering.md) / [#178](https://github.com/MediaNoxLabs/compact/issues/178).
- Focused issue: pending.
- Delivery: proposed; no implementation or proof claim.



### Tracking amendment — 2026-10-05

Focused [#180](https://github.com/MediaNoxLabs/compact/issues/180) was created in `MediaNoxLabs/compact` and assigned to `rust-backend-v2` before implementation. The checked inventory covers 167 curated positive sources, while the checkout contains 677 `.compact` files; 137 generated fixtures and 333 exported capability rows are a useful local subset, not proof of full TypeScript language or application parity. ADR-0079 reason ranking and all implementation/acceptance gates remain pending.

### Local delivery evidence — 2026-10-05

Schema-8 IR and runtime ABI 35 remain unchanged. The AST emitter now permits declared `Type::Unsigned` Cell write/read, routes unsigned literals through the checked `expression_with_calls` builder, and binds unsigned witness values through the existing metered witness bridge. It emits typed `CellSlot` recording calls; no runtime primitive or handwritten VM sequence was added.

The checked 137-entry fixture corpus moved from 160/333 to 170/333 exported recorded and observed-call APIs, with zero losses and no identity drift. The ten gains are `bounded_uint_oracle.set_small`, `uints_oracle.set_byte`, `cross_circuit_oracle.reset/reset_and_set`, `wide_uint_oracle.writeWide/readWide`, `bug11_oracle.set_tiny/set_medium/set_wide`, and `multi_pl_call_oracle.record_update`. This is a capability count, not full TypeScript parity; 163 corpus APIs remain missing and at least 100 known positive TypeScript example sources are outside the current 167-source inventory.

TypeScript captures now include exact four-dimensional gas, ordered public operations and private outputs for all ten gains. Focused Rust tests compare native, recorded and replayed results/state/effects against them. The 137 generated fixture libraries are fresh. Multi-query `reset_and_set` and `record_update` accumulate source-query gas in native/recorded results; the Compact TypeScript wrapper reports only the final query. Concatenated Verify replay uses one query, so its cost is a separate measurement; the tests compare replay state/effects, not a misleading gas equality. Uint<248> witness FAB alignment and direct read value alignment are checked.

Pinned local `compactc --target rust --rust-require-recording` produced ZKIR, binary ZKIR, prover and verifier artifacts for all ten methods. The packaged consumer and offline proof-to-ledger gate passed, including observed-call parity and validated/applied `set_byte(255)`, witnessed `writeWide(max)`, and `readWide(max)` from a separately seeded deployment. The read proof does not establish a chained proven write-to-read transaction history. Remote CI and push remain deferred by user direction. Targeted Rust 1.99 Clippy and the signed commit receipt are pending at the time of this entry.


### Signed local cut

`866129dc4d16e9412e278747b76dd96939ab6435` — `feat(rust-backend): record unsigned Cell circuits` (`Refs: #180`). The commit has a verified GPG signature and DCO sign-off. Focused package tests for bounded, byte, cross-circuit, wide, Bug-11 and multi-public-ledger calls passed; 66 renderer tests and the 137-fixture freshness gate passed. `cargo +1.99.0 fmt --all --check`, targeted Rust 1.99 all-target/all-feature Clippy with `-D warnings`, oracle manifest checks, and the packaged consumer plus offline proof-to-ledger gate passed. The main checkout retains only the unrelated user-owned `doc/ledger-adt.mdx` edit. The branch was not pushed and remote CI was not run.

Exact-head parity receipt: ${LOCAL_EVIDENCE}/compact-parity-866129dc.json records HEAD 866129dc4d16e9412e278747b76dd96939ab6435, immutable compiler SHA-256 1ec55e77a57a9994596b3e4710c6ad50e36bd7b9d86c84c118503397bb8d2365, 170 supported / 163 missing, zero unmatched rows and zero baseline drift.
