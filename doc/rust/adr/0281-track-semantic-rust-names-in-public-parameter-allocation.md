---
id: RUST-ADR-0281
alias: ADR-0281
source_sha256: 769c46479b9446c1b6c9f9dc1e839f1cd8b00040eaf89e89eb9c41a4569362aa
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0281 — Track semantic Rust names in public parameter allocation

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted. Parents R030-02/#346, R030-03/#347, R030-07/#351. Discovered during ADR0276 actual generated Args experiment, before promotion. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0281 — Track semantic Rust names in public parameter allocation

Status: accepted. Parents R030-02/#346, R030-03/#347, R030-07/#351. Discovered during ADR0276 actual generated Args experiment, before promotion.

### Problem and evidence

The public typed-IR rendering API accepts parameter names with raw Rust spelling. `public_parameter_idents` tests both raw and semantic names for occupancy but stores only the raw spelling. For parameters `r#foo` then `foo`, both are selected; emitted positional Rust fails with E0415. The Args prototype also fails with E0124. The reverse order already falls back, making the defect order-sensitive. Current Compact source does not admit this raw spelling: this is a typed-IR renderer/hygiene defect, not a claim that the text frontend emits it.

Exact red IR/generated Rust/rustc logs are retained under `/tmp/compact-adr276/ir-matrix/raw-parameter-collision.*`; the command/runtime artifact identity is `/tmp/compact-adr276/ir-matrix.json`. Existing circuit-name namespace validation already compares semantic names; no circuit validation change is required.

### Before / after and decision

```rust
// Before selected names for raw-first input:
fn example(r#foo: T, foo: T) { /* invalid duplicate binding */ }
// After: preserve the first spelling and allocate the existing stable fallback.
fn example(r#foo: T, __compact_param_1: T) { /* ordered forwarding */ }
```

Store every selected public parameter identifier in the occupancy set by its semantic Rust spelling (strip one optional r# prefix). Keep declaration order, first-name choice, reserved names, fallback base/suffix algorithm and argument forwarding intact. This repairs the existing documented uniqueness contract. It changes only previously invalid rendered interfaces; no runtime, ABI, IR schema or dependency change. Do not introduce a new name-mapping DSL or infer semantic uniqueness from source strings.

### Acceptance

- Regression for raw/plain duplicates in both orders and keyword raw/plain equivalents; reserved raw spellings still use safe fallbacks.
- Raw spelling of a generated fallback already occupies its semantic name, so a later fallback gets a deterministic suffix. Include the prior dollar/underscore and ordinary-name controls.
- Red-before/green-after test evidence and actual generated positional consumer compilation for the original typed-IR reproducer. Record the source-only vs IR-only boundary.
- Strict backend Clippy and the appropriate existing naming/declaration tests. Fixture corpus output is unchanged for supported source inputs; preserve immutable comparison if combined with concurrent work.
- ADR0276 prototype must retest its own duplicate field consumer against this exact fix before promotion. This ADR does not adopt Args.

Issue: https://github.com/MediaNoxLabs/compact/issues/405

### Pre-implementation clarification: pure and stateful reproductions

The first ADR0276 reproduction was a pure circuit: its direct function emitter allocates source identifiers independently. A second stateful Cell reproduction now isolates the shared public wrapper defect (E0415 in both native/borrowed public facades); prototype Args independently adds E0124. Exact second case is `/tmp/compact-adr276/ir-matrix/raw-stateful-collision.{json,rs}` and `raw-stateful-collision-compile.log`.

To repair both proven parameter-boundary cases coherently, the naming component will own one deterministic semantic-occupancy allocator with an explicit reserved-name list. Public stateful wrappers retain their existing reserved list. Pure function parameters use an empty reserved list, preserving ordinary source spellings (including `context`), while expression bindings map each original source name to its selected Rust identifier. Existing invalid-identifier and duplicate-source-name validation stays in its current order before expression lowering. Internal root imports preserve callers. No universal source-name normalization or circuit-name policy change is introduced.

Add a pure consumer pair that returns each of the two distinct source parameters, proving alias fallback does not change argument binding. Test dollar/underscore equivalence and raw fallback occupancy too. Both emitted consumers must compile. Previously unsupported typed-IR syntax is explicitly separated from frontend-supported source names. Any normal corpus change requires individual review.

### Delivered locally — 2026-10-07

ADR0281 fixes a reproduced Rust parameter namespace collision in both pure and stateful emission. One shared allocator reserves semantic identifiers; original source binding keys, declaration order and diagnostic precedence remain intact. Before-fix consumers fail E0415. After-fix generated consumers compile, and pure consumers execute both selected arguments correctly. Raw Rust identifiers in these cases enter through typed IR; no claim that the Compact text frontend admits them.

ADR0282 fixes the reproduced original shielded-send renderer abort on an ordinary debug worker. Thin native expression dispatch reduces repeated control/aggregate frames; all 38 original arm bodies retain their behavior. The permanent profile tests now render on ordinary workers with no enlarged nested stack. Exact-lock original default-worker failure, 16 MiB diagnostic control, frame diagnosis and successful candidate evidence remain preserved separately. Large remaining operation/pure frames and arbitrary-depth resource qualification remain open.

Combined final production sources passed 336 backend tests with RUST_MIN_STACK unset, strict all-target/all-feature Clippy, and identical complete Rust/capability output across 183 source closures. Joint evidence covers both fixes on top of ADR0278. No runtime, ABI, schema or dependency changes; existing generated consumers and proof receipts remain applicable without another identical proof run.

Signed conventional/DCO commit `f0775cc61f54dd836c362e1dc8712cba784bc676`. [ADR0281-0282 — Signed integration receipt](references-0.3.0.md#note-053) preserves exact combined validation scope. Parent acceptance remains 5/20.
