## MODIFIED Requirements

### Requirement: Expressions are rendered against their expected type

The Rust backend MUST render an expression using the Compact type required by its use position — the declared type of a `const` binding, the declared return type, the operand type of a comparison, the operand type of a `Field` binary `+`/`-`/`*`, the declared formal type of a call argument, the declared member type of a struct literal, the element type of a vector/array or native argument, and the destination field type of a ledger write. The typechecker's `(safe-cast <target> <src> expr)` wrapper MUST be materialised from `<target>`, not discarded. A call argument MUST be rendered at the callee's declared formal type even when the typechecker inserts no `safe-cast` because the argument's type is already the formal type. This applies to every kind of call: a native (including natives whose Rust signature differs from the Compact one), a user pure circuit, a witness, an impure or exported circuit, and an impure circuit inlined into a condition. For a generic native or circuit the declared formal type is the instantiated one.

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

#### Scenario: same-type aggregate argument at every call kind
- **WHEN** `[0 as Field, 1 as Field]` is passed to a `Vector<2, Field>` parameter of a pure circuit, a witness, an impure circuit, an impure circuit inlined into an `if` or `assert` condition, and `persistentHash` / `transientHash`
- **THEN** every element is emitted as an `Fr` value, the generated crate builds, and the ledger state after each call equals the `compactc --target ts` reference

#### Scenario: persistentCommit hashes the declared type
- **WHEN** `persistentCommit<Field>(v, opening)` is called with `v` written as a literal `N as Field` (for `N` both at or below and above `max-unsigned`)
- **THEN** the value is hashed as a `Field`, and the resulting commitment equals the one the TS target produces for the same inputs

#### Scenario: tuple/vector bridge at a call argument
- **WHEN** a value declared as a tuple is passed to a parameter declared as a `Vector` with the same element types, or the reverse
- **THEN** the argument is converted to the parameter's Rust type and the generated crate builds

#### Scenario: argument that already renders correctly
- **WHEN** a call argument either carries a `safe-cast` wrapper or has a Rust type fixed by its own declaration (a typed local or formal)
- **THEN** its emitted Rust is byte-identical to the output before this change

#### Scenario: Field-only literal argument without the fallback
- **WHEN** a literal above `max-unsigned`, written `N as Field`, is passed as a call argument
- **THEN** it renders as an `Fr` value through the callee's declared formal type, so the output does not depend on the Field-only-literal fallback
