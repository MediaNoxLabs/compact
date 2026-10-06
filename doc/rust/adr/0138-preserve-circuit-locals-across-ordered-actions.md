---
id: RUST-ADR-0138
alias: ADR-0138
title: "Preserve circuit locals across ordered actions"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["source-lowering", "recording", "lexical-scope", "returns"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 2a65d902596c57e9888b28b790decc4b5aacb9f0f344774dab7e7be8a5ca300b
---
# RUST-ADR-0138 — Preserve circuit locals across ordered actions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted a root-local action/return bridge that retains a pre-write Field Cell value across ordered actions and rejects escaping nested locals. The minimal original let-return fixture is proved; this slice only advances bboard to its next blocker and does not itself establish original bboard acceptance.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#240 closure](https://github.com/MediaNoxLabs/compact/issues/240#issuecomment-6017638306). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`01a4c702`](https://github.com/MediaNoxLabs/compact/commit/01a4c702ca1347e7d7d51a5e3a261d4ebc630d25) · [`f72e22ab`](https://github.com/MediaNoxLabs/compact/commit/f72e22abf7f779a7e48063671e28af3c931a1079). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: Implemented locally · 2026-10-05 · rust-backend-v2 · [issue #240](https://github.com/MediaNoxLabs/compact/issues/240)

### Problem and source boundary

The original ledger-8 `test-center/test-contracts/bboard.compact` compiles for TypeScript but Rust compilation stopped before metadata at a nested `__compact_Cell.write` in `take_down`. A circuit-local value is read before several ledger actions and returned afterward. The earlier Rust source extractor only recognized actions in an outer `seq`; it treated the complete `let*` as a value expression and rejected the write. The original qualified-coin source separately rejects `insertCoin`, coracle rejects a nested ledger query, and micro-dao rejects a standard-library expression. These are separate compiler gaps.

After this bridge, bboard progresses to `Rust backend does not yet support this circuit expression` at line 44, involving a Field-to-Bytes/counter expression. Its three exported circuits remain unassessed. This ADR deliberately proves the shared structural issue using a minimal original Compact source; it does not claim bboard source acceptance.

### Before and after

Minimal source, `examples/rust_backend/let_return_oracle.compact`:

```compact
import CompactStandardLibrary;
export ledger stored: Field;
export circuit replace(next: Field): Field {
  const previous = stored.read();
  stored.write(disclose(next));
  return previous;
}
```

Before: Rust compilation rejects the nested ledger query before contract metadata. A naive return expression that rereads `stored` would return the new value instead of the pre-write value.

After: the compiler emits the existing schema-12 `StateAction::Let`, containing a typed Field Cell-read binding and an ordered Cell-write action, with the final return referencing the bound local. The generated native Rust captures `previous` before writing. The recorded Rust frame records that read once, appends the write, and returns the saved value. Two successive calls `replace(9)` and `replace(13)` return `0` and `9` respectively, matching TypeScript.

Illustrative Rust-facing shape:

```rust
let previous: runtime::Field = stored.read(&mut frame)?;
stored.write(&mut frame, next)?;
Ok(previous)
```

The actual generated code uses typed ledger slots and the existing native/recorded frame APIs.

### Decision and ownership

The Scheme source extractor recognizes only a root `let*` whose tail returns one of its own bindings and whose body contains ordered state actions. It leaves checked unsigned-subtraction expressions on their existing path. It emits the existing typed `Let` action and parameter return. The Rust native emitter makes root bindings visible to that circuit return, while nested `Let` bindings stay action-local. The recorded emitter handles precisely one Field Cell-read root binding returned after ordered actions; it validates the declared Cell kind and ledger index. Other binding/value shapes remain unavailable rather than guessed.

The runtime, VM, ledger primitives and public API are unchanged. The generated crate continues to use midnight-ledger Cell reads/writes and the current recording frame.

### Schema compatibility

Private Rust IR schema 12 is an exact-version JSON contract. Adding a new `bind` action under schema 12 would make old schema-12 consumers misinterpret the same version. This design reuses `StateAction::Let`; it adds no JSON variant and needs no schema or runtime ABI bump. If a future bridge needs a new variant, that change must bump the private schema. Capability report schema 3 is unchanged.

### Guards and tests

- Renderer regression confirms root `Let` returns the pre-write binding and rejects a binding returned from a nested `Let` outside its scope.
- The exact minimal source compiles from the rebuilt Nix `compactc`, and all 148 generated fixtures are fresh.
- A checked independent TypeScript capture and Rust fixture compare two sequential results, serialized state, ordered six-op VM transcript, private outputs, replay, and all four per-query gas dimensions. TypeScript's aggregate `reportedGas` omits the first read in this source shape, so the test sums both TypeScript query meters instead.
- Focused local parity gate reports 1/1 proof-required recorded API. Pinned ZKIR key generation, proof, verification, and ledger-8 validation/application pass for `replace`.
- Full source inventory reports 196 sources, 686 exported circuits, 278 proof-available exports and 25 unassessed exports. `replace` is newly assessed and marked proof-required/recorded/observed; bboard's `post`, `take_down`, and `public_key` remain unassessed. Baseline gains exactly one identity and removes none.
- Fresh compiler: `${HISTORICAL_NIX_STORE}/5gxrwjh7wc32lmndgh1n6civfnjq93yi-compactc`, SHA-256 `92b5524f8d1840a1b109707af829bb9f370d1e2f99c244a2c0f287f09c30efa9`; Rust IR schema 12; runtime ABI 37. Local checks only, no remote CI.

### Remaining work

Issue #240 stays open for the original bboard Field-to-Bytes/counter lowering and subsequent source-level assessment. Coracle's nested query, qualified-coin `insertCoin`, and micro-dao's standard-library expression remain separate blockers. A wider typed root-local recorder would require its own decision and acceptance slices. The broader milestone remains open until its required local and remote gates are complete.


### Local delivery record

Conventional GPG-signed and DCO commit: `da1fb29f5c7fd1e74a079fe0eed9e7f9add0ce5c` on isolated `codex/adr138-qualified-set-bridge` (unpushed). Clean-head focused receipt: `${LOCAL_EVIDENCE}/compact-local-parity-wk_gqe4h/receipt.json`. Full compiler inventory receipt: `${LOCAL_EVIDENCE}/compact-adr138-full-inventory-signed.json`; one added baseline identity, 25 unassessed exports remain. Generated proof artifacts: `${LOCAL_EVIDENCE}/compact-adr138-proof3`; pinned Nix compiler: `${HISTORICAL_NIX_STORE}/5gxrwjh7wc32lmndgh1n6civfnjq93yi-compactc/bin/compactc`. Proof/application, 110 renderer tests, 23 inventory tests, 148 fixture freshness checks, Rust source parity and formatting pass locally. No push or remote CI.


### Parent integration

Integrated signed commit 01a4c702; combined four-source focused gate passes at that head: ${LOCAL_EVIDENCE}/compact-focused-01a4c702/receipt.json. Root inventory ${LOCAL_EVIDENCE}/compact-01a4c702-inventory.json reports 283/317 proof-required available, 34 known gaps and 25 unassessed exports. Commit f72e22ab adds packaged root-local artifact/capability checks and --let-return proof execution to the broad local gate. Combined renderer checks at f72e22ab pass all 116 tests, including root scoping and custody-guard rejection boundaries. Original bboard remains unassessed. Full package gate is in progress.
