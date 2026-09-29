# Proposal

## Why

The typechecker leaves a call argument unwrapped when its type is the same as the callee's formal type (`maybe-safecast`), but the Rust emitter takes a call argument's expected type only from that `safe-cast` wrapper. Such an argument renders with no expected type, so a `Vector<2, Field>` argument `[0 as Field, 1 as Field]` is emitted as `[0, 1]` for pure circuits and witnesses (#21, `cargo build` fails), as `AlignedValue::from(0)` in `persistentHash` / `transientHash` (fails), and as `persistent_commit(&0, …)`. The last one builds: Rust infers `i32`, which implements `BinaryHashRepr`, so the commitment silently hashes four zero bytes instead of the `Fr` encoding and differs from the TS target. This violates the existing `rust-codegen/type-directed-coercion` requirement that a call argument be rendered at "the declared formal type" (#91).

## What Changes

- Every call argument is rendered at the callee's declared formal type: natives (including the special-cased `persistentCommit`, `persistentHash`, `transientHash`), user pure circuits, witnesses, impure/exported circuits, and impure circuits inlined into `if`/`assert` conditions. An argument that already has a `safe-cast` wrapper renders exactly as before.
- The formal types come from the monomorphised native, witness and circuit declarations already in the IR, so generic natives use their instantiated types.
- As a consequence, a tuple-typed value passed to a `Vector` parameter (or the reverse) is bridged at call arguments too (the compact#83 shape).
- All call sites render arguments through one shared entry point, and a test enforces that.
- The #90 `quote`-clause fallback for literals above `max-unsigned` stays as a backstop. The #90 regression fixtures are shown, once, to regenerate byte-identically without it.
- Regression coverage in a new fixture with TS state parity, which pins the `persistentCommit` value, not just a successful build.
- Compiler version bump and `CHANGELOG.md` entry.

Closes #91 and #21, and the call-argument case of compact#83.

Non-goals:

- Rendering a `safe-cast` at its own target when no expected type is in scope (a leaf-level fix for positions that are not calls). Only call arguments change.
- Ledger ADT operation arguments and cell-write values, which share a helper with call arguments but are not calls. They are probed during implementation, and a follow-up issue is filed if they are broken.
- Removing the #90 fallback, or adding a test-mode switch for it.
- `Uint` values above `u128::MAX` (pre-existing limitation, unchanged).

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `rust-codegen/type-directed-coercion`: the requirement "Expressions are rendered against their expected type" gains scenarios pinning same-type call arguments (no `safe-cast`) at each call kind, arguments of an inlined condition call, `persistentCommit` / hash natives hashing the declared type, and the tuple/vector bridge at a call argument.

## Impact

- `compiler/rust-passes-emit.ss` (`call-rust`, `pure-call-arg-rust`, `native-vector-atoms`, table setup), `compiler/rust-passes-walker.ss` (`ctor-call-rust`, flat-body walker, hoisted calls, `inline-circuit-call`), `compiler/rust-passes-streaming.ss` (streaming walker call sites), `compiler/rust-passes-helpers.ss` / `rust-passes-prelude.ss` (formal-type table).
- New `examples/call_arg_declared_type_fixture.compact`, its generated crate under `tests-e2e-rust/contracts/`, a TS capture script and state file under `tests-e2e-rust/fixtures/`, a parity test, and a source-level guard test. Registration in `FIXTURES`, `tests-e2e-rust/Cargo.toml` and the workspace `Cargo.toml`.
- Emitted Rust changes only where output was previously broken; previously correct fixtures stay byte-identical.
- `CHANGELOG.md`, `compiler/compiler-version.ss`, `flake.nix`, `doc/ledger-adt.mdx` (version 0.31.122).
