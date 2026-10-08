---
id: RUST-ADR-0272
alias: ADR-0272
source_sha256: 0ba3f7826771ccb0f8a407579b2f09357064f61b34243c8ad204cb031ec7af02
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0272 — Reject explicit discriminants in Compact cell enums

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for implementation. Parents R030-03/#347, R030-04/#348, R030-07/#351 and R030-13/#357. Independent external review F3/H2. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0272 — Reject explicit discriminants in Compact cell enums

Status: accepted for implementation. Parents R030-03/#347, R030-04/#348, R030-07/#351 and R030-13/#357. Independent external review F3/H2.

### Problem

CompactCellValue currently checks that enum variants are unit variants but does not reject explicit Rust discriminants. It encodes by declaration ordinal. CompactEnum rejects explicit discriminants, so the two publicly exposed derives disagree on declaration validity. Generated Compact enums do not currently contain such discriminants; this affects malformed handwritten use of the derive boundary.

Before:

```rust
#[derive(CompactCellValue)]
enum Status { Revoked = 2, Active = 5 }
```

The derive emits ordinal0/1 cells while Rust discriminants are2/5. After: a source-spanned diagnostic rejects explicit discriminants for CompactCellValue, consistent with CompactEnum. Ordinary unit enums retain identical ordinal encoding.

### Decision

Add the missing discriminant validation to the existing macro input checker. Preserve separate diagnostics for enum payloads and nonconcrete types. Include literal and expression discriminants in unit rejection tests and a real external consumer compile-fail case, alongside a valid unit-enum consumer. No generated output, runtime codec, ABI, schema or package pin change.

The review's one-variant enum question is a separate hypothesis, not a proven defect. Root found the existing direct singleton enum CellValue/FAB/field/binary representation round-trip at runtime-rs/tests/enum_derive.rs:298, which the reviewer did not read. Rerun that test rather than duplicating it. It establishes zero-byte binary width and a single empty FAB value atom, with invalid ordinals rejected. No representation change is justified.

### Acceptance

Preserve a red regression proving the current malformed input is accepted. After the fix, the precise public diagnostic rejects it, valid enums compile and all existing macro/runtime representation tests pass with strict Clippy. Confirm unchanged generated valid fixture output. Retain explicit before/after source, commands, compiler versions and the singleton result. Independent external retest follows.

Issue: https://github.com/MediaNoxLabs/compact/issues/396

### 2026-10-07 — Delivered locally

Commit `6250d454db1a44f1261b7e903c6fa9ac5af6ecce`. CompactCellValue now rejects explicit Rust discriminants consistently with CompactEnum, preserving the separate payload diagnostic and ordinary ordinal encoding. The red test reproduced successful malformed expansion; the fixed result passes27 macro tests, three existing runtime enum representation tests, two generated hygiene tests and strict macro Clippy. A real external consumer accepts ordinary variants and rejects explicit values with the source diagnostic. No emitter/generated fixture, ABI, schema, dependency or runtime codec change.

The existing singleton enum representation test resolves external hypothesisH2: zero-byte binary width, one empty FAB atom, field representation and CellValue round trips are correct for the pinned primitives. No duplicate test or codec change was added. These are component checks; no broad corpus/proof rerun or full milestone audit acceptance is claimed.

[ADR0272 — Local delivery receipt](references-0.3.0.md#note-043). Independent final retest remains a parent audit obligation.
