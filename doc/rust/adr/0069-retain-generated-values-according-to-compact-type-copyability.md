---
id: RUST-ADR-0069
alias: ADR-0069
title: "Retain generated values according to Compact type copyability"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "generated-code", "ownership"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 8fe3895c7627a72d37c4d277b36ab122cadefecb82afe9da82bf0f0ca60c677a
---
# RUST-ADR-0069 — Retain generated values according to Compact type copyability

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept conservative type-directed retention and shorthand field syntax to remove unnecessary clones of Copy values. Non-Copy structs, Vectors and opaque values retain required ownership handling. Preserve staged lint progress and focused semantic tests without turning syntax cleanup into a speedup claim.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#168 closure](https://github.com/MediaNoxLabs/compact/issues/168#issuecomment-6017515250). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`da71b0f1`](https://github.com/MediaNoxLabs/compact/commit/da71b0f19b3fd1b162627ab8ccba55ed4ea4de67). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 69
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/168
```

## Historical decision and amendments

### Problem

After the AST backend and runtime test Clippy repairs, Rust 1.99 full-workspace Clippy reaches generated fixture libraries and reports at least 66 `clone_on_copy` diagnostics in an early failure sample: Field, Boolean, FixedBytes, tuples, enum, and bounded Uint values. It also reports explicit `value: value` struct fields. These come from emitter-owned ownership boundaries rather than hand-edited fixture code. Unconditional clone emission bloats developer-facing crates and fails the milestone gate. The same sample contains five generated high-arity method diagnostics; that is a separate decision and is not solved by this copyability slice.

### Before and proposed after

```rust
// Before: unconditional clones of values known to implement Copy.
let step = context.insert_map(0, (true).clone(), (field).clone())?;
let maybe = Maybe { is_some: true, value: value };

// After: ownership syntax follows the Compact type and Rust AST shape.
let step = context.insert_map(0, true, field)?;
let maybe = Maybe { is_some: true, value };

// A non-Copy generated struct or Vector keeps its required clone when reused.
let step = context.insert_map(0, key.clone(), record.clone())?;
```

### Decision and ownership

The compiler AST emitter owns this syntax. Reuse the existing conservative `copy_type(&Type)` classification: scalar Field/Boolean/bounded Uint/fixed Bytes/enum and recursively Copy tuples may be passed directly; generated structs, Vectors and opaque values retain cloning. Add one type-directed syntax helper only for values retained across ownership boundaries. Apply it to constructor VM writes and collection operations, pure/stateful field projection, recorded cell source and observed-call input paths where the emitter knows the Compact type. Do not run a textual post-pass or remove all `.clone()` tokens. Use `syn::FieldValue` shorthand only when the right-hand syntax is a single-segment path matching the field identifier exactly; preserve explicit syntax for expressions and cloned values. No runtime, macro, VM, proof adapter, IR variant, ABI 34 or private schema 8 change is expected.

### Acceptance and limits

Create a focused MediaNoxLabs issue in `rust-backend-v2` before code edits. Pin representative before/after renderer output for Copy Field/Boolean/tuple/Bytes/enum and a reused non-Copy struct/Vector. Compile representative generated fixtures under Rust 1.99 Clippy; regenerate and check all 137 fixture outputs and report source-size delta. Run focused native/recorded parity where changed ownership might affect evaluation; use the full proof/application gate only if a behavior path changes. Record conventional GPG/DCO commit and exact local evidence. Same-commit remote CI remains deferred until the local backlog is complete. This does not address generated methods with more than seven parameters; track that separately.

### Alternatives and risks

A global lint allow would hide future inefficient generated code. A regex rewrite of prettyprinted Rust could remove clones needed for non-Copy values and bypass the AST model. Treating every generated struct as Copy would be false for nested Vector/opaque fields. The existing `copy_type` predicate is conservative; if a new primitive is introduced, it must be classified before the emitter omits clones for it.

### Tracking

- Generated-code design subagent probe: at least 66 observed Copy clone lints and one shorthand lint in the partial Rust 1.99 workspace log; five high-arity lints are separate.
- Parent compiler/CI issues: [#167](https://github.com/MediaNoxLabs/compact/issues/167), [#106](https://github.com/MediaNoxLabs/compact/issues/106).
- Focused issue: pending creation before implementation.
- Delivery: proposed; no code or full-gate pass claimed.


### Tracking amendment — 2026-10-04

Focused [#168](https://github.com/MediaNoxLabs/compact/issues/168) was created and assigned to `rust-backend-v2` before emitter edits. The pending sentence above preserves proposal chronology.


### Local acceptance — 2026-10-04

Conventional GPG-signed/DCO `da71b0f19b3fd1b162627ab8ccba55ed4ea4de67` (`fix(rust-backend): make generated ownership and arity Clippy-safe`, `Refs: #168, #169`) implements the shared `retained_value` AST helper around the existing conservative `copy_type` classifier. Constructor Cell/Set/List/Map and loop values, pure/stateful struct projections, stateful Set/Map reads, recorded cell sources, witnessed transcript values and observed-call inputs now clone only non-Copy Compact values. Struct literal field shorthand is emitted only for an exact one-segment matching path. A generated Vector/struct still clones when reused; Field, Boolean, fixed Bytes, enum, bounded Uint and recursively Copy tuples do not. There is no textual post-pass. Runtime, VM, macro, proof adapter, ABI 34 and private schema 8 are unchanged.

The combined #168/#169 commit changes 83 files, 1,087 insertions and 1,380 deletions. Seventy-eight refreshed generated `lib.rs` fixtures account for 839 insertions and 1,315 deletions (net 476 fewer lines); the separate arity attribute adds about 20 lines to the twelve-argument fixture. Sixty-one renderer tests pass, including new Copy/non-Copy projection, struct shorthand and observed-call ownership probes. The source-rebuilt compactc reports `Checked 137 fixtures; 0 stale; 0 failed`. The external `--consumer` boundary/manifest gate passes, and focused all-features suites pass 2 constructor Map, 2 composite-key, 3 tiny and 4 non-Copy List tests with TypeScript parity and replay. `cargo fmt --all -- --check`, scoped diff and `git verify-commit` pass. The exact Rust 1.99 full-workspace Clippy run no longer reports `clone_on_copy` or `redundant_field_names`; it advances to separate generated Unit/condition-shape lints in passport and other oracle fixtures. No full-workspace Clippy pass or remote CI is claimed. The branch remains local and unpushed by user direction.
