---
id: RUST-ADR-0280
alias: ADR-0280
source_sha256: 92157c6068fe1babdb1f938b6b0405131c2188ee3f6d202ba8fd27029f0591b3
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0280 — Separate native state expressions and public facade from circuit assembly

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted. Parent R030-05/#349. Baseline fca7577b. Research: [ADR-0279 — Give typed value lowering a cohesive compiler owner](0279-give-typed-value-lowering-a-cohesive-compiler-owner.md). Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0280 — Separate native state expressions and public facade from circuit assembly

Status: accepted. Parent R030-05/#349. Baseline fca7577b. Research: [ADR-0279 — Give typed value lowering a cohesive compiler owner](0279-give-typed-value-lowering-a-cohesive-compiler-owner.md).

### Problem

stateful.rs combines three distinct responsibilities in3423 lines: recursive typed expression lowering with ordered query/witness effects, public method signatures, and action/return-plan circuit assembly. Engineers must distinguish native execution rules from convenience API rules inside the same source file. The existing function boundaries already separate these responsibilities.

### Decision and before / after

Move render_state_expression intact, including its existing argument-count annotation, to private stateful/expression.rs. Move render_contract_method intact to private stateful/facade.rs. Keep render_stateful_circuit, its Pending/BranchFrame lexical work stack, native-frame selection and circuit_analysis reexports in stateful.rs. Retain existing crate-private entry paths through explicit reexports. Each component has explicit imports and responsibility documentation.

```rust
// Before: all three bodies share stateful.rs.
pub(crate) fn render_state_expression(/* current parameters */) { /* current body */ }
pub(crate) fn render_contract_method(/* current parameters */) { /* current body */ }
pub(crate) fn render_stateful_circuit(/* current parameters */) { /* current body */ }

// After: stateful.rs is the native circuit assembly owner.
mod expression;
mod facade;
pub(crate) use expression::render_state_expression;
pub(crate) use facade::render_contract_method;
// render_stateful_circuit body stays identical.
```

Runtime ownership, ABI, IR, dependencies, generated public methods and type/witness order do not change. This does not split match arms or refactor effect flags. The expression module remains a large function; the benefit is navigable ownership, not reduced cyclomatic complexity. Subsequent semantic decomposition needs its own justification.

### Validation and coordination

Root owns stateful.rs and its two new modules. ADR0279 owns lib.rs/value_lowering.rs. The disjoint exact-body extractions may share one frozen combined verification cohort to avoid redundant builds. Preserve per-slice exact body comparisons and hashes, and clearly label the combined test source.

- Existing functions remain byte-identical, excluding imports/module documentation; no order/visibility changes to callable entry functions.
- Full backend tests use default worker stacks; strict Clippy.
- Compare immutable before/after renderers over all183 source fixtures: complete generated Rust and capabilities must be identical. Investigate any difference rather than updating fixtures.
- Reuse existing witnessed Cell/Counter, terminal lexical return, constructor and native-frame generated tests as a focused warm batch. No proof rerun for identical emitted source and unchanged runtime.
- Document the three navigation owners in the vault. No parent closure solely from a smaller file.

Issue: https://github.com/MediaNoxLabs/compact/issues/403

### Delivered locally — 2026-10-07

The shared Compact value-to-Rust carrier, checked coercion and Copy/retention rules now have one private value_lowering.rs owner. lib.rs falls from 3,879 to 3,518 lines. Native expression lowering and public method generation now live in stateful/expression.rs and stateful/facade.rs; stateful.rs owns action/return assembly. All moved function bodies remain exact except three required crate-private visibility changes in value lowering. No runtime, public API, ABI, schema, dependency or emitted-source change.

The combined frozen source passed 326 backend tests on default workers, strict Clippy and 34 focused generated behavior tests across 13 packages. Immutable before/after renderers produced identical complete Rust and capability reports for all 183 fixture sources. Generated behavior tests execute unchanged checked-in libraries; the renderer comparison establishes those outputs are preserved. This improves ownership and navigation, with no claim of reduced cyclomatic complexity or faster execution. Large expression and assembly functions still warrant future semantic review.

Signed conventional/DCO commit `d5d4a6c935f5ada7d20d43adf06126d601db19d4`. Exact combined receipt: [ADR0279-0280 — compact-adr279-280 — delivery-receipt.json](references-0.3.0.md#note-052). Individual moved-body receipts preserve per-owner provenance. Accepted parent outcomes remain 4/20.
