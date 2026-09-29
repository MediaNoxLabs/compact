## ADDED Requirements

### Requirement: Field-only literals materialise without an expected type

An integer literal larger than the largest representable `Uint` (`max-unsigned`, 2^248 - 1) can only enter a program as an explicit `N as Field`, so it is `Field` by construction. The Rust backend MUST render such a literal as an `Fr` value holding exactly that integer, at every position where it can appear, including positions that supply no expected type of their own (a stdlib native argument, a user circuit argument). It MUST NOT emit the literal as a bare Rust integer. The threshold MUST be `max-unsigned`, not `u128::MAX`: a literal in `(u128::MAX, max-unsigned]` is admissible as a `Uint` and keeps its existing typed rendering.

#### Scenario: literal above max-unsigned as a native argument
- **WHEN** a circuit calls `ecMul(p, 819310549611346726241370945440405716213240158234039660170669895299022906775 as Field)`
- **THEN** `compactc --target rust` emits an `Fr`-typed argument (`Fr::from_le_bytes(...)`), the generated crate builds, and the argument's little-endian bytes equal the literal's

#### Scenario: literal above max-unsigned as a user circuit argument
- **WHEN** a pure circuit `id(x: Field): Field` is called as `id(N as Field)` with `N > max-unsigned`
- **THEN** the emitted argument is an `Fr` value equal to `N`, and the generated crate builds

#### Scenario: literal above max-unsigned in typed positions
- **WHEN** a literal `N > max-unsigned` written `N as Field` appears as a `const` initialiser, a return value, a `Field` arithmetic operand, or an equality operand
- **THEN** each renders as the same `Fr` value equal to `N`

#### Scenario: boundary literal
- **WHEN** the literal `max-unsigned + 1` (2^248) is passed as `2^248 as Field` to a call argument
- **THEN** it renders as an `Fr` value equal to 2^248 and the generated crate builds

#### Scenario: Jubjub subgroup-check constant is exact
- **WHEN** the `(r + 1) / 8` constant, for the Jubjub prime-subgroup order `r`, is used as `ecMul(ecMul(G, c as Field), 8)` on a point `G` of the prime-order subgroup
- **THEN** the result equals `G`

#### Scenario: literal below max-unsigned is unaffected
- **WHEN** a literal `N` with `u128::MAX < N <= max-unsigned` is written `N as Field` at any of the positions above
- **THEN** its emitted Rust is byte-identical to the output before this change

#### Scenario: bare oversized literal is still rejected
- **WHEN** a literal `N > max-unsigned` appears without `as Field`
- **THEN** `compactc` rejects the program with a located error advising `N as Field`
