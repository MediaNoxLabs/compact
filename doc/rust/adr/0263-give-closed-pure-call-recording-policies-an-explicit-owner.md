---
id: RUST-ADR-0263
alias: ADR-0263
source_sha256: aa6e448321b334458b7e038b587a1de05393667b43f2e1e71ca8eba986741aa3
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0263 — Give closed pure-call recording policies an explicit owner

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted; implementation pending. Parents R030-05/#349 and R030-07/#351. Source inspection at c43e8fd3 with concurrent Point-composition work excluded from this slice. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0263 — Give closed pure-call recording policies an explicit owner

Status: accepted; implementation pending. Parents R030-05/#349 and R030-07/#351. Source inspection at c43e8fd3 with concurrent Point-composition work excluded from this slice.

### Problem

`recorded.rs` remains over9000lines. Eight existing helpers around lines1577–1939 decide whether particular pure source calls can participate in legacy recorded paths. They are mixed with ledger action lowering, return emission and generated function assembly. Engineers changing a pure-call admission rule must locate these policies inside the large emitter, and most boundary tests exercise them indirectly through generated fixtures.

These are deliberately different admitted languages: scalar Field arithmetic, bounded unsigned arithmetic, exact assertion helpers, paired-Field hashing, literal Field vectors, and struct/hash values. Combining them into a generic evaluator could accidentally broaden admitted effects. This decision preserves those distinctions.

### Decision and ownership

Move the existing helpers without semantic edits into `recorded/pure_calls.rs`, expose only the existing parent-used functions with `pub(super)`, and import them in recorded.rs. Keep the cell-type policy in its current owner; pass through the existing parent helper rather than duplicate it. Move the existing transitive unsigned characterization test alongside the new module, add tests for unknown/transitive/recursive callees, wrong types/arity/scopes, hidden effects and hash/vector layout requirements.

Before:
```rust
// recorded.rs combines pure-call policy and action/function emission.
fn closed_pure_field_call(...) -> bool { /* current full-body audit */ }
fn render_recorded_item(...) -> Result<RecordingOutcome<syn::Item>, RenderError> { /* ... */ }
```
After:
```rust
// recorded.rs retains emission and imports explicit policy owners.
mod pure_calls;
use pure_calls::closed_pure_field_call;
// recorded/pure_calls.rs retains the identical bounded full-body audit.
pub(super) fn closed_pure_field_call(...) -> bool { /* same body */ }
```

No macros, runtime changes, new admitted domain, public API or ABI/schema changes. A positive policy result still depends on normal source typing; these predicates are not a replacement for the frontend or renderer's complete type checks. No new blanket claim about effect freedom for arbitrary pure calls.

### Acceptance

- Mechanical body comparison confirms all moved production bodies unchanged apart from visibility.
- Focused tests characterize all policy families and refusals that would otherwise accidentally permit an effect or malformed call.
- Existing backend tests and strict Clippy pass.
- Fresh182fixture generated source and capability reports match the same compiler baseline after accounting only for the independent ADR0259 Point change. Use frozen baseline/new executables; do not compare mixed source snapshots.
- Keep current ordered profile selection and error precedence. Preserve exact public diagnostics and user dirty files.

### Maintenance result

One named policy module and nearby tests give engineers a direct place to inspect these recording admission boundaries. This is a bounded ownership extraction, not closure of the remaining large renderer/domain-model work.

### Delivered locally

`6e78bd2b0bac99068118bee480527cf9e80a0c69`; production bodies mechanically identical except visibility/whitespace. Eight focused tests (seven new),287backendtests/strictClippy pass;182frozen generated source+capability outputs identical. [ADR0263 — Pure-call policy extraction receipt](references-0.3.0.md#note-029). Larger model/refactor parent remains open.
