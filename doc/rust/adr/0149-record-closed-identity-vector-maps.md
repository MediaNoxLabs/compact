---
id: RUST-ADR-0149
alias: ADR-0149
title: "Record closed identity vector maps"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "vector", "pure-values"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 4d74d6ced8e5ec9a40c5dde81aa5d4a1bdae2a75201465deb1ed3881ae868a5f
---
# RUST-ADR-0149 — Record closed identity vector maps

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted closed literal identity VectorMap values before typed Cell writes for both original map sources, retaining their distinct constructor states. Both calls are proved; arbitrary lambda work/effects remain excluded. Preserve the 64 MiB proof-worker provision separately from default-thread generated fixture tests.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#251 closure](https://github.com/MediaNoxLabs/compact/issues/251#issuecomment-6017656047). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4360d651`](https://github.com/MediaNoxLabs/compact/commit/4360d65180e3626a27e3128bc2610200f359f575). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0149 — Record closed identity vector maps
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original map_fn_oracle and map_lambda_oracle sources compile natively but their exported ping calls remain unrecorded. Each call computes a statically sized Uint64 vector using an identity lambda over a closed literal tuple, binds it, then writes the vector Cell. Replaying the Cell write requires a typed pure value; no VM operation belongs to the map itself.

### Before and after

Compact: c = map((x: Uint<64>): Uint<64> => x, [0 as Uint<64>, 0, 0]);

Before: only ledger_contract::ping(context) is available. After: recorded::ping(context) and recorded::Contract.ping_call(&observed, private_state) reuse the existing typed vector Cell recording path. Native generated mapping and its checked coercions remain the authority for computing the value.

### Decision

Extend pure value admission for an exact VectorMap with identity parameter return, matching declared element/result types and length, and a closed unsigned literal source. Reuse expression_with_calls for type checking and AST construction, then existing scoped static bindings and Cell slots. Reject arbitrary lambda arithmetic, effects, witnesses, state reads, calls and mismatched lengths/types in this slice. No runtime, IR schema or ABI change. This shared structural rule covers both sources without source/circuit names.

### Acceptance

Independent TypeScript captures for both original sources, preserving distinct constructor states. Compare native and recorded ping result, serialized state, ordered VM operations, all gas dimensions, private outputs and Verify replay. Prove/verify/ledger-8 apply both generated observed calls. Positive and negative renderer cases; fresh generated fixtures; broad gate includes both selectors. Signed commit plus focused gate and exact inventory delta.

### Status

Recorded before implementation. Work follows completed ADR0147 on the isolated root checkout while main full checkpoint continues. Issue follows; no push or remote CI.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/251 (rust-backend-v2), created before implementation.

Focused validation now passes both sources: original constructor state and ping state, native/recorded result, public VM, private outputs, all gas dimensions and Verify replay. The capture hook snapshots only the circuit query list before the post-call ledger inspection, excluding that separate observer read. Renderer accepts identity/literal/exact-shape mapping and rejects nonidentity body and wrong length. Both constructors remain unchanged, including arithmetic in the map_lambda constructor. Pinned proof validation is in progress.

### Proof and fresh outputs

Both source calls passed pinned ZKIR 2.1.0 proof verification and ledger-8 deployment/call validation and application. Artifacts: ${LOCAL_EVIDENCE}/compact-adr149-map_fn-proof and ${LOCAL_EVIDENCE}/compact-adr149-map_lambda-proof. The proof-smoke selector uses the existing 64 MiB proof-worker convention after the combined proof body exceeded the macOS main stack; generated native/recorded fixture tests pass on default test threads. This is test-harness stack provisioning, not a generated API change. Each original ping has three VM operations, zero private outputs and gas 85000000/1233942932/48/48. All 148 generated fixtures are fresh. The two original constructor states remain separately verified.

### Signed delivery

GPG-good/DCO commit 4c9d51757837411d95c8ebacb1a5a830d0cab8bb on codex/adr149-identity-vector-map, based completed plain reset 30896453. Clean-head focused gate ${LOCAL_EVIDENCE}/compact-focused-adr149/receipt.json passes both original fixtures, 2/2 proof-required recorded and observed. Four fixture tests, the renderer admission/rejection test, all 148 fixture freshness checks and targeted all-target/all-feature Clippy pass. Both proof cases are included in the broad compactc gate with --rust-require-recording. No runtime, IR or ABI change, no push or remote CI. Main-branch integration follows the current combined gate.


### Main integration evidence — 2026-10-05

Main4360d651 passed the affected consumer/proof/ledger phase at ${LOCAL_EVIDENCE}/compact-consumer-4360d651/receipt.json, including this integrated slice. This closes the phase interrupted by the earlier stack overflow/harness TypeError. It does not claim a whole-workspace rerun at4360. Both Merkle fixtures additionally pass16/16 recorded at ${LOCAL_EVIDENCE}/compact-focused-4360d651/receipt.json; main renderer122 passes.
