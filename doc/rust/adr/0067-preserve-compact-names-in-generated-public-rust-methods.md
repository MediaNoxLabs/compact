---
id: RUST-ADR-0067
alias: ADR-0067
title: "Preserve Compact names in generated public Rust methods"
date: 2026-10-04
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler-cli", "diagnostics"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 54550ea0e77de085d1beeb06264624fdd7c40588c7ba5911525be3111553e73e
---
# RUST-ADR-0067 — Preserve Compact names in generated public Rust methods

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept source parameter names in public generated Rust signatures, with deterministic normalization and collision-safe fallbacks. Internal positional bindings and semantics remain unchanged. Preserve wrapper-only scope and the runner setup correction; meaningful names do not loosen typing or identifier safety.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#166 closure](https://github.com/MediaNoxLabs/compact/issues/166#issuecomment-6017511988). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`ed6fbb80`](https://github.com/MediaNoxLabs/compact/commit/ed6fbb80c3861888680b315cc5ad6c6d44706f0b). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 67
status: accepted-local
date: 2026-10-04
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/166
```

## Historical decision and amendments

### Problem and design probe

The AST Rust target emits synthetic positional names in public stateful methods, even when the Compact source has meaningful parameter names. Across the ABI-33 fixtures, 758 `__compact_param_N:` bindings appear in 67 of 137 generated libraries (including internal functions). The `tiny` Compact circuit `set(v: Field)` appears as `Contract::set(..., __compact_param_0: Field)`; election `vote_commit(ballot)` and zerocash `spend(dest_public_key, input_coin)` lose those names too. Pure passport functions retain names, and the older `codegen-rust` oracle emitted source names for representative stateful signatures. This is a developer-facing AST-backend regression: positional calls still work, but IDE signatures, generated documentation, and review diffs are less readable.

### Before and proposed after

```rust
// Before, generated public method (excerpt).
pub fn spend(&self, context: CircuitContext<Private>,
             __compact_param_0: public_key,
             __compact_param_1: coin_info) -> Result<_, CompactError>;

// After, same types and positional call order.
pub fn spend(&self, context: CircuitContext<Private>,
             dest_public_key: public_key,
             input_coin: coin_info) -> Result<_, CompactError>;
```

The first slice covers discoverable public `Contract` methods, borrowed and owned recording handles, and observed `*_call` builders. Existing free generated circuit functions and private helpers may still use synthetic internal bindings until a separate source-name aliasing slice is proven. Public call sites remain positional; this is readability, not a new named-argument feature.

### Decision and ownership

The private schema-8 `Parameter { name, ty }` already carries Compact spelling. Add one shared emitter helper that converts `$` to `_`, uses Rust raw identifiers where valid, and allocates a deterministic synthetic fallback for collisions with method locals (`self`, `context`, `witnesses`, `observed`, `private_state`, `input`, `recorded`) or another normalized parameter. Apply it only to public signature AST nodes and their wrapper call arguments; keep the same declaration order, types, internal function signatures and VM body. Avoid a text rewrite or body-wide macro. No runtime, derive, proc macro, ledger/zk primitive, proof adapter or IR schema change. Rust parameter names do not change the type ABI, so ABI 33 need not increment; test that old source compatibility assertions remain valid.

### Acceptance and limits

Create and assign a focused MediaNoxLabs `rust-backend-v2` issue before implementation. Inspect tiny, election, zerocash, stateful nested and passport controls; assert source names in public methods and stable positional forwarding. Test `$` normalization, Rust keywords, reserved method locals, normalized duplicates and twelve-parameter order with deterministic fallbacks. Regenerate all 137 fixtures and measure signature/source delta. Compile an external generated consumer and run focused native/recorded/observed parity tests; expand to proof only if the wrapper change exposes a behavior risk. Record conventional GPG/DCO commit and exact local evidence here. Remote CI is deferred until the local backlog is complete.

This slice does not shorten generated bodies or claim compile-time/runtime improvement. Free generated functions remain a separate design choice; a future amendment can add aliases without changing their internal semantic bindings.

### Tracking

- Parent generated ergonomics: [#110](https://github.com/MediaNoxLabs/compact/issues/110).
- Focused issue: pending creation before implementation.
- Delivery: proposed; no code or parity claim.

### Tracking amendment — 2026-10-04

Focused [#166](https://github.com/MediaNoxLabs/compact/issues/166) was created and assigned to `rust-backend-v2` before implementation. ADR-0066/#165 remains queued for the distinct Merkle VM read slice.

### Local implementation and evidence — 2026-10-04

Conventional GPG-signed/DCO commit `ed6fbb80c3861888680b315cc5ad6c6d44706f0b` (`feat(rust-backend): preserve Compact names in public methods`, `Refs: #166`) implements one shared `public_parameter_idents` allocator in the AST backend. It preserves source spelling after `$` normalization and uses Rust raw identifiers for valid keywords. Wrapper locals and normalized collisions receive deterministic unique `__compact_param_N` fallbacks. The four public emitter paths use those identifiers for both method signatures and positional forwarding: `Contract`, borrowed recording, owned recording, and observed `*_call`. Internal circuit functions still use their existing synthetic bindings. No runtime, macro, proof adapter, VM program, ABI 33, or private IR schema 8 change was made. `git verify-commit` reports a good signature and the commit includes DCO.

For example, regenerated tiny `BorrowedContract::set` and `Contract::set` now expose `v: runtime::Field`, and `set_call` uses `v` in both the FAB input and positional recorded call. Election `vote_commit` exposes `ballot`; zerocash `spend` exposes `dest_public_key` and `input_coin`. Before this commit those public positions were `__compact_param_0` and `__compact_param_1`. The original free circuit functions still keep synthetic names, as this ADR specifies.

Local checks at the committed source: the allocator unit test passes; all 58 renderer tests pass, including twelve-parameter forwarding; the fixture refresh checked 137 contracts, updated 63 generated libraries, and failed none. Across all fixture libraries, synthetic parameter declarations fell from 758 to 364; the remainder is in free/internal generated functions outside this slice. Four generated crates (tiny, election, zerocash, counter parameter) pass `cargo check --all-features --offline`. Tiny's three native/recorded/TypeScript parity tests and counter's two bounded-parameter/replay tests pass. The locally rebuilt `target/debug/compactc` with the pinned ledger-8 `compactc-scheme` passed `check_compactc_target.py --consumer`, including manifest, external consumer, and negative type probes, ending `compactc target boundary and manifest: passed`. `cargo fmt --all -- --check` and scoped `git diff --check` pass. The only remaining tracked worktree edit is the unrelated user-owned `doc/ledger-adt.mdx`.

The first local consumer invocation mistakenly pointed `COMPACTC_SCHEME` at the Nix `compactc` wrapper and caused recursive process spawning. That run is invalid evidence; the process chain was stopped and the successful rerun used the separate `${HISTORICAL_NIX_STORE}/c09kjdk3rapk46pms7yva6blh0hqhncq-compactc/bin/compactc-scheme` executable. This is a runner setup correction, not an emitter fix.

Name-only wrappers do not alter VM effects or proof inputs, so the 95-call proof gate was not repeated for this focused slice. Same-commit remote CI and a final backlog-wide proof gate remain open under the local-first delivery order. [#166](https://github.com/MediaNoxLabs/compact/issues/166) remains open.
