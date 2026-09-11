# rust-codegen/conditional-expressions Specification

## Purpose

The Rust code generation backend of the Compact compiler (`compactc --target rust`) accepts and correctly compiles conditional (ternary) expressions in every position and body route where the language permits them, matching the TypeScript backend's acceptance and the language specification's evaluation semantics.

## Requirements

### Requirement: Ternary expressions compile in all sub-expression positions

A contract that uses a conditional expression `c ? e1 : e2` anywhere a general expression is allowed — including as the right-hand side of a `const` binding, as an argument of `assert(...)`, as an interior operand of arithmetic, and in return/tail position — MUST compile successfully under `compactc --target rust` with no "unsupported Compact construct" error, in pure circuits, impure circuits, and constructors alike.

#### Scenario: const-binding ternary in a pure circuit
- **WHEN** a pure circuit contains `const x = cond ? a - 1 : a;`
- **THEN** `compactc --target rust --skip-zk` exits successfully and emits Rust for the binding

#### Scenario: ternary inside an assert argument
- **WHEN** a circuit contains `assert(isLeap ? day <= 29 : day <= 28, "...");`
- **THEN** compilation under `--target rust` succeeds

#### Scenario: ternary as an arithmetic operand
- **WHEN** a circuit contains an expression like `n - (flag ? 1 : 0)`
- **THEN** compilation under `--target rust` succeeds

#### Scenario: ternary in an impure circuit and in a constructor
- **WHEN** a ledger-writing circuit or a contract constructor initialises a value with a ternary
- **THEN** compilation under `--target rust` succeeds (no `circuit-body-emission` / `ctor-body-emission` / `expr-variant` failure)

#### Scenario: Field-joined ternary as a call argument
- **WHEN** a conditional with an integer-literal arm is passed as an argument to a circuit whose corresponding formal is `Field` — mixed-arm (`idf(flag ? x : 0)`) or both-literal (`idf(flag ? 1 : 0)`) — in a pure circuit, an impure circuit, or a constructor
- **THEN** the literal arm(s) render with the `Fr::from(<n>u64)` coercion derived from the callee's declared formal type, and the generated crate compiles under `cargo build`

#### Scenario: Field-joined ternary as a struct-literal member
- **WHEN** a struct with a `Field` member is initialised with a conditional carrying an integer-literal arm for that member
- **THEN** the member's declared type drives the same coercion and the generated crate compiles under `cargo build`

#### Scenario: unannotated both-literal const returned from a Field circuit
- **WHEN** a Field-returning circuit contains `const picked = flag ? 1 : 0; return picked;` (no type annotation on the const)
- **THEN** the let*-lifted tail renders with both arms coerced from the circuit's Field return type, and the generated crate compiles under `cargo build`

#### Scenario: unannotated no-literal Uint const returned from a Field circuit
- **WHEN** a Field-returning circuit contains `const picked = flag ? u : v; return picked;` with both `u` and `v` of type `Uint<N>` (no integer-literal arm)
- **THEN** the let*-lifted tail's `tfield←tunsigned` safe-cast wrapper renders as `Fr::from((<inner>) as <width>)` — the Rust width that losslessly holds the Uint range (`u64`, or `u128` when the range exceeds u64) — and the generated crate compiles under `cargo build`

#### Scenario: aggregate Field target filled from Uint values
- **WHEN** a value of aggregate type (`Vector<N, Uint<M>>`, nested included) flows into an aggregate `Field` slot — a `Vector<N, Field>` returned from a circuit, or a `Vector<N, Field>` ledger field written from Uint elements — through a tuple literal, a let*-lifted `const`, or a `default`
- **THEN** the element-wise `tvector<Field>←tvector<Uint>` safe-cast renders as `[Fr::from((<el>) as u64|u128), …]` (the aggregate value is bound to a temp and indexed when its elements are not syntactically visible), and the generated crate compiles and the `Vector<N, Field>` accessor reads back the same elements

### Requirement: Lazy branch evaluation is preserved

Generated Rust MUST evaluate a conditional expression exactly as the language specification requires: the condition is evaluated first and **only the selected branch** is evaluated. Branch-local trapping guards (e.g. the underflow assertion guarding unsigned subtraction) MUST be emitted inside the selected branch, so an untaken branch can never abort execution.

#### Scenario: untaken branch must not trap
- **WHEN** the executed circuit is `pick(c) { return c > 15 ? c - 10 : c; }` and it is invoked with `c = 9`
- **THEN** the subtraction `c - 10` (which would underflow) is not evaluated and the call succeeds returning `9`

#### Scenario: taken branch computes when selected
- **WHEN** the same circuit is invoked with `c = 20`
- **THEN** the taken branch's subtraction is evaluated and the call succeeds returning `10`

### Requirement: Typed branch values unify

Where both branches produce struct or enum values of a common type, the generated Rust MUST produce arms of unifiable types without compile errors in the generated crate (nested conditionals included).

#### Scenario: struct-valued branches
- **WHEN** a conditional's arms are two struct values of the same type, possibly nested inside another conditional
- **THEN** the generated crate compiles under `cargo build`
