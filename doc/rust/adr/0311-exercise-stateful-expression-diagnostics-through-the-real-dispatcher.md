---
id: RUST-ADR-0311
alias: ADR-0311
source_sha256: 1bc75fac82c19704f091fd8643f6a68c51b1e91b81df836164e603fb4873e74d
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0311 — Exercise stateful expression diagnostics through the real dispatcher

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** locally-delivered-test-only. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: locally-delivered-test-only
date: 2026-10-07
parent: R030-07
milestone: "0.3.0"
```

## ADR0311 — Exercise stateful expression diagnostics through the real dispatcher

### Problem statement

Historical mixed coverage at ec10f324 reached8331/8869 changed backend lines (93.93%). This is not coverage of the new dce3c9e9 Jubjub slice. Read-only review identified meaningful untested type/declaration/call refusals in the unchanged stateful expression owner. End-to-end positive fixtures alone do not establish independent argument validation and precise error propagation.

### Before / after examples

```rust
// Before: valid generated contracts cover the successful path.
// After: focused component tests also reach the real stateful dispatcher.
// Schematic only; tests use the owner's actual signature and IR types.
let result = render_state_expression(&invalid_generator_scalar, context);
assert!(matches!(result, Err(RenderError::TypeMismatch { .. })));
```

Each negative has a valid baseline. Tests assert exact expected/actual type diagnostics, declared target/index matching and callee/argument resolution. They must exercise dispatch and ownership boundaries rather than duplicate private helper logic.

### Decision and component ownership

Add a cohesive unit-test component next to stateful/expression.rs. Three initial slices: crypto argument validation (degrade/upgrade/scalar reduction/generator and binary point operations); numeric conversion/arithmetic and comparison operand boundaries; ledger declaration and callable/witness mismatches. Cover both operand positions where independent checks exist. Use real IR/domain values and the existing renderer, not a second implementation.

Production lowering bodies, runtime, ABI, schemas, dependencies and acceptance thresholds remain unchanged. A test-only module declaration is allowed. If a real implementation defect is found, preserve its reproducer and record a separate decision before changing behavior. Avoid unreachable dispatch fallbacks and tests that merely mirror syntax. Check no statements were appended only when rejection occurs before effectful operand lowering; do not invent blanket rollback semantics for the renderer.

Historical target locations are stateful/expression.rs crypto1843–2027, conversion1435–1486, arithmetic1531–1717, observations595/963/966 and calls1020/1048/1122–1127. These are research pointers, not an assumed current coverage denominator. Useful later lexical/Jubjub-profile scope controls can follow the initial diagnostic slice if they add distinct behavior.

### Verification and delivery

Prepare changes in scratch while the dce3c9e9 performance run holds the source freeze. Apply only after that lease is released. Run the focused component cohort and strict Clippy/format first. One fresh instrumented backend cohort on the final frozen source will measure coverage; do not claim95% from projected line hits. Preserve historical reports and exclusions. Root reviews a signed conventional/DCO commit; planning, ADR and history stay in Obsidian.

This is compiler diagnostic testing, separate from the stopped consumer/cross-instance investigation in ADR0285.

### Tracking

https://github.com/MediaNoxLabs/compact/issues/435; parent#351.


### 2026-10-07 — ADR0311 test-only delivery

Signed conventional/DCO commit `f1140869a1518017e09af4600cd78fecce47d0fc` delivers twelve new component methods. Nine tests exercise real native stateful expression dispatch: crypto operand domains/openings, conversions/widths, independent arithmetic/comparison operands, ledger name/index/kind lookup and pure/stateful/witness argument contracts. Three additional Jubjub tests cover mandatory-write/shape admission, expression-local shadowing/escape and actual return type. Valid controls accompany refusals; diagnostics are exact.

All nine new stateful methods and all fourteen Jubjub profile methods pass on first attempt. Combined strict library/test/all-feature Clippy and formatting pass. Three owned paths; entire production expression body remains byte-identical apart from its cfg(test) module declaration; other changes are test files. No runtime/ABI/schema/dependency changes or discovered production bug.

Archive [ADR0311 — Stateful diagnostics and Jubjub scope tests.zip](references-0.3.0.md#note-095), SHA256 `686b1029a6c81d030a13c07f2dd2116f74f22bb47da19b62496d21438df98b7d`. Issue#435 completes its test-only scope. Fresh four-owner instrumented tests and197-source render coverage now run on this signed revision; no projected95% claim. Coverage parent#351 stays open pending measured results and semantic obligations. Parent milestone count remains8/20.


### 2026-10-07 — Measured follow-up: control, lexical and aggregate semantics

The immutable f1140869 cohort passes653tests/197renders. Backend changed8579/9060=94.69%; current macro entrypoint coverage49/55 reflects reused compile-time cache. Historical reports remain separate.

Reopen#435 for the already contemplated second diagnostic slice: stateful non-Boolean conditions, branch result mismatch, equal-arm error propagation and a witness-bearing equal-arm condition evaluated once; falsely annotated/inaccessible lexical locals; struct projection and tuple receiver/index diagnostics; paired-hash helper local annotation and missing nested helper where existing real renderer boundaries make a meaningful test. Every refusal needs an accepted control. No unreachable fallback tests or new production behavior. Test-only component ownership follows the real expression/pure-helper owners.

A separate generated-consumer instrumented cohort will compile the exact maintained adt-list-enum source freshly, using a scratch manifest whose dependencies resolve to the same qualified runtime/macros/testkit. Preserve source bytes and external identities; record actual macro expansion/compiler artifacts. Its existing TS/recorded/VM behavior test supplies execution coverage. Do not delete shared caches or claim cached compilation executed derives. Fresh profile/object identities and explicit source-identical line unions may extend the current baseline; no changed-source or hidden mixed-denominator claim.

The instrumentation floor is not an excuse to duplicate implementation logic or conceal mandatory primitives. Even if floors pass, full export/effect/adoption obligations remain separate parent#351 work, including activatedACC.


### 2026-10-07 — ADR0311 control/scope follow-up and actual macro compilation

Signed conventional/DCO commit `3361d696f359a4c86be2256d8e0613f70a4a26aa` delivers seven more test methods: conditional domain/error propagation, exactly-once witness conditions with equal arms, lexical annotation/shadow/scope exit, aggregate projection and paired-hash helper resolution. All14stateful and9pure-helper methods pass; strict library/test/all-feature Clippy and formatting pass. Only two test files changed; existing production bodies and dependencies remain byte-identical.

A fresh instrumented191-library-test cohort passes. Explicit Boolean hit union with the source-identical f114653-test/197-render baseline adds eight changed backend lines: **8587/9060=94.78%**, total22141/24020=92.18%. No counts/raw profiles are added together and no new denominator lines appear. This is a documented mixed cohort, not a fresh whole-HEAD test run. The95% floor remains unmet;20more hits on this denominator would reach it, but useful semantic obligations govern the next tests.

The separate fresh standalone adt-list-enum consumer uses byte-exact generated library/test/oracle files and an isolated manifest, with no external dependency additions/upgrades. Cargo proves the macro/runtime/consumer actually compiled; its existing TS-oracle/native/recorded/VM test passes. Six missing changed macro entrypoint lines execute. Source-identical Boolean macro union becomes403/431total and55/55changed (100%). No backend profiles joined that cohort. Runtime remains4462/6764total and79/79changed; testkit374/381. Runtime total reflects selected cohorts, not a regression against broader historical fixture execution.

Archive [ADR0311 — Scope diagnostics and real macro compilation.zip](references-0.3.0.md#note-094), SHA256 `22eee400b7e58e9269ec02fe77ba64478d4e5dc478c4711b2b7d5fa0b935c9af`;73evidence files with verified manifest, including separate raw profiles/objects, exact source/lock checks and prior object preservation. [Coverage checkpoint at 3361d696](references-0.3.0.md#note-148) explains all denominators. Issue#435 closes its completed test/consumer slice; parent#351 remains open for backend floor and mandatory export/effect/adoption obligations. FullACC and milestone remain open at8/20.
