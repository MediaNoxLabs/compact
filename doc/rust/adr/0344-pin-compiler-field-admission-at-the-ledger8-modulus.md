---
id: RUST-ADR-0344
alias: ADR-0344
source_sha256: 646fb116d94ddeeadee1bc8dc3003a129303a37375b682e807704fd603f07097
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0344 — Pin compiler Field admission at the ledger8 modulus

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** not separately declared. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0344 — compiler Field canonical modulus neighbors

### Problem / requirement

Compiler `field_literal_bytes` must accept exactly canonical decimal ledger8 field values and return exact InvalidFieldLiteral payloads outside that domain. Existing controls cover malformed text, decimal overflow, selected accepted values and a distant noncanonical value, but not the exact canonical admission boundary. This is compiler IR literal validation, not runtime decoder/trust work.

### Independently pinned upstream provenance

Cargo.lock pins midnight-transient-crypto2.0.1 (checksum1f0e92f0e835fc85d3e22d54b2e0fbfd3d3777439475da8d10020ef7bf016bb1) and midnight-curves0.2.0 (checksum71df41292a1fd7796bf6c0c6eff7ad717d628406a59a79240e68579716c3566a).
- `/Users/ysh/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/midnight-transient-crypto-2.0.1/src/curve.rs`:51–55 aliases outer::Scalar to midnight_curves::Fq; line158 wraps it as Fr. SHA256 `8a3abd2773f2eb5e18d7549e2240deeba979bcc36426f51a4644bef5ee2e9417`.
- `/Users/ysh/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/midnight-curves-0.2.0/src/bls12_381/fq.rs`:43–48 contains four little-endian non-Montgomery modulus limbs; PrimeField::MODULUS at502–503 repeats its hex string. SHA256 `f12967e1d672175304ca4cac302a19e1e50d13d030759bd4a81003660174e874`.
- Hex modulus: `0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001`.

Pin decimal strings and expected bytes independently from the function under test. The following decimal conversion is ordinary Python integer arithmetic in the planning receipt, not the compiler parser or a generated expected value:

| Input | Decimal | Expected |
|---|---|---|
| p−1 | `52435875175126190479447740508185965837690552500527637822603658699938581184512` | Ok exact32little-endian bytes |
| p | `52435875175126190479447740508185965837690552500527637822603658699938581184513` | Err(RenderError::InvalidFieldLiteral(original.clone())) |
| p+1 | `52435875175126190479447740508185965837690552500527637822603658699938581184514` | Same exact variant with p+1 payload |

Expected p−1 little-endian hex: `00000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73`.
Expected p−1 byte array: `[0, 0, 0, 0, 255, 255, 255, 255, 254, 91, 254, 255, 2, 164, 189, 83, 5, 216, 161, 9, 8, 216, 57, 51, 72, 125, 157, 41, 83, 167, 237, 115]`.

### Scope / before / after

Own only `src/value_lowering/tests.rs`, optionally one focused public render regression in existing tests/render.rs if the owner judges useful. Prefer private helper triad plus existing public-render canonical test; no production change or new dependency. Before: distant range refusal. After: exact neighboring acceptance and two exact refusals, with documented upstream constant provenance. Do not obtain expected accepted bytes by calling field_literal_bytes or by round-tripping the implementation under test.

### Gates / limits

Run focused lowering literal tests on Rust1.99 offline/locked; strict backend lib Clippy and scoped rustfmt. A public-entry assertion, if added, should assert exact returned error variant and accepted render success; no need to compile/prove another fixture. Root records ADR/issue before implementation. Not an exhaustive Field-value proof, runtime canonical decoder test, cryptographic audit or upstream-TS disagreement claim.


### Root decision — 2026-10-07

Accepted for bounded test-only implementation. Parent R03017/#361 and R03007/#351 remain open. Preserve historical receipts and the stopped ADR0285 boundary. No runtime or production behavior change is authorized by this decision.
Root scope: own only value_lowering/tests.rs; the existing public-render rejection control is sufficient. No additional fixture or render test is needed.


### Delivery — 2026-10-07

Signed pushed commit5a9cd356;16lowering tests and strict Clippy/format pass. Production unchanged. [Compiler publication and Field boundaries — 2026-10-07](references-0.3.0.md#note-146) contains exact receipts, archive and limits.
