---
id: RUST-ADR-0121
alias: ADR-0121
title: "Record closed nested Uint4 conditional projections"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "ternary", "unsigned-arithmetic"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 31c9bd6679c9609d900a72274572db6161fd7e94ee0f6cd80ec0a657b73ee33a
---
# RUST-ADR-0121 — Record closed nested Uint4 conditional projections

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted closed two-level Uint4 conditional projection with validation of every arm and once-only retained conditions. All four walker and both seeded stream flag cases have proof/application evidence; preserve the ENOSPC attempt, subsequent 64 MiB proof-thread setup and distinction between inventory counts and proof counts.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#224 closure](https://github.com/MediaNoxLabs/compact/issues/224#issuecomment-6017610883). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`2502d327`](https://github.com/MediaNoxLabs/compact/commit/2502d32732eb1fce501ab9a962efd7a178ffcc81) · [`80f9b6a0`](https://github.com/MediaNoxLabs/compact/commit/80f9b6a043057828c0726b07df634dffa168264c). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
initiative: compact-rust-backend
milestone: rust-backend-v2
adr: 121
status: accepted
date: 2026-10-05
base: 2502d32732eb1fce501ab9a962efd7a178ffcc81
issue: https://github.com/MediaNoxLabs/compact/issues/224
```

## Historical decision and amendments

### Problem and source evidence

The original `examples/rust_backend/ternary_cond_oracle.compact` exports `walkerNestedIf(c,d)` and `streamNestedIf()`. Both are proof-required and native Rust capable, but the schema-12 capability report at base `2502d327` marks recording and observed calls unavailable. The first gap for each is `unsupported_action`, `StateAction::Let`: `actions[0]` for the walker and `actions[0].action` for the stream. The stream first records `flag.read()`. The compiler then binds a typed `Unsigned<4>` from an outer Boolean `If` whose two arms each contain another Boolean `If` over closed literals 1–4; an immediate `FieldCast` writes the selected value to `fieldCell`. The stream also increments `ops` after the Cell write. Existing single-level conditional lowering does not cover this typed nested local.

### Decision and ownership

The recorded emitter admits only this closed two-level `Unsigned<4>` shape. It validates both outer arms contain a nested `If`, checks every cast/coercion and literal stays within its declared bound, and obtains Boolean conditions only from typed parameters or already recorded local values. It requires the next action to project exactly that unsigned local into one `Field` binding. A following two-action `Sequence` is admitted only with that projection first; subsequent actions retain their existing typed lowering and order. Unsupported arithmetic, witness, ledger reads inside arms, deeper nesting, other unsigned widths and unrelated casts remain unavailable. This is an emitter change only: schema 12, runtime ABI 37, `BoundedUint`, `Field`, Cell and Counter slots, ZKIR and ledger-8 remain owned by their existing packages.

### Before and after generated Rust

Before, the generated crate exposes native `walkerNestedIf` and `streamNestedIf`, but has no `recorded::walkerNestedIf`, `recorded::streamNestedIf` or their typed observed-call methods. After, the recorded walker contains:

```rust
let selected = runtime::BoundedUint::<4>::new(if c {
    if d { 1u64 } else { 2u64 }
} else {
    if d { 3u64 } else { 4u64 }
} as u128)?;
let field: runtime::Field = runtime::Field::from(selected.value() as u64);
let frame = crate::ledger_slots::fieldCell.record_write(frame, field)?;
```

The actual generated names are compiler allocated. The stream first calls `flag.record_read(frame)?`, evaluates both nested conditions from that retained Boolean, writes the Field, and only then calls `ops.record_increment(frame, 1u16)?`. Both receive `recorded::` and `recording.*_call` APIs.

### Acceptance and limits

A checked schema-12 IR fixture and renderer test require both APIs and reject a typed unsigned arithmetic arm. Fresh TypeScript capture plus generated native/recorded Rust tests cover all four walker argument combinations and both stream flag states, including a seeded true Cell state. They compare serialized initial and final states, unit result, four gas dimensions, ordered public VM shape, zero private transcript outputs and replay state/effects. The per-query TypeScript costs sum to the native and recorded execution cost; TypeScript's wrapper reports the last query separately. Pinned ZKIR 2.1.0 must compile both prover/verifier keys, and typed observed calls must equal manual recorded prototypes and prove, verify and apply via ledger-8 for all six cases. The 37-oracle acceptance manifest links the new fixture; the full local compiler gate checks both capability rows and proof artifacts. No remote CI or push.

### Local evidence

- Exact a6 schema-12 Scheme frontend plus local AST emitter reports both exports `recorded=true`, `observed_call=true`, `proof_required=true`; prior statuses were both unavailable. This is a +2 API availability delta only, pending clean-head full inventory.
- Fresh six-case TypeScript/native/recorded parity and replay test passes. `streamTrue` uses an explicit pre-call Cell seed in both runtimes, which is excluded from call gas and transcript comparisons.
- Pinned `${HISTORICAL_NIX_STORE}/zlxf4bz9rd53wn6rawyahn0x65sqsjw7-zkir-2.1.0/bin/zkir` compiled `walkerNestedIf` and `streamNestedIf` ZKIR, binary ZKIR and prover/verifier keys.
- Ledger-8 proof/verify/apply passes for all six branches with pinned ZKIR 2.1.0; the first build failed with ENOSPC, then the isolated incremental cache was cleared and ordinary mode passed with its own 64 MiB thread stack.
- Renderer 93/93, ternary fixture 7/7, targeted Clippy, formatting, oracle source manifest and focused parity gate pass. Full integrated-head gate remains pending.


### Tracking

- [Milestone-v2 issue #224](https://github.com/MediaNoxLabs/compact/issues/224).
- Signed conventional GPG+DCO feature commit `60758cf45b0216fdfe42bbe2f648395e3d128efe` is ready for root cherry-pick. Issue [#224](https://github.com/MediaNoxLabs/compact/issues/224) has the delivery receipt.

### Focused inventory delta

Exact original ternary source inventory using the a6 schema-12 Scheme frontend and local emitter has 21 proof-required exports: recorded/observed APIs 15→17, gaps 6→4, unassessed 0. Only `walkerNestedIf` and `streamNestedIf` gain APIs; this is availability evidence, not yet proof execution. Receipt: `${LOCAL_EVIDENCE}/adr121-ternary-inventory.json`.

### Proven local evidence

Pinned ZKIR 2.1.0 produced both circuits’ binary ZKIR and prover/verifier keys. The focused proof smoke matched each typed observed call to the manual recorded prototype and proved, verified and applied all six calls through ledger-8: walker TT/TF/FT/FF and stream flag false/true. The state checks confirmed the selected Field values 1/2/3/4 and 4/1, unchanged flag, and Counter 0 for walker or 1 for stream. The first Cargo attempt hit ENOSPC and corrupted its isolated incremental cache; after removal, a direct 64 MiB stack run passed. The proof mode now allocates a 64 MiB thread stack internally; its ordinary invocation passed all six cases. No remote CI or push.
### Delivery

Signed conventional GPG+DCO local feature commit `60758cf45b0216fdfe42bbe2f648395e3d128efe` from isolated base `2502d327`; clean worktree, no push or remote CI. Focused local compiler + Cargo gate receipt: `${LOCAL_EVIDENCE}/adr121-focused-final/receipt.json` (1 source, 17/21 recorded). Full root integration and broad local gate remain to be completed.

### Integrated root receipt

Root integrated the signed feature commit as `80f9b6a0`. The exact clean schema-12 package is `${HISTORICAL_NIX_STORE}/1lg0x67f0whqrf6ca07wa8b7my0pbpw6-compactc`; root inventory reports 258/316 recorded APIs available, 58 missing, 25 unassessed, and zero source drift. Combined renderer 96/96, selected ternary 7/7, and formatting passed at integration. These repository-wide figures are API availability, not proof counts.
