---
id: RUST-ADR-0252
alias: ADR-0252
source_sha256: af757f9ca2f6eccfee55689a19f89656ba8dfa9fa1088e62aa9211ad94826fa4
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0252 — Characterize a coherent ledger8.1 compatibility profile

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-isolated-probe. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-isolated-probe
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/377
```

## ADR-0252 — Characterize a coherent ledger8.1 compatibility profile

### Problem
DID v0.7.0 combines onchain3.0 contract execution with ledger8.1 transaction APIs. Published Rust8.1 packages are real and change more than JS wrappers. Current8.0.3 source compilation cannot establish8.1 transaction acceptance. Bumping only one Rust crate risks duplicate incompatible carriers.

### Before / after and bounded decision
```toml
# Before: production acceptance profile
midnight-ledger = "=8.0.3"
# Candidate experiment: whole matched graph, not this single substitution alone
midnight-ledger = "=8.1.0"
```
Preserve production pins while probing a scratch copy with the coherent published8.1 component graph. Compile current runtime/macros, record dependency/API errors and duplicate carriers, then compare canonical serialized fixtures and changed Merkle/wallet behavior. No automatic production promotion from build success. Native DID work can continue on current3.0 with bounded claims. A later promotion needs original strict proof/ledger and existing wallet/observation gates.

### Ownership, alternatives, verification
Upstream ledger/zk retain semantics; adapter/runtime changes must be explicit, minimal and regression-tested. Reject blindly mixed exact pins or universal compatibility inferred from selected byte-identical source files. Preserve source/compiler/package checksum provenance and original TS capture profiles. No live-service mutation or CI. Detailed primary-source evidence below is research, not a tested compatibility result.

## R030-16 / R030-09: DID v0.7.0 ledger compatibility research

Read-only research, 2026-10-07. No manifest edits, dependency resolution changes, compilation, proof execution or service changes. Metadata and source comparisons are evidence; runtime compatibility is not yet established.

### Recommendation

Keep the current ledger-8.0.3 Rust profile supported while beginning unchanged DID source/native adoption. **Do not describe that as complete compatibility with DID's transaction stack.** For final pinned DID transaction/proof acceptance, prepare a separate coherent **ledger-8.1.0 compatibility profile** using the exact published Rust component graph, then promote it only after the gates below. Published Rust crates exist; no substitute VM or custom crypto implementation is needed.

Do not simply change the single `midnight-ledger` dependency: its required onchain-runtime 3.1.0 / Zswap 8.1.0 conflict with current exact 3.0.0 / 8.0.3 direct pins and could leave duplicate incompatible Rust carrier types. A profile change must deliberately select the entire relevant graph. No immediate whole-runtime rewrite is justified.

The important distinction: DID's **contract simulator and Schnorr signer use compact-runtime 0.16.0 → onchain-runtime-v3 3.0.0**, while its wallet/transaction layer uses **ledger-v8 8.1.0**. This mixed JavaScript graph is the actual upstream pinned baseline, not something to silently normalize to a single package version. Initial DID native comparisons against Rust's onchain 3.0.0 are therefore well motivated. The 8.1 transaction compatibility decision is separate.

### Immutable provenance

- DID tag `v0.7.0`: `4e7f6b0f69bf4e2c8506a9693f8d0c3dfe68e550`, verified through official GitHub API and local Git object.
- DID `pnpm-lock.yaml`: direct ledger-v8 8.1.0; compact-runtime 0.16.0; compact-js 2.5.0 also brings compact-runtime 0.15.0; both runtime packages resolve onchain-runtime-v3 3.0.0. zkir-v2 is 2.1.0. DID contract package itself still says 0.6.0; do not label all monorepo packages 0.7.0.
- DID flake lock pins MediaNoxLabs/flake-collection `fbc948baae944d6a1fe813fe2d626e538b210027`; its exact `compact-toolchain.nix` selects official compiler **0.31.1**, with platform hashes. This is not the current Rust branch's compiler version.
- Ledger 8.0.3 tag and installed published crate `.cargo_vcs_info`: `615be91b079ed8df4026c1fd75352ea6d49de1a4`.
- Ledger 8.1.0 tag and independently downloaded, checksum-verified published ledger and Zswap crates: `d89e0b6334f83bc9477152fb5edf7eca71660237`.
- Published `midnight-ledger 8.1.0` is not yanked; crates.io SHA256 `2182054f3a43ccabac514448fff2487437be293f517a01afc23905df701e8548`.
- Published `midnight-zswap 8.1.0` is not yanked; crates.io SHA256 `4c83f946d8ac03abea38ca470599801fa08e91e2ddf3cb0963027a7701180c7c`.
- npm ledger-v8 8.1.0 tarball SHA256 `16502aadeab3088a99775c26bce60383017bf1370da12a316e0034ace1e1ae1a`; verified SHA512 matches the DID lock exactly.
- npm ledger-v8 8.0.3 tarball SHA256 `5d84e9c92b89abaff569be57d5c596a511de8defd6a8ecfc5fc009ba1c43e249`; verified against registry SHA512.
- npm metadata does **not** expose gitHead; it names the artifacts repository. Accordingly, registry integrity plus published version/source-tag comparison are retained separately, not claimed as cryptographic proof of npm build provenance.

### Exact relevant Rust graphs

These are upstream release Cargo.lock resolutions, compared with current Compact Cargo.lock; broad Cargo.toml ranges alone are insufficient.

| Component | Current / upstream ledger 8.0.3 | Upstream ledger 8.1.0 |
|---|---|---|
| ledger / Zswap | 8.0.3 | 8.1.0 |
| onchain-runtime / VM | 3.0.0 | 3.1.0 |
| onchain-state | 3.0.0 | 3.0.0 |
| transient-crypto | 2.0.1 | 2.1.0 |
| base-crypto / derive | 1.0.0 | 1.0.0 |
| coin-structure | 2.0.1 | 2.0.1 |
| storage / storage-core | 2.0.1 / 1.1.0 | 2.0.1 / 1.2.0 |
| serialize | 1.0.0 | 1.1.0 |
| circuits / proofs / zk-stdlib | 6.0.0 / 0.7.0 / 1.0.0 | 6.1.0 / 0.7.1 / 1.1.0 |
| ZKIR | 2.1.0 | 2.1.0 |

Cached published new ZK crates all point to midnight-zk `0e3ebfa5039e3addbc2a7db662879863f8e25227`. Old crates have their own VCS pins in `zk-registry-vcs.json`. Their changelogs describe explicit prover RNG threading and foreign-ECC MSM blinding-point sampling changes; do not treat dependency patch/minor numbers as proof-byte identity.

### What actually changes

1. **Not a wrapper-only release.** Changelog emphasizes wallet/event WASM API additions, but Rust transient Merkle updates gain checked `try_update` / `try_update_hash` handling. VM insertion/removal maps these errors to `MerkleTreeError`; ledger/Zswap and wallet apply paths propagate failures rather than relying on prior unchecked updates. Wallet `apply`/`apply_tx` signatures change to `Result` in affected paths. Reusing current observation/wallet code therefore needs adaptation and negative tests.
2. **Wallet/event features:** finer-grained Dust tree/UTXO manipulation, raw event replay, public event contents, Zswap insert/remove-coin helpers, and ranged/already-hashed `findPathForLeaf`. Downloaded npm declaration diff confirms these APIs are in the public package, not merely unreleased source.
3. **Serialization:** serialize 1.1.0 adds tagged sequence deserialization. Existing single-object full-consumption path remains; new raw event APIs use the new surface. No evidence here that all tagged values or malformed-input behavior are universally interchangeable.
4. **Stable selected core source:** 13 inspected files are byte-identical after removing only copyright lines: coin/FAB/contracts, transient field representation/hash/curve/proof facade, ledger structure/verification, onchain context/transcript/cost model, and ledger WASM crypto. Hashes and exact paths are in `selected-source-comparison.json`. No changed tracked files under ledger/static or zswap/static. This narrows the risk but does **not** establish full dependency/wasm/proof equivalence.
5. **Proof dependencies:** circuits/proofs/zk-stdlib versions advance even though ZKIR stays 2.1.0 and selected built-in material is unchanged. Use proof verification plus strict ledger application, not byte equality of randomized proofs, to establish acceptance. Preserve exact key/IR/parameter provenance.

### DID exposure

`packages/contract/src/did.compact` uses Cells, Counters, string Sets/Maps, typed structs, Jubjub keys, timestamps, contract identity, and authorization hashes. The inspected original contract does not itself use shielded transfer or Merkle ADTs; its basic native storage scenarios do not require new 8.1 wallet tree APIs.

`packages/contract/src/test/did-simulator.ts` imports contexts from compact-runtime. `packages/jubjub-schnorr/src/signing.ts` imports EC operations from compact-runtime and normalizes signing scalars. The Compact verifier reduces the challenge to 248 bits with a quotient witness and verifies `ecMulGenerator(response)` against the announcement/key relation. This makes canonical scalar, invalid response, quotient bounds and malformed point behavior concrete parity cases. Existing historical Nix-versus-registry raw noncanonical EC differences must remain visible; do not claim all-Field parity.

DID transaction submission still involves ledger-v8 8.1.0 and Dust/funding/event handling even when application state is plain ledger. Lack of application shielded coins does not remove that acceptance boundary.

### Concrete gates, in dependency order

1. **Reproducible inputs:** freeze DID commit/import closure; source/lock hashes; compiler 0.31.1 baseline versus selected Rust compiler; actual loaded JS package paths/versions; all Rust crate checksums and VCS pins. Fail unknown profiles. Do not overwrite original JS captures or silently substitute ledger 8.0.3 for the DID lock.
2. **Unchanged-source generation:** compile the original DID and Schnorr imports to TS and Rust; Cargo-check unedited generated crate. Inventory all exports with actual proof applicability; preserve explicit native/recorded gaps.
3. **Native/recorded behavior:** port the maintained DID simulator scenarios (initialization, controller/recovery rotation, string maps/sets/relations/services, deactivate, authorization/replay/invalid-signature refusals). Compare return/state/effect/VM/private-output/witness-order evidence. Report summed query, wrapper-last-query and replay gas separately where required; don't normalize real differences.
4. **Small representation bridge executable:** run isolated adapters for the current profile and proposed complete 8.1 profile with exact lockfiles. Golden cases: Fr/Jubjub/hash domains, nested struct and bounded/vector FAB, aligned values, public state/transcript/contract operation serialization, canonical signatures, malformed lengths and noncanonical inputs. Exchange public bytes via files, not cross-version Rust carriers in one dependency graph.
5. **Focused changed-domain negatives:** collapsed/out-of-range Merkle update errors, ranged path lookup bounds; same-block wallet/event/root/frontier identity; raw event sequence corruption/trailing bytes; Dust UTXO/tree successor and missing/consumed entries. These apply to the version bridge, not an invented requirement that DID itself add Merkle storage.
6. **Strict proof boundary:** one real controller-authorized state mutation and one rejection/replay path against unchanged DID; independently verify circuit proof and default-strict fee-funded ledger application under the selected profile. Test public prepared bytes/decoding across JS 8.1 and matched Rust 8.1. Verify cold-cache built-in keys/SRS/keygen provenance. No waived balance checks.
7. **Qualification:** if promoting the Rust profile, rerun existing focused original-source proof/offer/observation and runtime gates because `try_update`/wallet API changes affect those consumers. Inspect `cargo tree -d` for accidental duplicate versions. A matching live node/indexer/proof-server profile requires separate actual evidence; this research supplies no live compatibility claim.

### Primary sources

- DID immutable lock: https://github.com/midnightntwrk/midnight-did/blob/4e7f6b0f69bf4e2c8506a9693f8d0c3dfe68e550/pnpm-lock.yaml
- DID source: https://github.com/midnightntwrk/midnight-did/blob/4e7f6b0f69bf4e2c8506a9693f8d0c3dfe68e550/packages/contract/src/did.compact
- DID Schnorr: https://github.com/midnightntwrk/midnight-did/blob/4e7f6b0f69bf4e2c8506a9693f8d0c3dfe68e550/packages/jubjub-schnorr/src/schnorr.compact
- Pinned toolchain: https://github.com/MediaNoxLabs/flake-collection/blob/fbc948baae944d6a1fe813fe2d626e538b210027/nix/packages/compact-toolchain.nix
- Ledger source comparison: https://github.com/midnightntwrk/midnight-ledger/compare/615be91b079ed8df4026c1fd75352ea6d49de1a4...d89e0b6334f83bc9477152fb5edf7eca71660237
- Ledger changelog: https://github.com/midnightntwrk/midnight-ledger/blob/d89e0b6334f83bc9477152fb5edf7eca71660237/CHANGELOG.md
- Published ledger metadata: https://crates.io/api/v1/crates/midnight-ledger/8.1.0
- Published Zswap metadata: https://crates.io/api/v1/crates/midnight-zswap/8.1.0
- npm metadata: https://registry.npmjs.org/@midnight-ntwrk%2fledger-v8/8.1.0
- Proof-library change log: https://github.com/midnightntwrk/midnight-zk/blob/0e3ebfa5039e3addbc2a7db662879863f8e25227/proofs/CHANGELOG.md
- Circuits change log: https://github.com/midnightntwrk/midnight-zk/blob/0e3ebfa5039e3addbc2a7db662879863f8e25227/circuits/CHANGELOG.md

All downloaded material is public source/package metadata or archive data. No credentials, private wallet snapshots, runtime mutation, Cargo build, or proof/network service execution was involved.

### Local delivery — 2026-10-07

Bounded probe completed: unchanged copied runtime compiles against exact coherent Midnight8.1 graph,37existing+4changed-API tests pass, representative golden bytes identical. Production remains8.0.3; strict proofs, observation identity and version-policy promotion gates remain open under parent#360. [ADR0252 — Coherent ledger8.1 probe receipt](references-0.3.0.md#note-019).
