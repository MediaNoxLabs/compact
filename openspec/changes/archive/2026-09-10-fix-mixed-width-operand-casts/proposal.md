# Proposal: fix-mixed-width-operand-casts

## Why

The dogfood fixture's registration gate (`add-digital-passport-dogfood-fixture`, task 3.2) is the first thing to ever `cargo build` the emitted digital-passport crate, and it found 13 `E0308` type errors: the rust backend emits Rust that does not type-check when a range-widened expression meets a narrower one inside a binary operation.

Mechanism (pinned with a 3-line repro, see design):

- Compact's typer tracks **value ranges**, not just bit widths: `q * 4` with `q: Uint<32>` types as `Uint<0..17179869181>`, and the emitter deliberately renders such arithmetic at the minimal Rust width covering the range (`u64`) — correct, and must not be "fixed" by narrowing.
- The typer accepts binary operations across operands of different ranges (e.g. `q * 4 <= yearAdjusted` where `yearAdjusted` is `Uint<32>`-ranged).
- The rust emitter renders each operand at **its own** minimal width (`u64` vs `u32`) and inserts **no widening cast** where they meet, so the generated crate fails to compile — 13 errors, all rooted in the `assertCivilDateMatchesEpochDays` / age-predicate helpers of the dogfood contract.

`fix-ternary-expression-codegen` verified only *emission* (compactc exit 0, zero `unimplemented!`); it could not see this because nothing cargo-compiled the output. This is the third real rust-backend gap surfaced by the dogfood program and it hard-blocks the dogfood change's task 3.2/3.3.

## What Changes

- Emitter fix in `compiler/rust-passes-emit.ss` (the shared `expr-rust` renderer): when the two operands of a binary operation have different minimal Rust widths, widen the narrower operand losslessly (`as <wider>`) at the operand boundary — for every operator where Rust requires identical operand types (comparisons `<= >= < > == !=` and arithmetic `+ - *` routes). Range-driven widths themselves are untouched.
- New fixture `examples/mixed_width_operand_fixture.compact` (Apache header): per-operator mixed-width circuits, including the exact dogfood shape (u32-ranged ternary joined with a u64-ranged product inside an assert chain), registered per the full recipe (workspace member, dev-dep, FIXTURES row).
- Full FIXTURES corpus regeneration in the same commit (AGENT.md discipline); uniform-width fixtures are expected byte-stable, and any fixture that does mix widths gets its diff inspected and categorized during regen.
- Executing parity test `tests-e2e-rust/tests/mixed_width_operand_fixture.rs` + TS reference capture, pinning that the widening cast zero-extends correctly at values crossing the 2³² boundary (an emitting-only fix could cast the wrong way and still "compile").
- Version bump 0.31.117 → 0.31.118 with embed-site sweep + CHANGELOG (changelog-check requires it; precedent: the ternary fix carried its own bump).

Impact on the paused change: `add-digital-passport-dogfood-fixture` resumes after this lands — its 3.1 regeneration is mechanical, and its task 6.3 version bump renumbers to 0.31.118 → 0.31.119.

## Capabilities

### New Capabilities

- `rust-codegen/mixed-width-operands`: the rust backend emits widening casts where range-based typing legalizes mixed minimal-width operands, so every expression the typer accepts produces Rust that compiles.

### Modified Capabilities

(none — `rust-codegen/conditional-expressions` is untouched; this is a sibling requirement area)

## Impact

- `compiler/rust-passes-emit.ss` (and only there, unless the fix reveals the width ladder lives elsewhere).
- New fixture + crate + registrations; regenerated `lib.rs` for any fixture whose sources mix widths; `Cargo.lock`.
- `tests-e2e-rust/fixtures/capture-mixed-width-operand-fixture.mjs` + committed JSON; `tests-e2e-rust/tests/mixed_width_operand_fixture.rs`.
- Version triple 0.31.117 → 0.31.118 across embed sites; CHANGELOG.
- Branch `feature/fix-mixed-width-operand-casts` cut from `feature/add-digital-passport-dogfood-fixture` (inherits the vendored dogfood + its staged registrations, so the dogfood build gate is this change's acceptance test).
