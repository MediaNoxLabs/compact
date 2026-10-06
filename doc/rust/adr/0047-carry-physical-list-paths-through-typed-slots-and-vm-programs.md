---
id: RUST-ADR-0047
alias: ADR-0047
title: "Carry physical List paths through typed slots and VM programs"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: cb074674d0d9d667861a252d009dea4f1bfa31d29b0cd047d9689ae03f19e74d
---
# RUST-ADR-0047 — Carry physical List paths through typed slots and VM programs

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept complete physical List paths throughout typed slots, native/recorded builders and witness metering, deriving stack offsets from actual path depth. Preserve root-byte compatibility and the real chunked-source VM evidence. Historical dirty archive rehearsal is not a clean tagged or published release.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#146 closure](https://github.com/MediaNoxLabs/compact/issues/146#issuecomment-6017477002). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`37d65715`](https://github.com/MediaNoxLabs/compact/commit/37d657152daff80cd31b8bac57e9db8bfbf73bbb). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 47
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/146
```

## Historical decision and amendments

### Problem

The ledger-8 Compact frontend assigns a List after fifteen scalar ledger declarations physical path `[1,14]`. The current ABI-24 AST renderer rejects this valid source before writing Rust with `unsupported ledger field kind at chunked path [1, 14]`. That rejection prevents silent wrong-state access: generated `ListSlot<T>`, native `CircuitContext`, recorded `RecordingFrame`, witness metering and the shared List VM builders currently accept only a root `u8` index. This is a real language-acceptance and correctness gap, not merely a missing observed-call method.

### Before and proposed after

For an otherwise valid `items: List<Field>` source at `[1,14]`:

```text
compactc --target rust chunked-list.compact out
# unsupported ledger field kind at chunked path [1, 14]
```

Proposed developer surface after independent TypeScript/ledger evidence:

```rust
assert_eq!(ledger_slots::items.path(), &[1, 14]);
let call = contract.recording.prepend_call(&observed, (), Field::from(42))?
    .prepare(verifier, randomness)?;
```

Existing root List consumers should continue to use the same generated `items.push_front`, `recording.prepend`, and observed `prepend_call` methods. The slot constructor can change because generated/runtime ABI will bump; direct low-level `u8` runtime calls should remain source-compatible through `LedgerPath: From<u8>`.

### Decision proposal

Carry the compiler's complete `LedgerField::physical_path()` through `ListSlot<T>` and every native, recorded and metered witness List operation. Generalize shared List VM programs from one `u8` to `LedgerPath` while preserving root-byte output exactly. The List's internal head/tail/length indexes remain local `0/1/2`; only the outer ledger path changes. Derive cached `Ins.n` from path depth. The TypeScript compiler's real `[1,14]` program uses `Idx(pathLength=2)`, `Ins(n=3)` for push front; pop front uses `Ins(n=2)`; reset indexes parent `[1]`, inserts child `14`, then restores parent. Do not synthesize a separate Rust VM implementation for chunked paths.

This should remove the AST's List-specific chunked-path rejection only after native, recorded, constructor and witness paths are backed by the same path model. Keep other type and declaration checks. The private IR schema already carries full paths and need not change. A public generated/runtime ABI bump is expected because `ListSlot` changes shape.

### Emitter/runtime ownership

The frontend owns the physical path and source spans. `tools/compact-rust-backend/src/lib.rs` validates expected ledger paths and emits typed slots; `stateful.rs`, constructor lowering and `recorded.rs` should call those slots. `runtime-rs/src/slots.rs` owns the fixed List element type and physical path. `runtime-rs/src/ledger/collections.rs` owns the one canonical VM program per List operation and read-only/metered views. `CircuitContext`, `RecordingFrame` and `WitnessReadMeter` delegate to those builders; no proc macro, derive or new FAB representation is proposed. Upstream ledger-8 `LedgerPath`, `Op`, `QueryContext` and `CostModel` remain authoritative.

### Evidence and acceptance before delivery

The local probe in ignored `target/chunked-list-full.compact` is the ordinary six-circuit List example preceded by fifteen scalars. The pinned TypeScript output shows the actual `[1,14]` path and the depth-aware push/pop/reset programs, but no committed capture or Rust parity yet. Check in a reproducible source and TypeScript capture for prepend, drop, reset, head, length, empty, plus constructor and witnessed reads if the source can exercise them. Compare serialized ledger state, results, four-dimensional per-query gas, complete ordered transcript, native/recorded replay and malformed path/type rejection. Use a one-dependency generated consumer, exact manual/generated pre-proof bytes, and independent ledger-8 proof/verification/validation/application; keep all root List fixtures and proof cases green. Report generated source size and compile-time evidence honestly. A fresh packaged `compactc`, remote CI, clean signed release, registry consumer and branch publication are separate gates.

### Tracking and amendments

- Focused issue: pending creation, then assign to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2) before implementation.
- Related: ADR-0046 / #145 establishes chunked Set recording; ADR-0018/0023/0024/0025 cover root List witness and recorded semantics.
- Delivery state: proposal/research only. No runtime, emitter, ABI or branch mutation is claimed here.
- Append actual before/after, corrections, measured proof and package evidence, signed/DCO commit and unresolved limits rather than rewriting this rationale.

### Delivery amendment — 2026-10-04

The decision is implemented locally in signed and DCO commit `37d657152daff80cd31b8bac57e9db8bfbf73bbb` (`feat(rust-backend): support chunked List ledger paths`). Focused [#146](https://github.com/MediaNoxLabs/compact/issues/146) remains in `rust-backend-v2`.

#### Concrete before and after

Before, `compactc --target rust` stopped at `unsupported ledger field kind at chunked path [1, 14]`. Root List generation used `ListSlot::new(1u8)`; constructor lowering called `context.push_front_list(1, value)`. That root index would identify the parent chunk, so rejection was required.

After, the same source generates:

```rust
pub const items: runtime::slots::ListSlot<runtime::Field> =
    runtime::slots::ListSlot::new(&[1u8, 14u8]);
let step = context.push_front_list(&[1u8, 14u8], value)?;
let recorded = contract.recording.prepend(context, Field::from(7_u64))?;
let call = contract.recording.prepend_call(&observed, (), Field::from(7_u64))?;
```

The `ListSlot<T>` descriptor owns a static physical path. Native `CircuitContext`, recorded `RecordingFrame`, metered witness views and canonical ledger-8 List VM builders accept `LedgerPath`. The outer `Idx` uses all path segments; push and pop derive cached `Ins.n` from depth; reset indexes the parent, replaces the child, then restores the parent. Constructor lowering now uses `ledger_path_expr`, the gap found after the first generated fixture. Root low-level `u8` calls still coerce through `From<u8>`. ABI changed 24→25; private IR schema stays 8. No separate VM, FAB encoder, macro or derive was introduced.

#### Verification

A checked-in sixteen-declaration Compact source, TypeScript capture, six-circuit Rust fixture and generated-only consumer exercise `item_count`, `items_empty`, `first_item`, `prepend`, `drop_first`, and `clear_items`. The source constructor inserts one Field at path `[1,14]`. Native and recorded results, ledger state, effects and gas match; replay reaches the same state. The TypeScript capture matches serialized state, ordered public transcript shape and all four gas dimensions for each query. The typed List witness view matches the direct metered path view, including gas. A neighboring scalar path is rejected as a List. The generated-only consumer compiles all observed calls and rejects a Boolean passed to `prepend_call`.

All 135 generated fixtures are fresh; 57 renderer tests, root List recording tests, typed slot witness tests, formatting, and the full all-features workspace check pass. A fresh local aarch64-darwin Nix build produced packaged `compactc` at `${HISTORICAL_NIX_STORE}/r7l6r98al93ghjvm07pricgs3z3hs36j-compactc`. Its generated-consumer/manifest gate passed 76 independent prove, verify, validate and apply calls. For the six new calls, manual and generated observed prototypes match before proof at 510, 545, 710, 673, 493 and 547 serialized bytes respectively. The generated Rust source is 25,545 bytes / 578 lines for the chunked List contract; the smaller root List fixture is 20,073 bytes / 479 lines but lacks the fifteen scalar declarations and constructor, so this is not an optimization comparison. Compile-time impact was not measured.

#### Limits and review questions

The branch remains local and unpushed. Remote macOS/Linux CI, a clean signed release candidate, registry consumer, and wallet/node submission have not been run. The runtime source has a public ABI change, so generated ABI-24 crates must retain matching runtime sources. Keep #146 open for those production gates and for review of whether public low-level empty List paths should return a typed error rather than rely on compiler path validation.
#### ABI-25 runtime archive rehearsal — 2026-10-04

The two publishable runtime crates packaged and verified twice from local commit `37d65715`. The second run reproduced the saved archive manifest at `target/rust-runtime-release-abi25.json`; the macro crate has 9 archive entries and runtime crate 243. The manifest records a dirty worktree solely because the pre-existing user-owned `doc/ledger-adt.mdx` remains unstaged. This is an archive and local dependency verification gate, not registry publication or a clean signed release candidate.
