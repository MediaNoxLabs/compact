---
id: RUST-ADR-0287
alias: ADR-0287
source_sha256: 7db98df535509074d893509223c9cd2f72fc06d425a9b8de9d2bc1bb68de57ae
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0287 — Compare unchanged DID against its original release compiler and runtime

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-isolated-oracle. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-isolated-oracle
date: 2026-10-07
milestone: "0.3.0"
parents: [R030-09, R030-16]
issue: https://github.com/MediaNoxLabs/compact/issues/411
```

## ADR-0287 — Compare unchanged DID against its original release compiler and runtime

### Problem
The DID v0.7.0 source is adopted unchanged, but current TypeScript parity captures use branch compiler0.31.133/runtime0.16.101. The upstream release pinned compiler0.31.1/runtime0.16.0 with onchain-runtime-v3 3.0.0, and ledger-v8 8.1.0 for transactions. Branch agreement does not establish original-release behavioral compatibility.

### Decision
Run an isolated original-release oracle recapture, approved by root before implementation. Preserve branch oracles and all live compiler/runtime/generated-contract sources. Freeze current constructor/lifecycle/pure/point/alias/service/Schnorr/JWK scenario definitions and run the original compiler/runtime in a separate directory. Digest/relation additions remain explicitly open until separately delivered. No production dependency pin promotion, transaction bridge, audit investigation, CI or publication is included.

| Before | After |
|---|---|
| Unchanged DID source → compiler0.31.133 → runtime0.16.101 branch captures | Separate unchanged DID source → compiler0.31.1 → runtime0.16.0 original-release captures, compared against frozen branch rows |

```js
// Before: capture is coupled to branch runtime and enforces branch realpath.
import * as r from "../../../runtime/dist/index.js";
// After: isolated test-only adapter resolves an explicit pinned runtime root,
// verifies package identity and generated module resolution, and records both.
const r = await import(verifiedOriginalRuntimeEntry);
```

Adapter changes are limited to runtime selection/provenance and output destination where necessary. Scenario bodies, generated source, assertions/authentication and source contracts are not rewritten to obtain agreement. Version-specific API differences must be recorded before any adapter change. No coercion may erase a semantic difference.

### Immutable input identities
DID tag v0.7.0 commit `4e7f6b0f69bf4e2c8506a9693f8d0c3dfe68e550`. did.compact SHA256 `632f34af543924edb185fe9c8eda54aa4ca0503b0b9e8ee3ff06ec470f6d7456`; schnorr.compact `f072731d730d72f8b76b183df2c5187b233a0838d9462894cfb0a6d2f0f66bff`. Original lock SHA256 `e9e73ac3b4212f523504bebc13ffbd763b95e9d96e2737a950c3febe903c689c`. Toolchain flake collection `fbc948baae944d6a1fe813fe2d626e538b210027` selects official0.31.1 aarch64-darwin archive with Nix fetchzip hash `sha256-QKfLjKbOBSIuxJXfYhkPgDJkn4CcsRsV6M1ULSRem9o=`; this is unpacked Nix content identity, not raw archive SHA256. Record actual executable/archive hashes and verify provenance independently. Preserve original lock integrity for runtime0.16.0 and transitive onchain3.0.0; compact-js also brings runtime0.15.0, which must not silently replace the contract runtime.

### Acceptance
- Record exact source/compiler/package/loaded-module paths and hashes; unknown or wrong profiles refuse.
- Unchanged source compiles and all frozen scenarios execute against original runtime with separate original and branch evidence.
- Compare typed outputs, serialized state, error text/type, witness/effect order, private outputs and query/program evidence. Preserve query-sum, wrapper-last-query and replay gas separately.
- Classify real semantic/compiler/wire/ordering differences and create minimal regressions under R030-11; do not rewrite expectations until differences are understood.
- Matching finite cases establish only those semantics. No8.1 transaction/proof/network acceptance is claimed. DID and compatibility parents remain open.

### Ownership
Only `/tmp/rust030-adr287` and ADR/evidence notes owned by this experiment. Root owns journal/dashboard/register and reviews any future production harness changes. Reuse existing installed/preserved artifacts where verified; no unnecessary Rust rebuild.

### Local experiment delivered
All seven scopes match original-release and branch captures exactly:18constructor attempts,203calls,102pure vectors; complete deterministic repeat; explicit missing/wrong-version/wrong-realpath runtime refusals. No production files changed. Compiler0.31.1 reports ledger-8.0.2, while the original application lock uses ledger-v8 8.1.0; preserve both. Root review tightened the evidence runner to fail closed for missing/failed scopes and nonzero differences; focused subprocess refusal tests pass without rerunning unchanged source captures. The current receipt hash is in [ADR0287 — Original DID release oracle recapture](references-0.3.0.md#note-068); this ADR deliberately does not embed the receipt hash to avoid a circular file digest. Parent acceptance and8.1transaction boundary remain open.
