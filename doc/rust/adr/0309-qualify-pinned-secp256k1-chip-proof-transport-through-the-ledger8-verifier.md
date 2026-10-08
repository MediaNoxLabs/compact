---
id: RUST-ADR-0309
alias: ADR-0309
source_sha256: 267ad6bc806a30643e92f6b9228297cf23d752ff8bc2a26d6770344635354985
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0309 — Qualify pinned secp256k1 chip proof transport through the ledger8 verifier

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** completed-isolated-feasibility-probe. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: completed-isolated-feasibility-probe
date: 2026-10-07
parent: R030-12
milestone: "0.3.0"
```

## ADR0309 — Qualify pinned secp256k1 chip proof transport through the ledger8 verifier

### Problem

ACC PR177 requires foreign-curve primitives absent from the current shared frontend/ZKIR2 path. The pinned zk-stdlib1.0.0 already contains k1 chips and architecturev1, but source availability does not prove that their key/proof survives unmodified ledger8 verification. Determine that boundary before proposing compiler/producer integration. P256 remains separate and is not solved by a k1 result.

### Before / after

```rust
// Before: source inspection only; no executed transport evidence.
// Probe: constrain the pinned N=T=1 example relation, then use real ledger crypto.
let key = VerifierKey::from(vk);
let roundtripped = tagged_deserialize(&mut serialized_key_cursor)?;
roundtripped.init()?;
roundtripped.verify(&PARAMS_VERIFIER, &proof, formatted_public_inputs)?;
```

This is schematic API flow, not a claimed compiled sample. The experiment records actual code, output and exact crate versions.

### Decision and ownership

Use an isolated scratch consumer specializing pinned bitcoin_ecdsa_threshold example to N=T=1. Preserve its constrained relation, use existing cached Midnight SRS (not its Filecoin example SRS), TranscriptHash and real proof generation. Serialize/read/init the actual ledger VerifierKey through its50000-byte payload limit; preserve architecturev1 and default verifier parameters. The canonical12-field public statement uses the pinned Instantiable helpers, including foreign (element-1) encoding.

Restrict the deterministic sample to a nonidentity key with reconstructed x below scalar order. The example's x comparison does not qualify full ECDSA x-mod-order semantics. Avoid the source-only ECDSASig::from_bytes_le helper finding (s reads the first half); do not patch dependencies or call it a reproduced product defect. Sample generators are not an independent ECDSA oracle. A cryptographic proof of this relation is not contract communication binding, a transaction application, full ECDSA/ACC adoption or a security audit.

Keep source/frontend/runtime/IR/ABI/protocol unchanged. Reuse exact qualified dependency identities and an explicitly leased warm target; zero new external package identities/checksum changes. No network/SRS download or global install. Root holds other work on the leased target until release.

### Acceptance / bounded failure

Capture source/lock/feature/tool/SRS identities, canonical inputs/vector, preflight k/rows, key/proof sizes, timings/RSS and complete key roundtrip. One valid proof must verify through the deserialized key and unchanged PARAMS_VERIFIER. Changed message, changed key, missing input and truncated proof must refuse. Optional12 single-field mutations are verifier-binding checks only.

Stop at preflight k>16, key reader limit, protocol/static-parameter failure or resource budget. Planning watchdogs:5min preflight,15min keygen,15min one proof; preserve incomplete outcomes. Do not weaken the relation, switch verifier, substitute mock verification or silently raise k. With approximately6GiB free disk, stop before less than3GiB remains; reuse cached dependencies. These are experiment limits, not performance promises.

A successful result authorizes a later design decision about exact ACC semantics and constrained contract communication/versioned producer integration. It does not itself authorize new IR instructions or any P256 shortcut. Full source plan: [ACC PR177 — K1 proof transport experiment plan](references-0.3.0.md#note-005).

### Tracking

https://github.com/MediaNoxLabs/compact/issues/433; parent#356 and ACC inventory#429.


### 2026-10-07 — ADR0309 k1 proof transport passed

The pinned N=T=1 relation produced one5072-byte proof, accepted by the actual serialized/deserialized ledger8 VerifierKey and unchanged PARAMS_VERIFIER. Architecturev1; k15;26226 rows;12 public fields;1899-byte processed verifier payload (below50000-byte reader bound),1927-byte tagged key. Key and ProofVersioned::V2 envelopes roundtrip. All16 changed-binding/truncation controls refuse.

The single debug run measured9.405s proving,48.288s total,604340224-byte peak child RSS. These are experiment measurements, not performance baselines. The same trusted cached k16 SRS was downsized in memory to measuredk15 after an initial setup-size mismatch; the failed preproof run is preserved.330 external dependency identities/checksums match the qualified graph, mock verification absent. No live repository/protocol/IR/runtime changes.

This establishes chip/key/proof transport feasibility only. Exact general ECDSA semantics, Compact communication/transcript binding, strict transaction application, generated primitives and P256/WebAuthn are unqualified. Full ACC adoption remains open. The source-only sample-parser finding was avoided, not patched or reproduced.

Durable archive [ADR0309 — Pinned k1 ledger8 proof transport.zip](references-0.3.0.md#note-092), SHA256 `4a66b17310222176eba40eb0b9b84eec3863ca3fd3ee87f41f4854a96c091be4`. Root reviewed the driver and actual outputs. https://github.com/MediaNoxLabs/compact/issues/433 closes its bounded research scope; parent#356/#429 remain open and milestone acceptance stays8/20. Next work requires a separate ADR.
