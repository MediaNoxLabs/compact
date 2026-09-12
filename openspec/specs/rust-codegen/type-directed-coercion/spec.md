# rust-codegen/type-directed-coercion Specification

## Purpose

The Rust code generation backend of the Compact compiler renders every expression against the Compact type expected at its use position, materialising the typechecker's `safe-cast` decisions losslessly at a single decision point, so that a value's Rust type and width are correct wherever the language lets it appear.

## Requirements

### Requirement: Expressions are rendered against their expected type

The Rust backend MUST render an expression using the Compact type required by its use position — the declared type of a `const` binding, the declared return type, the operand type of a comparison, the operand type of a `Field` binary `+`/`-`/`*`, the declared formal type of a call argument, the declared member type of a struct literal, the element type of a vector/array or native argument, and the destination field type of a ledger write. The typechecker's `(safe-cast <target> <src> expr)` wrapper MUST be materialised from `<target>`, not discarded.

#### Scenario: Field-typed literal in a const binding
- **WHEN** a circuit contains `const x: Field = 0;`
- **THEN** `compactc --target rust` emits a `Fr`-typed value (e.g. `Fr::from(0u64)`), not a bare integer

#### Scenario: literal in a struct member
- **WHEN** a struct with a `Field` member is constructed with a bare literal
- **THEN** the emitted Rust compiles with the member coerced to the member's declared type

#### Scenario: literal in a call or native argument
- **WHEN** a bare literal or a both-literal expression is passed to a pure circuit, a witness, `some<T>`, or a native
- **THEN** the argument is coerced from the callee's declared formal type and the emitted crate compiles

#### Scenario: vector element feeding a native
- **WHEN** a bare literal is an element of an array passed to a native such as `persistentHash`
- **THEN** the element is coerced from the enclosing vector's element type

### Requirement: Uint-to-Field coercion is lossless

Wherever a `Uint`-ranged value flows into a `Field` position, the backend MUST materialise the coercion as a lossless zero-extension: `u64` for ranges up to `u64::MAX`, `u128` for larger ranges (the field modulus is far above `2^128`, so `From<u128> for Fr` does not reduce). A range above `u128::MAX` MUST be refused loudly. Aggregate targets (e.g. `Vector<N, Field>`) MUST be coerced element-wise, including nested aggregates.

#### Scenario: scalar Uint into Field
- **WHEN** a `Uint<8>` value or literal is used in a `Field` position
- **THEN** the emitted Rust is `Fr::from((x) as u64)` and evaluates to the same field element

#### Scenario: wide Uint rung
- **WHEN** a `Uint<128>`-ranged value flows into a `Field` position
- **THEN** the emitted Rust is `Fr::from((x) as u128)` and does not reduce modulo the field

#### Scenario: aggregate target
- **WHEN** a `Vector<N, Uint<…>>` value flows into a `Vector<N, Field>` slot
- **THEN** every element is coerced element-wise and the cell commits Field-aligned bytes, not Bytes-aligned bytes

#### Scenario: unrenderable range refuses
- **WHEN** a range above `u128::MAX` would need to flow into a `Field` position
- **THEN** `compactc --target rust` refuses with a located error and writes no `lib.rs`

### Requirement: Mixed minimal-width operands widen losslessly

Where the typechecker wraps one comparison/equality operand in `(safe-cast <wider> <narrower> …)` because their ranges differ, the backend MUST materialise a lossless zero-extending cast on the narrower operand (and leave the wider side at its own minimal width). Branch arms of a conditional whose arms join to a wider type MUST be widened the same way. Two ranges that share the same Rust width MUST NOT gain a cast. UNSIGNED binary `+ - *` operands MUST keep their existing `mbits` width normalisation and MUST NOT be pushed an expected type (materialising the operand's `safe-cast` too would double-cast). `Field` binary `+ - *` operands have no width normalisation and MUST instead be materialised against each operand's coerced type, so an untyped integer operand cannot reach `Fr` arithmetic.

#### Scenario: mixed-width comparison compiles
- **WHEN** a `Uint<32>`-ranged expression that widened to `u64` (e.g. `q * 4`) is compared with a `Uint<32>`-ranged value
- **THEN** the narrower operand is emitted as `(operand) as <wider>` and the generated crate compiles

#### Scenario: same Rust width gains no cast
- **WHEN** two ranges that both render at `u16` (e.g. `Uint<0..256>` and `Uint<0..65535>`) are compared
- **THEN** neither operand gains a cast and the emitted bytes are unchanged

#### Scenario: width is pinned by execution
- **WHEN** the mixed-width fixture is executed with values whose product crosses `2^32`
- **THEN** the result is correct, so a truncating or wrongly-directed cast cannot pass by merely compiling

#### Scenario: Field arithmetic operand is materialised
- **WHEN** a `Field` circuit evaluates `x + 1`, `1 + x`, `x - 1`, `x * 2`, or `x + u` where `u: Uint<8>`
- **THEN** each operand is materialised as an `Fr` (`(x) + (Fr::from(1u64))`, `(x) + (Fr::from((u) as u64))`), a literal above `u64::MAX` renders via `field-literal-rust` (`Fr::from_le_bytes`), and the generated crate compiles

### Requirement: No sentinel reaches emitted Rust

The backend MUST NOT emit a sentinel or placeholder (e.g. `#f`, a `/* TODO` marker, or an unrendered sentinel) into generated Rust. Any expression the renderer cannot lower MUST raise a located refusal instead.

#### Scenario: unrenderable expression refuses
- **WHEN** an expression reaches a position the renderer cannot lower
- **THEN** compilation exits non-zero with a source location and emits no partial crate

#### Scenario: no sentinel splice
- **WHEN** any fixture or probe is compiled
- **THEN** the emitted Rust contains no sentinel splice, verified by a post-emit guard

### Requirement: TypeScript feature parity

Any program that `compactc --target ts` accepts MUST either compile under `compactc --target rust` or be refused with a documented, tracked limitation. A refusal at a position the TS target accepts is a defect.

#### Scenario: TS-accepted positions do not refuse
- **WHEN** a position is exercised that the TS target compiles
- **THEN** the Rust target compiles it, or `docs/rust-backend-limitations.md` records the refusal as a known limitation

### Requirement: Neutral output is byte-stable

For expressions whose emitted Rust was already correct and unambiguous, this rendering MUST be byte-identical to the previous emitter.

#### Scenario: neutral fixtures unchanged
- **WHEN** the whole `FIXTURES` corpus is regenerated after the change
- **THEN** every previously-correct fixture is byte-identical, and any changed fixture corresponds to a position that previously emitted wrong or ambiguous Rust
