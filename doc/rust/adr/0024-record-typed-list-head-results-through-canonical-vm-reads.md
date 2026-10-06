---
id: RUST-ADR-0024
alias: ADR-0024
title: "Record typed List head results through canonical VM reads"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 313eef2924b5993576cd70223e47788fdaec8f20bdda582e1219c82350c511a0
---
# RUST-ADR-0024 — Record typed List head results through canonical VM reads

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed root List-head recorded results when the declared element and concrete Maybe shape match supported representations. The record-valued positive/absent proofs are bounded evidence. ADR0025 resolves the subsequently discovered instantiation-name collision; it supplements rather than supersedes this return-lowering decision.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#124 closure](https://github.com/MediaNoxLabs/compact/issues/124#issuecomment-6017439015). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`551b7066`](https://github.com/MediaNoxLabs/compact/commit/551b70660a411256272ac9cf1c30cb16af6eae0f) · [`adccaf0d`](https://github.com/MediaNoxLabs/compact/commit/adccaf0d7209e27ceb06123868e86d1eed845f11). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 24
status: accepted
date: 2026-10-03
milestone: rust-backend-v2
extends: 23
issue: 124
```

## Historical decision and amendments

### Problem and evidence

A fresh ledger-8 compile accepts `List<Packet>` and `first_packet(): Maybe<Packet>`. The AST Rust backend emits the native method but omits `recorded::Contract::first_packet`: `recorded.rs` restricts `StateReturn::ListHead` to Boolean or Field elements. The generic runtime `ListSlot<T>::record_head<M>` and `RecordingFrame::head_list<T,M>` already take `CellValue` types and construct the canonical ledger-8 gather/verify programs. ADR-0023 established typed List writes and witness heads, but did not cover a public recorded head result.

### Before and after developer code

```rust
// Before: native execution works, but this method is missing.
let native = contract.first_packet(context)?; // Maybe<Packet>
let call = contract.recording.first_packet(context)?; // compile error
```

```rust
// After: generated recorded method returns the typed Compact Maybe struct.
let recorded = contract.recording.first_packet(context)?;
assert_eq!(recorded.result.value, expected_packet);
let prepared = prepare_call(&recorded, ...)?;
```

### Decision and ownership

Permit a root `List<T>::head()` recorded return when the result exactly matches the compiler's `Maybe<T>` shape and `T` is a representation already supported by generated `CellValue` and `Default`. Keep the result type generated from IR. Emit the existing `ledger_slots::<field>.record_head::<Maybe,_,_>(frame)` call; do not append VM text or duplicate runtime programs. The runtime and private IR schema stay unchanged; generated/runtime ABI stays 11. Prove one record-valued head circuit on both empty and populated state, preserving the default absent payload, result, gas, Verify op order, state, and ledger application. Check a separate one-dependency generated crate consumer. Reuse the fresh TypeScript oracle source and compare behavior where supported.

### Alternatives, limits and risk

A new runtime List-head helper would duplicate the generic canonical operation. Allowing arbitrary result shapes would weaken the IR type invariant. The current compiler Rust IR assigns a single generated name to distinct `Maybe<T>` shapes in one source: adding both `Maybe<Packet>` and `Maybe<Choice>` to a probe produced a type mismatch. This ADR does not claim that name-collision case; keep it as a separate source/type registration issue. Nested physical List paths remain outside this root-path decision.

### Acceptance and delivery

- Add `first_packet` to unchanged typed List source and verify ledger-8 TypeScript/Rust target output; preserve oracle capture.
- Generate a recorded method without hand editing output; assert typed `Maybe<Packet>` result on empty/populated state and exact native/recorded gas and trace behavior.
- Prove, validate and apply the generated head call with packaged artifacts and an external consumer.
- Pass backend/runtime/focused/workspace/fixture/rejection/package gates; record a conventional signed/DCO local commit and evidence in this ADR, focused GitHub issue and rust-backend-v2 milestone.
- Keep remote CI and release/publication as milestone exit gates.

### Delivery evidence — 2026-10-03

Local conventional GPG-signed/DCO commit `adccaf0d7209e27ceb06123868e86d1eed845f11` implements [#124](https://github.com/MediaNoxLabs/compact/issues/124) on `codex/rust-backend-ast`. `recorded::Contract::first_packet` now uses the existing typed `ListSlot<Packet>::record_head<Maybe>` and `RecordingFrame::head_list`; the emitter retains exact `Maybe<Packet>` IR matching. Generated/runtime ABI stays 11, private IR schema stays 8, and the runtime VM program is unchanged.

A fresh ledger-8 TypeScript compile and capture pins empty and populated `Maybe<Packet>` results, the zero-valued absent payload, exact tagged state, private state, no private FAB, one ordered head query, four-dimensional gas, and the complete public VM transcript. Generated Rust native/recorded results, gas, state and normalized Verify ops match; replay effects match. The separate one-dependency generated crate consumer calls the new API. The packaged gate proves, verifies, validates and applies `first_packet` on an empty List through ledger-8, raising the offline suite to 51 call cases. Populated head behavior is parity-tested locally; the proof application case covers empty head.

Passed: 53 backend renderer/four CLI tests, full runtime suite, three focused List tests, all-target offline workspace check, 132 fresh generated fixtures, 37 pinned oracle sources, two rejection probes, packaged consumer/proof gate and clean-source `target/rust-runtime-abi11-clean.json` write/verify (`dirty: false`; 8 macro and 229 runtime crate entries). TypeScript oracle re-capture matches byte-for-byte. The unrelated `doc/ledger-adt.mdx` edit was excluded and restored byte for byte. Branch publication and remote CI remain open; the separate multi-`Maybe<T>` name collision and nested List paths remain milestone work.


### Follow-up — 2026-10-03

ADR-0025 / [#125](https://github.com/MediaNoxLabs/compact/issues/125) resolves the distinct `Maybe<T>` name collision found in this slice. Local signed/DCO `551b7066` validates compiler-assigned instantiation names and proves both Packet and Choice List-head calls; the broader remote CI/release gates remain open.
