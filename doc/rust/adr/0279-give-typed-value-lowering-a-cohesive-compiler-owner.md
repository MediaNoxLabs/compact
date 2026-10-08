---
id: RUST-ADR-0279
alias: ADR-0279
source_sha256: 5f271019bcae4b9c2bcd794c5bcb1c67075d97afc4eeced80921f824da05a457
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0279 — Give typed value lowering a cohesive compiler owner

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for the first bounded value-lowering extraction only. Parent R030-05/#349. Basis: current fca7577b and the read-only component proposal below. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0279 — Give typed value lowering a cohesive compiler owner

Status: accepted for the first bounded value-lowering extraction only. Parent R030-05/#349. Basis: current fca7577b and the read-only component proposal below.

### Problem and decision

lib.rs mixes final contract assembly with the shared scalar, carrier, checked coercion and Copy/retention rules used by pure, native and recorded emitters. Move the complete shared value-lowering family into a private value_lowering.rs module. Preserve function bodies, evaluation/error order and crate-private imports at the root. No new evaluator, generic abstraction or macro. All subsequent component splits in the attached map are proposed, not authorized by this ADR.

### Before / after

```rust
// Before: root lib.rs owns assembly and typed carrier rules.
fn rust_type(ty: &Type) -> Result<syn::Type, RenderError> { /* existing body */ }
pub(crate) fn coerce_expression(/* existing signature */) { /* existing body */ }

// After: private value_lowering.rs owns the same rule bodies.
pub(crate) fn rust_type(ty: &Type) -> Result<syn::Type, RenderError> { /* identical body */ }
// lib.rs retains explicit crate-private imports for existing callers.
use value_lowering::{rust_type, coerce_expression /* other exact helpers */};
```

Emitter ownership changes only. Runtime, schema, package/dependency graph, generated interfaces and all admitted/refused shapes stay identical. Internal helpers remain private where possible. No generated fixture refresh is permitted to hide a source difference.

### Acceptance

- Exact moved bodies except necessary internal visibility/imports; clear module docs and explicit imports.
- Full backend tests on default workers, strict Clippy, and frozen complete Rust/capability equality for all 183 sources.
- Existing scalar, wide-unsigned, aggregate/tuple and nested-map generated behavior tests as one focused warm batch; no redundant proof run for identical code and runtime.
- Retain exact source/lock/compiler evidence and update vault ownership navigation.
- This extraction improves ownership/navigation; it does not claim lower cyclomatic complexity or full large-file decomposition completion.

### Research and later component roadmap

## Next cohesive emitter decomposition — lib.rs and stateful.rs

Read-only, 2026-10-07. Current inspected source: lib.rs3879 lines and stateful.rs3423 lines. Root owns the separate ADR0277 recorded.rs extraction; this proposal neither modifies nor reserves that work. No builds, edits, ADR reservation or API change here.

### Recommendation: start with one shared value-lowering owner

A bounded first extraction is `value_lowering.rs`: move the existing scalar/type and coercion functions **intact**, with crate-private imports/reexports preserving current internal callers. This is a cohesive owner for mapping typed Compact values to Rust carriers and checked conversion syntax, used by pure, native and recorded emission. It is not a new evaluator or admission policy.

Exact move set from lib.rs:

- :364–494 — `field_literal_bytes`, `UnsignedMaximum`, `unsigned_maximum`, `wide_uint_type`, `field_to_bytes_32_syntax`, `unsigned_cast_syntax`, `unsigned_arithmetic_syntax`.
- :495–574 — `rust_type`, `map_slot_value_type`, `map_slot_types`.
- :950–1108 — `coerce_aggregate_fields`, `coerce_expression`, `copy_type`, `retained_value`.

Approximately370 existing lines move into one independently navigable component; lib.rs falls to about3510 plus a small import. This intentionally completes the *shared typed-value* owner rather than moving only three convenient scalar helpers and leaving their mutual coercion/type dependencies scattered. The module must explicitly say it emits syntax/delegates runtime operations and does not run the ledger or validate whole-circuit purity.

Dependencies: `ir::{Expr, Type}`, `RenderError`, `naming::ident`, `proc_macro2::Span`, `syn`; pinned transient-crypto field parser used by the existing literal validator. No `Contract`, witness map, source traversal, constructor, query context or capability dependency. `wide_uint_type`, aggregate helper and map-value helper stay private. Functions called across siblings become pub(crate) within this private module, not exported by the library. A crate-private `use value_lowering::{...}` at lib root can preserve existing `crate::rust_type`/`crate::UnsignedMaximum` imports for this body-preserving slice; later direct-owner imports can be reviewed separately rather than churn ADR269/277 files now.

The module groups type-directed operations without inventing an AST-builder trait, service object, context bag or macro. Human benefit: a developer changing Uint range/cast/coercion behavior has one owner and can see all small/wide branches and checked runtime selection together. It is not a claim that the remaining large emitter is solved.

### Complete target component map, delivered in bounded stages

| Owner | Existing implementation to move intact | Responsibility and dependency direction |
|---|---|---|
| `value_lowering.rs` (~370) | Above exact functions | Typed carriers, literal bounds, checked conversion/arithmetic syntax. Leaf dependency of every emitter. First recommended slice. |
| `pure_expression.rs` (~900) | `unit_statements` :1319–1438, `expression_with_calls` :1439–2218; `lift_block_condition` :1251–1288 | Existing pure expression/Unit sequencing with declared pure-call lookup. Depends on value lowering/naming/shared expression syntax. Keep recursive functions together. No native ledger/witness fallback or recorded admission added. |
| `expression_syntax.rs` (~50 plus moved existing test) | `discard_expression` :1211–1246, `condition_needs_statement` :1247–1250, `discarded_expression_tests` :1289–1318 | Shared materialized-expression discard and condition shape helpers, used by both pure and native emitters. Keep these out of pure_expression so native code does not import a nominally pure semantic owner merely for statement construction. |
| `type_inventory.rs` (~455) | `collect_named_types` :575–641, `collect_expression_types` :642–815, `collect_action_types` :816–917, `collect_return_plan_types` :918–949; `collect_constructor_step_types` :2244–2326 | IR-only traversal collecting named declarations. Does not render syntax. Constructor collection remains here because it recursively calls type collectors. Explicit duplicate/type error order unchanged. |
| `constructor.rs` (~670) | `infallible_constructor_expr` :2219–2243; `constructor_step_uses_witness` :2327–2410; `render_constructor_vm_steps` :2411–2972 | Existing initialization effect sequencing/admission. Depends on circuit_analysis, native state expressions, pure expression, value lowering and ledger path syntax. Never fold into runtime constructor semantics. |
| `stateful/expression.rs` (~1640) | Entire `render_state_expression` :39–1681 including its clippy annotation | Native stateful expression lowering, ordered witness/query effects and pure fallback. Keep recursive arm order and existing mutable declaration/statement arguments verbatim. |
| `stateful/facade.rs` (~50) | `render_contract_method` :1683–1732 | Public convenience method syntax over already emitted native functions, matching recorded/facade's separate role. No VM or admission logic. |
| `stateful.rs` (~1715 after two moves) | Existing `render_stateful_circuit` :1734–3423 plus imports/reexports | Circuit assembly and existing action/ReturnPlan work stack. It owns scopes/branches and native-frame selection. Keep Pending/BranchFrame local during this extraction. |
| `lib.rs` (~1400 after staged moves; estimate, not target) | Public errors/render entry points, namespace preflight, final module/slot/artifact assembly and syntax passes | Top-level orchestration. Keep current preflight before map construction and retain public paths. Split further only against a demonstrated owner, not an arbitrary line limit. |

These are a component plan, **not one request to implement every move at once**. First value lowering is low-risk and reusable. Next native expression+facade extraction gives stateful.rs a coherent assembly role; pure_expression plus its tiny shared syntax helper can then reduce lib.rs without importing stateful semantics backwards. Constructor/type inventory follow once that dependency direction is stable.

`stateful/expression.rs` remains a large function. Moving it establishes ownership and navigation but does not reduce cyclomatic complexity or make its effect flags intrinsically safer. A later arm-family decomposition needs its own tests/decision; splitting individual arms now would change control/context threading and exceed a verbatim extraction. Likewise `render_stateful_circuit` remains a large assembly function; its lexical continuation behavior is delicate, so extracting local work-stack variants or rearranging branches is specifically outside this proposal.

### Name/shape helpers left for deliberate ownership

`ledger_path_expr` (:57) and `expected_ledger_paths` (:2973) could later share a tiny ledger-layout owner, but do not mix that unrelated move into value lowering. `is_compact_struct_instantiation` (:2999) and `list_head_result_type` (:3012) are stdlib ADT shape interpretation, not scalar conversion; leave them in place or later add an explicit stdlib-shape component beside existing coin_shapes. Do not silently generalize them during this extraction. `CompactSignatureLint` and `CompactBooleanLiterals` are syntax postpasses and should remain visibly separate from semantic lowering.

### Internal/public boundaries

- Public `render`, `render_with_capabilities`, `render_with_proof_capabilities`, RenderedContract, RenderError and report/reason paths stay unchanged.
- No ABI, IR/capability schema, runtime method, package/dependency or generated API changes.
- New components are private modules. Internal reexports can be retained temporarily; explicit named imports are preferable to `use super::*` because they document owner dependencies.
- No source-name allowlist, fallback/admission changes, altered tuple or witness parameter order, clone/Copy changes, temporary renumbering, formatting transform or error precedence change.
- `circuit_analysis` stays the IR-dependency owner; don't move its reexports or duplicate witness/call graph logic in the new expression modules.
- Avoid concurrent source freezes with ADR269/proof gate and root277. First extraction need not edit their typed_plan/recorded files if root reexports remain.

### Existing verification to reuse

1. Compare moved syntax/token bodies before/after (normalize only visibility/import paths). Unit tests should not be added solely to mirror moved implementation.
2. Existing full backend tests + strict Clippy. Specifically retain render.rs tests `tuple_coercion_widens_uints_but_rejects_narrowing` (:3975), `field_cast_requires_an_unsigned_operand` (:5074), `single_element_tuple_is_a_tuple_in_type_and_expression` (:5340), `unsigned_add_and_cast_reject_field_inputs` (:5503), `unsigned_literal_checks_the_declared_maximum` (:5540), `field_literal_requires_canonical_ledger_field_value` (:5567), `rejects_noncanonical_or_unsupported_unsigned_maxima` (:5756), and `terminal_let_preserves_return_scope_but_siblings_and_branches_do_not_escape` (:7717). Current names should be resolved again after concurrent source additions.
3. Freeze before/after immutable compiler/Scheme and compare all registered fixture lib.rs + capability reports byte-for-byte. Run normal freshness, not a blanket baseline refresh: a single difference is an investigation, not permission to regenerate. Include original DID current capabilities so a component move cannot hide an admission change.
4. For value lowering, execute already captured literal-coercion, wide-unsigned, aggregate/tuple and nested-map generated tests. For stateful extraction, reuse terminal lexical return, witness Cell/Counter, constructor and native-frame controls; original DID native/recorded parity remains a selected cross-domain control if root wants it.
5. No repeated full proof run merely for byte-identical generated code and unchanged runtime; existing gate receipts remain tied to their actual heads. No build-time or performance claim from line relocation.

Disk is constrained; run one warm-cache focused integration batch after owner freeze. This read-only proposal used no build target or copies.

Issue: https://github.com/MediaNoxLabs/compact/issues/402

### Delivered locally — 2026-10-07

The shared Compact value-to-Rust carrier, checked coercion and Copy/retention rules now have one private value_lowering.rs owner. lib.rs falls from 3,879 to 3,518 lines. Native expression lowering and public method generation now live in stateful/expression.rs and stateful/facade.rs; stateful.rs owns action/return assembly. All moved function bodies remain exact except three required crate-private visibility changes in value lowering. No runtime, public API, ABI, schema, dependency or emitted-source change.

The combined frozen source passed 326 backend tests on default workers, strict Clippy and 34 focused generated behavior tests across 13 packages. Immutable before/after renderers produced identical complete Rust and capability reports for all 183 fixture sources. Generated behavior tests execute unchanged checked-in libraries; the renderer comparison establishes those outputs are preserved. This improves ownership and navigation, with no claim of reduced cyclomatic complexity or faster execution. Large expression and assembly functions still warrant future semantic review.

Signed conventional/DCO commit `d5d4a6c935f5ada7d20d43adf06126d601db19d4`. Exact combined receipt: [ADR0279-0280 — compact-adr279-280 — delivery-receipt.json](references-0.3.0.md#note-052). Individual moved-body receipts preserve per-owner provenance. Accepted parent outcomes remain 4/20.
