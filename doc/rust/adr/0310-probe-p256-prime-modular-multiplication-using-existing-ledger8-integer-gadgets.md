---
id: RUST-ADR-0310
alias: ADR-0310
source_sha256: 1c053803067d60b1bc692aca85a2caff44fd160ae6864d44ce7f70bc4363187f
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0310 — Probe P256-prime modular multiplication using existing ledger8 integer gadgets

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

## ADR0310 — Probe P256-prime modular multiplication using existing ledger8 integer gadgets

### Problem statement

ACC PR177 requires P256/WebAuthn. New native architecture-v2 keys cannot be read by ledger8 architecture v1. The generic foreign chips configure modulus/curve-specific verifier gates and are not interchangeable with P256. Determine whether existing native/range instructions can express one useful P256 base-field relation without changing ledger8 verifier configuration.

### Before / after code

```rust
// Before: no executed P256-prime arithmetic transport evidence.
// After, proposed schematic composition (actual checked code is evidence):
let a = integers.assign_biguint(layouter, a_value, 256)?;
let b = integers.assign_biguint(layouter, b_value, 256)?;
// Constrain a < fixed_p and b < fixed_p explicitly.
let product = integers.mul(layouter, &a, &b)?;
let (_, remainder) = integers.div_rem(layouter, &product, &fixed_p)?;
// Bind a, b and remainder as nine unshifted 96-bit public limbs.
```

### Decision, emitter/runtime impact

An isolated scratch consumer only; no production emitter/runtime/IR/ABI changes. Reuse qualified BigUint gadget APIs and architecture-v1 native/range gates with foreign flags disabled. Fix the canonical P256 prime from a pinned authoritative source and record that source; no witness-selected modulus. Explicitly constrain canonical inputs. Use public mul/div_rem and matching public-input encoding; do not assume private mod_mul is callable or widened product has exactly512 bits. No new external package identities, ff_derive, patched dependencies, frontend instruction, protocol change or mock-for-real substitution.

First inspect/preflight the exact shape, then bounded behavioral checks for zero/one/p-minus-one and cross-limb inputs. Reject a=p and b=p, altered public remainder, missing/extra limbs and truncated proof. Public-API witness computations are not malicious quotient/carry injection tests: do not claim that coverage.

If measured k<=16 and cached SRS suffices, produce one genuine proof, serialize/read/init its ledger VerifierKey and verify using unchanged PARAMS_VERIFIER. Preserve key/proof envelope roundtrips, binding negatives, actual source/tool/lock/feature/SRS identities, k/rows, sizes, timings/RSS and all failed attempts. Existing k1 transport driver mechanics may be reused; its relation and shifted foreign-field encoding may not.

Operational bounds: five-minute preflight,15-minute keygen,15-minute proof; stop below3GiB free disk or above k16. No network/download/global install. Use a separately leased warm target and no root source changes during the concurrent Jubjub gate.

### Acceptance and limits

A passing result establishes one canonical modular-product relation under the P256 prime and its ledger8 transport. It does not establish P256 group operations, inversion, ECDSA, WebAuthn, Compact integration, strict contract transaction application, security audit or full ACC adoption. General arithmetic soundness and performance remain later scoped work. The parent remains open regardless of this bounded probe result.

Source research: [ACC PR177 — P256 architecture-v1 composition feasibility](references-0.3.0.md#note-008). Prerequisite transport mechanics: ADR0309; independent curve arithmetic evidence is still required.

### Tracking

https://github.com/MediaNoxLabs/compact/issues/434; parent#356/#429.


### 2026-10-07 — ADR0310 bounded modular-product transport passed

One real3440-byte proof for canonical a*b modulo the fixed P256 prime passed the actual serialized/deserialized ledger8 VerifierKey and unchanged PARAMS_VERIFIER. Architecturev1 with foreign flags disabled; nine public limbs; k9,425 circuit rows,487 table rows;1227-byte processed verifier payload. Five valid and three invalid mock cases preceded the real proof; all13 real binding/truncation controls refuse. Mock and genuine proof evidence are separately labeled.

The first build and proof passed without a failed attempt.330external identities/checksums match the qualified root lock. P256 source is pinned prime-reference evidence only, not a new dependency. Python integer/limb checks independently rechecked stored sample arithmetic. The probe uses existing BigUint constraints; it does not inject malicious internal quotient/carry witnesses.

Single debug proving time0.096651s; key setup0.093858s/0.027657s; child-rusage maxRSS49,463,296 bytes includes tiny sampler children. The2s watchdog missed intermediate phases, so its startup RSS sample is not a representative peak. These are single experiment measurements, not P256 cost/performance baselines.

Root reviewed actual relation/source and output. Archive [ADR0310 — P256-prime integer gadget transport.zip](references-0.3.0.md#note-093), SHA256 `a2adc8c2ef45746a2c3a4631ab33755753f2dbf80160a28693c21506a8bf450a`; 30files with verified manifest. Issue#434 completes this bounded arithmetic experiment. Full P256 group/inversion/ECDSA/WebAuthn, Compact communication binding and strict contract application remain unqualified; ACC#356/#429 and milestone remain open at8/20.
