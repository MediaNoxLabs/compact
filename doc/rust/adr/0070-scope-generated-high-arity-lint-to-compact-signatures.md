---
id: RUST-ADR-0070
alias: ADR-0070
title: "Scope generated high-arity lint to Compact signatures"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "generated-code", "ownership"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 77730421b9e23d135c333da4dcc883816beb1a99663f4bb58e98404f9c909c8c
---
# RUST-ADR-0070 — Scope generated high-arity lint to Compact signatures

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept function-local high-arity lint allowances only when generated Rust inputs exceed the threshold imposed by Compact's positional API. Do not replace the API with a synthetic argument object or suppress lints crate-wide. Preserve tool-version rationale and separately tracked Unit/condition lints.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#169 closure](https://github.com/MediaNoxLabs/compact/issues/169#issuecomment-6017516960). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`da71b0f1`](https://github.com/MediaNoxLabs/compact/commit/da71b0f19b3fd1b162627ab8ccba55ed4ea4de67). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 70
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/169
```

## Historical decision and amendments

### Problem

Compact permits circuits with twelve positional arguments, and rust-backend-v2 deliberately accepts and proves such calls. Rust 1.99 Clippy under `-D warnings` reports at least five `too_many_arguments` errors in the generated `observed-composite-keys` fixture: free recorded `record_twelve`, recording handle, borrowed/owned Contract wrappers, and observed `record_twelve_call` (13–15 Rust inputs after context/receiver). This is generated API shape, not a handwritten compiler function. The user-facing method should retain Compact parameter names and order; changing them to a tuple or argument bag would alter the public Rust API and wallet input conversion. ADR-0069 addresses unnecessary Copy clones but will not clear this lint.

### Before and proposed after

```rust
// Before: faithful generated signature, but Clippy rejects it.
pub fn record_twelve<Private>(context: CircuitContext<Private>, a: Field, b: Field /* ... */) -> Result<...>;

// After: the same signature with a narrow generated-code rationale.
#[allow(clippy::too_many_arguments, reason = "preserves the declared Compact circuit signature")]
pub fn record_twelve<Private>(context: CircuitContext<Private>, a: Field, b: Field /* ... */) -> Result<...>;
```

### Decision and ownership

The AST emitter should add a method-level Clippy attribute only when the emitted Rust signature has more than seven inputs, counting receiver and context as Clippy does. Apply the same rule to all generated function/method paths, including pure, native, recorded, Contract and observed-call wrappers. The attribute carries a reason explaining the Compact signature contract. `allow` is chosen here because generated crates must compile with ordinary rustc and different Clippy versions without an unfulfilled-expectation warning; handwritten compiler lowering functions remain under the narrower ADR-0068 `expect` decision. Do not add a module or crate-wide lint suppression. No Compact source, runtime, VM, macro, proof adapter, ABI 34 or IR schema 8 change is intended.

### Acceptance and limits

Create a focused MediaNoxLabs issue assigned to `rust-backend-v2` before source edits. Test 7-input and 8-input boundaries in rendered AST and a real twelve-Field oracle source, including recording and observed-call methods; verify parameter names/order and unchanged aligned input. Refresh 137 fixtures and compare source delta. Rust 1.99 Clippy should cease reporting generated high-arity diagnostics; exact full-workspace gate is shared with ADR-0069 and may expose other generated issues. Run focused local gates first and proof only if call encoding changes. Record conventional GPG/DCO delivery and defer same-commit remote CI until the local backlog is complete.

### Alternatives and risks

A new generated argument struct could improve call-site readability but would be a public API design change requiring wallet/serialization and migration evidence; it is outside this lint repair. A global Clippy threshold or module-wide allow would hide unrelated signatures. A function-level attribute is explicit in the generated crate and does not change call semantics.

### Tracking

- Related generated ownership cleanup: [#168](https://github.com/MediaNoxLabs/compact/issues/168), ADR-0069.
- Related full Clippy repair: [#167](https://github.com/MediaNoxLabs/compact/issues/167), ADR-0068.
- Focused issue: pending creation before implementation.
- Delivery: proposed; no code or full-gate pass claimed.


### Tracking amendment — 2026-10-04

Focused [#169](https://github.com/MediaNoxLabs/compact/issues/169) was created and assigned to `rust-backend-v2` before emitter edits. The pending sentence above preserves proposal chronology.


### Local acceptance — 2026-10-04

The same conventional GPG-signed/DCO `da71b0f19b3fd1b162627ab8ccba55ed4ea4de67` implements this distinct AST signature decision alongside ADR-0069. The backend enables `syn` visit-mutate and traverses the final typed `syn::File`, adding a reasoned method/function-level `#[allow(clippy::too_many_arguments)]` only when `signature.inputs.len() > 7`. Item functions, impl methods and trait methods are covered, including nested generated modules; ordinary signatures remain unannotated. This preserves Compact parameter names/order and aligned wallet input without changing runtime, ABI 34 or schema 8. Renderer probes show four Compact parameters stay below the effective seven-input boundary and five cross it on observed-call methods; the twelve-argument source parses and receives multiple scoped attributes.

Only `observed-composite-keys/lib.rs` changed in the second fixture refresh for the high-arity attribute (about 20 added lines); the full combined fixture delta is recorded in ADR-0069. All 61 renderer tests, 137 fresh fixtures, the external generated consumer/negative gate, and focused 2 composite-key tests pass. The exact Rust 1.99 workspace Clippy log no longer reports `too_many_arguments`; it later fails on separate generated Unit and condition-shape syntax. Formatting, diff and `git verify-commit` pass. Full-workspace and remote CI remain open under local-first delivery; this issue remains open until same-commit remote CI.
