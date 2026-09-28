# Design

## Context

See proposal.md - Why. The shape of the defect comes from how the typechecker types an integer literal, which splits literals into three ranges:

```
 0              u128::MAX            max-unsigned (2^248-1)          max-field
 |--------------------|-----------------------|-------------------------------|
   Uint<0..N>            Uint<0..N>              Field only
   `N as Field`          `N as Field`            `N as Field`
   -> safe-cast          -> safe-cast            -> bare (quote N), no safe-cast
      Uint -> Field         Uint -> Field           (analysis-passes.ss ~L3126-3138)
```

- A literal up to `max-unsigned` is typed `Uint<0..N>`; `N as Field` then wraps it in `(safe-cast Field Uint<0..N> (quote N))`.
- A bare literal above `max-unsigned` is a located error ("use N as Field", `analysis-passes.ss` ~L2549-2554; pinned by `compiler/test.ss` ~L41913 and ~L41924).
- `N as Field` above `max-unsigned` is special-cased to a bare `(quote N)` typed `Field` (`analysis-passes.ss` ~L3126). That is the only way such a quote reaches the Rust emitter.

In the Rust emitter, positions whose type is set by their context (const initialiser, return, `Field` arithmetic, comparison, struct member, vector element) bind an expected type or call `field-literal-rust` directly. A call argument has no context of its own: `pure-call-arg-rust` (`rust-passes-emit.ss` ~L2728) and the walker's `arg-rust-clone-if-var` take the expected type only from the argument's own `safe-cast` (`expr-expected-type`). A literal above `max-unsigned` has no wrapper, so it reaches the integer branch of the `quote` clause in `expr-rust` (`rust-passes-emit.ss` ~L2100-2116) with no expected type and is printed as bare digits.

The existing "huge literal" coverage (`examples/literal_coercion_fixture.compact`, `callArgHugeFieldLiteral` et al.) uses 2^200, which is below `max-unsigned`, so it always takes the `safe-cast` path.

## Goals / Non-Goals

**Goals:**
- A single, provably safe decision point that renders a Field-only literal as `Fr` whatever position it reaches the emitter from.
- No change to emitted Rust for any program that builds today.

**Non-Goals:**
- Changing how call arguments obtain their expected type (#91).
- Supporting `Uint` values above `u128::MAX` in the Rust backend (see Risks).

## Decisions

### D1. Fall back in the `quote` clause, not at the call sites

When the integer branch of the `quote` clause has no expected type and `datum > (max-unsigned)`, render `(field-literal-rust datum)`. The existing branches (expected `Field`, expected `Uint`) are unchanged and take precedence.

- Every call-argument renderer (`pure-call-arg-rust`, `arg-rust-clone-if-var`, `ctor-expr-rust`'s fall-through), ternary arms and tuple elements end in this clause. Two independent reviews (gpt-6-sol, glm-5.3) traced these paths and found none that prints an integer quote without passing through it. The `.addi` VM path formats a `Uint` immediate, not a Rust literal, and the arithmetic width suffix is only appended to all-digit output, so it cannot touch `Fr::from_le_bytes(...)`.
- Alternative: render call arguments at the callee's declared type. That also fixes #90, but it changes every un-wrapped argument across several renderers and needs a snapshot audit (#91). The `quote` fallback remains useful as a backstop even after #91 lands.

### D2. Threshold is `max-unsigned`, not `u128::MAX`

- Above `max-unsigned` the literal is `Field` by construction (the typechecker admits it only through `N as Field`), so the fallback cannot mistype a value.
- `u128::MAX` (as #90 proposes) would also catch literals in `(u128::MAX, max-unsigned]`, which are admissible `Uint` values; silently rendering them as `Fr` would hide the backend's lack of wide-`Uint` support behind a type error elsewhere. Those literals reach call arguments with a `safe-cast`, so they are already rendered through the typed path and need no fallback.
- `max-unsigned` is exported by `langs.ss`; use it rather than a hard-coded constant.

### D3. Test the constant through group arithmetic, not `Field` arithmetic

`c = (r + 1) / 8` is defined modulo the Jubjub prime-subgroup order `r`, which differs from Compact's `Field` modulus (`max-field + 1`). `8 * c` is 1 mod `r` but not mod the `Field` modulus, so the check cannot be a `Field` equation. The fixture instead exposes `ecMul(ecMul(G, c as Field), 8)` for a subgroup point `G` (e.g. `ecMulGenerator(1)`), and the Rust test asserts it equals `G`. Exactness of the literal itself is asserted with little-endian byte equality (the style of the existing 2^200 test), and the test computes `(r + 1) / 8` with integer arithmetic to document where the constant comes from.

## Risks / Trade-offs

- [A future frontend change makes a quote above `max-unsigned` reach the emitter as a non-`Field`] → The rejection tests in `compiler/test.ss` pin the typechecker rule D2 relies on; they must keep passing.
- [The fallback masks a missing expected type that #91 should fix] → #91 tracks the general fix. Its acceptance requires the #90 regression tests to pass without relying on this fallback.
- [Known limitation, unchanged] A `Uint` literal above `u128::MAX` in a `Uint`-typed position still renders with a `u128` suffix and fails `cargo build`; a non-literal `Uint` range above `u128::MAX` flowing into `Field` is refused as specified. `docs/rust-backend-limitations.md` ("`Uint` wider than `u128` into a `Field`") documents the latter. This change neither widens nor narrows that surface.
- [Adding pure circuits to the fixture changes its TS reference capture] → Pure circuits are not contract operations, so the captured state is not expected to change; re-run the capture and commit it only if it does.
