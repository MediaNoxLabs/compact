# Tasks

All commands run under `nix develop` (see AGENT.md §2.1). "Regen gate" means `nix build .#compactc` followed by `cargo test -p tests-e2e-rust --test codegen_regression`. A group passes it only if every fixture it did not intend to change stays byte-identical, and every diff is classified as previously broken output (design D6).

## 1. Formal-type table and `persistentCommit` (silent wrong hash first)

- [ ] 1.1 Build the `callee-formal-types` table (id → list of formal types) from the native, witness and circuit declarations next to the existing id tables (`compiler/rust-passes-emit.ss` ~L151), and expose it as a parameter alongside `current-circuit-id-ht` (design D3). Verify with `./compiler/go`, and a temporary debug print on a scratch contract (a pure circuit and a witness taking `Vector<2, Field>`, `persistentHash<Vector<2, Field>>`, `persistentCommit<Field>`) showing concrete types for `persistentHash<Vector<2, Field>>`, `persistentCommit<Field>` and a witness. Remove the print before committing.
- [ ] 1.2 Render `persistentCommit`'s value at its formal type (keeping the `&` borrow) and its opening at `Bytes<32>` (design D4). Verify: `persistentCommit<Field>(0 as Field, o)` emits `&Fr::from(0u64)` and not `&0`, and the dogfood contract's six `persistent_commit` calls are byte-identical under the regen gate.
- [ ] 1.3 Create `examples/call_arg_declared_type_fixture.compact` with exported impure circuits writing to the ledger: `persistentCommit<Field>` of `N as Field` for a small `N`, a `u128`-range `N`, and `N > max-unsigned`. Register the generated crate in `FIXTURES` (`tests-e2e-rust/tests/codegen_regression.rs`), `tests-e2e-rust/Cargo.toml` and the workspace `Cargo.toml`. Verify the crate builds (`cargo build -p compact-contract-call-arg-declared-type-fixture`).
- [ ] 1.4 Add `tests-e2e-rust/fixtures/capture-call-arg-declared-type-fixture.mjs` (modelled on `capture-literal-coercion-fixture.mjs`), generate `call-arg-declared-type-fixture-ts-state.json`, and add `tests-e2e-rust/tests/call_arg_declared_type.rs` asserting that the Rust ledger state bytes after each circuit equal the TS capture. Verify it passes. Verify it fails on the pre-fix emitter, by checking out the parent commit's emitter and regenerating, so the test is known to catch the `&0` hash.

## 2. `call-args-rust` in the `expr-rust` renderer (`call-rust`)

- [ ] 2.1 Add `call-args-rust fn-id expr*` in its `expr-rust` form: it pairs each argument with its formal, binds `(or (expr-expected-type e) formal)` and keeps `pure-call-arg-rust`'s clone decision (design D1, D2). Switch the `some`/`none`, 1:1 native and user pure circuit branches of `call-rust` to it. Verify the regen gate.
- [ ] 2.2 In `native-vector-aligned-slice`, use the formal type as `native-vector-atoms`' target when the argument has no wrapper (design D4). Verify `transientHash<Vector<2, Field>>([0 as Field, 1 as Field])` emits `AlignedValue::from(Fr::from(0u64))`-style atoms, and the regen gate passes.
- [ ] 2.3 Extend the fixture: `[0 as Field, 1 as Field]` into a pure circuit's `Vector<2, Field>` parameter from a pure body; the same vector into `persistentHash` and `transientHash`; the #90 literal as a pure-circuit argument; and a tuple-typed value into a `Vector` parameter and the reverse (#83). Re-capture the TS state and extend `call_arg_declared_type.rs`. Verify the crate builds and the parity test passes.

## 3. `call-args-rust` in the walker and streaming paths

- [ ] 3.1 Add the walker form of `call-args-rust` (keeps `arg-rust-clone-if-var`'s clone decision and `render-pure-circuit-arg`'s enum ledger-read decode). Switch `ctor-call-rust` (stdlib, pure) and the flat-body walker sites in `compiler/rust-passes-walker.ss`: `const =` / bare witness, pure and impure calls, `emit-hoisted-impure-calls`, `emit-hoisted-witnesses`. Verify the regen gate.
- [ ] 3.2 Switch the streaming walker sites in `compiler/rust-passes-streaming.ss`: `const =` / bare witness, pure and impure calls, mid-branch (A26) and terminal (A17) impure calls. Verify the regen gate.
- [ ] 3.3 In `inline-circuit-call`, render each actual with its formal as the expected type, with no clone logic added (design D5). Verify the regen gate.
- [ ] 3.4 Extend the fixture: the #21 vector into a witness (`const` and bare), into a pure circuit from an impure body, into an impure circuit (`const` and bare, including in an `if` arm), and into an impure circuit inlined into an `if` condition and an `assert` condition. Add Rust witness implementations in the test, re-capture the TS state and extend `call_arg_declared_type.rs`. Verify the crate builds and the parity test passes.

## 4. Structural guard

- [ ] 4.1 Add a test in `tests-e2e-rust/tests/` that reads `compiler/rust-passes-*.ss` and fails if `arg-rust-clone-if-var` or `pure-call-arg-rust` is called outside `call-args-rust` and a named allowlist of non-call users (ledger ADT operation arguments, cell-write values), per design D8. The failure message must name the offending file and enclosing function. Verify it passes, and fails when one call site is temporarily reverted to a direct call.

## 5. Integration checks

- [ ] 5.1 One-off #90 fallback check (design D7): temporarily remove the `(> datum (max-unsigned))` branch of the `quote` clause, run the regen gate for `literal-coercion-fixture` and the new fixture, and confirm both are byte-identical. Restore the branch, and record the outcome here and in the PR description.
- [ ] 5.2 Probe ledger ADT operation arguments and cell-write values with the same shape (a `[0 as Field, 1 as Field]` argument to e.g. a `Set<Vector<2, Field>>` `insert` / `member`, and a `Vector<2, Field>` cell write) in a scratch contract, not committed. If any emits wrong Rust, file a follow-up issue. Record the result here either way.
- [ ] 5.3 Full-corpus byte-stability review: regenerate all `FIXTURES` and verify that only `call-arg-declared-type-fixture` changed, or that each other diff is a previously broken position (spec: neutral output is byte-stable). Also re-run the task 1.1 scratch contract and confirm every call kind now emits `Fr` elements.
- [ ] 5.4 Bump the compiler to 0.31.122 in `compiler/compiler-version.ss`, `flake.nix` and `doc/ledger-adt.mdx`, and add a `CHANGELOG.md` entry citing #91, #21 and #83. Include the `persistentCommit` value fix. Verify `grep -rn "0\.31\.121" compiler/ flake.nix doc/ledger-adt.mdx` is empty.
- [ ] 5.5 Run the full pre-push checklist (AGENT.md §2.1): `cargo fmt --check`, `cargo clippy -D warnings`, `nix build .#compactc`, and `cargo test -p midnight-compact-runtime -p tests-e2e-rust` (including `ts_rust_target_parity`, `digital_passport_credential` and `rejection_corpus`). Verify all green.
- [ ] 5.6 In the PR description, reference #91 and #21 (fixes) and #83 (call-argument case), and call out the `persistentCommit` silent-wrong-hash fix explicitly.
