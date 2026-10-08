---
id: RUST-ADR-0312
alias: ADR-0312
source_sha256: 58f1ce7b1b6a06866694689b2ac968a3d2f010d9061a93a9379fbb908f6cf4b8
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0312 — Measure a complete P256 variable-base scalar component on ledger8

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** superseded-owner-excluded-from-0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: superseded-owner-excluded-from-0.3.0
date: 2026-10-07
parent: R030-12
milestone: "0.3.0"
```

## ADR0312 — Measure a complete P256 variable-base scalar component on ledger8

### Problem

ADR0310 establishes one canonical modular product, not the dominant arithmetic required by ACC P256 signatures. Full ECDSA needs variable-base scalar multiplication with complete exceptional-case handling. The original producer lacks a P256 instruction. Measure the arithmetic feasibility before selecting a substantial producer integration.

### Before / after code

```rust
// Before: isolated fixed-prime modular product only.
// Proposed schematic component, actual constrained code is the evidence:
let base = points.assign_nonidentity_on_curve(public_base)?;
let scalar = scalars.assign_canonical(witness_scalar)?;
let result = points.mul_full_width(base, scalar)?;
points.constrain_public(result, public_result)?;
```

Canonical P256 base and scalar integers have separate domain wrappers and fixed moduli. Reduction is distinct from decoding. Point identity has an explicit constrained representation. No unconstrained host predicate supplies acceptance.

### Decision and component model

Build one isolated reusable component under scratch using qualified architecture-v1 BigUint/native/range instructions. Internal field, point and scalar components own arithmetic and canonicality. Adapt the cached attributed RustCrypto complete A=-3 formulas (primeorder0.13.6 Algorithms4–6) only after pinning source/archive/VCS identity and preserving license notices. Those CPU formulas are source references, not ready-made constraints or new dependencies. Every selection, identity, on-curve relation, modular reduction, scalar bit and inverse relation must be constrained. No guessed substituted foreign-chip configuration.

Acceptance relation Q=[s]P uses a nonidentity canonical on-curve variable base and a canonical scalar0<=s<n with all256bits constrained. Q may be identity for scalar0 and must use a canonical public encoding. Circuit shape cannot depend on scalar bits. Generator multiplication uses the same component. Include complete-law controls P+O,O+P,P+P,P+(-P), scalar0/1/n-1, independent non-generator/full-width vectors, off-curve/noncanonical inputs, and wrong result. Use existing pinned JS/RustCrypto vector sources as independent host references without adding unqualified Rust dependencies. Do not substitute tiny scalars for the full-width cost relation.

### Resource and verification boundaries

Scratch-only source and locked/offline dependency reuse. No production/compiler/runtime/ABI/protocol changes, no network/SRS download or new external package identities. Wait until the current performance measurement releases its CPU/source freeze before building or executing the probe. Lease the released scratch target explicitly; never modify another run's artifacts.

First run component unit/mock obligations with bounded shape. Measure the actual complete256bit relation before keygen. Five-minute cost/preflight ceiling,15-minute keygen and15-minute single-proof ceiling; stop below3GiB free disk and cap the child RSS at3GiB. Only attempt a real transport proof if k<=16 with existing cached trusted parameters. If measured k>16, retain exact cost/source/test evidence and stop for the next resource/implementation decision; do not lower scalar width or omit checks. Finite-bound outcomes are useful evidence, not full adoption.

If feasible within those bounds, one real ledger8 key-read/init/static-verifier roundtrip and public binding/infinity/missing-input/truncation negatives are required. Otherwise report real proof as not attempted. Preserve failed attempts, phases, bytes, row/k counts, tool/feature/lock/SRS identities and timing/RSS limitations.

### Limits and later work

This component is not ECDSA, WebAuthn, a security audit, frontend codegen or contract proof communication binding. A separate producer/ABI ADR must select one honest integration route. Existing K1/P256 arithmetic proofs cannot be attached to an ordinary ACC call without preserving ProofPreimage transcripts and communication commitments. Full ACC#356 stays open. Independent constraint review is required before any production adoption.

See [ACC PR177 — Faithful P256 and producer adoption path](references-0.3.0.md#note-003) for exact signature/WebAuthn requirements and upstream measurements on the different ledger9 stack.

### Tracking

https://github.com/MediaNoxLabs/compact/issues/436; parent#356/#429.


### 2026-10-07 — ADR0312 resource stop, component candidate retained

The isolated attributed complete A=-3 candidate has separate canonical base/scalar domains, explicit identity and on-curve constraints, complete projective add/double, constrained selection and a fixed256-round variable-base relation. Seventeen bounded component mock expectations pass, including complete-law cases and scalar decode/decomposition boundaries. Independent OpenSSL-derived non-generator/full-width vectors are retained, but their full scalar relation was not executed.

During actual full-width cost modeling, the watchdog stopped the process at the approved3GiB RSS ceiling after194.06seconds total. Sampled RSS3,225,141,248bytes includes a3.9MB sampling overshoot. Component mocks took104.05seconds; the subsequent model phase had not returned. Full k and rows are **unknown**. No full-width scalar mock, key generation or real proof was attempted; component rows do not supply a full-model estimate. No post-stop retry, raised limit, lower scalar width or omitted constraint.

This is evidence that the current straightforward candidate cannot finish this cost-model run within the chosen3GiB budget. It does not establish impossibility of P256 on ledger8, constraint soundness, full ACC adoption or a production-ready component. Root reviewed candidate ownership/formula adaptation at source level; independent constraint audit and full-width functional validation remain open. Existing production backend/runtime/IR/ABI and dependencies are unchanged.

Archive [ADR0312 — Complete P256 candidate and resource stop.zip](references-0.3.0.md#note-096), SHA256 `df3554c4fc3deeb61e00ec5aba7f8166f640b3025613ed20fb85108e571488b7`;34verifiedfiles. Issue#436 stays open with a measured stop disposition, as does ACC#356/#429. Next work needs an explicit resource/implementation decision; no automatic escalation is implied by this evidence. Parent acceptance remains8/20.


### 2026-10-07 — Owner excludes P256/WebAuthn; ADR0313/#438

The owner explicitly dropped P256 support from the ledger8 ACC target following ADR0312's 3 GiB model stop. [ADR-0313 — Exclude P256 and WebAuthn from the ledger8 ACC variant](0313-exclude-p256-and-webauthn-from-the-ledger8-acc-variant.md) supersedes the earlier requirement to port that arm. P256, WebAuthn and full-width P256 producer/resource work are outside required 0.3.0 acceptance. Existing research evidence stays intact; #436 closes as **not planned**, not implemented.

An isolated patch removes the exact P256/WebAuthn import block: 35 P256 circuits, two WebAuthn helper circuits and one policy re-export are excluded. The remaining 80 circuit exports (35 Jubjub, 35 k256, 10 shared) and 25 ledger fields are preserved byte-for-byte after the already qualified scalar adaptation. Shared RP fields remain part of Jubjub grant commitments. Both TS and Rust frontend probes still refuse `Secp256k1EcdsaSignature`; secp256k1 is separate and remains an explicit open requirement. Full ACC adoption is not yet accepted. No production backend/runtime/IR/ABI changes or P256 retries.

Receipt SHA256 `b7ced885480a100fe084a3edbbdccce1321ddfac363d6dc1d1d7b985714cb70a`. Verified archive [ADR0313 — ACC P256 exclusion and frontend boundary.zip](references-0.3.0.md#note-097), SHA256 `08da1dcd9c711aa3f4f5ef3882c66f32dedd9ee8f42d7c078f46cb13a98fa936`. See [ACC PR177 — Ledger8 variant without P256 and WebAuthn](references-0.3.0.md#note-007). Parent denominator remains 20; accepted parents remain 8. The remaining scope requires independent behavior, recording, replay and strict proof evidence as applicable.
