<!--
This file is part of Compact.
Copyright (C) 2026 Midnight Foundation
SPDX-License-Identifier: Apache-2.0
-->

# ADR 0001 — Struct-name disambiguation in the Rust codegen

- Status: Accepted
- Date: 2026-07-14
- Scope: `compiler/rust-passes-*.ss` (the `compactc --target rust` target)
- Supersedes / relates to: the initial disambiguation introduced with the
  `digital-passport` fixture (commit `218d2c8`)

## Context

Compact's frontend pass `expand-modules-and-types` inlines every
`module … { … }` and monomorphises every generic **before** the IR reaches
`Ltypescript`, where the Rust backend runs. A consequence is that two
different sources can contribute two **distinct** structs that share one
name:

- a generic message-protocol module instantiated twice (e.g. for `Issuance`
  and `Verification`) emits two `RequestMessage` / `ResultMessage` structs
  with type-parameter-substituted bodies — exactly what `digital-passport`
  produces; or
- two separately-imported modules that each `export struct Rec { … }`.

Rust has no module-prefix escape hatch here: both structs want to be
`pub struct <Name>` at crate scope. The backend must therefore assign each
colliding variant a distinct Rust name (`Name`, `Name_1`, …) and use that
name **consistently** everywhere the struct appears.

The original implementation (commit `218d2c8`) built an `eq?`-keyed table
mapping each tstruct IR **node** to its disambiguated name, populated by
scanning **signatures, typedefs, and ledger declarations**. Two defects
followed, both flagged by automated review:

1. **Incomplete fingerprint.** Two same-named structs were treated as "the
   same" (and merged into one Rust struct) when their field *types*
   coincided, even if their field *names* differed. The fingerprint keyed on
   `(struct-name, field-types)` and ignored field names. The merged struct
   carried one variant's field names; the other variant's field accesses
   then referenced fields that did not exist (`error[E0609]`).

2. **Rename applied only in type positions.** The disambiguated name was
   consulted when rendering *types* (`type-rust`) and the struct
   *definition*, but **not** at value/decode sites: struct literals
   (`Name { … }`), the decoder turbofish
   (`decode_via_field_repr::<Name>`), and default expressions
   (`Name::default()`) all emitted the raw name. A struct renamed `Name_1`
   in its signature would still be *constructed* as `Name`, a type mismatch
   (`error[E0308]` / `error[E0422]`).

Both defects were **latent** on `digital-passport`: its colliding structs
happen to differ by field type (so #1 never mis-merged them) and are never
constructed in a circuit body (so #2 was never reached). They would fire on
the next contract whose colliding structs differ only by field name, or
that constructs a disambiguated struct in a body.

Root cause common to both: the disambiguation was keyed on **node
identity** and populated from **signatures only**, but the emitter must
resolve names at **body sites** too, where the IR nodes are distinct objects
the signature scan never saw.

## Decision

Resolve disambiguated struct names by **structural fingerprint**, and make
the fingerprint complete.

1. **Recursive structural fingerprint.** `tstruct-fingerprint` keys on
   `(struct-name, [(field-name, field-fingerprint) …])`, where each field's
   fingerprint comes from `type-fingerprint` — a disambiguation-independent
   key that recurses through nested struct / enum / vector / tuple /
   transparent-alias structure (leaf and nominal-alias forms delegate to the
   stable `type-rust`). Same-name structs that differ in field *names*, field
   *types*, **or the body of a nested colliding struct** are kept distinct. A
   shallow key (bare field-type name) would merge two `Outer { inner: Inner }`
   whose `Inner`s differ, since both would record the bare string `Inner`.
   `type-fingerprint` never consults the rename tables, so it is identical
   whether computed while the tables are empty (build) or full (resolution).

2. **Fingerprint-keyed fallback for body sites.**
   `build-struct-rust-name-ht` returns two tables: the existing `eq?`-node
   table (unchanged, so signature/typedef sites resolve byte-identically)
   **and** an `equal?`-keyed fingerprint→name table. `struct-rust-name`
   tries the node table first; on a miss (any body-site node) it computes
   the node's fingerprint — with the disambiguation tables forced empty so
   nested user-struct fields render as bare names, matching how the table
   was built — and looks it up. On a miss it falls back to the bare name, so
   non-colliding structs are unaffected.

3. **All value/decode/default sites route through `struct-rust-name`.**
   Struct literals (both the body-walker and pure-expression paths), the
   ledger decoder turbofish, and default expressions now emit the
   disambiguated name instead of the raw `struct-name`.

4. **Stdlib structs are excluded from disambiguation.** `Maybe`,
   `MerkleTreePath*`, `ContractAddress`, etc. resolve through their runtime
   mapping / bare name and legitimately recur with different type arguments
   (`Maybe<Fr>` vs `Maybe<bool>`); recording them would fabricate bogus
   `Maybe_1` splits once body sites resolve through `struct-rust-name`. They
   are skipped when the tables are built.

### Files

- `compiler/rust-passes-types.ss` — `tstruct-fingerprint` keys on field
  name + type.
- `compiler/rust-passes-naming.ss` — `build-struct-rust-name-ht` returns
  `(node-ht . fp-ht)`; stdlib structs excluded.
- `compiler/rust-passes-helpers.ss` — `current-struct-rust-name-fp-ht`
  parameter; `struct-rust-name` fingerprint fallback.
- `compiler/rust-passes-emit.ss` — `struct-rust-name-of` helper; struct
  literal, decoder, and default sites routed through it.
- `compiler/rust-passes.ss` — installs both tables.

## Consequences

### Positive

- Colliding structs that differ only by field name are now emitted as
  separate Rust structs, and every value/decode/default site uses the
  matching disambiguated name. The class of `E0609` / `E0308` / `E0422`
  failures this could produce is closed at the root.
- Resolution is now **identity-independent**: any IR node — signature or
  body, however duplicated by module inlining — resolves to the same name by
  structure. This is the property the eq?-node table lacked.

### Neutral / guarded

- **Byte-parity preserved.** The `eq?`-node path is unchanged, so all
  signature/typedef/definition sites render exactly as before. Body sites
  for non-colliding structs resolve to the bare name (fp miss → bare), which
  equals the previous raw output. All 28 existing `codegen_regression`
  fixtures regenerate byte-identically; `digital-passport` is unaffected (no
  body construction of its colliding structs, definitions unchanged).

### Verification

- New fixture `examples/struct_collision_fixture.compact` +
  `tests-e2e-rust/contracts/struct-collision-fixture/`, registered in
  `codegen_regression`. Two modules export `struct Rec` with the same field
  type but different field names (the flat case), plus
  `struct Wrap { inner: Inner }` where each module's `Inner` differs (the
  nested case). Each circuit constructs its struct in the body and reads a
  field back. Against the pre-fix compiler this **fails to compile** (merged
  structs, wrong-named construction); against the fixed compiler it splits
  into `Rec`/`Rec_1`, `Wrap`/`Wrap_1`, `Inner`/`Inner_1`, constructs each
  correctly (`wrap_alpha → Wrap_1 { inner: Inner_1 {…} }`), and compiles.
- The fixture crate is a `tests-e2e-rust` dev-dependency, so
  `cargo build -p tests-e2e-rust --tests` (the CI build gate) actually
  compiles it — `codegen_regression` only byte-compares generated output, so
  without the dev-dep a compile regression in the disambiguated code would go
  unnoticed.

## Alternatives considered

- **Collect body-site nodes into the eq? table too.** Rejected: fragile —
  any un-walked emission path silently regresses to the bare name. The
  fingerprint fallback resolves *any* node by construction.
- **Prefix struct names by module (as the TS backend does).** Rejected:
  larger change to the emitted API surface and to byte-parity references;
  disambiguation by suffix is the minimal, TS-parity-preserving choice.

## Follow-ups

- ~~`digital-passport` is not yet registered in `codegen_regression` (no
  committed `.compact` source in this repo), so its generated `lib.rs` is
  guarded only by `cargo fmt`/`clippy`, not regen byte-parity.~~
  **Closed by removal rather than registration.** That fixture was dropped
  when the corpus was de-branded for upstreaming, so the gap it described
  is gone: every crate under `tests-e2e-rust/contracts/` now has a
  committed `.compact` source and is byte-parity gated, none rely on
  `fmt`/`clippy` alone. The colliding-struct shape it incidentally carried
  is covered directly by `examples/struct_collision_fixture.compact`.
- Enum-name collisions across module instantiations are not disambiguated
  (the tables are struct-only). No contract exercises this yet; the same
  fingerprint mechanism would extend to enums if needed.
