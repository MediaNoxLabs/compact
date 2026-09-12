# Delta Spec: rust-codegen/conditional-expressions

## Purpose

The Rust code generation backend of the Compact compiler (`compactc --target rust`) accepts and correctly compiles conditional (ternary) expressions in every position and body route where the language permits them, matching the TypeScript backend's acceptance and the language specification's evaluation semantics.

## ADDED Requirements

### Requirement: Ternary expressions compile in all sub-expression positions

A contract that uses a conditional expression `c ? e1 : e2` anywhere a general expression is allowed MUST compile successfully under `compactc --target rust` with no "unsupported Compact construct" error, in pure circuits, impure circuits (including the streaming route), and constructors alike.

#### Scenario: const-binding ternary in a pure circuit
- **WHEN** a pure circuit contains `const x = cond ? a - 1 : a;`
- **THEN** `compactc --target rust --skip-zk` exits successfully and emits Rust for the binding

#### Scenario: ternary inside an assert argument
- **WHEN** a circuit contains `assert(isLeap ? day <= 29 : day <= 28, "...");`
- **THEN** compilation under `--target rust` succeeds

#### Scenario: ternary as an arithmetic or comparison operand
- **WHEN** a circuit contains an expression like `n - (flag ? 1 : 0)` or `x == (flag ? 1 : 0)`
- **THEN** compilation under `--target rust` succeeds

#### Scenario: ternary in an impure circuit, streaming route, and constructor
- **WHEN** a ledger-writing circuit (including a body that forces the streaming route) or a contract constructor initialises a value with a ternary
- **THEN** compilation under `--target rust` succeeds (no `circuit-body-emission` / `ctor-body-emission` / `expr-variant` failure)

#### Scenario: ternary as a call argument, struct member, or vector element
- **WHEN** a ternary is an argument to a pure circuit, witness, `some<T>`, or native, or a struct-literal member or array element
- **THEN** compilation under `--target rust` succeeds

### Requirement: Lazy branch evaluation is preserved

Generated Rust MUST evaluate a conditional expression exactly as the language specification requires: the condition is evaluated first and **only the selected branch** is evaluated. Branch-local trapping guards (e.g. the underflow assertion guarding unsigned subtraction) MUST be emitted inside the selected branch, so an untaken branch can never abort execution.

#### Scenario: untaken branch must not trap
- **WHEN** the executed circuit is `pick(c) { return c > 5 ? c - 10 : c; }` and it is invoked with `c = 9`
- **THEN** the subtraction `c - 10` (which would underflow) is not evaluated and the call succeeds returning `9`

#### Scenario: taken branch traps as specified
- **WHEN** the same circuit is invoked with `c = 6`
- **THEN** the taken branch's subtraction succeeds and the call returns a value computed from it

### Requirement: Arm values are typed by the use position

Each arm of a conditional MUST be rendered at the Compact type expected at the conditional's use position, so that branch values whose Compact types unify also produce unifiable Rust. This requirement is satisfied by routing arms through `rust-codegen/type-directed-coercion`; this change MUST NOT add arm-specific coercion logic.

#### Scenario: struct-valued branches
- **WHEN** a conditional's arms are two struct values of the same type, possibly nested inside another conditional
- **THEN** the generated crate compiles under `cargo build`

#### Scenario: literal and Uint arms in a Field position
- **WHEN** a conditional with a literal arm or a Uint-ranged arm is used where a Field is expected
- **THEN** each arm is coerced losslessly as `rust-codegen/type-directed-coercion` requires, and the generated crate compiles

### Requirement: Ledger writes preserve state-byte parity

Where a conditional's value is written to a ledger cell, the generated Rust MUST commit the same serialized state bytes as the TypeScript backend (not merely the same decoded value).

#### Scenario: conditional literal into a wide field
- **WHEN** `const p = flag ? 10 : 20;` is written to a `Uint<64>` ledger field
- **THEN** the committed cell is 8-byte aligned, byte-equal to the TS reference, and a decoded-value-only check would not be sufficient to prove it

### Requirement: Coverage is exhaustive over reachable positions

Every combination of body route, sub-expression position, and branch value shape that the language makes reachable MUST have an executable probe. Any combination not supported MUST be refused loudly and recorded in `docs/rust-backend-limitations.md`; it MUST NOT be silently omitted or emit a sentinel.

#### Scenario: matrix has no blank cells
- **WHEN** the coverage matrix in the change's tasks is reviewed
- **THEN** every reachable cell names an executable probe, and every unsupported cell names a documented refusal
