# CoPS-001 — Ledger decoding resource limits

Status: **byte-admission mitigation delivered for 0.3.0; aggregate decoded resource containment deferred by owner approval**
Date: 2026-10-08
Owner: Compact Rust runtime / upstream ledger serializer integration
Delivery: [#494](https://github.com/MediaNoxLabs/compact/issues/494), signed [`9ffd7880`](https://github.com/MediaNoxLabs/compact/commit/9ffd7880405ed5cfd62909ed4aac64a638c7fc02)
Deferred problem: [#495](https://github.com/MediaNoxLabs/compact/issues/495), no milestone assigned
Decision: [ADR-0359 — Select a practical ledger decoding byte policy](../rust/adr/0359-select-a-practical-ledger-decoding-byte-policy.md); earlier byte admission: [ADR-0353 — Admit serialized ledger inputs with explicit byte limits](../rust/adr/0353-admit-serialized-ledger-inputs-with-explicit-byte-limits.md)

## Problem

Contract-state bytes and verifier-key artifacts are decoded at an application boundary. The pinned ledger serializer checks format and has recursion/initial-allocation precautions, but its API does not expose an aggregate per-request decoded heap, object-count or CPU budget. An input-size check alone cannot establish those guarantees.

This concerns runtime ingestion of ledger artifacts. It does not change Compact source typing, Rust AST lowering, circuit execution semantics, or proof verification. A successfully decoded state/key is not thereby authenticated.

**Evidence classification:** source-established missing resource contract, with tested byte-admission mitigation. No denial-of-service exploit, amplification ratio, arbitrary-input panic or production incident was reproduced. Do not label all TS or ledger8 decoding vulnerable from this evidence.

## Does it also exist in TypeScript and mainstream ledger8?

**Yes, the missing aggregate resource-budget API is also present in the inspected TS/ledger8 paths.** The native Rust and TS/WASM integrations have different wrappers around related upstream decoding. The Rust limited APIs additionally expose an encoded-byte policy; the inspected TS wrappers do not.

| Target | Pinned identity | Actual entrypoint | Byte admission / aggregate resource contract |
|---|---|---|---|
| Rust-AST legacy |MediaNoxLabs/compact 9ffd7880; ledger 8.0.3 / serializer 1.0.0 |`ObservedContractState::decode`, `decode_verifier_key` |No added encoded-byte policy; upstream behavior preserved. No per-call aggregate decoded budget. |
| Rust-AST limited |Same source |`decode_with_limit`, `decode_verifier_key_with_limit` |Checks slice length before upstream parsing; explicit 64 MiB opt-in preset or custom override. No unlimited fallback. Does not bound decoded heap/CPU or acquisition. |
| Mainstream Compact TS ledger-8 |[eb72a5ab](https://github.com/LFDT-Minokawa/compact/tree/eb72a5ab6085bf9b8564cc80c98c973e04825c2d); compact-runtime 0.16.101; locked onchain-runtime-v3 3.0.0 |Reexports `ContractState` / `ContractOperation` |No Compact-owned binary admission wrapper at these reexports. Delegates to the next row. |
| TS onchain JS/WASM 3.0.0 |Source tag [3e72c131](https://github.com/midnightntwrk/midnight-ledger/tree/3e72c13160328368ade84fb4b73fb944caeeb781); installed package 3.0.0/integrity retained |`ContractState.deserialize(raw)`; `ContractOperation.verifierKey` setter |JS calls WASM without a budget option; shared adapter copies `Uint8Array.to_vec()` then calls `tagged_deserialize`. No pre-copy encoded admission or caller-selected aggregate budget in the inspected adapter. |
| Ledger-v8 JS/WASM 8.0.3 |[615be91b](https://github.com/midnightntwrk/midnight-ledger/tree/615be91b079ed8df4026c1fd75352ea6d49de1a4) |Reexported state/operation wrappers; versioned verifier-key construction |Verifier constructor copies bytes and decodes version v3; rejects unsupported version labels. No encoded-size policy parameter at that boundary. |

This is a comparison of the stated pinned release/source interfaces. It does not assess ledger 9/10, every application transport, or every node endpoint. Installed npm WASM hashes are retained; tagged-source-to-binary correspondence was not rebuilt reproducibly. WASM address-space/host limits are not a documented successful per-request decoder budget.

### Source anchors

- [Compact TS reexports](https://github.com/LFDT-Minokawa/compact/blob/eb72a5ab6085bf9b8564cc80c98c973e04825c2d/runtime/src/index.ts#L101).
- [WASM ContractState decoding](https://github.com/midnightntwrk/midnight-ledger/blob/3e72c13160328368ade84fb4b73fb944caeeb781/onchain-runtime-wasm/src/state.rs#L445) and [verifier-key setter](https://github.com/midnightntwrk/midnight-ledger/blob/3e72c13160328368ade84fb4b73fb944caeeb781/onchain-runtime-wasm/src/state.rs#L490).
- [Shared JS-to-Rust decode adapter](https://github.com/midnightntwrk/midnight-ledger/blob/3e72c13160328368ade84fb4b73fb944caeeb781/onchain-runtime-wasm/src/lib.rs#L59).
- [Ledger8 versioned verifier-key constructor](https://github.com/midnightntwrk/midnight-ledger/blob/615be91b079ed8df4026c1fd75352ea6d49de1a4/ledger-wasm/src/contract.rs#L339).
- [Pinned serializer checks and Vec decoding](https://github.com/midnightntwrk/midnight-ledger/blob/3e72c13160328368ade84fb4b73fb944caeeb781/serialize/src/deserializable.rs#L29) and [allocation/read helpers](https://github.com/midnightntwrk/midnight-ledger/blob/3e72c13160328368ade84fb4b73fb944caeeb781/serialize/src/util.rs#L36).
- [Rust runtime byte policy](https://github.com/MediaNoxLabs/compact/blob/9ffd7880405ed5cfd62909ed4aac64a638c7fc02/runtime-rs/src/transaction/decoding.rs).

## Existing upstream protections

Exact tag/type decoding rejects malformed, incomplete and trailing data. Participating types use recursion checks (50 in debug / 250 otherwise in pinned source). Vec initial reservation uses an estimate capped at 32 MiB; subsequent element decoding can grow it. Byte-vector reading uses 4,096-byte chunks. These are real precautions, but an initial reservation cap is not a total allocation cap, and query gas does not meter this deserialization route.

## Delivered mitigation and practical sizing

Before, callers either used a compatibility decoder or chose their own byte policy. Now they can opt into a documented preset:

```rust
let limit = EncodedSizeLimit::default(); // 64 MiB = 67,108,864 encoded bytes
let state = ObservedContractState::decode_with_limit(
    address, state_bytes, observation, limit,
)?;
let verifier = decode_verifier_key_with_limit(key_bytes, limit)?;
```

A deployment may choose a different limit using `EncodedSizeLimit::new(max_bytes)`; `max_bytes()` exposes the configured value for acquisition policy. Explicit overrides are not clamped to the preset. Legacy decode calls are unchanged and do not implicitly apply it. Refusal never retries without a limit.

Retained sizing covers 43 verifier samples (35 unique; maximum 2,119 bytes) and 1,262 ContractState samples (535 unique; maximum 17,450 bytes).64 MiB is about 3,846 times the largest measured state. Samples include DID, reducers, Jubjub and historical Counter/shielded artifacts. The adopted passport family is pure and supplies no applicable verifier/state artifacts.

This partial measured corpus supports generous practical headroom. It does not establish a maximum legal future state, a population-wide size bound or a safe 64 MiB input process-memory ceiling. Legitimate larger deployments can increase their policy. Applications must bound acquisition before constructing a slice when that input is untrusted.

## Verification

Seven small deterministic decoder integration tests pass, including real state and seed-353 verifier roundtrip, default/custom policies, exact/one-under/zero size admission, malformed tags, truncation, trailing input and size-error precedence. Scoped Clippy and formatting pass. No preset-sized stress allocation, proof generation or TS adversarial execution was performed for this finding. All actual decoder function bodies remain byte-identical; only policy construction/access and documentation were added.

[Decoder preset sizing — 2026-10-08](evidence/decoder-preset-sizing.md), [CoPS-001 — Pinned decoder comparison](evidence/pinned-decoder-comparison.md) and [ADR0359 — Practical byte policy receipt](evidence/decoder-policy-receipt.md) retain hashes, source snapshots, raw commands and execution limits. Independent sibling review found no actionable issue in the additive policy change. Existing external decoder source review remains scoped to its frozen source.

## Deferred resolution

Owner explicitly chose byte admission for 0.3.0 and deferred aggregate decoded memory/object/CPU containment. This amends the decoder-specific assurance scope; it does not claim the broader promise was implemented. The problem remains tracked in #495.

A future design must specify acquisition, encoded bytes, recursion, decoded allocations/objects and work limits; choose upstream accounting or another reviewed containment mechanism; preserve legitimate configurable artifacts and cross-language errors; and test enforcement of the exact promised budget. Process isolation is one possible approach, not a decided implementation. A JS length check alone would improve admission but would not resolve aggregate containment.

## History

- ADR0353/#481 added explicit byte admission and a direct deterministic codec corpus.
- Owner approved the narrower decoder resource scope for 0.3.0 and requested generous sizing plus cross-backend problem statements.
- ADR0359/#494 delivered the 64 MiB opt-in preset in 9ffd7880; CoPS-001/#495 preserves the cross-runtime limitation and follow-up.

Publication destination at milestone closeout: `doc/cops/0001-ledger-decoding-resource-limits.md`. This series remains separate from `coips/` and `doc/rust/adr/`.
