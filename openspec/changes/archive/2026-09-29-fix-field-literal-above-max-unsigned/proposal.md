# Proposal

## Why

The Rust backend emits a bare integer for a `Field` literal larger than the largest representable `Uint` (`max-unsigned`, 2^248 - 1) when that literal is a call argument, e.g. `ecMul(p, 8193...775 as Field)`. `compactc` exits 0 but `cargo build` fails with "integer literal is too large" (#90). This blocks Rust codegen for `@midnight-ntwrk/credential-compact@0.2.0`, whose Jubjub subgroup check (a security-relevant torsion-point rejection that consumers must not strip) passes exactly such a literal, `(r + 1) / 8`, to `ecMul`.

## What Changes

- A `Field` literal above `max-unsigned` renders as an `Fr` value (`Fr::from_le_bytes(...)`) even at a position that supplies no expected type — today, a call argument (stdlib native or user pure circuit) whose literal the typechecker left without a `safe-cast`.
- The threshold is `max-unsigned`, not `u128::MAX` as #90 proposes: the typechecker admits literals in `(u128::MAX, max-unsigned]` as `Uint`, and only admits a larger literal through an explicit `as Field`, so above `max-unsigned` the literal is `Field` by construction.
- Regression coverage for literals above `max-unsigned` in every position (native argument, user-circuit argument, const, return, arithmetic, comparison), the `max-unsigned + 1` boundary, and the real `(r + 1) / 8` constant checked through group arithmetic. The existing "huge literal" tests use 2^200, which is below `max-unsigned`, so they never exercised this path.

Non-goals:

- Rendering every call argument at the callee's declared parameter type (the general fix for this bug class, also behind #21). Tracked separately in #91.
- Re-vendoring the `digital-passport-credential` dogfood fixture to credential-compact 0.2.0. A separate dogfooding change; this change only verifies the 0.2.0 build by hand once.
- `Uint` literals above `u128::MAX` in `Uint`-typed positions. A pre-existing Rust-width limitation, unchanged here.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `rust-codegen/type-directed-coercion`: adds a requirement that a `Field`-only literal (above `max-unsigned`) materialises as `Fr` wherever it appears, including positions that carry no expected type.

## Impact

- `compiler/rust-passes-emit.ss`: the integer branch of the `quote` clause in `expr-rust`.
- `examples/literal_coercion_fixture.compact`, the `tests-e2e-rust/contracts/literal-coercion-fixture` crate, `tests-e2e-rust/tests/literal_coercion.rs`, and the compiler rejection tests.
- No change to emitted Rust for any program that builds today: the new branch fires only on output that currently fails `cargo build`.
- Unblocks Rust codegen for credential-compact 0.2.0 and the `midnight-vc-passport` migration.
