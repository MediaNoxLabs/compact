---
id: RUST-ADR-0306
alias: ADR-0306
source_sha256: 2f7ecf139b0df059d207982a2c4de84157dbc65b40f8f2c94e3bec10426e911e
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0306 — Preserve ACC Jubjub scalar cast reduction on ledger8

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** locally-delivered-bounded-scope. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: locally-delivered-bounded-scope
date: 2026-10-07
parent: R030-12
milestone: "0.3.0"
```

## ADR0306 — Preserve ACC Jubjub scalar cast reduction on ledger8

### Problem

User-selected ACC PR177 at `b9e1357c21855c0d5680299faa7ba66a673cc401` uses the Compact0.35 `JubjubScalar` type in cast expressions. The qualified ledger8 frontend has no such declared type. All occurrences in the account import closure are casts, including public point derivation, cofactor checks, signatures and recovery identity/reset expressions. An alias to Field or deleting a cast would not establish the original conversion semantics.

Read-only ADR0305 research inspected official Compact0.35 source tree `debb05f9414b9d1e176741c2be289bb32233f0fc`. `reduce-to-zkir.ss:1068–1075` lowers native Field/Uint-to-JubjubScalar casts to `jubjub_scalar_from_native`. Existing ledger8 `jubjubScalarFromNative` and Rust `natives::jubjub_scalar_from_native` already implement subgroup-order reduction. Raw Rust ec_mul/ec_mul_generator deliberately require canonical scalar input, so erasing reduction changes valid inputs at/above the subgroup order.

### Decision and before/after

Qualify one bounded semantic compatibility slice, preserving the immutable original source beside a separately labeled local port.

```compact
// Original0.35
return ecMulGenerator(scalar as JubjubScalar);
// Ledger8 compatibility source
return ecMulGenerator(jubjubScalarFromNative(scalar));
```

Replace each exact cast with its explicit reducer call while preserving expression evaluation/order and point/scalar arguments. Retain a source diff and a per-site mapping to original lines and operand expression. No broad textual regex over unknown Compact syntax; the current finite cast inventory must be enumerated and checked. Do not introduce `type JubjubScalar = Field`, modify original fixtures in place, drop authentication, or map foreign curves into Jubjub. Full account generation will still encounter independent foreign-curve gaps.

### Emitter/runtime ownership

Prefer the existing shared frontend native, ZKIR2 lowering, Rust typed expression owner and upstream EmbeddedFr conversion. No new runtime primitive, ABI, schema, dependency or ledger pin is authorized by this slice. If a new emitter defect is exposed, isolate it and record a further decision instead of silently widening this scope. Application source compatibility owns the explicit conversion; upstream primitives own arithmetic.

### Qualification

- Pin an isolated official0.35 compiler oracle and the original runtime0.20 locked closure; verify archive/npm integrity. The oracle does not replace production dependencies. Retain source/profile identities and exact commands.
- Minimal original/ported pure sources cover generator and supplied non-generator point multiplication. Compare canonical output/encoding on0,1,8,q−1,q,q+1 and maximum canonical native Field, using upstream subgroup/native moduli. Include the identity point where valid. Preserve any malformed-point/type refusal separately and only when the actual API exposes it.
- Compare original0.35/runtime0.20, ledger8 TypeScript and generated Rust behavior. Frozen expected tokens or CPU-only success are insufficient. The seven minimal missing-type probes remain historical layer attribution.
- A separate small stateful/asserting wrapper must establish real proof applicability, native/recorded/replay state and one strict ledger8 proof/apply with changed binding and replay refusal. Pure helper exports remain proof-not-applicable. Constructor execution is not claimed proved.
- Register the accepted small source/generated/oracle fixture and focused backend type/refusal tests if this fills a real gap. Source freshness and bounded targeted gates precede any wider rerun. Recheck full account generation after the source patch and retain its exact next failure; partial scalar success is not ACC adoption.

### Alternatives and limits

Deleting casts or raw Field aliasing is shorter but loses the canonical reduction contract. A new runtime scalar wrapper is unnecessary for this finite source adaptation because the existing reducer already owns the semantic conversion. Backporting all modern scalar types is a separate broad frontend/API decision. Native k1 chip reuse and constrained P-256 lowering remain ADR0305 research; no proof of full-ledger8 feasibility is inferred here.

Work starts isolated in `/tmp/rust030-acc-scalar`; root owns live-source integration and signed commits. Obsidian is planning/evidence source of truth. No upstream modifications, remote CI, push or publication. Accepted parents remain8/20.

### Tracking

https://github.com/MediaNoxLabs/compact/issues/430; parent#356, source activation#429.


### 2026-10-07 — First ACC scalar compatibility slice verified

PR177 is pinned to `b9e1357c21855c0d5680299faa7ba66a673cc401`. The verified official Compact0.35/runtime0.20 oracle and ledger8 TypeScript/generated Rust agree on21 point results (seven scalar boundaries across generator, supplied non-generator and identity), including coordinates and FAB bytes. Both TS targets refuse four invalid inputs; malformed-point diagnostics differ. Rust separately refuses raw noncanonical scalar use, malformed point construction and native-modulus decoding. The314-package external Rust closure adds no qualified identity/checksum changes.

Exactly20 account casts were adapted to the existing `jubjubScalarFromNative` reduction in an isolated source copy. Full account next refuses `Secp256k1EcdsaSignature`; foreign-curve adoption remains open. The minimal stateful wrapper compiles natively but recording refuses `Expr::JubjubScalarFromNative` at `actions[0].bindings[0].value`. No stateful proof or full ACC acceptance is claimed. ADR0306/#430 remains open until its stateful proof obligation is resolved; a further ADR must precede recording changes.

Durable archive [ACC PR177 — Pinned baseline and scalar compatibility.zip](references-0.3.0.md#note-009), SHA256 `96d11d5f03b5ba6e78934256f6478d24edf926e4ea96eed38335b2b505ad7cfa`; 214 hash-verified files including original repository tarball, source patches, command logs and oracle/consumer outputs. Compiler/package binaries are reproducible from retained verified identities and are not duplicated. Host Rust1.98.1 evidence only; no new MSRV or audit claim.


### 2026-10-07 — Typed Jubjub scalar Cell locally accepted

Signed conventional/DCO commit `dce3c9e90c1cfd5b391c44355ea9f86e862bf317` delivers ADR0308/#432 and the bounded stateful qualification of ADR0306/#430. Four shared typed Jubjub operations support one checked point Cell write and point result, with lexical scope and source-order evaluation. Unsupported helpers, effects and foreign curves still refuse recording. Runtime ABI, schema and all452 external dependency identities/checksums remain unchanged.

Evidence:446 backend tests;11 new profile test methods include typing, lexical scope, unsupported-effect refusals and ordinary-worker recursion;3 fixture methods cover seven apply boundaries and canonical raw success/noncanonical raw refusal.197 maintained fixtures are fresh.106 Python methods, strict backend/fixture/proof-runner Clippy and formatting pass. The declaration baseline adds exactly apply/raw/constructor for the new source.

Fresh strict gate: seven real apply proofs, each3296 bytes, key k11/1190 rows. Default-strict ledger8 application accepts every expected state; all seven changed bindings and seven replays refuse, with replay state unchanged. Generated observed facade preparation matches the checked manual call. Two additional existing Unit-return reducer proofs pass after the harness output adapter generalization. Constructor data is deployed; constructor execution is not proved. Raw multiplication has behavior/rollback coverage, not a real proof qualification claim.

Fresh ledger8TS capture matches the retained original0.35/runtime0.20 oracle plus complete ledger8 state/transcript/gas records. The original oracle was independently captured earlier; this run does not claim to rerun original0.35. The failed initial copied-Node loader run and inventory drift are preserved, followed by corrected passing runs. Node now runs in place with executable/external-library identities rechecked. OS shared-library names are recorded without byte qualification.

Archive [ADR0306-0308 — Typed Jubjub recording and strict proof delivery.zip](references-0.3.0.md#note-089), SHA256 `e38642bbda270347c5ee3c4e323f33717f7e3c5b710063c4d527e1329742e44c`;211 evidence files with verified manifest. Root receipt binds all28 delivered paths and the actual proof receipt; gate ran on the uncommitted candidate over ec10f324 before its exact source was signed.

Issues#430/#432 close their bounded scopes. Full ACC#356/#429 remains open: k1 transport is a separate completed experiment, P256 arithmetic is now ADR0310/#434, and frontend/signature/WebAuthn/contract integration remains undelivered. Parent acceptance stays8/20 (40%); no push or remote CI.
