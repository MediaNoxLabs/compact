---
id: RUST-ADR-0147
alias: ADR-0147
title: "Record plain Merkle reset to default"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merkle", "reset"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: ecc2fa2c6268d7a7796cec1ccad1001b5146d15ff621f2d8b4d5d3228b2fbda2
---
# RUST-ADR-0147 — Record plain Merkle reset to default

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted plain Merkle reset through one shared canonical native/recording program with the declared depth. Empty/populated behavior and an applied reset proof restore blank state and frontier; historic history initialization remains separate.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#247 closure](https://github.com/MediaNoxLabs/compact/issues/247#issuecomment-6017649521). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4360d651`](https://github.com/MediaNoxLabs/compact/commit/4360d65180e3626a27e3128bc2610200f359f575) · [`f72e22ab`](https://github.com/MediaNoxLabs/compact/commit/f72e22abf7f779a7e48063671e28af3c931a1079). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0147 — Record plain Merkle reset to default
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original merkle_tree_oracle reset_tree export remains native-only. All seven other proof-required exports in that source now have recorded and observed-call APIs. Native reset already uses the canonical ledger-8 program; recording must reuse it, including the declared depth and storage accounting, rather than substitute an unmetered state replacement.

### Before and after

Compact source:
```compact
export circuit reset_tree(): [] { t.resetToDefault(); }
```
Before: ledger_contract::reset_tree(context) executes natively. After: ledger_contract::recorded::reset_tree(context) returns replayable verification operations and ledger_contract::recorded::Contract.reset_tree_call(&observed, private_state) can prepare a proof-ready call.

### Emitter and runtime decision

Extract the existing plain reset VM sequence into one internal program builder used by both native execution and RecordingFrame. Add a plain-only typed MerkleSlot recording method. Admit only StateAction::MerkleResetToDefault with the exact plain Merkle declaration, physical field/index and declaration depth. Historic reset and its root-history initialization stay separate. Reuse midnight-ledger StateValue, Op and Merkle constructors. No new private IR variant, schema version or VM implementation.

### Required evidence

Capture reset_tree from the original TypeScript contract after the existing populated-tree sequence. Compare generated native and recorded results, serialized state, first-free position, roots, all four gas dimensions, ordered VM operations, private outputs and independent Verify replay. Test empty and populated trees plus renderer wrong-kind/index rejection. Generate pinned ZKIR proof artifacts, verify the observed call and apply through ledger-8. Update the broad compactc proof gate and strict recording expectation, fixture freshness, inventory, signed commit and integration evidence. Record exact scope; this closes only the plain-tree source, not historic or whole-repository parity.

### Status

Proposed before implementation. Waiting for the current combined full local gate to finish before changing its source tree. Tracking issue follows. No remote CI or push.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/247 (rust-backend-v2), created before implementation.

### Implementation and focused evidence

Implemented in reused isolated checkout merkle-root-recording on codex/adr147-plain-merkle-reset based f72e22ab while the main checkout remains frozen for full gating. The internal merkle_reset_program is shared by native and RecordingFrame; the public plain slot pins path and depth. No new VM implementation or IR change. All 117 renderer tests and eight plain Merkle tests pass, including empty/populated reset, wrong-kind/index rejection, historic reset exclusion, and zero-gas rejection. Fresh TypeScript capture: each reset has three VM operations, zero private outputs; empty gas 85000000/1168755395/0/0, populated gas 85000000/1234450028/92/1122 (read/compute/written/deleted). Native, recorded and replay match state, root, first-free and all gas dimensions. Proof and whole-fixture freshness are still running; do not treat this paragraph as proof completion.

### Proof and freshness

Pinned ZKIR 2.1.0 artifacts in ${LOCAL_EVIDENCE}/compact-adr147-proof compiled all eight exports with --rust-require-recording. The generated reset_tree trace replayed/partitioned; its observed call matched direct recording, proof verified, and deployment/call validated and applied through ledger-8. Applied state equals the original blank constructor state and first-free is zero. All 148 generated fixtures are fresh. Targeted Clippy is running before the signed delivery commit.

### Signed delivery

GPG-good and DCO-signed commit 308964533b85f2a426655f4c84348090a4c77024 (feat(rust): record plain Merkle reset to default). Targeted all-target/all-feature Clippy passed for runtime, backend, fixture and proof-smoke. Clean signed-head focused gate ${LOCAL_EVIDENCE}/compact-focused-adr147/receipt.json passed with 8/8 proof-required recorded/observed calls. Runtime ABI 37 and IR schema 12 unchanged. The broad gate now compiles the original source with --rust-require-recording and invokes --merkle-reset. Main-branch integration awaits completion of its ongoing f72e22ab full checkpoint; no push or remote CI.


### Main integration evidence — 2026-10-05

Main4360d651 passed the affected consumer/proof/ledger phase at ${LOCAL_EVIDENCE}/compact-consumer-4360d651/receipt.json, including this integrated slice. This closes the phase interrupted by the earlier stack overflow/harness TypeError. It does not claim a whole-workspace rerun at4360. Both Merkle fixtures additionally pass16/16 recorded at ${LOCAL_EVIDENCE}/compact-focused-4360d651/receipt.json; main renderer122 passes.
