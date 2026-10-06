---
id: RUST-ADR-0168
alias: ADR-0168
title: "Checked literal Bytes-to-Field normalization"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler", "Field", "literal"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 49ee462e8a7d82f30a3bbf7c1847eb3839a8b3e207acef2e4963d0a04b04f133
---
# RUST-ADR-0168 — Checked literal Bytes-to-Field normalization

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Supported Bytes-to-Field literals are normalized as little-endian natural integers and checked against the field range without modular reduction. Compile-time rejection can differ in timing from the TypeScript oracle's evaluation-time failure.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#272 closure](https://github.com/MediaNoxLabs/compact/issues/272#issuecomment-6017690339). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
The unchanged original micro-dao source fails Rust IR emission in standard-library expressions before any of its eleven exports can be assessed. A read-only analyzed-AST trace identifies four literal `cast-from-bytes` nodes: the 28-byte nonce-evolution domain and the 30-byte alternate domain used by evolveNonce/sendShielded/mergeCoin. The shared emitter currently has no case for them.

### Before / after
```compact
// Existing original standard-library expression, currently rejected by Rust IR
"midnight:kernel:nonce_evolve" as Field
```
```json
// After: existing typed IR Field literal, with the checked little-endian integer
{"kind":"field_literal","value":"<canonical decimal integer>"}
```

### Decision
Normalize supported byte literals to their little-endian natural integer and validate with the existing compiler field? predicate (0 through max-field, inclusive). Reuse field_literal IR and native Field construction. No modular reduction, runtime API, ABI or schema change. Unsupported dynamic Bytes casts remain explicit diagnostics. Preserve source locations and reject target types outside this slice.

### Semantics evidence
Existing TypeScript runtime convertBytesToField folds bytes little-endian and rejects values above MAX_FIELD. Compiler field? uses the same canonical scalar range. The source type checker does not permit Bytes<0>→Field casts, although the internal TS helper maps an empty sequence to zero; tests must preserve that language boundary rather than silently admit it.

### Validation
Independent original TypeScript literal values for nonce domains, zero and byte-order probes, 32-byte canonical boundary and overflow; explicit empty/dynamic rejection where source support is absent. Checked generated Rust values and existing IR/runtime range guards. Re-run original micro-dao source unchanged to report the next real blocker; this slice does not claim complete DAO native admission. Coordinate schema14 Scheme build, fixture freshness and targeted tests; signed local delivery and exact receipt where applicable.

### Remaining micro-dao blockers
ADR0164 only adds qualified Set.insertCoin. Original micro-dao still needs shielded receive/send/mint/merge and pot.writeCoin semantics. These will be diagnosed after removing the constant-cast blocker and kept as explicit backlog, without substituted or reduced-contract parity claims.


### Accepted implementation and receipt (2026-10-05)
Issue: https://github.com/MediaNoxLabs/compact/issues/272 (rust-backend-v2).
Signed local commit: 3c7c09b5977d1da202dca389f1d62989e390af01. No runtime or schema change (ABI39/schema14).

Shared Scheme literal helper serves pure/stateful expressions and stateful return. Closed byte vectors include the canonical maximum; emitted IR is FieldLiteral with decimal 52435875175126190479447740508185965837690552500527637822603658699938581184512. Rust continues using existing Field::from and checked Field::from_le_bytes.

#### Evidence
- New Scheme: ${HISTORICAL_NIX_STORE}/x22f37782r8rb9dadk2qpm5bganbdg26-compactc-binary-nixos/bin/compactc-scheme.
- Independent generated TS oracle: AB=16961, zero32, both nonce domains, canonical max. Constructor and stateful assertion/read match native/recorded state, public VM shape, replay and private transcript.
- TS reports one query gas while making two actual queries. Capture retains both aggregate and per-query costs; Rust total matches actual query sum: read340000000, compute2499829766, written/deleted0. No actual charges normalized away.
- Six negative guards: dynamic bytes, dynamic vector, empty cast, Uint target, max+1, all-FF32. Empty remains a source type error.
- Explicit failure-timing difference: Rust rejects overflowing closed constants during compilation; independently generated TS accepts compilation and rejects checked evaluation. Dynamic casts stay unsupported.
- Direct stateful literal return compiles natively as Expression(FieldLiteral); its existing recorded-return capability gap remains. Recorded fixture uses typed Cell return to test supported composition.
- 155 fixtures fresh, zero stale/failures; 8 backend library +12 CLI +139 renderer tests; new fixture test; targeted fixture/proof-runner Clippy; workspace formatting; all source guards.
- Snapshot proof verified and applied through ledger8 with pinned ZKIR2.1.0. Persistent artifacts ${LOCAL_EVIDENCE}/compact-adr168-proof retain ZKIR/BZKIR, prover22643B and verifier1351B. Runner: cargo +1.99.0 run -q -p compact-rust-proof-smoke -- --literal-bytes-field ${LOCAL_EVIDENCE}/compact-adr168-proof, dedicated compactc-consumer target.
- Frozen focused receipt PASSED, 1/1 recorded: ${LOCAL_EVIDENCE}/compact-focused-3c7c09b5-literal-bytes-field/receipt.json.

#### Remaining original source work
Unchanged test-center/test-contracts/micro-dao.compact passes all four standard-library literal nonce casts. Next failure is line192:22: nested Counter.read inside no.lessThan(yes). Shielded receive/send/mint/merge and pot.writeCoin remain separate admission work. No claim of complete eleven-export DAO admission or production readiness.
