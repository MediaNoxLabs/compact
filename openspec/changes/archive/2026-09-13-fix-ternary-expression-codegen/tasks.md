# Tasks: fix-ternary-expression-codegen

Prerequisite: `type-directed-expression-coercion` merged. All work inside `nix develop`; `nix build .#compactc` before any byte-parity test.

## 1. Compiler fix (one clause, no per-position logic)

- [x] 1.1 Add the `(if ,src ,c ,e1 ,e2)` clause to `expr-rust` in `compiler/rust-passes-emit.ss`, mirroring the TS clause at `typescript-passes.ss:2850`: render `if <cond> { <arm1> } else { <arm2> }` with each arm via `expr-rust-typed` at the expected type supplied by the use position; branch `seq` guard blocks render via the existing `seq` clause **inside the arm**. Verify: a minimal `const x = c <= 2 ? c - 1 : c;` pure circuit compiles under `result/bin/compactc --target rust --skip-zk`.
- [x] 1.2 Add the recursive `(if ...)` arm to `expr-supported?` in `compiler/rust-passes-walker.ss` (before `[else #f]`). Verify: a ternary in a ledger-writing circuit and in a constructor both compile.
- [x] 1.3 Confirm **no** ternary-specific coercion was added: `git diff` touches only the clause and the gate arm (no literal/width logic). Verify: grep the diff for coercion helpers returns nothing new.
- [x] 1.4 Confirm laziness in the emitted bytes: the underflow guard for a branch subtraction appears **inside** the arm, never hoisted. Verify: read the generated Rust for 1.1.

## 2. Coverage matrix (every reachable cell has a probe)

- [x] 2.1 Author probes for **routes × positions**: {pure, impure-walker, impure-streaming, constructor} × {const RHS annotated, const RHS unannotated/seq-lifted, return tail (pure stmt-lifted / I3b-4 / A19 chain), assert arg, arith operand, comparison operand, call arg (pure/witness/ctor), struct member, vector element, native arg, nested if, ledger cell write, inline write value}. Verify: a table in the PR/notes shows each cell checked or explicitly refused (with a `docs/rust-backend-limitations.md` link).
- [x] 2.2 Author probes for **value shapes** in each applicable position: {both-literal, mixed literal/expr, Uint arm into Field, differing Uint widths, Field-typed arms, struct-valued (non-Copy), enum-valued, literal > `i32::MAX`, literal > `u64::MAX` (Uint<128>), branch-local underflow guard}. Verify: each shape has an executing assertion.
- [x] 2.3 Author `examples/ternary_cond_fixture.compact` (Apache header) collecting the probes; verify `compactc --target ts --skip-zk` compiles it.
- [x] 2.4 Generate the crate; register it (root workspace member, `tests-e2e-rust` **dev-dependency**, `FIXTURES` row). Verify: `cargo build -p tests-e2e-rust --tests --locked` compiles it.
- [x] 2.5 Write `tests-e2e-rust/tests/ternary_cond_fixture.rs` with an executing assertion per probe: laziness both ways (`pick(3)` → `Ok(3)` no trap; `pick(12)` → `Ok(2)`), each circuit's pass and `Err(AssertionFailed)` sides, and the constructor/impure variants. Verify: `cargo test -p tests-e2e-rust ternary_cond` green.
- [x] 2.6 Add TS reference captures (`fixtures/capture-ternary-cond-fixture.mjs` → JSON) and a **serialized state-byte** parity assertion for every ledger-write cell. Verify: parity assertions pass and would fail if the destination width were wrong.

### Route × position coverage matrix (task 2.1)

Every cell names the `examples/ternary_cond_fixture.compact` probe that
exercises it (the executing assertion lives in
`tests-e2e-rust/tests/ternary_cond_fixture.rs`), or explains why it is
refused / not applicable. No cell is blank. The value shapes of task 2.2
are probes in the pure row: both-literal (`constAnnotatedBothLiteral`),
mixed literal/expr (`returnTailMixed`), Uint arm into Field
(`uintArmIntoField`), differing Uint widths (`differingUintWidths`),
Field-typed arms (`fieldTypedArms`), struct-valued non-`Copy`
(`structValuedArms` — generated `Box` has no `Copy` derive), enum-valued
(`enumValued`), literal > `i32::MAX` (`literalAboveI32`), literal >
`u64::MAX` (`literalAboveU64`), branch-local underflow guard
(`constUnannotatedSeqLifted` / `branch_local_guard_is_lazy`).

| Route \\ Position | const-ann | const-unann / seq-lifted | return tail | assert arg | arith operand | comparison operand | call arg (pure/witness/ctor) | struct member | vector element | native arg | nested if | ledger cell write | inline write value |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **pure** | `constAnnotatedBothLiteral` | `constUnannotatedSeqLifted` | `returnTailMixed`, `returnTailNested` | `assertArg` | `arithOperand` | `cmpOperand` | `callArgPure`, `callArgCtor` | `structMember`, `structValuedArms` | `vectorElement` | `nativeArg` | `returnTailNested` | n/a ᵃ | n/a ᵃ |
| **impure-walker** | `walkerConstAnnotated` | ✗ ᵇ | `pure_circuits::walkerReturnTail` ᶜ | ✗ ᵇ (ordering); `=` via ᵈ | ✗ ᵇ | `walkerCompareEq` | `walkerCallPure`, `walkerCallCtor` ᶜ, `witnessArg` | `walkerStructMember` | `walkerVectorElement` | `walkerNativeArg` | `walkerNestedIf` | `walkerWrite` | `walkerInlineWrite` |
| **impure-streaming** | `streamConstAnnotated` | ✗ ᵇ | ✗ ᵉ | `streamAssertEq` (equality); ✗ ᵇ (ordering) | ✗ ᵇ | `streamCompareEq` | `streamCallPure`, `streamCallWitness` | `streamStructMember` | `streamVectorElement` | `streamNativeArg` | `streamNestedIf` | `streamWrite`, `streamIncrement` | `streamIncrement` |
| **constructor** | `a: Uint<8> = c ? 1 : 2` | `y = c ? x : 0` ᶠ | n/a ᵍ | `assert(c ? (x == x) : (x == x), …)` | `y + b.f + (z as Field) + …` | `cmp = (0 == (c ? 1 : 0))` | `r = idf(c ? 7 : 8)` ʰ | `b = Box { f: c ? 1 : 2 }` | `vecCell = [c ? 1 : 2, c ? 3 : 4]` | `p = hashToCurve<Field>(c ? 3 : 4)` | `z = c ? (d ? 1 : 2) : (d ? 3 : 4)` | `fieldCell`, `wideCell` | `walkerInlineWrite` ᶜ (no ctor probe) |

- ᵃ **n/a** — a `pure circuit` performs no ledger operation, so a ledger-cell write / inline write cannot occur in it.
- ᵇ **refused** `circuit-body-emission` — the impure body walker admits no inline arithmetic (including the trapping-subtraction guard the typer inserts for `a - 1`) and no ordering comparison; the offending *shape*, not the `? :`, is what the gate rejects. See `docs/rust-backend-limitations.md` → “Impure bodies past the walker's expression shapes”. Equality (`==`, `!=`) is admitted.
- ᶜ **checked (shared clause)** — a side-effect-free `export circuit` lowers to the pure route (`pure_circuits::walkerReturnTail` / `walkerCallCtor`); the same `expr-rust` clause renders the position, so it is exercised by that probe. `inline write value` is not probed inside the constructor; it is probed by `walkerInlineWrite` / `streamIncrement` and would use the same clause.
- ᵈ an impure `assert` whose argument is an equality ternary is admitted; the equality form is probed by `streamAssertEq` (streaming) and the constructor assert. The ordering form is refused (ᵇ).
- ᵉ **refused** `circuit-body-emission` — a body that both reads a ledger cell and returns a value is not lowered.
- ᶠ the constructor also accepts the seq-lifted subtraction form (`c ? a - 1 : a`); `y` is the mixed literal/expr form.
- ᵍ **n/a** — a `constructor` has no return value.
- ʰ the `some<T>` sub-case is shared with `pure::callArgCtor`; the witness sub-case is n/a in a constructor (no witness context), so the constructor row's `call arg` cell probes the pure-call sub-case.

## 3. Flip the oracle probes

- [x] 3.1 Convert the ternary-site probes added by `vendor-digital-passport-harness` from REJECTION to ACCEPTION (they must now compile). Verify: `rejection_corpus` green with the fixed compiler; the probes assert successful emission.
- [x] 3.2 Confirm the mixed-width oracle probe was already flipped by `type-directed-expression-coercion` and is not left as a stale REJECTION. Verify: `grep` finds no mixed-width REJECTION entry.
- [x] 3.3 Flip the **whole-entry** oracle gate `rust_backend_dogfood_entry_is_refused_pre_fix` in `tests-e2e-rust/tests/rejection_corpus.rs` from asserting refusal to asserting acceptance (exit 0, `contract/lib.rs` emitted, no `pure-circuit-body-emission`). This change is what makes the vendored entry compile, so the gate must flip here or `rejection_corpus` goes red, contradicting task 3.1's verification. Rewrite the gate's doc comment, which currently credits `add-digital-passport-dogfood-fixture`. Verify: `cargo test -p tests-e2e-rust rejection_corpus` green with the fixed compiler.

## 4. Regression sweep, traces, version

- [x] 4.1 Regenerate **every** fixture and classify each diff per AGENT.md §5.2; STOP on unexplained diffs. Verify: `codegen_regression` green.
- [x] 4.2 Traceability: map every `#### Scenario:` in the delta spec to a named test/command. Verify: no scenario without a test, no test without a scenario.
- [x] 4.3 Mutation check: temporarily revert the `if` clause (or the `expr-supported?` arm) and confirm a specific matrix test goes red; restore. Verify: recorded evidence.
- [x] 4.4 Local gates: `cargo fmt --all --check`, touched-crate clippy, full `cargo test -p midnight-compact-runtime -p tests-e2e-rust`. Verify: all green.
- [x] 4.5 Bump 0.31.117 → 0.31.118 (`compiler-version.ss`, `flake.nix`, regenerate `doc/ledger-adt.mdx`, grep old triple) + CHANGELOG `### Fixed` entry. Verify: zero stale embeds; `changelog-check` satisfied.

### 4.1 regen sweep — classification

`result/bin/compactc --target rust --skip-zk` (built from this change's
`compiler/**`) regenerated all 37 `FIXTURES` rows (with rustfmt 1.95.0 on
`PATH`, the lane's formatter). Every regenerated `lib.rs` was byte-identical
to its committed file — **zero diffs to classify** — so there is no
unexplained shape change (§5.2 step 4). The only new artifact is
`contracts/ternary-cond-fixture/`, whose `lib.rs` matches the committed copy
byte-for-byte and whose `Cargo.toml` follows the fixture recipe (package /
lib renamed, path dependency on `../../../runtime-rs`).

### Scenario → test traceability (task 4.2)

Every `#### Scenario:` in the delta spec
(`specs/rust-codegen/conditional-expressions/spec.md`) maps to a named
executing test (all under `tests-e2e-rust`):

| Delta-spec scenario | Named test / command |
|---|---|
| const-binding ternary in a pure circuit | `ternary_cond_fixture::const_annotated_both_literal_round_trips`, `::return_tail_mixed_round_trips`, `::branch_local_guard_is_lazy` (seq-lifted const); `rejection_corpus` ACCEPTIONS `ternary in const RHS` |
| ternary inside an assert argument | `ternary_cond_fixture::assert_arg_both_sides` (pure), `::stream_assert_eq_holds` (streaming), `::ternary_cond_init_true_byte_parity` (ctor assert); `rejection_corpus` ACCEPTIONS `ternary in assert argument` |
| ternary as an arithmetic or comparison operand | `ternary_cond_fixture::arith_operand_round_trips`, `::cmp_operand_round_trips`, `::walker_compare_eq_selects_the_arm`, `::stream_compare_eq_selects_the_arm`; `rejection_corpus` ACCEPTIONS `ternary as interior arithmetic operand` |
| ternary in an impure circuit, streaming route, and constructor | walker: `::walker_write_selects_the_arm` + `::ternary_cond_walker_*_byte_parity`; streaming: `::stream_increment_takes_the_else_arms` + `::ternary_cond_stream_*_byte_parity`; ctor: `::ternary_cond_init_{true,false}_byte_parity`; whole entry: `rejection_corpus::rust_backend_dogfood_entry_compiles` |
| ternary as a call argument, struct member, or vector element | `ternary_cond_fixture::call_arg_pure_round_trips`, `::call_arg_ctor_round_trips`, `::struct_member_round_trips`, `::vector_element_round_trips`, `::native_arg_round_trips` (+ `walker_*` / `stream_*` variants) |
| untaken branch must not trap | `ternary_cond_fixture::pick_is_lazy` (c=3), `::branch_local_guard_is_lazy` (`false,0`) |
| taken branch computes its value | `ternary_cond_fixture::pick_is_lazy` (c=12 → 2), `::branch_local_guard_is_lazy` (`true,5` → 4; `true,0` traps) |
| struct-valued branches | `ternary_cond_fixture::struct_valued_arms_round_trips`, `::nested_conditional_round_trips` |
| literal and Uint arms in a Field position | `ternary_cond_fixture::uint_arm_into_field_round_trips`, `::field_typed_arms_round_trips`, `::literal_above_i32_round_trips`, `::literal_above_u64_round_trips`, `::differing_uint_widths_round_trips` |
| conditional literal into a wide field | `ternary_cond_fixture::ternary_cond_init_{true,false}_byte_parity` (Uint<64> `wideCell`), `::ternary_cond_stream_increment_byte_parity` |
| matrix has no blank cells | the route × position matrix above (task 2.1); `codegen_regression::rust_codegen_byte_parity_against_committed_fixtures` (fixture registered) + `::every_fixture_crate_is_a_dev_dependency` |

Commands: `cargo test -p tests-e2e-rust --test ternary_cond_fixture`,
`cargo test -p tests-e2e-rust --test rejection_corpus`,
`cargo test -p tests-e2e-rust --test codegen_regression`.

Reverse direction (no test without a scenario): every test in
`ternary_cond_fixture.rs` drives exactly one cell of the coverage matrix
(2.1/2.2), and each cell realises one of scenarios 1–5 or the coverage
scenario; the running / laziness assertions double as scenarios 6–7 and the
byte-parity tests as scenario 10. The three `rejection_corpus` ternary
ACCEPTIONS correspond one-to-one with the three upstream gap sites of
scenarios 1–3, and `rust_backend_dogfood_entry_compiles` with scenario 4.

### 4.3 mutation check — recorded evidence

Baseline (clause present): `codegen_regression` green. Mutation: deleted the
`(if ,src ,expr0 ,expr1 ,expr2)` clause (lines 2080–2133) from
`compiler/rust-passes-emit.ss` — the file then matched `HEAD` byte-for-byte
(`git diff` empty) — and rebuilt with `nix build .#compactc`. With the mutated
compiler, `cargo test -p tests-e2e-rust --test codegen_regression` **FAILED**:

```
Exception: compactc --target rust: unsupported Compact construct
  (expr-variant): unhandled Expression variant in expr-rust
... panicked at tests-e2e-rust/tests/codegen_regression.rs:
  compactc failed for ternary_cond_fixture.compact (exit Some(255))
test result: FAILED. 1 passed; 1 failed
```

So the byte-parity half of the matrix gate is the specific failing test. The
clause was then restored from a backup (`git diff` back to `+54`), rebuilt
(`nix build .#compactc` returned the same store path as before the mutation), and
`codegen_regression` returned green.

### 4.4 gates — recorded results

All under rustc/rustfmt/cargo 1.95.0 (`target/.rustc_info.json` pins the lane
toolchain), with rustfmt on `PATH` so the emit post-pass formats identically to
the committed fixtures:

- `cargo fmt --all --check` → exit 0.
- `cargo clippy -p midnight-compact-runtime -p midnight-compact-runtime-macros -p tests-e2e-rust --all-targets --all-features -- -D warnings` → exit 0.
- `cargo test -p midnight-compact-runtime -p tests-e2e-rust` → exit 0 (all
  suites green, incl. `ternary_cond_fixture` 52/52, `rejection_corpus` 3/3,
  `codegen_regression` 2/2).

### 4.5 version bump — recorded result

`compiler/compiler-version.ss` `(make-version 'compiler 0 31 118)`, `flake.nix`
`packages.compactc.version = "0.31.118"`, and `doc/ledger-adt.mdx` regenerated via
`./compiler/go` (full compiler suite green, 79% coverage) to
“compiler version 0.31.118”; `CHANGELOG.md` gained a
`## [Toolchain 0.31.118, language 0.23.103, runtime 0.16.100]` `### Fixed` entry.
`grep -rn '0\.31\.117' compiler/ flake.nix doc/ledger-adt.mdx` → no matches
(zero stale embeds; the only remaining `0.31.117` in the tree is the prior
release header in `CHANGELOG.md`, which is history). Rebuilt `result/bin/compactc`
reports `0.31.118`; `codegen_regression` stays green (the compiler version is
not embedded in generated `lib.rs` — only the runtime `0.16.100` is).
`changelog-check` needs `CHANGELOG.md` + `compiler/compiler-version.ss` in the
diff; both are present.
