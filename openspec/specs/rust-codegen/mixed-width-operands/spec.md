# rust-codegen/mixed-width-operands Specification

## Purpose

The Rust code generation backend of the Compact compiler (`compactc --target rust`) emits code that type-checks whenever the compiler's own type checker accepts the source — in particular for binary operations whose operands have different minimal Rust widths because the typer's value-range typing widened one side.

## Requirements

### Requirement: Mixed-width binary operands are widened where they meet

For every binary operator whose Rust rendering requires identical operand types — comparisons `<= >= < > == !=` and arithmetic `+ - *` — where the typer accepts operands whose minimal Rust widths differ, the emitted Rust MUST widen the narrower operand losslessly at the operand boundary (or render both operands at the operator's joined width) such that the generated crate compiles without type errors.

#### Scenario: range-widened product compared against a narrow binding
- **WHEN** a pure circuit contains `const y = m <= 2 ? y32 - 1 : y32; assert(q32 * 4 <= y, "...")` (product range `Uint<0..17179869181>` rendered `u64`, `y` rendered `u32`)
- **THEN** `compactc --target rust --skip-zk` exits 0 and the emitted comparison compiles under `cargo build` (the `u32` operand is cast to `u64`)

#### Scenario: narrow binding subtracted from a range-widened binding
- **WHEN** a circuit computes `let r = y - q32 * 4` with `y` narrow-ranged and the product range-widened, including the guarded-subtraction route
- **THEN** the emitted `wrapping_sub` receives same-width operands and the crate compiles

#### Scenario: the real dogfood contract compiles
- **WHEN** `cargo build -p compact-contract-digital-passport-credential` runs on the crate regenerated from `examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact`
- **THEN** it compiles with zero `E0308` errors (previously 13, all rooted in `assertCivilDateMatchesEpochDays` / the age predicate)

### Requirement: Range-driven widths are not narrowed by the fix

The widening fix MUST NOT narrow range-widened arithmetic back to the declared operand width: an expression whose value range exceeds a narrower Rust width keeps its wider rendering, and only operand *boundaries* gain casts.

#### Scenario: product of a Uint<32> field and 4 stays u64
- **WHEN** the emitter renders `q * 4` for `q: Uint<32>`
- **THEN** the multiplication renders at `u64` (value range up to 17179869181), exactly as before this fix

### Requirement: The existing fixture corpus stays byte-stable where no widths mix

Fixtures whose sources contain no mixed minimal-width operands MUST regenerate byte-identically after the fix; any fixture whose bytes do change MUST correspond to a source-level mixed-width site, inspected and categorized during regeneration.

#### Scenario: regen sweep after the fix
- **WHEN** `cargo test -p tests-e2e-rust rust_codegen_byte_parity` runs over the full FIXTURES table after the fix
- **THEN** every fixture regenerates byte-identically to its committed crate once updated fixtures are committed, and the regen diff shows changes only at mixed-width sites
