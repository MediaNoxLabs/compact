---
id: RUST-ADR-0336
alias: ADR-0336
source_sha256: c96b983f13d21b496e7647738c48e5f72734f48848c3b37dc033b44f0d6445fc
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0336 — Preserve empty-vector element types during Rust emission

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for implementation; source reproduction confirmed. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0336 — Preserve empty-vector element types during Rust emission

Date: 2026-10-07
Status: accepted for implementation; source reproduction confirmed
Branch: codex/rust-backend-ast
Parent: 0.3.0 compiler quality and generated API audit

### Problem
A valid Compact nested map whose inner callback returns `[]: Vector<0, Field>` and whose outer callback ignores that argument emits an unconstrained Rust element type. Unedited generated output fails E0282 (`Vec<FixedVector<_, 0>>`). Three controls compile: direct empty vector, literal map into typed fold, and nested map returning typed default. This is distinct from ADR0330's delivered array-length inference fix.

### Before
```compact
return map((ignored: Vector<0, Field>): Field => 1,
  map((value: Field): Vector<0, Field> => [], values));
```
```rust
__compact_mapped.push(runtime::FixedVector::new([]));
```

### Decision and intended after
Preserve the IR element type explicitly for empty Expr::Vector literals, using the existing Rust type mapper. Keep nonempty output unchanged unless evidence requires otherwise.
```rust
__compact_mapped.push(runtime::FixedVector::<runtime::Field, 0>::new([]));
```
Exact spelling follows existing emitter conventions. Add a source-level nested-map regression with ignored downstream argument and executable expected output. Include representative non-Field element control when economical. No new runtime primitive, schema, recording admission or public API.

### Validation and delivery
First reproduce failure against dbfd7dc2; then focused emitter tests and generated consumer compilation/execution on supported Rust toolchains. Refresh affected compiler-generated fixtures, run shared freshness after regeneration, and record exact commands/source identities. Broader gating only if changed output warrants it. Signed conventional DCO commit and issue closure after validation.

### Evidence
/tmp/rust030-empty-vector-probes/REPORT.md; check-receipt.json and map_literal-check.log. Probe manifest SHA256 66d88fb2af7405a97d85928137efb435223f34e5fd0412391cf2e3a8798d7079. Ordinary Compact frontend admission is confirmed; no runtime or proof claims arise from compilation.


### ADR0336 delivery — 2026-10-07

Signed GPG/DCO commit `e35688eb143aebd2b35867da26d2723e1e276468` fixes the independently reproduced empty-vector element inference error. Only empty Expr::Vector emission receives an explicit type using the existing mapper. Nonempty branch unchanged; existing vector-widen module content retained and two new source exports appended.

Actual Rust1.88 and1.99 each pass5consumer tests and7focused renderer tests; strict all-target/all-feature Clippy and formatting pass. Root whole-cohort generation confirms **198 fixtures,0stale,0failed**, with stable source hashes. An initial root attempt omitted COMPACTC_SCHEME and stopped all198before frontend; that setup error and successful corrected run remain separate receipts.

[ADR0336 — Empty-vector element type regression.zip](references-0.3.0.md#note-114): 242 hashed entries, SHA256 `62505537fafba7009cea66767af83051394ce162a8766c005001509d297ed95c`. Includes original failing sources, successful controls, exact patch, generated source, toolchain logs, commit binding and whole-cohort freshness. Large compiler binaries stay local with hashes retained. Root reviewed the typed output and expected Field/Boolean results. No runtime ABI/schema, dependency, recording or proof change. Current coverage percentage is not remeasured: dbfd7dc2 retains its historical95.11%changed-line result.

Issue#460 owns this delivered primitive. #452 original coding-agent findings have delivered dispositions; broader security/milestone gates stay open. Full198freshness also completes ADR0334's pending joined render check. Parent acceptance remains11/19.


### Independent compiler inference follow-up — 2026-10-07

Claude external review confirms ADR0336 fixed with no concrete correctness finding. Root separately reconciled the supplied-bundle freshness limitation and the consumer-versus-generated-output type checks. [Independent empty-vector inference review — 2026-10-07](references-0.3.0.md#note-153) retains prompt, model/tool identity, raw verdict, source manifests, limitations and root disposition. Complete milestone audit remains open; no parent count change.
