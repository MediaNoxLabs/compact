---
id: RUST-ADR-0305
alias: ADR-0305
source_sha256: d4b250101d3c0dbc2cb549b25fcdb9339e1cfbc76d55e439d7102818e0e05a53
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0305 — Pin Passport ACC PR177 and preserve authorization across the ledger8 port

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-research-and-local-port. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-research-and-local-port
date: 2026-10-07
parent: R030-12
milestone: "0.3.0"
```

## ADR0305 — Pin Passport ACC PR177 and preserve authorization across the ledger8 port

### Problem and user decision

After DID and digital-passport local acceptance, the user activated ACC adoption and supplied https://github.com/midnightntwrk/passport/pull/177, explicitly requesting a local ledger8 port here. Pin its head **b9e1357c21855c0d5680299faa7ba66a673cc401**, branch `nicolasdp/inbox-view-envelope-experiment`; base observed at **45721e1d322357ef99a2786a3ba27d64382527d8**. The PR is an open draft stacked on #175 and includes account, P-256/WebAuthn and encrypted viewing-inbox work. Preserve the complete head snapshot rather than treating only its final patch as the contract.

The exact contract package selects Compact **0.35.0 `--feature-zkir-v3`**, runtime **0.20.0** and `@midnightntwrk/ledger-v9` **1.0.0-rc.5**. Account imports `webauthn.compact` and `account-p256.compact`; WebAuthn is instantiated for a21-byte origin. Retain package-lock, license, dependent contract sources, test vectors and upstream scenario evidence. Upstream evidence remains upstream evidence until reproduced locally under the new profile.

Read-only baseline compile of unchanged `account.compact` with the accepted ledger8 compiler fails at line2882 with **unbound identifier JubjubScalar** in both `--target rust` and `--target ts`. The first TS attempt used the invalid spelling `typescript`; its CLI refusal is retained and superseded by the correct `ts` run. This is a shared frontend/library gap before Rust emission. The source additionally names Secp256k1 and Secp256r1 types/native verifiers. Existing pinned midnight-zk-stdlib1.0.0 contains secp256k1 foreign-curve machinery; its existence does not establish that current ZKIR2/frontend/prover can express the required operations. Investigate reuse before choosing any backport.

### Before / intended after

```text
Before: PR177 account + P-256/WebAuthn imports
        -> Compact0.35/ZKIR3 -> runtime0.20/ledger9
        -> existing upstream vectors and scenario evidence

After:  immutable original + explicit local compatibility patch
        -> accepted ledger8 compiler / typed Rust backend
        -> reused qualified ledger8/zk primitives where available
        -> independent TS/Rust behavior and strict ledger8 proof/apply
```

The after diagram is the acceptance target, not delivered capability.

### Domain ownership and invariants

Keep account device authority, scoped grantee authority and viewing access distinct. Preserve enrolled device/grantee nonce/version checks, recovery thresholds and delays, RP/origin/flags/signed-byte reconstruction, canonical scalar/point validation and expected signature acceptance/rejection semantics. The envelope is opaque192-byte inbox data; knowing a viewing secret must not create device or spending authority. Exact coin/state/offer behavior must be checked independently of signature arithmetic.

No unsupported authorization primitive may disappear, become an unchecked Boolean or move into a trusted witness while claiming equivalent adoption. Type aliases are allowed only after representation/range/operation equivalence is proved. Do not silently map a canonical scalar to an unconstrained field or infer P-256 from a same-shaped point. Original sources remain immutable; every source compatibility edit needs a patch and before/after evidence. Native ledger pins stay on the accepted ledger8 graph unless a separate explicit decision is necessary.

### Stepwise delivery

1. Preserve immutable source/import/package/toolchain identities and enumerate root exports, helper domains, required witnesses and upstream scenarios. Source text counts are provisional until compiler metadata exists. Keep wallet/control/faucet support separate from the account source closure.
2. Reproduce each first failing layer with a small valid Compact source: scalar type/carrier, foreign curve/verification, standard-library API, ledger operation, imported module/language construct, resource bound, or Rust admission/runtime mapping. Each independent design change gets a child ADR/issue before implementation.
3. Start with source-compatible pure and simple state slices. Reuse upstream primitives; compare against original-profile TS/crypto vectors. Admission/type tests and generated Rust behavior are separate obligations.
4. Rejoin full account exports only after each required primitive passes. Preserve the original authorization/effect paths; compile, execute, replay and strictly prove applicable ledger8 transactions. Add meaningful wrong-key/replay/recovery/nonce/RP/origin/refusal tests from the pinned contract, without reopening previously stopped unrelated security work.
5. Report original-to-ported semantic differences explicitly. No ledger9 upstream network receipt counts as a local ledger8 result. Constructor proof, wallet/browser integration and live network evidence remain separate claims.

### Alternatives and scope

A version-string-only downgrade already fails before Rust emission. Dropping P-256 or retaining only the Jubjub arm would be a partial study, not PR177 ACC adoption. Moving the native Rust dependency graph to ledger9 would violate the requested port. A pure compatibility implementation or bounded compiler/ZKIR backport may be feasible but must be measured and qualified before choosing it. No conclusion of impossibility is made from the first parser/type failure.

Work is local on `codex/rust-backend-ast`, planning/progress in midnight. Current accepted parents remain8/20; ACC becomes active, not accepted. No remote CI, push, upstream modifications or deployment is implied. Coverage on the preceding signed source can finish independently while this source investigation stays in scratch.

### Tracking

https://github.com/MediaNoxLabs/compact/issues/429; parent#356. Created before compatibility implementation.


### 2026-10-07 — Owner excludes P256/WebAuthn; ADR0313/#438

The owner explicitly dropped P256 support from the ledger8 ACC target following ADR0312's 3 GiB model stop. [ADR-0313 — Exclude P256 and WebAuthn from the ledger8 ACC variant](0313-exclude-p256-and-webauthn-from-the-ledger8-acc-variant.md) supersedes the earlier requirement to port that arm. P256, WebAuthn and full-width P256 producer/resource work are outside required 0.3.0 acceptance. Existing research evidence stays intact; #436 closes as **not planned**, not implemented.

An isolated patch removes the exact P256/WebAuthn import block: 35 P256 circuits, two WebAuthn helper circuits and one policy re-export are excluded. The remaining 80 circuit exports (35 Jubjub, 35 k256, 10 shared) and 25 ledger fields are preserved byte-for-byte after the already qualified scalar adaptation. Shared RP fields remain part of Jubjub grant commitments. Both TS and Rust frontend probes still refuse `Secp256k1EcdsaSignature`; secp256k1 is separate and remains an explicit open requirement. Full ACC adoption is not yet accepted. No production backend/runtime/IR/ABI changes or P256 retries.

Receipt SHA256 `b7ced885480a100fe084a3edbbdccce1321ddfac363d6dc1d1d7b985714cb70a`. Verified archive [ADR0313 — ACC P256 exclusion and frontend boundary.zip](references-0.3.0.md#note-097), SHA256 `08da1dcd9c711aa3f4f5ef3882c66f32dedd9ee8f42d7c078f46cb13a98fa936`. See [ACC PR177 — Ledger8 variant without P256 and WebAuthn](references-0.3.0.md#note-007). Parent denominator remains 20; accepted parents remain 8. The remaining scope requires independent behavior, recording, replay and strict proof evidence as applicable.


### 2026-10-07 — Jubjub only; full ACC deferred by authorized fallback (ADR0314/#439)

The owner also excludes secp256k1 and permits removing full ACC adoption from 0.3.0. [ADR-0314 — Restrict ACC authorization to Jubjub and bound adoption work](0314-restrict-acc-authorization-to-jubjub-and-bound-adoption-work.md) supersedes ADR0313's retained k256 obligation. The isolated Jubjub-only variant removes 35 k256 exports and four helpers, retains 45 circuit exports and all 25 ledger fields, and preserves retained declarations byte-for-byte. TS generation passes; Rust refuses `Kernel operation blockTimeLessThan`. Full retained-account behavior/recording/proof qualification remains substantial and unaccepted. Use the authorized fallback: defer full ACC beyond 0.3.0; #356/#429 remain open, outside the milestone. No further ACC/foreign-curve work is scheduled for 0.3.0.

Branch audit finds no Compact P256/k1 implementation to delete. Preserve DID JWK metadata, immutable upstream history and k256 required by pinned native ledger8 crates. Preserve delivered generic Jubjub functionality and its maintained proof gate. Remove ACC-only adoption/resource/coverage/audit closure obligations; all other gates remain. Frozen original inventory: 8 accepted, 1 deferred, 11 open; effective required outcomes 19. Deferral adds zero completed outcomes; midpoint CI remains ten accepted original parents.

[ACC PR177 — Jubjub-only variant and milestone deferral](references-0.3.0.md#note-004) contains the boundary and future entry criteria. Receipt SHA256 `5885463ffc4f2d4b0955119c2979423f5494519ff2136f0b3e8cdabc2db92631`; archive [ADR0314 — Jubjub-only ACC probe and deferral.zip](references-0.3.0.md#note-098), SHA256 `0e37c82f44da56d354c7c93ae48ae78d13ce49fcd5a3ece8369c18d572f265a8`. No production changes, signing/commit, push or remote CI.
