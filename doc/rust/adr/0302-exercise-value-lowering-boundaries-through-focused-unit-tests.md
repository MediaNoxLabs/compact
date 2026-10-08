---
id: RUST-ADR-0302
alias: ADR-0302
source_sha256: ff4ed0d8a95b187f8a89a2d218446f22dd0587d3cf3d5d623ec94d08c9cd2ce2
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0302 — Exercise value lowering boundaries through focused unit tests

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for a bounded local test slice. Parent R030-07/#351. Source baseline fa7600bc. No emitter/runtime behavior change proposed. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0302 — Exercise value lowering boundaries through focused unit tests

Status: accepted for a bounded local test slice. Parent R030-07/#351. Source baseline fa7600bc. No emitter/runtime behavior change proposed.

### Problem

The frozen d5d4a6c9 production-line coverage receipt measured value_lowering at 251/272 executable lines. Its missing-line review identified decimal overflow, unsupported Map carrier/error propagation and aggregate coercion guards. Generated scenarios exercise many successful paths, but the shared value owner has no adjacent unit-test component to explain its boundary contracts. The old percentage is not current coverage.

### Before / after engineer-facing examples

Before: malformed decimal bounds or a Map carrier require a broader render scenario to inspect the shared helper's exact refusal.

After, direct unit examples distinguish:
```text
field_literal_bytes("256") -> little-endian [0, 1, 0, ...]
field_literal_bytes(2^256) -> InvalidFieldLiteral(original_text)
unsigned_maximum(2^248 - 1) -> Wide { high: 2^120 - 1, low: u128::MAX }
unsigned_maximum(2^248) -> InvalidUnsignedMaximum(original_text)
Map<String, Map<String, Field>> -> nested MapNode carrier
Map<Map<...>, Field> -> no supported Map slot carrier
Map<String, Uint<invalid>> -> precise invalid bound error
coerce Uint<255> -> Uint<15> -> TypeMismatch with both original types
```

Add adjacent tests in value_lowering/tests.rs, preserving the production owner. Test canonical decimal/byte boundaries against fixed independent mathematical vectors, original error payloads through nested Map/aggregate lowering, shape mismatch refusals, and accepted widening controls. Use AST inspection only where syntax is itself the output contract; avoid snapshots that merely restate each implementation arm. Existing generated integration tests remain the runtime semantics authority.

### Emitter/runtime changes and acceptance

Production behavior, generated API, runtime, private IR, capabilities, ledger pins and proof semantics remain unchanged. Only a cfg(test) module declaration and focused tests are planned. Run the bounded library test selection and formatting/strict Clippy using an existing cache when available; do not repeat proof suites for test-only source additions. A fresh whole-backend coverage cohort is deferred until relation/resource changes are stable. Test counts are not a replacement coverage percentage. Any discovered production defect requires a documented follow-up decision and regression.

Keep this decision and progress in midnight; publish repository ADRs at milestone closeout. No push or remote CI.

Issue: https://github.com/MediaNoxLabs/compact/issues/426

### Delivery

ADR0302/#426 delivered at `c8450ee37485cfd64451bb009a930b07fafff067`, conventional GPG+DCO verified. Twelve adjacent shared value-lowering unit methods pass. They exercise canonical decimal byte order, malformed decimal payloads, overflow beyond 256 bits, 128-bit carrier transition, 248-bit unsigned limit, nested Map carrier classification and precise propagated errors, aggregate shape mismatch and leaf narrowing refusals with accepted widening controls.

Only a cfg(test) module declaration and test component were added. Root byte comparison verifies the complete production lowering body is unchanged. No runtime/generated API/admission/proof semantics changed. Formatting and strict library/all-test Clippy pass. Test-only additions do not require repeated proof suites; no whole-codebase coverage percentage is inferred.

[ADR0302 — Value lowering unit tests.zip](references-0.3.0.md#note-088) retains source, exact commands, receipt and passing logs (SHA256 `66e5bfbcf4c77cfeaf924fff332238a6c5e334727f307858d7088c9e3fe62894`). Initial test compilation exposed the existing syn no-Debug configuration; assertions were corrected to inspect Result::err without enabling new dependency features. A redundant cache warm-up was canceled; the accepted run used the existing parity target. These attempts are not counted as passes.

Parent R030-07/#351 remains open pending a fresh stable-source coverage cohort and remaining disposition. Accepted parents remain 6/20. No CI or push; user ledger documentation retains its recorded SHA256.
