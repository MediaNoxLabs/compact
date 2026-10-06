---
id: RUST-ADR-0135
alias: ADR-0135
title: "Record distinct one-field struct constructor calls"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "struct", "pure-helpers"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 6107c6fa2a266d9b46e3ad0507d5ca8ca7286163d2aaeacb4b46f9ac251b2a72
---
# RUST-ADR-0135 — Record distinct one-field struct constructor calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted exact one-Field struct constructor helpers and matching projections, preserving disambiguated module result identities rather than structural coincidence. Both proof-required calls are proved; module wrapper exports remain nonproof, and swapped identities or assertion-bearing helpers remain refused.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#237 closure](https://github.com/MediaNoxLabs/compact/issues/237#issuecomment-6017633643). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`967eee59`](https://github.com/MediaNoxLabs/compact/commit/967eee596f0fe732911c6de1e90210c571dd0543). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
initiative: compact-rust-backend
milestone: rust-backend-v2
adr: 135
status: accepted
date: 2026-10-05
base: 967eee59
```

## Historical decision and amendments

### Problem and source evidence

The exact source `examples/rust_backend/struct_collision_oracle.compact` has two proof-required exports, `runAlpha` and `runBeta`, with native Rust and a TypeScript state fixture. The schema-12 capability receipt marks both recorded and observed-call APIs unavailable at `StateAction::Let`, `actions[0]`, `unsupported_action`. Each local is a `Call` to a module-expanded pure helper, followed immediately by field-zero `StructField` into a Field Cell write. `makeAlpha(Field)` returns `RecCompact1 { alpha: Field }`; `makeBeta(Field)` returns `Rec { beta: Field }`. The distinct names and field identities are the source fixture's disambiguation invariant. There is no recorded/proof evidence for these calls yet.

### Decision and ownership

Add one guarded AST recording rule for a pure helper with exactly one Field parameter and an exactly matching one-field struct result. The helper body must be a `StructLiteral` of that exact result type whose sole Field value derives from that parameter without effects. The caller must bind that exact result type, pass one closed typed Field source, immediately project the matching field at index zero from the bound local, then write precisely that projected Field to the existing typed Cell path. Retain the result and projection as typed Rust locals before the Cell write. Resolve the callee through the schema-12 pure-circuit table, preserve its exact disambiguated result type, and reject any differing struct identity, field, shape, source, or helper body. Do not infer purity from the helper's name or merely from a matching shape at the call site. No runtime, ledger, ZKIR, ABI, or schema change is intended.

### Before and intended generated Rust

Before, generated `ledger_contract::runAlpha`/`runBeta` work natively but `recorded::runAlpha`/`runBeta` and typed observed-call entry points are absent. The intended recording mirrors the checked native call and threads the existing frame:

```rust
let arg: runtime::Field = x;
let retained: crate::types::RecCompact1 = crate::pure_circuits::makeAlpha(arg)?;
let projected: runtime::Field = retained.alpha;
let frame = crate::ledger_slots::lastAlpha.record_write(frame, projected)?;
```

For Beta the exact type/member/Cell are `Rec`, `beta`, and `lastBeta`. Generated local names and formatting may differ.

### Required acceptance

Add a focused schema-12 renderer positive fixture for both distinct result identities and negative fixtures for a mismatched identity/member and an effectful or otherwise nontrivial helper body. Capture fresh TypeScript Alpha/Beta calls and compare generated native/recorded state, unit result, all four gas dimensions, ordered VM operations, private effects, and replay. Compile pinned ZKIR 2.1.0 and prove, verify, and apply both typed observed calls through ledger-8. Update the original-source inventory only when the recorded/observed APIs exist; distinguish API availability from actual proof evidence. Run focused local checks and necessary exact-head gate only, with no push or remote CI.

### Tracking

Milestone-v2 issue and signed conventional DCO/GPG feature commit to follow. This proposed ADR claims no implementation or parity result.

### Observed outcome

The guarded AST recorder is implemented for one Field parameter, an exact one-field `StructLiteral` helper body, and the immediate matching projection and Cell write. The generated crate retains the disambiguated `RecCompact1`/`.alpha` and `Rec`/`.beta` types at value sites. The source proof-required availability moves from 0/2 to 2/2 recorded and typed observed-call APIs; the two module wrapper exports remain nonproof. This is an API availability count, separate from the proof runs below.

The schema-12 renderer fixture covers both distinct types. Its negative cases reject a cross-module helper swap at typed validation, an unrelated Cell value at recording, and an assertion-bearing helper body. All 106 renderer tests pass. Fresh TypeScript capture for `runAlpha(5)` and `runBeta(7)` matches the checked generated Rust crate for initial/final serialized state, unit result, four gas dimensions, ordered three-operation VM shape, empty private outputs and replay. The earlier sequential native state fixture continues to pass. Pinned `midnight-zkir 2.1.0` compiled both binary ZKIR prover/verifier key pairs; the typed calls matched direct recorded prototypes, then each proved, verified, validated and applied through ledger-8, storing Field 5 or 7 in its distinct Cell.

The focused local source gate passed 1 fixture and 2/2 recorded APIs at `${LOCAL_EVIDENCE}/adr135-focused-gate/receipt.json`; original-source inventory receipt is `${LOCAL_EVIDENCE}/adr135-inventory.json`. Selected generated crate tests (2/2), targeted Clippy, formatting, deterministic TypeScript recapture, and `git diff --check` pass. The exact integrated-head full local gate remains for parent integration; no push or remote CI.

### Tracking and delivery

[Milestone-v2 issue #237](https://github.com/MediaNoxLabs/compact/issues/237) tracks this change. Signed conventional GPG+DCO commit to follow. The source compiler for this isolated receipt used the pinned schema-12 Scheme sibling `${HISTORICAL_NIX_STORE}/1lg0x67f0whqrf6ca07wa8b7my0pbpw6-compactc/bin/compactc-scheme` with the updated local Rust backend from base `967eee59`; root will produce an exact integrated-head package after cherry-pick.

Signed conventional GPG+DCO feature commit `c48e092fe984832f11fc49f92de17d16cf55e6dc` is ready for parent cherry-pick from isolated base `967eee59`; `git verify-commit` reports a good signature and the worktree is clean. Exact integrated-head gate remains pending.
