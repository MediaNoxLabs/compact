---
id: RUST-ADR-0156
alias: ADR-0156
title: "Preserve typed root locals in bboard returns"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["native-lowering", "lexical-scope", "returns", "original-contracts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 4cfdcda642c8cd45cae037df82a1c3162c60dfde764dd546e5ef3f712c87f771
---
# RUST-ADR-0156 — Preserve typed root locals in bboard returns

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted native final top-level Let scope retention across preceding actions and a pre-clear opaque return, rejecting earlier sibling or nested escapes. Complete original bboard now compiles and matches selected native TS behavior, but this decision explicitly supplies neither recorded calls nor proofs nor byte-for-byte public VM parity.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#260 closure](https://github.com/MediaNoxLabs/compact/issues/260#issuecomment-6017670491). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`64383e7b`](https://github.com/MediaNoxLabs/compact/commit/64383e7b4f2f1f2331e54b4ff5449b177f393bc2). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: Implemented locally, pending integration · 2026-10-05 · rust-backend-v2 · [issue #260](https://github.com/MediaNoxLabs/compact/issues/260)

### Problem

The exact ledger-8 `test-center/test-contracts/bboard.compact` now passes the explicit Field-to-Bytes cast (ADR-0145) but Rust compilation stops at `take_down` line 42: `unknown parameter "former_msg"`. Compact binds `const former_msg = message.value`, then changes `state`, increments `instance`, clears `message`, and returns the pre-clear Opaque string. The existing ADR-0138 root-`Let` bridge carries a Field Cell value across actions, but this original optional opaque field exposes a top-level tail-`Let` scope gap after preceding assertions. Three bboard exports remain unassessed until the complete source compiles.

### Before and after

Before, the complete source fails before Rust metadata; simply rereading `message.value` after the clear would return the wrong value. The relevant source is:

```compact
const former_msg = message.value;
state = STATE.vacant;
instance.increment(1);
message = none<Opaque<"string">>();
return former_msg;
```

The Scheme compiler already emits the existing schema-13 `StateAction::Let` with a typed `OpaqueString` binding and an ordered action sequence. It is the last top-level action after two assertions; its return is a `Parameter(former_msg)`. After, native emission retains that tail-`Let` binding for the circuit return. The circuit return refers to that scoped binding. Illustrative generated Rust:

```rust
let former_msg: runtime::OpaqueString = message_value_before_write;
// Apply the state, Counter and message writes in source order.
Ok(former_msg)
```

The emitted code uses the existing typed `CellSlot`, `CounterSlot`, `OpaqueString` and native frame APIs. No string conversion or textual code generation is added.

### Decision and boundaries

Reuse the schema-13 `Let`, `LocalBinding`, typed expression and native return domain model; preserve declared type, lexical scope, and evaluation order. A `Let` that is the final top-level action is in scope for its own circuit return after its inner actions, including when assertions or other actions precede it. A nested, branch-local, or earlier sibling `Let` cannot escape. Resolve the compiler's source name only through that validated tail binding; do not globally treat unknown identifiers as parameters. Its typed `StructField(CellRead(message), value)` must capture the opaque value before later writes. Native lowering may accept a typed optional-field projection only when its source and result types are proven. Keep recording/observed-call unavailable for bboard paths until their full VM and opaque output shapes are verified. This bridge changes only native emitter scope selection; Scheme already emits the required typed nodes. It adds no private IR variant or public runtime primitive, so schema 13 and runtime ABI 37 remain unchanged.

### Acceptance

1. Compile the entire original bboard source with Rust target and inspect metadata for `post`, `take_down` and `public_key`; report any next independent blocker rather than editing the original.
2. Capture independent TypeScript and native Rust behavior for empty board, Unicode message, and a retained pre-write `take_down` return. Compare serialized state, result, ordered ledger effects, and per-query gas where emitted semantics are available.
3. Test root-local type and scope guards, including rejection of a local used outside its lexical scope and wrong result type. Preserve ADR-0138 Field pre-write behavior and existing generated fixtures.
4. Produce exact compiler inventory attribution. Do not claim proof readiness or bboard recording without a pinned proof/ledger validation.
5. Conventional GPG-signed DCO commit locally; no push or remote CI.

### Related

ADR-0138 established the root `Let` lifetime; ADR-0145 passed bboard's Field-to-Bytes blocker. This ADR addresses the next original-source failure.

### Local result and evidence

The precise renderer fix retains a `Let` binding for circuit return when that `Let` is the final top-level action; preceding assertions are allowed. The existing typed scope map still rejects earlier sibling and nested bindings, and declared type mismatch. No Scheme, private IR schema or runtime ABI change was needed.

The **complete original bboard source** now compiles with Rust target, and its generated crate builds. Compiler metadata contains `post` and `take_down` as proof-required, plus pure nonproof `public_key`. The capability report publishes `post` and `take_down` as native-only with `recorded=false` and `observed_call=false`; `--rust-require-recording` rejects the source. Neither call was proved.

The checked TypeScript capture and Rust fixture use a rehydrated initial board, an empty-board `take_down` rejection, and a Unicode `post` then `take_down`. They agree on the pre-clear returned message, initial/post/clear serialized contract states, state/Counter/message/poster values, witness invocation count and output count, and all four gas dimensions summed from TypeScript's per-query meter. The TypeScript capture fixes ordered query operation tags; native Rust has no replayable public trace for these calls, so byte-for-byte VM operation parity is **not claimed**.

Focused renderer, source target, positive cohort, fixture freshness and full inventory gates pass locally. Inventory on the isolated base: 197 sources, 173 Rust-compiled sources, 688 exported circuits, 299/320 proof-required APIs available, 21 known proof gaps, and 22 unassessed exports. Bboard moves three exports from unassessed: one pure nonproof and two known native-only proof APIs. The checked source cohort is `parity_positive_test_center_bboard_sources.json`; source fixture is `tests-rust-backend/test-center-bboard`. The broader milestone remains open.


Local delivery: signed DCO commit `6d51a89a7e9c46ef16b7267c057f5aa38945b23f` on `codex/adr156-bboard-root-binding`. Focused results: 125 renderer tests; 1 independent bboard TS/native parity test; 24 inventory tests; 153 fixture outputs fresh; source cohort and strict recording rejection pass; full inventory pass; backend and bboard fixture Clippy pass. These results use the isolated `64383e7b` base. No remote CI or push.
