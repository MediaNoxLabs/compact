---
id: RUST-ADR-0025
alias: ADR-0025
title: "Accept distinct Compact Maybe instantiations for List heads"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 6bc0659ea65149c4b9e87083044b2d4a4d13234fcd0b73ce47af417240695b72
---
# RUST-ADR-0025 — Accept distinct Compact Maybe instantiations for List heads

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept compiler-assigned concrete Maybe instantiation names while validating the full ordered fields and nested element type. This repairs distinct generic instantiations without inventing a public generic Rust type model or weakening shape checks. Preserve independent Packet/Choice evidence and the remaining path/type limits.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#125 closure](https://github.com/MediaNoxLabs/compact/issues/125#issuecomment-6017440871). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`551b7066`](https://github.com/MediaNoxLabs/compact/commit/551b70660a411256272ac9cf1c30cb16af6eae0f). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 25
status: accepted
date: 2026-10-03
milestone: rust-backend-v2
extends: 24
issue: 125
```

## Historical decision and amendments

### Problem and proof

The unchanged ledger-8 Compact source can declare `List<Packet>` and `List<Choice>` and export both `first_packet(): Maybe<Packet>` and `first_choice(): Maybe<Choice>`. The Scheme Rust IR assigns the two concrete struct shapes `MaybeCompact1` and `Maybe` using `struct-rust-name`; this is intentional because Rust cannot define two non-generic structs named `Maybe`. A fresh compiler probe emits schema-8 IR with exact shapes, but rendering rejects `first_packet` because both native and recorded `StateReturn::ListHead` validation hardcode `name: "Maybe"`. The prior ADR-0024 delivered one typed List-head result; the second instantiation proves the invariant is incorrectly tied to one Rust spelling.

### Before and after developer code

```compact
export circuit first_packet(): Maybe<Packet> { return packets.head(); }
export circuit first_choice(): Maybe<Choice> { return choices.head(); }
```

```rust
// Before: compactc --target rust rejects this valid source with an IR type mismatch.
// After: both compiler-assigned concrete types are generated and callable.
let packet = contract.recording.first_packet(context)?; // MaybeCompact1
let choice = contract.recording.first_choice(context)?; // Maybe
```

The suffix is an implementation name assigned by the compiler's concrete-shape registry; it is not a separate Compact type. A future generated API design may expose a stable generic facade or aliases, but must preserve exact ledger representation and distinguish the concrete shapes.

### Decision and ownership

Keep the closed schema-8 `Type::Struct { name, fields }` domain model for this repair. Validate a List-head result as one compiler-assigned instantiation of the Compact `Maybe` struct by using the existing `is_compact_struct_instantiation(name, "Maybe")` rule, then compare the two fields in order and the full nested value type against the List declaration. Construct the expected IR type using the *actual* compiler-assigned name so errors preserve the source's concrete type. Both native and recorded emitters share the same helper to avoid drift. `MaybeX`, `MaybeCompact`, `MaybeCompactX`, or a wrong field/value shape must still be rejected. Emit `syn` types through the current AST renderer and reuse `ListSlot<T>::head` / `record_head`; no runtime VM, ABI, or private IR schema change.

### Alternatives and risks

Hardcoding one suffix fixes only one traversal order. Ignoring the name entirely would accept an unrelated struct with the same fields from hand-built IR. Introducing a generic runtime `Maybe<T>` would require generic representation derives, currently intentionally restricted to concrete generated types, and a versioned IR design. Stable ergonomic aliases remain a separate generated-code API decision. The current suffix naming is traversal-dependent; fixture and cross-target tests must establish exact shape and operations, while long-term public API stability remains a release gate.

### Verification and delivery

- Add both List-head circuits to one unchanged source and require the ledger-8 TypeScript and Rust targets to compile with no generated-code edits.
- Verify generated native and recorded methods return distinct concrete Rust types with exact empty/populated result, state, four-dimensional gas, and public VM op parity against a fresh TypeScript capture.
- Check a one-dependency generated-crate consumer, rejected malformed `Maybe` IR forms, and packaged proof/verify/validate/apply for both new head calls.
- Pass backend/runtime/focused/workspace, 132 fresh fixtures, 37 pinned oracle sources, rejection and clean-source package gates; record a conventional GPG-signed/DCO local commit and delivery in the ADR, focused issue, and milestone.
- Keep branch publication, remote CI, registry release and wallet/node submission as broader M2 exit gates.

### Delivery evidence — 2026-10-03

Local conventional GPG-signed/DCO commit `551b70660a411256272ac9cf1c30cb16af6eae0f` delivers [#125](https://github.com/MediaNoxLabs/compact/issues/125) on `codex/rust-backend-ast`. One shared backend helper recognizes the Scheme compiler's `Maybe`/`MaybeCompactN` concrete-instantiation names and constructs the expected exact `is_some: Boolean, value: T` type. Native and recorded List-head emission use this helper; invalid names and wrong value shapes are rejected. Generated/runtime ABI remains 11 and private IR remains schema 8; no runtime VM program was added.

The same Compact source now exports `first_packet(): Maybe<Packet>` and `first_choice(): Maybe<Choice>`. Rust generates distinct `MaybeCompact1` and `Maybe` concrete structs and native/recorded methods without output edits. A fresh ledger-8 TypeScript capture and generated Rust match empty/populated results, default absent payloads, private state, exact serialized state, every four-dimensional gas cost, and the complete ordered public VM operations for both heads. Recorded Verify replay effects match. A separate one-dependency generated-crate consumer calls both types. The packaged offline gate proves, verifies, validates and applies each empty-head call, bringing the suite to 52 cases; populated behavior is locally parity-tested.

Passed: 53 backend renderer/four CLI tests, full runtime suite, four focused List tests, all-target offline workspace check, 132 fresh generated fixtures, 37 pinned oracle sources, two compiler rejection probes, packaged consumer/proof gate, byte-for-byte TypeScript oracle re-capture and clean-source `target/rust-runtime-abi11-clean.json` write/verify from commit `551b7066` (`dirty: false`; 8 macro / 229 runtime archive entries). The unrelated `doc/ledger-adt.mdx` change was excluded and restored byte for byte. Branch publication and remote CI remain open. Stable semantic result aliases, nested physical List paths, registry release and wallet/node submission remain wider M2 work.
