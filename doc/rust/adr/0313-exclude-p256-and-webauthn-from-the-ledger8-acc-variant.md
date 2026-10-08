---
id: RUST-ADR-0313
alias: ADR-0313
source_sha256: 7c895f0b1006033f00144de748180bb29a7fb00c49e9024e6290cd32b064e455
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0313 — Exclude P256 and WebAuthn from the ledger8 ACC variant

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-scope-amendment. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.

**Later scope:** see ADR0314 and ADR0316 for subsequent curve/ACC removal decisions. The original experiment below remains historical.

## Original decision and amendments

```yaml
status: accepted-scope-amendment
date: 2026-10-07
parent: R030-12
milestone: "0.3.0"
```

## ADR0313 — Exclude P256 and WebAuthn from the ledger8 ACC variant

### Problem and owner decision

The user explicitly requested dropping P-256 after ADR0312's full-width cost model hit the 3 GiB RSS ceiling. The pinned Passport PR177 contract already offers Jubjub Schnorr authorization. Continuing a P256 backport would spend milestone effort on functionality the owner has now excluded.

Source remains PR177 head `b9e1357c21855c0d5680299faa7ba66a673cc401`. Its original Compact 0.35/ZKIR3/runtime 0.20/ledger9 package is preserved. This decision defines an explicit ledger8 ACC variant, not parity with every export in that original package. The scope amendment supersedes ADR0305's requirement to retain the P256 arm and ADR0312's pending full-width work. Earlier research receipts remain historical evidence.

### Before / after source boundary

```compact
// Before: account.compact imports all three authorization arms.
import CompactStandardLibrary;
include "webauthn";
import WebAuthn<21>;
export { WebAuthnPolicy, webauthn_challenge_base64, webauthn_client_data_hash };
include "account-p256";

// After: ledger8 variant removes these P256/WebAuthn entry points.
import CompactStandardLibrary;
// Existing Jubjub and separate k256 account declarations remain.
```

The exact contiguous import block includes two profile comments; removing it excludes 35 exported P256 circuits plus the three WebAuthn re-exports (one policy type and two pure helpers). Source counts are inventory, not compiler acceptance. Preserve every account declaration below this boundary, including the existing 20 independently qualified scalar-cast adaptations from ADR0306. Keep immutable original sources, a reviewable patch, export manifest and hashes.

### Decision and domain model

- Drop P256/secp256r1, its device/grant APIs and the wa-json134 WebAuthn profile from required 0.3.0 ACC adoption. Stop further P256 full-width modeling, key generation, proving and producer integration. No automatic retry with a larger memory budget.
- Continue with existing Jubjub Schnorr device authorization, grants and recovery, reusing qualified ledger8 primitives. Preserve signature, challenge, nonce, epoch, recovery and shared custody semantics. Device entries are opaque shared hashes; callers must not enroll an excluded P256 entry expecting it to remain usable in this variant.
- Secp256k1 (`k256`) is a distinct co-resident arm. This decision does not remove it or assert that it works on ledger8. Keep its unsupported frontend/producer boundary explicit and assess it separately. A later scope decision would be needed to exclude it from acceptance.
- Existing compiler/runtime/IR/ABI and ledger pins need no production removal: P256 work is isolated scratch research only. No verifier is replaced by a host Boolean or a trusted witness. Unsupported input continues to fail closed.
- Report variant coverage over retained obligations; list excluded exports separately. Never count removed exports as implemented, tested or proved. Parent denominator remains 20 and accepted parents remain 8.

### Implementation and validation

1. Store this ADR and its issue before editing the isolated port.
2. Copy ADR0306's account port into a new isolated directory; remove the exact P256/WebAuthn include block. Verify all retained declaration bytes and export names against the scalar-adapted source. Keep excluded source files only as archived provenance, outside the active port closure.
3. Run bounded `--skip-zk` frontend checks for both TS and Rust, preserve actual diagnostics and any remaining secp256k1 refusal. Do not spend time on broad proof/build suites for an import-only scratch edit.
4. Update the delivery dashboard, backlog, ACC parent/inventory and resource/coverage scope. Close ADR0312/#436 as not planned following owner direction, not as successful P256 delivery. Research ADR0310 remains a completed historical experiment.
5. Continue small Jubjub contract slices and full retained-account adoption under existing gates. Prior seven scalar-cell proofs validate that slice only; full ACC remains open.

### Alternatives and consequences

Keeping P256 would contradict the owner's revised scope. Moving the runtime to ledger9 would contradict the requested ledger8 port. A source variant makes the support boundary reviewable and reduces required work while preserving the original for future ledger9-oriented research. P256/WebAuthn clients cannot use the removed API; no automatic credential migration is supplied. The separate k256 gap and other ACC compatibility requirements remain visible.

### Tracking

Parent #356; source inventory #429; supersedes pending P256 work in ADR0312/#436. Planning and progress remain in the midnight vault until milestone closeout.

Decision and delivery issue: https://github.com/MediaNoxLabs/compact/issues/438


### 2026-10-07 — Owner excludes P256/WebAuthn; ADR0313/#438

The owner explicitly dropped P256 support from the ledger8 ACC target following ADR0312's 3 GiB model stop. [ADR-0313 — Exclude P256 and WebAuthn from the ledger8 ACC variant](0313-exclude-p256-and-webauthn-from-the-ledger8-acc-variant.md) supersedes the earlier requirement to port that arm. P256, WebAuthn and full-width P256 producer/resource work are outside required 0.3.0 acceptance. Existing research evidence stays intact; #436 closes as **not planned**, not implemented.

An isolated patch removes the exact P256/WebAuthn import block: 35 P256 circuits, two WebAuthn helper circuits and one policy re-export are excluded. The remaining 80 circuit exports (35 Jubjub, 35 k256, 10 shared) and 25 ledger fields are preserved byte-for-byte after the already qualified scalar adaptation. Shared RP fields remain part of Jubjub grant commitments. Both TS and Rust frontend probes still refuse `Secp256k1EcdsaSignature`; secp256k1 is separate and remains an explicit open requirement. Full ACC adoption is not yet accepted. No production backend/runtime/IR/ABI changes or P256 retries.

Receipt SHA256 `b7ced885480a100fe084a3edbbdccce1321ddfac363d6dc1d1d7b985714cb70a`. Verified archive [ADR0313 — ACC P256 exclusion and frontend boundary.zip](references-0.3.0.md#note-097), SHA256 `08da1dcd9c711aa3f4f5ef3882c66f32dedd9ee8f42d7c078f46cb13a98fa936`. See [ACC PR177 — Ledger8 variant without P256 and WebAuthn](references-0.3.0.md#note-007). Parent denominator remains 20; accepted parents remain 8. The remaining scope requires independent behavior, recording, replay and strict proof evidence as applicable.


### 2026-10-07 — Jubjub only; full ACC deferred by authorized fallback (ADR0314/#439)

The owner also excludes secp256k1 and permits removing full ACC adoption from 0.3.0. [ADR-0314 — Restrict ACC authorization to Jubjub and bound adoption work](0314-restrict-acc-authorization-to-jubjub-and-bound-adoption-work.md) supersedes ADR0313's retained k256 obligation. The isolated Jubjub-only variant removes 35 k256 exports and four helpers, retains 45 circuit exports and all 25 ledger fields, and preserves retained declarations byte-for-byte. TS generation passes; Rust refuses `Kernel operation blockTimeLessThan`. Full retained-account behavior/recording/proof qualification remains substantial and unaccepted. Use the authorized fallback: defer full ACC beyond 0.3.0; #356/#429 remain open, outside the milestone. No further ACC/foreign-curve work is scheduled for 0.3.0.

Branch audit finds no Compact P256/k1 implementation to delete. Preserve DID JWK metadata, immutable upstream history and k256 required by pinned native ledger8 crates. Preserve delivered generic Jubjub functionality and its maintained proof gate. Remove ACC-only adoption/resource/coverage/audit closure obligations; all other gates remain. Frozen original inventory: 8 accepted, 1 deferred, 11 open; effective required outcomes 19. Deferral adds zero completed outcomes; midpoint CI remains ten accepted original parents.

[ACC PR177 — Jubjub-only variant and milestone deferral](references-0.3.0.md#note-004) contains the boundary and future entry criteria. Receipt SHA256 `5885463ffc4f2d4b0955119c2979423f5494519ff2136f0b3e8cdabc2db92631`; archive [ADR0314 — Jubjub-only ACC probe and deferral.zip](references-0.3.0.md#note-098), SHA256 `0e37c82f44da56d354c7c93ae48ae78d13ce49fcd5a3ece8369c18d572f265a8`. No production changes, signing/commit, push or remote CI.
