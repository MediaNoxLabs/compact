---
id: RUST-ADR-0330
alias: ADR-0330
source_sha256: c3b2a411728ce668cad29d7372ad072a5d752e34a85aa805e0dbe4b5bd8eb371
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0330 — Preserve fallible vector coercions without expanding generated arrays

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** planned remediation; 2026-10-07; independent audit ADR0327/#452 and R03013/#357; milestone rust-backend-v0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0330 — Preserve fallible vector coercions without expanding generated arrays

Status: planned remediation; 2026-10-07; independent audit ADR0327/#452 and R03013/#357; milestone rust-backend-v0.3.0.

### Problem
Independent ADR0327 review found a valid Compact Vector<2,Uint<8>> to Vector<2,Uint<16>> coercion that renders but fails Rust compilation with E0277: generated array.map closure contains a fallible cast and cannot use ?. Root delegate reproduced ordinary source acceptance and exact compiler failure. Existing AST-only tests missed compilation.

### Before / after
```rust
// Before: closure returns a value, so ? is invalid.
FixedVector::new(source.into_array().map(|item| cast_unsigned(item)?))
// After (fallible elements): ordinary Result-owner block, early failure.
let mut converted = Vec::with_capacity(N);
for item in source.into_array() { converted.push(cast_unsigned(item)?); }
FixedVector::new(converted.try_into().expect("vector coercion preserves length"))
```
Use fully qualified/generated hygienic names in actual AST. Preserve infallible .map emission using explicit private coercion fallibility metadata, propagated through aggregates, rather than token inspection. No runtime API/ABI/IR-schema change. Nested fallible elements must propagate failure to the same enclosing Result and evaluate each source once in source order.

### Alternatives and cost
Unconditional compile-time unrolling is rejected: current resource traversal counts the element type once, not nested vector length products, so unrolling creates a new AST expansion hazard. A temporary Vec only on fallible conversion paths bounds generated size by type depth and uses stable Rust1.88; measure/document this allocation tradeoff without claiming a speedup. Do not introduce unsafe initialization or unstable array::try_map. Typed metadata avoids regressing infallible output and duplicates no conversion policy.

### Verification
First retain failing valid-source consumer. Add meaningful nested/fallible/identity and diagnostic controls under the value owner and compile/run unedited generated minimal contracts on1.99/1.88 with source-matched runtime/lock. Verify order/once-evaluation and first-error behavior where source semantics permit. Test large nested type rendering remains compact. Focused backend tests/lints first; then197render comparison identifies exact changed outputs, expanded gate only as necessary. No proof rerun solely for pure conversion syntax. Signed GPG/DCO commit and independent external retest required.


### Independent retest: explicit result dimensions

The external source review identified an unpinned const length when fallible vector coercion feeds map/fold source bindings. Root's worker reproduced E0284 with valid Compact map and named-fold sources. Extend this decision to emit `<[_; N]>::try_from(values)` so the conversion carries its own declared length and never relies on the consumer's inference. Add both real source paths to the widening fixture and execute them. The remaining `.expect` still requires Debug on the conversion error; all supported target carriers provide it. The review's suggestion that explicit array length removes Debug was incorrect. No runtime API or dimension unrolling is introduced.


A second valid-source composition reproduces the same missing-dimension ownership in the pre-existing VectorMap result conversion, even without widening: map output immediately used by fold. Apply the explicit target array length to this conversion too and add a composed source regression. Regenerate any affected maintained outputs only after confirming the differences are this explicit dimension annotation. Keeping conversion types self-contained avoids relying on downstream bindings to repair inference.


### Final delivery

Locally delivered in signed GPG/DCO `a136f58f`, pushed as part of dbfd7dc2. [Joined compiler audit remediation — 2026-10-07](references-0.3.0.md#note-155) records the 701-test final core gate, 197 fresh fixtures, 95.11% changed-backend coverage, actual consumer/MSRV checks, independent retest and exact source-to-commit binding. Earlier attempt receipts retain their original source scope.
