---
id: RUST-ADR-0112
alias: ADR-0112
title: "Iterate typed constructor vectors in Welcome"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["constructor", "vector", "schema", "original-contracts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 04f485622946c6875c4fc1491382a364e21350bfca39c1446456616262947744
---
# RUST-ADR-0112 — Iterate typed constructor vectors in Welcome

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted schema-12 constructor iteration over typed parameter vectors with borrowed generated Maybe/FixedVector values. Empty and one-participant Welcome cases match state/private/witness behavior; ConstructorResult exposes no Rust constructor VM or gas receipt, so that parity is not claimed. Preserve the initial stack-overflow attempt and later successful pinned Nix result.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#215 closure](https://github.com/MediaNoxLabs/compact/issues/215#issuecomment-6017595319). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1e521ffc`](https://github.com/MediaNoxLabs/compact/commit/1e521ffcdcfd36d6019f4ef96d86527a48460f73). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
date: 2026-10-05
status: accepted-local
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/215
base: 1e521ffc
```

## Historical decision and amendments

### Problem

The exact TypeScript-positive `test-center/test-contracts/welcome.compact` source compiles for TypeScript but the Rust target rejects its constructor `for (const maybe_participant of disclose(initial_eligible_participants))` with "Rust constructor fold needs a literal iterable". The constructor fold is already a typed, unit-accumulator Lnodisclose operation. The existing Rust IR `ForEach` stores expanded literal values, so it cannot represent a typed Vector parameter. A diagnostic copy with only this loop removed compiles to a generated Rust crate; this copy is not source acceptance evidence.

### Before and after

Compact source remains unchanged:

```compact
constructor(initial_eligible_participants: Vector<5000, Maybe<Opaque<"string">>>) {
  organizer_pks.insert(public_key(local_sk_or_error()));
  for (const maybe_participant of disclose(initial_eligible_participants)) {
    if (maybe_participant.is_some) {
      eligible_participants.insert(maybe_participant.value);
    }
  }
}
```

Before, the Rust target stops before creating a crate. Intended emitted Rust shape:

```rust
let values: [Option<OpaqueString>; 5000] = initial_eligible_participants;
for maybe_participant in values {
    if maybe_participant.is_some() {
        context = context.insert_set(eligible_participants_path, maybe_participant.unwrap())?.context;
    }
}
```

The example is illustrative; generated code must retain the compiler's typed Maybe and Opaque encoding and ledger-8 VM order. The runtime owns Set insertion and cost accounting; the emitter must not duplicate ledger algorithms.

### Decision and ownership

Add a typed constructor Vector-source loop IR variant alongside the existing literal `ForEach`, and a bounded constructor conditional action if the Lnodisclose body requires one. Validate the fold length and element type in Scheme, retain the typed source expression and binding, and emit a checked Rust loop with source order. Version the IR schema. Reuse existing constructor context, Set slots, and ledger-8 primitives; change runtime API only if an observed incompatibility requires it. Reject non-Vector, multiple-iterable, non-unit-accumulator, non-inline, spread or effectful iterable forms explicitly.

### Proof boundary

Constructor compilation does not imply proof parity. A diagnostic no-fold copy reports `add_participant`, `add_organizer`, and `check_in` as proof-required but recording-unavailable at their first assertions. Opaque Set membership and hash/witness composition require separate recording work. This ADR claims a source-complete Rust crate only if the exact original source compiles; capability rows must remain unavailable unless independent TS/native/recorded/replay and pinned proof-to-ledger validation pass.

### Verification plan

Pin exact TypeScript and Rust compiler outputs, generated crate build, constructor state and VM operation order for empty and one-participant inputs, and negative renderer/IR tests for unsupported forms. Compare TypeScript and Rust constructor state plus four gas dimensions where TS exposes them. Run source-backed inventory and preserve lexical identity. A proof API gain requires independent TS/native/recorded/replay and pinned ZKIR/ledger-8 evidence. Run focused local fixture, renderer, schema, Clippy and formatting gates; root runs integrated full gate. No push or remote CI.

### Open limits

The Vector length is 5000, and Opaque string encoding plus conditional Maybe extraction need real trace validation. Other test-center sources remain separate. Registry publication and remote CI remain milestone work.


### Local acceptance — 2026-10-05

Signed conventional GPG/DCO commit `dfb568148181fc07d32eddc1e62bcf2ded9150f3` from `1e521ffc` implements schema-12 `ForEachVector` and constructor `If`. Runtime ABI37 and ledger primitives are unchanged. The actual generated shape uses the concrete `Maybe` struct, correcting the illustrative `Option` sketch above:

```rust
for maybe_participant in initial_eligible_participants.0.iter() {
    if maybe_participant.clone().is_some {
        context = context.insert_set(eligible_participants_path, maybe_participant.clone().value)?.context;
    }
}
```

The emitter borrows the 5000-element FixedVector and clones individual values only when needed. The first version cloned the entire vector twice and overflowed the default 2 MiB Rust test stack; the borrowed version passes on that stack. Typed element and length checks, parameter-only iterable rejection, and effectful condition rejection stay explicit.

The exact original Welcome source compiles for TypeScript and Rust, its generated standalone crate builds, and the checked TS capture regenerates byte-for-byte. Empty and one-present-participant constructor cases match TS/Rust serialized public state, eligible/organizer Set contents, private state, and witness calls. The TS fixture records 4 versus 5 constructor queries with ordered VM tags and four-dimensional per-query gas. Rust `ConstructorResult` exposes state/private but not constructor gas or an ordered trace, so neither gas nor VM parity is claimed. The default-stack Rust test, 86 renderer tests, 22 inventory tests, 146 fixture freshness checks, targeted Rust 1.99 Clippy, formatting, and source-cohort check pass. The exact-source inventory adds three proof-required missing rows and one pure nonproof row: 305→308 proof-required, 236 available unchanged, 69→72 missing, 343→344 nonproof, 36→32 unassessed, zero identity drift on the isolated base. `--rust-require-recording` rejects exactly `add_participant`, `add_organizer`, `check_in` at `StateAction::Assert actions[0]`; no proof-to-ledger claim. Final Nix package and integrated root gate remain pending; no push or remote CI.


Final pinned compiler verification: `${HISTORICAL_NIX_STORE}/68gvdx05bdhqm2is7kn9n657z96k2awz-compactc/bin/compactc` compiled the exact source for both targets, matched the checked generated fixture, and passed 146/146 fixture freshness checks. The pinned full inventory is `${LOCAL_EVIDENCE}/adr112-pinned-inventory.json`: 194 sources, 684 exported circuits, 308 proof-required, 236 available, 72 missing, 344 nonproof, 32 unassessed, no missing compiler proof rows and no baseline identity drift. IR12/runtime ABI37. This supersedes the pending Nix note above; root integration gate remains pending.
