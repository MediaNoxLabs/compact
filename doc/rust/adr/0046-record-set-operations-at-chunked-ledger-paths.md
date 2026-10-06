---
id: RUST-ADR-0046
alias: ADR-0046
title: "Record Set operations at chunked ledger paths"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: f27c8fc1c5b1ff61deff6eb10ff435002a0d29be6397cdd181a64d98cea74bff
---
# RUST-ADR-0046 — Record Set operations at chunked ledger paths

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept complete recorded Set operations at compiler-declared chunked physical paths using existing slots and strict declaration/type identity. Preserve the scoped composite-key tests and exact pre-proof comparisons. Do not generalize path-length acceptance into arbitrary nested collection semantics or a clean-release claim.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#145 closure](https://github.com/MediaNoxLabs/compact/issues/145#issuecomment-6017475343). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`805c1fc6`](https://github.com/MediaNoxLabs/compact/commit/805c1fc6e1ba751fccc3f3f1dfadd1226af915f2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 46
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/145
```

## Historical decision and amendments

### Problem

The ledger-8 Compact frontend chunks a contract with sixteen ledger declarations into multi-segment physical paths. An exact local probe with fifteen `Uint<64>` fields followed by `Set<[Field, Boolean]>` assigns the Set path `[1, 14]`. The AST generated native `put_key` method uses `ledger_slots::keySet.insert(context, key)` and compiles, but `recorded.rs` refuses `StateAction::SetInsert` when `physical_path().len() != 1`. There is no generated `contract.recording.put_key` or `put_key_call`, so a developer cannot use this otherwise supported circuit in the observed proof flow.

### Before and proposed after

```rust
// Current ABI-23 output for a sixteen-field source:
let step = ledger_slots::keySet.insert(context, key)?;
// contract.recording.put_key(...) and put_key_call(...) do not exist.
```

```rust
// Proposed generated surface after parity and proof evidence:
let call = contract.recording.put_key_call(&observed, (), key)?
    .prepare(verifier, randomness)?;
// The typed slot supplies its compiler-derived physical path [1, 14].
```

A second circuit should insert, remove and query the same tuple key, making the Boolean result and final empty Set observable. The probe is local and ignored under `target/arity12-probe/chunked-set16.compact`; it is not yet a checked-in fixture or delivery claim.

### Decision and rationale

Permit complete recorded Set expressions and actions at a valid compiler-declared physical path regardless of segment count. Derive the path only from `LedgerField::physical_path()` and pass through the generated `SetSlot<T>`. Keep the existing declaration/action index identity check and typed key coercion. Do not manually encode a path in the observed-call emitter. The runtime `SetSlot<T>` already forwards its full `&[u8]` to the ledger-8 frame, so a new path or FAB primitive is unnecessary if measured parity passes.

This is a Set-specific first slice. The other root-only recording guards for Cell, Counter, Map, List and Merkle need separate evidence; removing all guards at once would advertise unproved operations, and ListSlot currently stores only an index.

### Emitter and runtime ownership

The AST emitter owns `Expr::SetMember`, `StateAction::SetInsert`, `SetRemove`, `SetReset`, and Set return size/isEmpty/member eligibility. It should keep `LedgerFieldKind::Set { ty }`, typed `CellValue` key restrictions, action index checks and source-location diagnostics. `lib.rs` already emits `SetSlot<T>::new(&physical_path)` for all paths. The runtime owns `RecordingFrame::{insert_set,remove_set,member_set,...}` and its upstream ledger-8 VM program; it should not duplicate the compiler's path model. No new derive or proc macro is proposed. Public generated/runtime ABI changes only if the supported generated API expands; private IR schema 8 already stores physical paths.

### Verification and risks

Check in the minimal sixteen-declaration Compact source and independent TypeScript input/state/gas/ordered transcript capture. Compare native and recorded Rust result, all four gas dimensions, effects, state and replay; compare exact manual/generated prototypes and tagged pre-proof bytes; independently prove, verify, validate and apply both paths with a typed Set view at `[1,14]`. Exercise a generated-crate-only external consumer and wrong-typed tuple rejection. Keep all existing fixtures and proof calls green, measure fixture source growth and report compile-time evidence honestly. A multi-segment path must be verified against the actual ledger state layout; the local probe alone only establishes compiler output, not transaction parity. Remote CI, clean signed release and branch publication remain wider milestone gates.

### Tracking and delivery

- Focused MediaNoxLabs issue: pending creation; assign to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2) before implementation.
- Related: ADR-0045 / #144 for root composite Set keys; ADR-0044 / #143 for typed observed inputs.
- Delivery state: proposal only; no emitter/runtime edit, ABI change or branch push yet.
- Append actual before/after code, exact evidence, signed/DCO commit, unresolved limits and any correction. Preserve this proposal history.


Issue created: [#145](https://github.com/MediaNoxLabs/compact/issues/145), assigned to `rust-backend-v2` before implementation. The proposal and exact local probe now have a focused review thread.


### Scope correction during implementation — 2026-10-03

The proposal listed `StateAction::SetReset` among potential path gates. The exact Compact probe `keySet.reset()` is rejected by the ledger-8 frontend as an undefined Set operation, so this delivery keeps the existing recorded reset guard unchanged. The implemented path change is limited to Set insert, remove, member, size and isEmpty, all present in a real sixteen-declaration source. This preserves the proposal history while narrowing the exposed API to executable Compact operations. No proof claim is made by this note; see the later delivery amendment for the gates.


### Local delivery — 2026-10-03: ABI-24 chunked Set recording

**Decision:** accept recorded and observed Set insert, remove, member, size and isEmpty at compiler-declared multi-segment ledger paths. Keep Set reset outside this slice: `keySet.reset()` is rejected by the ledger-8 frontend, and the recorded reset guard remains. Local conventional GPG-verified/DCO commit `805c1fc6e1ba751fccc3f3f1dfadd1226af915f2` (`feat(rust-backend): record chunked Set paths`) is committed on `codex/rust-backend-ast`, local and unpushed. Focused [#145](https://github.com/MediaNoxLabs/compact/issues/145) stays open in `rust-backend-v2` for remote/release gates.

#### Actual developer-facing before/after

The checked-in `chunked_set_observed.compact` declares fifteen `Uint<64>` ledger fields followed by `Set<[Field, Boolean]>`. The compiler assigns the Set `[1,14]`. With ABI 23, the same source emitted native `insert_key`, `roundtrip_key`, `key_count` and `empty` methods, but none had a recorded method or generated observed call. The native insert used the typed slot:

```rust
let step = crate::ledger_slots::keySet.insert(context, key)?;
// No contract.recording.insert_key_call method existed for this source.
```

With ABI 24, a consumer uses the generated crate alone and supplies a typed tuple key:

```rust
let call = contract.recording
    .insert_key_call(&observed, (), (Field::from(42_u64), true))?
    .prepare(verifier, randomness)?;
```

The generated `keySet: SetSlot<(Field,bool)>` contains `[1,14]`; the method derives the circuit identity and public FAB input from typed IR. `roundtrip_key_call`, `key_count_call` and `empty_call` are also generated. A wrong `(bool, Field)` key fails the external consumer with Rust E0308. The old and new generated source for the exact same contract is 271→454 lines (+183); this expansion reflects four complete recorded and observed methods. Isolated compile time was not measured, and no source-size improvement is claimed.

#### Emitter/runtime/compatibility boundary

The `syn`/`quote` recorded emitter removes the one-segment restriction in exactly four Set paths: member expression, insert/remove action, Set size return, and Set isEmpty return. It keeps the declaration/action index check, declared tuple type, unsupported-shape suppression and root-only Set reset guard. Generated `SetSlot<T>` already owns `LedgerField::physical_path()` and forwards it to the existing `RecordingFrame` methods; the runtime adds no path encoder, VM builder, FAB conversion, derive or macro. The runtime constant and generated assertion bump ABI 23→24 for the larger public generated API. Private IR schema stays 8, and the transaction, prover and wallet implementations are unchanged.

#### Independent evidence

Reproducible TypeScript compilation and `capture-chunked-set-observed.mjs` capture the actual `[1,14]` path. `insert_key(42n,true)` gives Field atom `[42]`, one-byte Boolean atom `[1]`, Field/byte alignment, and ordered `Idx(pathLength=2), Push, Push, Ins(n=1), Ins(n=2)` operations. `roundtrip_key` returns `false` after insert/remove/member with the expected fourteen operations and unchanged final state. `key_count` returns zero and `empty` returns true, each retaining a two-segment `Idx`. The capture includes serialized initial/final ledger state and each ledger query's gas in all four dimensions; the Rust fixture compares all of these, exact normalized operation shapes, native/recorded result, effects, gas, state and replay. The capture reproduces byte for byte.

The current local ABI-24 `compactc` pairs the Rust launcher with the pinned packaged ledger-8 Scheme frontend and ZKIR 2.2.0. The generated-only separate consumer compiles four typed call methods, checks the slot path, and rejects a wrong key type. The rebuilt `check_compactc_target.py --consumer --proof` passes **70** offline prove/verify/validation/application calls. For the four new circuits, manually prepared and generated calls match complete prototypes and exact tagged pre-proof bytes of **557, 719, 491 and 512**, respectively; both paths independently prove and apply the expected typed Set state. The focused Rust test, 57 renderer tests, 134 fresh fixture outputs, full all-features offline workspace check, `cargo fmt --all -- --check`, compiler manifest hashes and staged diff check pass.

The ABI-24 macro/runtime archives package, compile unpacked, and match the repeated local manifest verification at `target/rust-runtime-release-abi24.json`, whose source commit is `805c1fc6`. The manifest is **dirty** because the pre-existing user-owned `doc/ledger-adt.mdx` edit remains unstaged. The full gate used a locally built current Rust launcher plus pinned packaged Scheme, not a fresh ABI-24 Nix `compactc` package. No clean signed release candidate, remote CI, registry publication, wallet/node submission or branch push is claimed.

#### Remaining limits

Set reset is not exposed by this Compact source. Other ADTs still have root-only recorded guards, and a multi-segment List slot still carries only an index. This acceptance covers a two-segment root ledger declaration containing a composite tuple key; it does not prove Set nested inside another collection or arbitrary depth. Compile-time impact, current-head Nix package, remote macOS/Linux CI, clean signed release, registry consumer and branch publication remain open under [#145](https://github.com/MediaNoxLabs/compact/issues/145) and the wider milestone.



### Packaged compiler evidence amendment — 2026-10-03

After the local ABI-24 delivery, a fresh `aarch64-darwin` `nix build .#compactc --no-link --print-out-paths --max-jobs 1 --cores 4` succeeded at `${HISTORICAL_NIX_STORE}/kgm5r3swizg1ihylrihzl2xif256xaik-compactc`. Its packaged `compactc --version` reports 0.31.133 and, with ledger-8 ZKIR 2.2.0 on PATH, the packaged binary passed the complete `check_compactc_target.py --consumer --proof` gate: standalone generated-crate consumers, manifest hashes, four chunked Set verifier/prover/ZKIR pairs and **70** independently proven/verified/validated/applied calls. The four new exact manual/generated pre-proof payloads remain 557/719/491/512 bytes. This closes the earlier **fresh current-head Nix package** caveat; the preceding local compiler result remains accurate for its time. The Nix source was dirty only from pre-existing user-owned `doc/ledger-adt.mdx`, so this does not establish a clean signed release candidate, remote CI, registry publication or branch publication.
