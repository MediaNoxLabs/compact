---
id: RUST-ADR-0308
alias: ADR-0308
source_sha256: b538d53ef07b7f00b683212777a0ac034185e11fef7bb09ff32f122f43a13949
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0308 — Record typed Jubjub value operations and a point Cell write

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

## ADR0308 — Record typed Jubjub value operations and a point Cell write

### Problem and evidence

ADR0306/#430 establishes exact pure cast/reduction parity for ACC PR177 on original0.35, ledger8TS and Rust. Its minimal stateful wrapper is proof-required, yet recorded/observed APIs are unavailable: Expr::JubjubScalarFromNative at actions[0].bindings[0].value. The first diagnostic hides further unsupported EcMulGenerator/EcMul/EcAdd, root point-result admission and extracted nested return scope. Existing runtime operations already provide the correct semantics. Adding only a legacy Field-expression branch would duplicate evaluation and expose the next gap.

Read-only proposal: [ACC PR177 — Recording gap ownership review](references-0.3.0.md#note-010). Source baseline329bf1bc, test-only currentHEAD ec10f324.

### Before / after

```compact
export ledger result: JubjubPoint;
export circuit apply(point: JubjubPoint, scalar: Field): JubjubPoint {
  const reduced = jubjubScalarFromNative(scalar);
  const value = ecAdd(ecMulGenerator(reduced), ecMul(point, reduced));
  result = disclose(value);
  return value;
}
```

Before: native code exists; recording declines the first reduction expression.
After: a bounded typed recording plan evaluates each operand/local once in source order, records one checked point Cell write, returns the same bound point and supplies the existing borrowed observation/proof facade. Generated code calls runtime::jubjub_scalar_from_native, runtime::ec_mul_generator, runtime::ec_mul and runtime::ec_add; raw noncanonical multiplication still fails, while explicit reduction succeeds.

### Decision / ownership

Add a small cohesive Jubjub value profile alongside existing typed_plan domain owners. It accepts structural Field/JubjubPoint inputs, lexical lets/sequences and the four operations above, one declared point Cell/one write, and a point return. It is independent of source names. Reuse terminal_returns::adapt and Plan::return_plan to preserve the terminal continuation scope; do not recompute a return after recording or copy an EC evaluator into legacy recorded.rs.

Share a crate-private operation syntax/type helper for already lowered operands between the pure owner and recorder when exact emitted output can be preserved. The shared helper checks actual operand types and preserves fallible multiplication. Each evaluator continues to own evaluation order, scoped values and effects. Do not route arbitrary expressions through native lowering as a fallback. Keep added point-cell admission specific to this profile; do not broaden unrelated profiles globally. Existing constructors retain their existing native owner.

Domain excludes helper calls, witnesses, branches, additional writes, Map/Set mutations, point construction/negation/hash-to-curve and all foreign curves. Every declaration/index/path and actual local/result type is checked. Extra/unsupported effects must remain unavailable. Supported source renames and valid lexical shadowing must work. This slice does not address the stopped consumer/cross-instance investigations.

No new runtime primitive, ledger/proof dependency, ABI or IR/capability schema. Any discovered need to widen that scope requires a separate decision and reproducer. Account adoption and k1/P256 remain open.

### Validation

1. Focused operation typing and direct/nested scope tests: wrong type, unbound/falsely annotated local, wrong terminal scope, declaration/index/path mismatch and extra/unsupported effects refuse. Alpha-renaming remains valid.
2. Regenerate the unedited minimal wrapper and verify recorded/observed capability. Execute seven scalar boundaries comparing native/recorded results and complete ledger/transcript/gas/replay against TS. Raw q multiplication refuses before any accepted mutation; explicit reduction succeeds. Do not assert complete proof acceptance from recording alone.
3. Real ledger8 strict proof/apply for the minimal wrapper, changed binding and replay refusals; constructor execution remains unproved. Reuse qualified runner and pinned keys/toolchain with source-bound receipts.
4. Sharing syntax must preserve existing emitted fixture outputs; run focused pure/point/terminal tests, strict Clippy/format and resource worker controls. Account for any extra recursive helper frame in finite render-path calibration. Broaden to fixture freshness/backend suite when integration warrants it.

Implementation and proofs stay local. Keep planning/research/history in Obsidian; only source/tests land in the branch until milestone closeout. Root reviews and signs integration. No production-readiness claim until all parent obligations are accepted.

### Tracking

https://github.com/MediaNoxLabs/compact/issues/432; depends on ADR0306/#430; parent#356.


### 2026-10-07 — Typed Jubjub scalar Cell locally accepted

Signed conventional/DCO commit `dce3c9e90c1cfd5b391c44355ea9f86e862bf317` delivers ADR0308/#432 and the bounded stateful qualification of ADR0306/#430. Four shared typed Jubjub operations support one checked point Cell write and point result, with lexical scope and source-order evaluation. Unsupported helpers, effects and foreign curves still refuse recording. Runtime ABI, schema and all452 external dependency identities/checksums remain unchanged.

Evidence:446 backend tests;11 new profile test methods include typing, lexical scope, unsupported-effect refusals and ordinary-worker recursion;3 fixture methods cover seven apply boundaries and canonical raw success/noncanonical raw refusal.197 maintained fixtures are fresh.106 Python methods, strict backend/fixture/proof-runner Clippy and formatting pass. The declaration baseline adds exactly apply/raw/constructor for the new source.

Fresh strict gate: seven real apply proofs, each3296 bytes, key k11/1190 rows. Default-strict ledger8 application accepts every expected state; all seven changed bindings and seven replays refuse, with replay state unchanged. Generated observed facade preparation matches the checked manual call. Two additional existing Unit-return reducer proofs pass after the harness output adapter generalization. Constructor data is deployed; constructor execution is not proved. Raw multiplication has behavior/rollback coverage, not a real proof qualification claim.

Fresh ledger8TS capture matches the retained original0.35/runtime0.20 oracle plus complete ledger8 state/transcript/gas records. The original oracle was independently captured earlier; this run does not claim to rerun original0.35. The failed initial copied-Node loader run and inventory drift are preserved, followed by corrected passing runs. Node now runs in place with executable/external-library identities rechecked. OS shared-library names are recorded without byte qualification.

Archive [ADR0306-0308 — Typed Jubjub recording and strict proof delivery.zip](references-0.3.0.md#note-089), SHA256 `e38642bbda270347c5ee3c4e323f33717f7e3c5b710063c4d527e1329742e44c`;211 evidence files with verified manifest. Root receipt binds all28 delivered paths and the actual proof receipt; gate ran on the uncommitted candidate over ec10f324 before its exact source was signed.

Issues#430/#432 close their bounded scopes. Full ACC#356/#429 remains open: k1 transport is a separate completed experiment, P256 arithmetic is now ADR0310/#434, and frontend/signature/WebAuthn/contract integration remains undelivered. Parent acceptance stays8/20 (40%); no push or remote CI.
