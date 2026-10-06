---
id: RUST-ADR-0079
alias: ADR-0079
title: "Explain missing Rust recording capabilities from typed lowering"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-partial"
topics: ["compiler-cli", "diagnostics", "typed-ir"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: bc49baec78389e36732b5a09bf82ae4cd343cb116f378d4daa36330418301f84
---
# RUST-ADR-0079 — Explain missing Rust recording capabilities from typed lowering

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-partial. Accept the delivered partial typed recording-gap diagnostics from the existing lowerer, keeping malformed IR as a hard error and observed-call collisions separate. The latest amendment still acknowledges enclosing reasons where Option helpers collapse deeper causes; do not claim the original full leaf-precision plan was completed. Historical333-export counts are not the final inventory.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#178 closure](https://github.com/MediaNoxLabs/compact/issues/178#issuecomment-6017531983). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4656cd31`](https://github.com/MediaNoxLabs/compact/commit/4656cd31cb411bc1ada7f808bdb996df476a7b9d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 79
status: accepted-partial
date: 2026-10-05
milestone: rust-backend-v2
issue: 178
```

## Historical decision and amendments

### Problem

The Rust compiler writes `contract/rust-capabilities.json` with only `recorded` and `observed_call` booleans. The current corpus baseline is 159/333 exported circuits with both APIs and 174/333 without recording across 63 sources. A missing API is therefore visible, but a developer cannot tell whether the first blocker is an IR action, nested expression, unsupported type, missing runtime slot, shared callee, or a call-name collision. Choosing the next high-fan-out parity slice by source count alone risks optimizing the wrong cause. The TypeScript target has no corresponding limit.

`tools/compact-rust-backend/src/recorded.rs` already decides support while lowering: `render_recorded_item` delegates to expression helpers, `append_steps` and return lowering, many of which return `Option`/`bool`. A separate syntax or generated-text scanner would duplicate this semantic decision and drift. Existing `RenderError` represents invalid IR and must remain a hard error rather than being mislabeled as a capability omission.

### Before and after

Before, a generated consumer sees a native method but no proving method, and the report only says:

```json
{"name":"set_small","recorded":false,"observed_call":false}
```

After, existing booleans remain and the same lowering attempt adds a stable typed primary reason:

```json
{
  "name": "set_small",
  "recorded": false,
  "observed_call": false,
  "recording_unavailable": {
    "code": "unsupported_type",
    "ir_node": "StateAction::CellWrite",
    "path": "actions[0].value",
    "detail": "Uint<255> Cell recording value is unsupported"
  }
}
```

The exact `set_small` diagnosis above is an illustrative target, not a verified output of the current compiler. Consumer code should be able to show `source.file:line:column` plus `recording_unavailable.code`, and `--rust-require-recording` should print that reason. Generated Rust API shape is unchanged; when a future slice implements the missing case, its existing typed `recorded::set_small` and `set_small_call` methods appear without a special fallback path.

### Decision

Replace silent unsupported returns in the *existing* recorder with an internal typed `RecordingGap` result. Use an explicit `RecordingOutcome<T> = Result<T, RecordingFailure>` (or equivalent), where `RecordingFailure` distinguishes `Unsupported(RecordingGap)` from `Invalid(RenderError)`. Convert leaf lowering, `append_steps`, return lowering and helper planning in small compile-tested stages. The first unsupported branch reached in source-order traversal is the primary reason. Keep one primary reason per missing API; dependent reasons may be added later only if they come from the same traversal. No second eligibility pass or AST text pattern matching.

`RecordingGap` has a stable machine `code` and typed IR node discriminator, a structural path such as `actions[0].value.left`, and a short human detail. The existing circuit-level `source` remains authoritative. Do not fabricate nested source spans: only emit a nested span if schema-8 IR already carries one; otherwise report the circuit location plus structural path. `observed_call_unavailable` is separate because a recorded method may exist while its `_call` name collides with an exported circuit. The report retains `recorded` and `observed_call` exactly for old readers and increments only the *capability-report* schema from 1 to 2. Private Compact IR schema 8 and runtime ABI are unaffected.

Recommended staged conversion within this issue:

1. Add typed gap/report model and compatibility tests. Convert the outer action and return dispatch plus the observed-call collision reason. Every unsupported circuit must have a nonempty reason at this checkpoint; a temporary nearest-parent reason is acceptable only during implementation, never in the delivered report.
2. Convert nested helper returns (`amount_source`, `cell_source`, `vector_bindings`, `field_expression`, `boolean_expression`, `scalar_expression`, `shared_call_argument`, `shared_field_call`) and recursive `append_steps` to propagate exact first failure with path. Preserve emission order and the existing hard-error cases. Avoid broad `Option::or` logic that overwrites the earliest reason.
3. Update `--rust-require-recording` diagnostics and run an inventory command over all 137 oracle sources. Assert all missing API rows have a typed reason and aggregate counts by code/node/path prefix; publish that ranking in the vault to choose the next parity slice.

### Acceptance

- Focused renderer tests cover unsupported action, nested expression/type, unsupported shared callee, unsupported return, action-free no-recorded-effect, and recorded/observed-call name collision; they assert deterministic first reason and retained `recorded`/`observed_call` booleans.
- `RenderError` still rejects malformed IR and invalid source. The report schema is 2, keys are stable, and previously supported circuits emit identical Rust methods and VM behavior.
- `--rust-require-recording` includes source location and reason code/path. The 137-source inventory produces a reason for every unsupported exported circuit; classify actual cause counts only after instrumentation.
- Run focused backend/CLI tests during conversion, then all fixture freshness and representative consumer compilation because report serialization and emitter control flow change. Full proof/ledger gate is required if generated recording methods or VM behavior change; otherwise compare generated source and supported capability booleans exactly and defer that expensive gate to the next behavior slice. Run exact workspace Clippy at integration checkpoint.

### Alternatives and limits

A standalone analyzer over IR or generated Rust was rejected because it would duplicate the recorder's support rules. A single generic `unsupported` value is insufficient for prioritization. This ADR reports why recording was omitted; it does not itself add any of the 174 missing APIs, prove TypeScript parity, or resolve nested source-span coverage.

### Tracking

- Accelerator: [Milestone 2 — Full TS parity delivery accelerator](references.md#private-note-10).
- Parent capability issue: [#152](https://github.com/MediaNoxLabs/compact/issues/152).
- Focused issue: pending creation before code.
- Implementation and validation: pending.


### Tracking amendment — 2026-10-05

Focused [#178](https://github.com/MediaNoxLabs/compact/issues/178) was created in `MediaNoxLabs/compact` and assigned to `rust-backend-v2` before any ADR-0079 code changes. This ADR remains proposed; the example reason above is illustrative until the lowering pass produces measured diagnoses.


### Corpus baseline amendment — 2026-10-05

ADR-0078's ABI-35 Counter reset slice added the previously missing `counter_parameter.reset_round` recorded and observed-call APIs. The fresh local corpus is now **160/333 supported, 173/333 missing**; the earlier 159/333 and 174/333 figures above remain the pre-reset baseline. Root-cause counts are still unknown until this ADR's actual lowering diagnostics run.


### Local partial delivery — 2026-10-05

The existing `recorded.rs` traversal now returns a typed `RecordingOutcome`: supported syntax, an unsupported `RecordingGap`, or a preserved hard `RenderError`. Capability report schema 2 retains `recorded`/`observed_call` and adds a code, IR-node discriminator, structural path and detail for each unavailable API. An observed-call name collision has its own `name_collision` reason; a missing recorded method gives the observed-call field a `recording_unavailable` dependency reason. Strict `--rust-require-recording` prints the circuit's source location and reason code/path. Private IR schema 8, runtime ABI 35, supported generated syntax and VM behavior are unchanged.

This is **accepted partial** against the original decision. The outer action and return dispatch, recursive Sequence/Let and inlined callee action paths, CellWrite type/value checks, and selected Field/Boolean expression exits report their first definite failed node. Remaining `Option` expression helpers still collapse some nested causes to an honest enclosing `unsupported_action` or `unsupported_return`; they do not yet distinguish every inner argument/conversion. A future pass should convert the remaining helpers to `RecordingOutcome` and retain exact nested paths, then refine shared-callee distinctions. No second eligibility scan or generated-text scanner was added.

Fresh ABI-35 corpus: 137 oracle sources compiled, 333 exported circuits, 160 supported and 173 missing. The first-reason ranking is `unsupported_action` 83, `unsupported_return` 60, `unsupported_expression` 16, `unsupported_type` 14. Leading nodes are `StateReturn::Expression` 47, `StateAction::Let` 44, `StateAction::Assert` 17, `StateAction::CellWrite` 14, `StateReturn::CellRead` 10, and `Expr::Call` 9. Counts are **before ADR-0080 unsigned Cell recording** and describe the first known blocked node, not a full leaf-cause taxonomy. The temporary JSON receipt is `${LOCAL_EVIDENCE}/compact-capability-inventory-adr79.json` on this host.

Local gates: 68 renderer tests, rejection/publication/proof-capability gate, 137 byte-identical generated fixtures with a report invariant requiring a nonempty reason for every false API, targeted Rust 1.99 all-target/all-feature Clippy with `-D warnings`, and compiler target/manifest plus separate generated Cargo consumer passed. No full proof rerun was needed because every supported generated library remained byte-identical. Remote CI and branch publication remain deferred under the local-first policy. Commit reference follows.


Delivery reference: local conventional GPG-verified/DCO commit `6ee8d125ed5f8e684ae5d930fb605d34a70b97ca` on `codex/recording-reasons`. This branch was not pushed; #178 stays open for nested helper precision and later same-revision CI.

### Integration with unsigned Cell — 2026-10-05

The isolated implementation `6ee8d125` was cherry-picked into `codex/rust-backend-ast` as conventional GPG-verified/DCO `4656cd31cb411bc1ada7f808bdb996df476a7b9d`, after ADR-0080. The combined compiler was snapshotted to `${LOCAL_EVIDENCE}/adr79-adr80-compactc` (SHA-256 `e25c3992b900d0a2ce5a2b7840be7b206814f1d9f3618d25e532095b510d3ca6`). The ABI-35 137-source, 333-circuit report now has 170 recorded and 163 missing: 82 `unsupported_action`, 59 `unsupported_return`, 16 `unsupported_expression`, 6 `unsupported_type`. The previous 160/173 ranking above remains the pre-ADR-0080 baseline. All 137 fixtures regenerate byte-identically, their missing-reason invariants pass, 68 renderer tests pass, and strict rejection plus separate generated consumer pass on the integrated compiler. Exact workspace Clippy and Nix package gates are in progress as an integration checkpoint. Deeper nested reason precision remains open under #178.


Integration gate closure: exact Rust 1.99 workspace all-target/all-feature Clippy with `-D warnings`, fmt, and GPG verification passed. Fresh Nix package `${HISTORICAL_NIX_STORE}/cyz7c897hdx8akzjnmdrk3xvwcri3s7j-compactc` passed all 137 fixture freshness/reason checks and separate generated Cargo consumer/manifest gate. The source-built and packaged compilers agree on checked generated output. The user-owned unstaged `doc/ledger-adt.mdx` kept the Nix source dirty; no clean release or remote CI claim. [Integrated issue evidence](https://github.com/MediaNoxLabs/compact/issues/178#issuecomment-5983175856).
