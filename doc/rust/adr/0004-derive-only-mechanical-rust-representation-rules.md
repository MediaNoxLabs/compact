---
id: RUST-ADR-0004
alias: ADR-0004
title: "Derive only mechanical Rust representation rules"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api", "macros"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: bec6780bf8a4d0043902932547813f6fcdb825ad32c1425e2b8f09dc8ac5e1cc
---
# RUST-ADR-0004 — Derive only mechanical Rust representation rules

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept the delivered mechanical CompactEnum derive with width inferred from declared variants, paired with CompactCellValue and upstream representation traits. The opening binary_bytes attribute proposal was not the delivered syntax, and compatibility moved to ABI 4. Keep invalid-shape/ordinal checks, source-size measurements and the absence of a reliable timing improvement explicit.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#109 closure](https://github.com/MediaNoxLabs/compact/issues/109#issuecomment-6017413016). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`6b3b9685`](https://github.com/MediaNoxLabs/compact/commit/6b3b9685be3759c8f799b928c35e7d75e3c6e4ff). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 4
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 109
```

## Historical decision and amendments

### Problem

The AST emitter hand-generates enum ordinal codecs repeatedly. That is long, review-heavy output. A macro can remove repetition, but it must not hide Compact's checked ordinal, width, FAB alignment, or source diagnostics. The existing struct path already reuses upstream ledger derives and local `CompactCellValue` for mechanical Cell/FAB rules.

### Before and candidate after

```rust
// Before: emitted for each enum (excerpt).
impl FieldRepr for Choice { /* match yes => 0, no => 1 */ }
impl BinaryHashRepr for Choice { /* write a checked one-byte ordinal */ }
impl FromFieldRepr for Choice { /* reject every value except 0 and 1 */ }

// Candidate after, only if parity is proved:
#[derive(Clone, Copy, CompactCellValue, CompactEnum)]
#[compact_enum(binary_bytes = 1)]
pub enum Choice { yes, no }
```

The proposed `CompactEnum` syntax is not implemented. It must generate checked conversion code and reject malformed attributes or unsupported enum shapes at compile time.

### Decision boundary and ownership

Keep upstream `BinaryHashRepr`, `FieldRepr`, and `FromFieldRepr` derives for struct representations where they match Compact, plus the local `CompactCellValue` and Merkle derives for missing mechanical rules. Consider a dedicated enum derive only after exact byte, field, and Cell parity for all declared widths and invalid ordinals. A derive may append impl items; it cannot rewrite the source item. Do not introduce a body-wide attribute macro just to shorten output.

The renderer would emit the derive and metadata; `runtime-rs-macros` would own expansion; the runtime would retain codec traits and upstream primitive delegation. Keep generated source and macro expansion inspectable in review.

### Evidence and acceptance

Current baseline: `tests-rust-backend/enum-identity/lib.rs` and `election-oracle/lib.rs` hand-emit enum conversions, while `runtime-rs-macros/src/lib.rs` already implements `CompactCellValue`. No production enum derive has been delivered. Acceptance requires parity tests for 1-byte and wider ordinals, default values, invalid tags, FAB alignment, binary length, and round-trip through a ledger Cell; clear compiler errors and no ABI drift. Measure generated lines and compile time before selecting it.

### Tracking

- Parent: [M2 stable runtime crates #106](https://github.com/MediaNoxLabs/compact/issues/106).
- Focused issue: [#109](https://github.com/MediaNoxLabs/compact/issues/109). Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Delivery: proposed; no implementation claim.


### Decision history

- 2026-10-02: Created ADR-0004 from the generated-crate research probes with status `proposed`; linked MediaNoxLabs/compact#109 in `rust-backend-v2`. Local delivery evidence is recorded above. Future changes should append a dated amendment or link a superseding ADR.


### Amendment — 2026-10-02: implement a checked unit-enum derive

#### Problem and chosen before/after

The emitter currently repeats `Default`, `FieldRepr`, `BinaryHashRepr`, and `FromFieldRepr` implementations for every generated Compact enum. `CompactCellValue` already derives FAB alignment and Cell decoding from the same ordinal order, so the handwritten blocks create two places that can drift on width or invalid-tag rules.

```rust
// Before: the generated crate repeats ordinal matches and width logic.
#[derive(CompactCellValue)]
pub enum Choice { yes, no }
impl FieldRepr for Choice { /* ordinal into one field */ }
impl BinaryHashRepr for Choice { /* little-endian ordinal bytes */ }
impl FromFieldRepr for Choice { /* reject undeclared ordinals */ }

// After: the variant declaration is the single source of ordinal order.
#[derive(CompactCellValue, CompactEnum)]
pub enum Choice { yes, no }
```

The derive computes binary width from the number of declared variants, exactly as the current emitter and `CompactCellValue` do; no `binary_bytes` attribute is needed. For 1, 256, and 257 variants the width is 0, 1, and 2 bytes respectively. The first variant remains the default. Unsupported generics, tuple/struct variants, and an empty enum must report compile-time errors. This is a narrow mechanical derive, not a macro over circuit bodies.

#### Ownership and acceptance

The emitter emits the two derives and drops repetitive trait impl blocks. `runtime-rs-macros` owns checked expansion and should share the width calculation with `CompactCellValue`; `runtime-rs` reexports the derive and keeps using upstream representation traits. The generated Rust ABI remains 3. Measure source lines on enum identity, election, and passport. Validate field and binary bytes, FAB/Cell behavior, default and invalid ordinals, a 257-variant width boundary, generated fixtures, and consumer compilation. The decision advances [#109](https://github.com/MediaNoxLabs/compact/issues/109) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2); record the commit and evidence after implementation. The current date's earlier proposed text remains the historical baseline.


#### Compatibility refinement

The generated source now imports `CompactEnum`, which is absent from the older ABI-3 runtime. Therefore the emitter and runtime deliberately move to ABI 4 together, while the private JSON IR remains schema 6. Generated output includes an ABI-4 const assertion, so a shared runtime from the older line fails at the compatibility boundary. Toolchain version moves from 0.31.130 to 0.31.131. The runtime crate is still an unpublished local `0.1.0` source package; the release decision and order remain in ADR-0007.


#### Probe evidence and emitter detail (in progress)

The first compile of election and passport caught an important constraint: upstream `BinaryHashRepr`/`FieldRepr`/`FromFieldRepr` struct derives expand with unqualified `Fr` and `MemWrite`, so modules containing structs retain those imports. Enum-only modules import just `CompactCellValue` and `CompactEnum`. This keeps the derive migration from breaking unrelated generated structs.

Direct `cargo expand` inspection of `enum-identity` found one each of `Default`, `FieldRepr`, `BinaryHashRepr`, `FromFieldRepr`, `Aligned`, and `CellValue` for `Choice`, plus both invalid-ordinal fallback paths. The checked-in source lines changed from 87 to 47 for enum identity, 1,082 to 954 for election, and 4,611 to 4,532 for passport; these are source-size measures, not compile-time measurements. The runtime test verifies one-byte and 257-variant two-byte field, binary, FAB/Cell, default, and invalid-tag behavior. Broader fixture/workspace and packaged compiler gates are still running and must be recorded before delivery.


#### Validation and measured cost — 2026-10-02

The checked `CompactEnum` derive now owns `Default`, `FieldRepr`, `BinaryHashRepr`, and `FromFieldRepr`; `CompactCellValue` retains FAB alignment, conversion, and Cell decoding. Both derives share the binary-width function. The runtime test passes exact field/binary/FAB byte and Cell round trips for one, two, and 257 variants, rejects invalid field and Cell ordinals, and checks the 256-to-257 width boundary. Macro tests reject generic, data-carrying, explicit-discriminant, and empty enum shapes. `cargo expand` for generated `Choice` shows exactly one implementation of each expected trait and checked invalid-ordinal fallbacks. The generated ABI is deliberately 4, the toolchain is 0.31.131, and IR schema remains 6.

The independent Cargo check timing used the pre-change generated source from local Git `HEAD` with only its ABI assertion changed from 3 to 4, versus fresh generated source, in the same temporary external crate with a warm dependency graph. Three alternating `cargo check --offline` samples gave medians (old to new): enum identity 1.474s to 1.692s; election 1.655s to 1.589s; passport 1.881s to 1.671s. Samples varied substantially, so this does not establish a compile-time speedup or regression. The developer-facing source reduction is exact: 87 to 47, 1,082 to 954, and 4,611 to 4,532 lines respectively.

Local gates passed: 129 compiler fixtures current with 0 stale/failed; all 129 generated libraries type-check in `cargo check --workspace`; 48 renderer tests; macro and enum representation tests; enum identity, election, and passport scenario tests; the Nix-built packaged compiler (`0.31.131`) with `--consumer --proof` replay/partition/prove/verify/validate/apply; two source rejection probes; both runtime `.crate` package rehearsals; formatting, license-header validation, and diff checks. The broad `cargo test --workspace --exclude compact` run compiled the entire workspace and passed the fixture tests reached, then was stopped before the full serial suite completed. No full-suite or remote-CI pass is claimed. Publication, wider derive review, and remote CI remain open in [#109](https://github.com/MediaNoxLabs/compact/issues/109) and the M2 milestone.


#### Local delivery record

Conventional GPG-signed/DCO commit `6b3b9685` delivers this accepted-partial decision on the local `codex/rust-backend-ast` branch. [Focused issue #109 delivery comment](https://github.com/MediaNoxLabs/compact/issues/109#issuecomment-5945802015) records the evidence and limits; [parent release issue #106 update](https://github.com/MediaNoxLabs/compact/issues/106#issuecomment-5945804986) records the ABI-4 publication dependency. The worktree is clean, 35 commits ahead of the remote M1 branch, and no M2 branch push or remote CI result is claimed.
