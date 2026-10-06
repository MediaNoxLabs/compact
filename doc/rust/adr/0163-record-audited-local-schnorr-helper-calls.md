---
id: RUST-ADR-0163
alias: ADR-0163
title: "Record audited local Schnorr helper calls"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "local-call", "Schnorr"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: fffdb7b6fb75cf7940c0514d0878fd4a0ae64dc27ba2cef13b9073815c594154
---
# RUST-ADR-0163 — Record audited local Schnorr helper calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Native-local calls preserve a guarded frame around helpers with no public VM operation, enabling local Schnorr behavior. The audit rejects changed public query, gas, Zswap or context state; it is not a general effectful-call escape hatch.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#268 closure](https://github.com/MediaNoxLabs/compact/issues/268#issuecomment-6017683289). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
title: ADR-0163 — Record audited local Schnorr helper calls
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem and actual source model

The unchanged ledger-8 `schnorr_attest_oracle.compact` exports `verifyAttestation` and `acceptAttestation`, both proof-required and currently unavailable at `StateAction::CircuitCall`. Fresh schema-13 IR and generated Rust show ordinary generated `schnorrVerifyDigest -> schnorrVerify` helpers. The latter is a local cryptographic circuit with a transient hash, tuple-returning `getSchnorrReduction` witness, unsigned/Field reconstruction, Jubjub EC operations, identity guards, and equality checks. The fixture's old comment about name-routed runtime Schnorr lowering is stale; this ADR does not edit it without updating oracle provenance. The exported roots each read `open` and `attestorKey`; `acceptAttestation` then increments `acceptedCount` by one.

### Before and intended after

Before: `ledger_contract::verifyAttestation` and `acceptAttestation` execute natively, but no `recorded::*` or `recording().*_call` methods exist. After, the generated recorded path should use declared Cell slots for the open and key reads, call an audited local helper with the typed digest/signature/key, preserve its witness effects, and record the optional Counter increment. Conceptual generated Rust:

```rust
let (frame, open) = ledger_slots::open.record_read(frame)?;
assert!(open, "attestor is closed");
let (frame, key) = ledger_slots::attestorKey.record_read(frame)?;
let (frame, ()) = frame.call_local(|context| {
    schnorrVerifyDigest(context, witnesses, digest.clone(), signature.clone(), key)
})?;
// acceptAttestation only: acceptedCount.record_increment(frame, 1)?;
```

The exact API and syntax remain subject to implementation and review.

### Proposed decision and reuse boundary

Use a generic, structurally audited native-local-call bridge in `RecordingFrame` for helpers that cannot emit a public VM operation. It should consume a frame, run the generated native helper, assert that query state/effects and Zswap state are unchanged, then adopt only the helper's private state, metered gas, private transcript outputs, and result. Reject at runtime if an unrecorded public effect appears. Static admission must recursively audit the called IR tree for local expressions, assertions, typed witness calls and local helper calls, rejecting Cell/Counter/Set/Map/List/Merkle actions and reads inside the helper. The exported root still records its Cell reads and Counter through existing typed slots. Reuse ADR-0162 typed-value/lexical-binding planning where it gives a principled way to carry the digest/signature/key across actions; avoid another contract-name matcher. No name-based Schnorr shortcut. A runtime ABI increment may be needed for the bridge; decide after implementation constraints are verified.

### Negative guards

A helper gaining a public ledger action/read, changed root Cell or Counter declaration/index, effectful nested call, wrong argument provenance/type, missing open assertion, altered order, or additional root action must stay unavailable. Native invalid signature, identity key/announcement, bad reduction, and closed-state failures must match original TypeScript; failures must not produce an applicable recorded transaction.

### Acceptance

Fresh original TypeScript oracle and native Rust versus recorded Rust and independent Verify replay for verify and accept, with valid signature and meaningful invalid/closed cases. Compare serialized state, ordered public VM operations, all four gas dimensions, private state/effects/tuple witness output, and witness calls. Prove/verify each root with pinned ZKIR 2.1.0 and ledger-8 validate/apply; verify-only leaves ledger state unchanged and accept increments exactly once. Add renderer positive/negative IR mutations, generated fixture freshness, focused exact-head gate, strict Clippy, conventional GPG+DCO commit. Local only: no push, remote CI or Nix rebuild.

### Research status

Fresh schema-13 IR inspected on 2026-10-05. Coordinating with ADR-0162 typed-value planner before implementation. Issue and delivery evidence to follow.


### Decision and delivered evidence (2026-10-05)

Issue: https://github.com/MediaNoxLabs/compact/issues/268. ABI 38 adds `RecordingFrame::call_local`. The frame checks the entire public query state/effects/address/call context, Zswap coins/spends/outputs/tree/free index, coin key, cost model, and gas limit before adopting a generated native helper result. It then adopts private state, private transcript outputs, result, and helper gas. The renderer admits only a typed two-argument exported root with ordered Boolean open Cell read, typed point-key Cell read, and an audited Unit callee; accept may follow with a declared one-unit Counter increment. The helper audit is transitive through `StateAction::CircuitCall`, rejects unknown and recursive calls, every public state expression/action, native witness built-ins and unproven pure calls, and visits both operands of transient commitments. This is structural, with no Schnorr name dispatch. ADR-0162 typed planning was intentionally not reused because its pure-only scope excludes witnessful helpers.

Actual generated code is `let (frame, ()) = frame.call_local(|context| super::schnorrVerifyDigest(context, witnesses, digest, signature, public_key))?;` after `open.record_read` and `attestorKey.record_read`. Accept then calls `acceptedCount.record_increment(frame, 1_u16)`.

Fresh unchanged Compact source TypeScript output and native/recorded Rust agree for verify and accept on ordered public VM (6 and 9 operations), private witness transcript and state, all four per-query gas dimensions, and replay state/effects. The TypeScript circuit result gas reports its last query: verify `(170000000,1249915903,0,0)`, accept `(170000000,1323481916,356,356)` in `(readTime,computeTime,bytesWritten,bytesDeleted)`. Summing the actual call queries gives verify `(340000000,2499830756,0,0)` and accept `(510000000,3823312672,356,356)`; this is the native/recorded result gas. One combined replay uses a different VM cost profile: verify `(340000000,1425344287,0,0)` and accept `(510000000,1674339734,356,356)`. These three measures are separate and are asserted separately. No constructor seed or manual closed-state seed cost enters either recorded call gas comparison.

Negative evidence: malformed tuple reduction fails `Invalid challenge reduction`; identity announcement fails `Schnorr verification requires a non-identity key and announcement`; closed attestor fails before the key read with `attestor is closed`. Renderer mutations reject effectful nested Counter action, a Cell read hidden in transient commitment opening, missing guard, wrong key type, unknown helper, and recursion. Runtime test rejects an unrecorded Cell write and retains nonzero local gas/private outputs.

Pinned ZKIR 2.1.0 generated two prover/verifier key pairs in `${LOCAL_EVIDENCE}/compact-adr163-proof`. `compact-rust-proof-smoke --schnorr-attestation` replayed and partitioned both generated traces, prepared typed calls, verified both proofs, validated and applied both ledger-8 transactions. Verify left public ledger state unchanged; accept incremented once. Fixture updater checked 152 generated crates, 152 refreshed for ABI 38, zero failures; subsequent freshness and strict Clippy gate run before commit. Local only, no push or remote CI.
