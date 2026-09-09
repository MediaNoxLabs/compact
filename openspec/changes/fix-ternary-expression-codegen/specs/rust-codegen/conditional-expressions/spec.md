# Delta Spec: rust-codegen/conditional-expressions

## Purpose

The Rust code generation backend of the Compact compiler (`compactc --target rust`) accepts and correctly compiles conditional (ternary) expressions in every position and body route where the language permits them, matching the TypeScript backend's acceptance and the language specification's evaluation semantics.

## ADDED Requirements

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

### Requirement: Lazy branch evaluation is preserved

Generated Rust MUST evaluate a conditional expression exactly as the language specification requires: the condition is evaluated first and **only the selected branch** is evaluated. Branch-local trapping guards (e.g. the underflow assertion guarding unsigned subtraction) MUST be emitted inside the selected branch, so an untaken branch can never abort execution.

#### Scenario: untaken branch must not trap
- **WHEN** the executed circuit is `pick(c) { return c > 5 ? c - 10 : c; }` and it is invoked with `c = 9`
- **THEN** the subtraction `c - 10` (which would underflow) is not evaluated and the call succeeds returning `9`

#### Scenario: taken branch traps as specified
- **WHEN** the same circuit is invoked with `c = 6`
- **THEN** the taken branch's subtraction succeeds and the call returns a value computed from it

### Requirement: Typed branch values unify

Where both branches produce struct or enum values of a common type, the generated Rust MUST produce arms of unifiable types without compile errors in the generated crate (nested conditionals included).

#### Scenario: struct-valued branches
- **WHEN** a conditional's arms are two struct values of the same type, possibly nested inside another conditional
- **THEN** the generated crate compiles under `cargo build`
