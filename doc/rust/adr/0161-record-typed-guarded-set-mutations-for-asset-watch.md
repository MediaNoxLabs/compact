---
id: RUST-ADR-0161
alias: ADR-0161
title: "Record typed guarded Set mutations for asset watch"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "typed-plan", "Set"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 814b23447da67ce4c3ae2d6248f8cb04fbf380d0808be6fb35625799ee9cdffb
---
# RUST-ADR-0161 — Record typed guarded Set mutations for asset watch

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Closed typed guard for the asset-watch cross-Map Add/Drop Set mutation is recorded. Admission is structural and scoped to the audited operands and branches; it does not generalize arbitrary Set mutations.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#265 closure](https://github.com/MediaNoxLabs/compact/issues/265#issuecomment-6017678528). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
title: ADR-0161 — Record typed guarded Set mutations for asset watch
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original ledger-8 asset registry `setWatch` is proof-required, but the generated Rust capability stops at its guarded `StateAction::Assert` after two scoped disclosures. It accepts an opaque record key and a `ListMutation`, checks writable state and cross-Map existence, then conditionally adds or drops that key in `watchList` with opposite presence guards before `recordWrite`.

### Before and after

Before: native `ledger_contract::setWatch(context, witnesses, recordId, mutation)` runs, while no recorded or observed-call API exists. After: `ledger_contract::recorded::setWatch(...)` and `contract.recording().setWatch_call(...)` expose a replayable typed Add/Drop transition. The generated crate keeps `OpaqueString` and `ListMutation` as developer-facing parameters.

### Decision

Recognize a closed typed mutation structure from schema-13 IR, not a circuit name. Validate the two scoped aliases; exact `Unspecified/Add/Drop` enum guard; transitive writable/write helpers; the same two distinct declared opaque-key Map operands in `recordExists`; declared `Set<OpaqueString>` slot and source order of Add absent-check/insert versus Drop present-check/remove. Reuse ledger-8 typed Map and Set recording slots. Preserve short-circuit cross-Map reads, original assertion messages, witness/private effects, and four gas dimensions. No runtime ABI or IR schema change.

### Negative guards

Reject changed key or enum provenance, changed Map or Set declaration/index/type, reordered guards, altered Add/Drop direction or presence check, extra or missing effects, invalid mutation, and effectful helper variants. Runtime must reject unknown record, duplicate Add, absent Drop, closed, and frozen states like original TypeScript without a successful write.

### Acceptance

Freshly compile unchanged original Compact source to TypeScript and Rust. Compare Add/Drop with a nonempty Unicode opaque key and record prestate across TypeScript, native Rust, recorded Rust, and independent Verify replay: serialized state, ordered VM operations, four gas dimensions, private state/effects/outputs, and witness calls. Cover all rejection cases, renderer negative mutations, generated fixture freshness and focused local gate. Produce pinned ZKIR 2.1.0 proofs for Add and Drop; verify and ledger-8 validate/apply. Commit conventionally with GPG and DCO. No push, remote CI, or Nix rebuild for this slice.

### Delivery

Decision recorded before implementation. Issue and commit to follow.


### Local delivery — 2026-10-05

Issue: https://github.com/MediaNoxLabs/compact/issues/265 (rust-backend-v2). Conventional GPG-signed and DCO commit `fef95a49f8151bcba994b8695e99d8b76b43cdff` admits the closed typed cross-Map existence and Add/Drop Set mutation structure. `setWatch` now emits recorded and observed-call APIs; all 10 exported asset-registry circuits are recorded and `--rust-require-recording` succeeds on the unchanged original source. The matching domain is deliberately bounded to the declared `Unspecified/Add/Drop` mutation variants. The ADR-0157 class assertion now follows the declared first enum variant and struct member without relying on their oracle spellings, verified by a renamed enum/member renderer case. No runtime ABI or IR schema change.

Fresh original TypeScript capture for Unicode key `record-α-1` equals the checked-in fixture. Add and Drop match native Rust, recorded Rust and independent Verify replay: serialized state, 32/31 ordered VM operations, four gas dimensions, private state/effects/outputs and witness calls. Original TypeScript and Rust agree for unknown record, duplicate Add, absent Drop, invalid mutation, closed and frozen rejections. Renderer mutations reject changed provenance, enum direction, ordering, Set declaration, Drop guard and collapsed cross-Map existence.

Pinned ZKIR 2.1.0 generated `setWatch` keys and circuits; Add and Drop were proven, verified, validated and applied through ledger 8 using the generated observed call. Artifacts: `${LOCAL_EVIDENCE}/compact-adr161-proof`. Local checks: 128 renderer tests, eight asset-registry integration tests, strict Clippy, 152 fixture outputs with zero stale/failed, format/diff checks, and exact-head focused gate 10/10 recorded (receipt `${LOCAL_EVIDENCE}/compact-focused-fef95a49/receipt.json`). The broad local gate requires the capability, keys and both-branch proof/apply selector. Branch is local and unpushed; parent full integration gate is pending.
