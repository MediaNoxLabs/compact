---
id: RUST-ADR-0232
alias: ADR-0232
title: "Prove both ledger Boolean branches of conditional Counter circuits"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["proof", "Counter", "conditional"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: f25322d5c3640d3c7e77aa0bf3433ab036749b3ae65f25300d0d9dedaba0473f
---
# RUST-ADR-0232 — Prove both ledger Boolean branches of conditional Counter circuits

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. Proof smoke now covers both Boolean ledger branches of the conditional Counter circuits and a sequential unsigned reset/set case, preserving old false paths. These are selected seeded proof/application cases, not every branch of the corpus.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#336 closure](https://github.com/MediaNoxLabs/compact/issues/336#issuecomment-6017799957). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c) · [`157d7033`](https://github.com/MediaNoxLabs/compact/commit/157d703312d8b37a097379c9ae437f4e4694cf17). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0232 — Prove both ledger Boolean branches of conditional Counter circuits
status: accepted for local implementation
milestone: rust-backend-v2
head_before: 157d703312d8b37a097379c9ae437f4e4694cf17
```

## Historical decision and amendments

<!-- compact-adr-final-acceptance:03c03a39a2d39c43c91143b9526ca1c2ffae706c:ADR-0232 -->
### Final remote acceptance — 2026-10-06

At published commit `03c03a39a2d39c43c91143b9526ca1c2ffae706c`, all 10 required remote workflows passed on the same revision; the root-reviewed acceptance record and closed rust-backend-v2 milestone cover this bounded decision. [Issue #336 closure record](https://github.com/MediaNoxLabs/compact/issues/336#issuecomment-6017799957) and the [milestone closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) document the verified scope.

[Final evidence bundle](references.md#private-note-06) · [Architecture successor](references.md#private-note-07)

Earlier status statements, observations, tests and limits below remain historical evidence. Final acceptance supersedes their pending wording and does not expand implementation scope.

### Historical decision and delivery record


### Problem

The original `ternary_cond_oracle.compact` exports `streamConstAnnotated` and `streamIncrement` after a `flag.read()`. ADR-0086/#187 and ADR-0087/#189 delivered recorded APIs and independent TypeScript/native/recorded/replay parity for both Boolean values. The proof-smoke selector currently proves, verifies and applies only the default `false` flag path for those two circuits (`main.rs` lines 794–795). Thus the `true` path has execution parity evidence but no ZK proof plus ledger application evidence. The selector already proves both parameterized `streamWrite` branches; this change closes the two state-backed cases without changing source or compiler capability.

### Before / after

Compact source, unchanged:

```compact
export circuit streamIncrement(): [] {
  const f = flag.read();
  ops.increment(f ? 3 : 4);
  wideCell = f ? 10 : 20;
}
export circuit streamConstAnnotated(): [] {
  const f = flag.read();
  const x: Uint<16> = f ? 1 : 2;
  ops.increment(x);
  fieldCell = disclose(1 as Field);
}
```

Before proof selector: `("streamConstAnnotated", false, 2)` and `("streamIncrement", false, 4)` only. After: retain both false cases; add `("streamConstAnnotated", true, 1)` and `("streamIncrement", true, 3)`. Seed the typed Boolean Cell to true **before** deployment and run the recorded call against that same seeded state. Verify the initial flag is true, selected Counter delta, Uint64 Cell `10` versus `20`, Field Cell `1`, proof verification, and ledger application. Do not seed a post-deployment state unseen by the ledger.

### Decision and boundaries

- This is a test-only proof/application extension in `tools/compact-rust-proof-smoke/src/main.rs`. No emitter, private IR schema, runtime ABI, runtime method, generated fixture, ZKIR, Compact source, capability metadata, or TypeScript oracle change.
- Reuse the existing pinned original-source compiled output and keys, the current `check_generated_trace` and `check_transaction` path, and the existing ledger-8 test static assets. `check_transaction` uses an unbalanced smoke setting; describe application with that exact strictness, not default-strict funded acceptance.
- Keep the existing false rows so the test explicitly covers both source branches. Independent TypeScript/native/recorded/replay parity remains the existing test in `tests-rust-backend/ternary-cond-oracle/tests/ternary_cond_oracle.rs` lines 1017–1072 and its capture.
- This does not establish all source branches, all state combinations, full lifecycle, or node submission.

### Acceptance

1. Focused original-source proof selector runs all six rows (two streamWrite plus four state-backed rows), proving, verifying and applying through pinned midnight-ledger for each; false rows remain successful.
2. `streamConstAnnotated(true)` ends with true flag, Counter 1, Field Cell 1; `streamIncrement(true)` ends with true flag, Counter 3, Uint64 Cell 10. False rows retain Counter 2/4 and Uint64 Cell 20.
3. Each recorded trace is checked against its own deployed seeded state; no test-time mutation of the source or generated crate.
4. Run focused Rust test and strict Clippy as needed, record exact source/key/compiler/ledger and test receipts; signed conventional DCO commit, no push from this isolated branch.

### History

This ADR is created before implementation. It closes the residual true-flag proof/application checkpoints named by ADR-0086/#187 and ADR-0087/#189; it does not revise their established parity or API claims. Remote CI result remains a separate later acceptance item.


### Preimplementation amendment — #180 sequential unsigned proof residual

Read-only review of #180/ADR-0080 found a separate specific proof checkpoint left open by its delivery comment: the pinned proof-smoke gate covers `set_byte(255)`, `writeWide(max)` and `readWide(max)`, but has no `bounded_uint_oracle.set_small` or `cross_circuit_oracle.reset_and_set` selector. `reset_and_set` is the smallest consequential missing shape: original source calls `reset()` then writes a bounded `Uint<64>` in one frame, and its existing TypeScript/native/recorded/replay test compares both ordered writes and summed gas. This ADR's test-only acceptance now includes one original-source `reset_and_set(7)` proof, verification and ledger application, with ordered two-operation trace and final Uint64 Cell `7`. The proof runner should use the already compiled cross-circuit fixture as a new typed dependency/selector and wire that selector into the existing `--proof` gate. This is one representative sequential-write proof; `set_small` remains only compiled/keyed and behavior-tested, so #180's full representative small/wide/read acceptance should not be called complete solely from this slice. This amendment precedes implementation; no source or runtime semantics change.


### Local delivery — 2026-10-06

Conventional GPG-verified/DCO commit `1971823c6cd718a33c27ec401aac355537d2c656` on isolated `codex/adr232-conditional-true-proof` implements this test-only decision; issue [#336](https://github.com/MediaNoxLabs/compact/issues/336) is in `rust-backend-v2`. Source/compiler/runtime/schema/ABI and existing false cases are unchanged. The existing full `--proof` script now invokes the conditional Counter selector and the new sequential unsigned selector using its compiled original-source proof outputs.

At this signed HEAD, `streamWrite(false/true)`, `streamConstAnnotated(flag=false/true)` and `streamIncrement(flag=false/true)` each replay, prove, validate and apply. The true flag was written before deployment; final true-branch Counter values are 1 and 3, and `streamIncrement(true)` stores Uint64 10 (false stores 20). Original `cross_circuit_oracle.reset_and_set(7)` records the two source-ordered Cell-write VM triplets, replays, proves, validates and applies with final Uint64 Cell 7. These use `check_transaction`'s unbalanced smoke strictness (`enforce_balancing=false`), not a funded default-strict transaction or node submission. The retained old ternary keys match the current d5dd original-source `.zkir` SHA256; the cross-circuit keys were generated from that original source by the pinned portable compiler.

Focused existing TypeScript/native/recorded/replay tests for both ternary values and cross-circuit sequential writes pass. Strict proof-smoke all-target/all-feature Clippy, Cargo formatting, Python gate syntax and GPG/DCO checks pass. Exact source/key/compiler/log hashes and status are in `${LOCAL_EVIDENCE}/compact-adr232-delivery/receipt.json` (SHA256 `4da81a5fbe41f485467b929a404d753a8043b35408f726176a01795a31f5d603`). This focused receipt does not replace the parent's integrated full gate or remote CI. `set_small` retains behavior/key evidence but no dedicated proof case here; #180's broader representative acceptance should be assessed separately before closing it.
