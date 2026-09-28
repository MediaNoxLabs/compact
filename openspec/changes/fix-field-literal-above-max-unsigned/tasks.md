# Tasks

All commands run under `nix develop` (see AGENT.md §2.1).

## 1. Emitter fallback

- [ ] 1.1 In the integer branch of the `quote` clause of `expr-rust` (`compiler/rust-passes-emit.ss`), add a branch after the expected-`Field` / expected-`Uint` branches: with no expected type and `(> datum (max-unsigned))`, render `(field-literal-rust datum)`, with a comment citing the typechecker rule (`analysis-passes.ss` `N as Field` special case) per design D1/D2. Verify with the #90 reproduction (`compactc --rust --skip-ts` on the issue's snippet): the `ecMul` and `id` arguments render `Fr::from_le_bytes(...)` and no bare digits remain.
- [ ] 1.2 Confirm the typechecker rule the fallback relies on is still pinned: `./compiler/go` passes, including the "larger than the largest representable Uint" rejection tests for `max-field` and `max-unsigned + 1` (`compiler/test.ss` ~L41913, ~L41924).

## 2. Regression fixture and tests

- [ ] 2.1 Extend `examples/literal_coercion_fixture.compact` with pure circuits covering a literal above `max-unsigned`, using `c = 819310549611346726241370945440405716213240158234039660170669895299022906775` (Jubjub `(r + 1) / 8`): as an `ecMul` argument, as a user pure-circuit argument (`idf`), as a `const` initialiser, a return value, a `Field` arithmetic operand and an equality operand; a boundary circuit passing `2^248 as Field` (452312848583266388373324160190187140051835877600158453279131187530910662656) to `idf`; and a subgroup-check circuit returning `ecMul(ecMul(ecMulGenerator(1), c as Field), 8)`. Update the fixture's header comment listing the covered positions. Verify `compactc --rust` succeeds on it.
- [ ] 2.2 Regenerate `tests-e2e-rust/contracts/literal-coercion-fixture/lib.rs` from the updated example and verify the diff adds only the new circuits (every pre-existing function, including the 2^200 cases, is byte-identical) and contains no bare integer above `u128::MAX`.
- [ ] 2.3 Re-run the TS reference capture (`tests-e2e-rust/fixtures/capture-literal-coercion-fixture.mjs`) and verify `literal-coercion-fixture-ts-state.json` is unchanged; commit it only if it changed, and explain why in the commit.
- [ ] 2.4 Add a test to `tests-e2e-rust/tests/literal_coercion.rs` that asserts: the `c` returned from every new position has the same little-endian bytes as `c`; the boundary circuit returns the bytes of 2^248; `(r + 1) / 8 == c` computed with integer arithmetic in the test (design D3), not as `Field` operations; and the subgroup-check circuit returns `ecMulGenerator(1)`. Verify with `cargo test -p tests-e2e-rust --test literal_coercion`.

## 3. Integration checks

- [ ] 3.1 Run the byte-stability gate: `nix build .#compactc` then `cargo test -p tests-e2e-rust --test codegen_regression`. Verify that every fixture other than `literal-coercion-fixture` is byte-identical (spec: neutral output is byte-stable).
- [ ] 3.2 Run the full pre-push checklist: `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo test -p midnight-compact-runtime -p tests-e2e-rust` (including `ts_rust_target_parity` and `rejection_corpus`). Verify all green.
- [ ] 3.3 One-time downstream check (not committed): compile `midnight-vc-passport@develop` `packages/midnight-vc-passport/src/digital-passport-credential.compact` against `@midnight-ntwrk/credential-compact@0.2.0` with the fixed `compactc --rust`, then `cargo build` the output. Record the result in the PR description. If it fails for a reason unrelated to #90, file a new issue rather than widening this change.
- [ ] 3.4 Reference #90 (fixes) and #91 (follow-up) in the PR description, and note in #90 that the threshold is `max-unsigned` rather than the proposed `u128::MAX`, with the reason (design D2).
