---
id: RUST-ADR-0068
alias: ADR-0068
title: "Keep AST backend clean under Rust 1.99 Clippy"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["distribution", "provenance", "validation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b2f92bba932b06e136f1926987d24a63f33686dc8c8916510dad4a60aa55ce39
---
# RUST-ADR-0068 — Keep AST backend clean under Rust 1.99 Clippy

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept mechanical backend Clippy repairs and narrow documented recursive-signature allowances. The compiler-only checkpoint was not a full-workspace pass; subsequent runtime/generated fixes are separate decisions. Preserve unchanged generated output and error/evaluation behavior as the evidence boundary.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#167 closure](https://github.com/MediaNoxLabs/compact/issues/167#issuecomment-6017513579). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`03bb3daa`](https://github.com/MediaNoxLabs/compact/commit/03bb3daa3bc5caed5c06bf474695c9c0a4b5fa02) · [`1e5e8849`](https://github.com/MediaNoxLabs/compact/commit/1e5e884958283ec36e3b24646bb6ff63866030d3). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 68
status: implemented-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/167
```

## Historical decision and amendments

### Problem

The exact `cargo +1.99.0 clippy --all-targets --all-features -- -D warnings` milestone gate at local `03bb3daa` fails in the AST Rust backend with 15 diagnostics before it can assess the rest of the workspace. Nine are `too_many_arguments` on established recursive expression/constructor lowering boundaries; the others are one derivable `Type::default`, two eager `Option` errors, two nested conditions, and an identical Set/Map branch. The earlier runtime-only ADR-0064 repaired a different 53-warning failure. A green full gate is required for rust-backend-v2.

### Before and proposed after

```rust
// Before: a hand-written default and a duplicated Set/Map emission branch.
impl Default for Type { fn default() -> Self { Self::Unit } }
if is_map { emit_slot_call(); } else { emit_slot_call(); }

// After: one declarative default and one slot emission path.
#[derive(Default)]
enum Type { #[default] Unit, /* other variants */ }
emit_slot_call();
```

The long signatures in `recorded.rs`, `stateful.rs` and constructor emission carry AST lowering inputs, typed declarations and recursion/output state. Prefer a grouped context when it reduces ownership complexity and source churn. If a scoped refactor would widen this CI repair beyond its behavior-neutral boundary, use a function-level `#[expect(clippy::too_many_arguments, reason = "...")]` with a specific reason; `expect` becomes a failing warning when the lint no longer applies. Never use a crate-wide allow or raise the global argument threshold.

### Decision and ownership

This is compiler backend maintenance, not generated API or runtime behavior. Fix the six mechanical diagnostics directly and document any narrow recursive-signature expectations at the function definitions. Preserve error behavior, evaluation order, rendered Rust tokens and diagnostics. No runtime, VM, macro, proof adapter, generated ABI 34 or private schema 8 change is intended. Recheck all 137 fixture outputs and focused renderer tests. Run the exact Rust 1.99 full workspace Clippy command until it exits successfully, repairing any newly exposed workspace diagnostics in bounded follow-up slices. Use focused local gates first; repeat proof/application only if generated output or runtime semantics change.

### Acceptance and limits

Create a focused MediaNoxLabs issue in `rust-backend-v2` before source edits. Capture the 15-error baseline and exact command, then pass the same command, formatter, renderer and fixture freshness checks. Record source delta, any `expect` sites and rationale, GPG/DCO conventional commit, and remaining remote CI gate. If the full gate reveals unrelated package findings, do not conflate them with this compiler decision; track and fix them separately. This ADR is not a general emitter redesign or a claim that all long lowering interfaces are ideal. A future context-model refactor may retire the narrow expectations after behavior parity is pinned.

### Tracking

- Related runtime Clippy repair: [#163](https://github.com/MediaNoxLabs/compact/issues/163), ADR-0064.
- Focused issue: pending creation.
- Delivery: proposed; no source or full-gate pass claimed.


### Tracking amendment — 2026-10-04

Focused [#167](https://github.com/MediaNoxLabs/compact/issues/167) was created and assigned to `rust-backend-v2` before the Clippy repair. The pending sentence above preserves proposal chronology. The baseline command exited 101 with 15 AST-backend diagnostics at signed local `03bb3daa`.


### Compiler-only checkpoint — 2026-10-04

Conventional GPG-signed/DCO `1e5e884958283ec36e3b24646bb6ff63866030d3` (`fix(rust-backend): clear Rust 1.99 AST lint findings`, `Refs: #167`) repairs the initial 15 library diagnostics and four subsequently exposed CLI/renderer-test diagnostics. Nine recursive lowering functions carry narrow `#[expect(clippy::too_many_arguments)]` attributes with explicit reasons; no crate-wide allow or threshold change was made. The other ten findings were fixed directly: derive `Type::Default`, simplify two `Option` errors, two nested conditions, identical Set/Map emission, CLI `Option::map`, and three boxed test mutations. `git verify-commit` reports a good signature and the DCO trailer is present.

The exact full Rust 1.99 Clippy command advanced past the backend library, binary and renderer test target after these edits, then failed in two unrelated layers: runtime Merkle test identity conversions and generated fixture `clone_on_copy`/redundant-field patterns. Thus a full-workspace Clippy pass is not claimed. All 59 renderer tests pass; the source-rebuilt compiler reports `Checked 137 fixtures; 0 stale; 0 failed`; formatter and scoped diff checks pass. Backend source delta: six files, 75 insertions and 53 deletions. Runtime-test findings are tracked under ADR-0064/#163; generated-output findings need a separate AST emitter decision. Same-commit remote CI remains deferred.
