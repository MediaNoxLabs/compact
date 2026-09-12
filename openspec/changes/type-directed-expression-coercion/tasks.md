# Tasks: type-directed-expression-coercion

All work inside `nix develop` (AGENT.md §1 rule 2). Run `nix build .#compactc` before any byte-parity test (AGENT.md §2.1 step 3). This change MUST be green against the pre-existing corpus before the ternary change builds on it.

## 1. Central machinery

- [ ] 1.1 Add `current-expr-expected-type` (default `#f`) to `compiler/rust-passes-helpers.ss`. Verify: no behaviour change when unset (byte-parity sweep green).
- [ ] 1.2 Add `materialize-at-type` in `rust-passes-helpers.ss`: literal + `tunsigned` → width suffix; literal + `tfield` → `Fr::from(<n>u64)`; `tfield ← tunsigned` scalar → `Fr::from((x) as u64|u128)` via `uint-coercion-cast-width`; **`tunsigned ← tunsigned` → `(x) as <width>` when `uint-rust-width` differs** (never when only the range differs but the Rust width matches); aggregate targets recursed element-wise; same type *and same `uint-rust-width`* → inner rendering unchanged. Verify: unit-level probes for each rung, including a `Uint<0..256>` vs `Uint<0..65535>` pair (both `u16`) rendering with **no** spurious cast.
- [ ] 1.3 Add `expr-rust-typed expr expected` in `rust-passes-emit.ss`: bind `current-expr-expected-type`, call `expr-rust`. Change `expr-rust`'s `safe-cast` clause and `ctor-expr-rust`'s fall-through to consult the expected type and route through `materialize-at-type` instead of peeling. Verify: `nix build .#compactc` succeeds.

## 2. Wire every boundary (one line per site; no per-position logic)

- [ ] 2.1 `const` RHS (all routes) uses the declared type. Verify: `const x: Field = 0;` and `const u: Uint<64> = 0;` emit correctly.
- [ ] 2.2 Return tail (pure statement-lifted tail, impure I3b/4 single-if-expression matcher, A19 if/else-if chain) uses the declared return type. Verify: `return 0;` in a `Field` circuit emits `Fr::from(0u64)`.
- [ ] 2.3 **Comparison/equality operands only** use the joined operand type (a mixed-width relational/equality comparison is where the typechecker wraps the narrower operand). Preserve the bare-literal peel; **do not** thread an expected type into `+ - *` (already same-width via `arith-binop-rust`'s `mbits` cast — a push would double-cast). Verify: `q * 4` vs `q: Uint<32>` emits `((x) as u64)` on the narrow side and leaves the wide side u64; uniform-width sites are byte-identical.
- [ ] 2.4 `assert` argument (boolean). Verify: unaffected numerically, no regression.
- [ ] 2.5 Call arguments use the argument's `safe-cast` target: pure-circuit calls, witness calls, constructor calls, `some`/`none`, and natives. Verify: `some<Field>(0)`, `persistentHash([0])`, `idf(0)` compile.
- [ ] 2.6 Struct-literal members use the field's declared type. Verify: `Box { f: 0 }` into a `Field` member compiles.
- [ ] 2.7 Vector/array elements and the persistentHash/transientHash tuple split use the enclosing `safe-cast`'s element type. Verify: `persistentHash([c ? 1 : 0])` (post-ternary) and `persistentHash([0])` compile.
- [ ] 2.8 Ledger cell writes use the destination field type. Verify: a `Uint<8>` value into a `Uint<64>` field commits the field's width (state-byte parity, not just decoded value).
- [ ] 2.9 Wire the **constructor path's** comparison renderer (`coerce-cmp-operand-rust`) through the typed entry, not only `expr-rust`. Verify: a mixed-width equality in a constructor/impure body compiles.

## 3. Walker walkability (pre-existing, coercion-independent)

- [ ] 3.1 Skip declaration-only `const` statements in the constructor walker, mirroring the streaming walker (`rust-passes-streaming.ss:81,371`). `const-decl-only?` (`rust-passes-emit.ss:646`) matches the forward declaration the typechecker emits whenever a `const` RHS lifts temps via `maybe-bind` (`analysis-passes.ss:2092`); the constructor path (`emit-body-or-fallback`, `rust-passes-walker.ss:1906`) does not skip it, so `constructor { const diff = base - q * 4; … }` is wholly unwalkable. **This is a pre-existing walkability defect, independent of coercion and reachable at uniform widths; it is folded here only because this change ports the constructor fixture that exercises it.** Verify: a constructor with a guarded-subtraction const compiles; the mixed-width ctor fixture (5.5) and the ternary matrix's constructor const-lifted cell are unblocked.

## 4. Refusal hygiene

- [ ] 4.1 Add the no-sentinel post-emit guard (reject `#f`/unsafe sentinel splices with a located `rust-feature-error`); remove the reliance on `rendered-has-todo?` for correctness. Verify: a deliberately failing render refuses with a source location and writes no `lib.rs`.
- [ ] 4.2 Refuse `Uint → Field` above `u128::MAX` via `field-uint-coercion`; add a `rejection_corpus` entry. Verify: exit non-zero, precise message, no partial output.
- [ ] 4.3 Recompute and update `docs/rust-backend-limitations.md` counts by its live grep recipe. Verify: counts match a fresh grep.
- [ ] 4.4 Convert the mixed-width operand oracle probe from `vendor-digital-passport-harness` from REJECTION to ACCEPTION (this change owns that flip). Verify: `rejection_corpus` green with the probe asserting successful emission.

## 5. Fixtures and matrix

- [ ] 5.1 Author `examples/literal_coercion_fixture.compact` covering each newly-covered position: Field const literal, struct Field member literal, `some<Field>(0)`, `persistentHash([0])`, native arg literal, pure-call arg literal, return-tail literal, Uint→Field scalar, `Uint<128>` rung, aggregate element-wise vector. Verify: `compactc --target ts --skip-zk` compiles it.
- [ ] 5.2 Generate the crate, register it (root workspace member, `tests-e2e-rust` **dev-dependency**, `FIXTURES` row). Verify: `cargo build -p tests-e2e-rust --tests --locked` compiles it.
- [ ] 5.3 Write the executing test proving each coerced value round-trips, plus a state-byte parity check for the ledger cell. Verify: `cargo test -p tests-e2e-rust literal_coercion` green.
- [ ] 5.4 Add a negative/parity test asserting the TS target compiles every fixture circuit the Rust target compiles (the parity criterion), and that every residual refusal is listed in `docs/rust-backend-limitations.md`. Verify: no TS-accepted position refuses unexplained.
- [ ] 5.5 Port `mixed_width_operand_fixture.compact` and its test/capture from PR #70 (`origin/feature/add-digital-passport-dogfood-fixture`) — per-operator mixed-width comparisons, guarded subtraction, impure inline equality, and a constructor route — registered per the full recipe with a **dev-dependency**. Verify: values driven past the `2^32` boundary so a truncating or wrongly-directed cast cannot pass by merely compiling.

## 6. Regression sweep, version, docs

- [ ] 6.1 Regenerate **every** fixture and classify each diff hunk per AGENT.md §5.2; STOP on any unexplained diff. Verify: `codegen_regression` green; only intentional positions changed.
- [ ] 6.2 Local gates: `cargo fmt --all --check`, touched-crate clippy, full `cargo test -p midnight-compact-runtime -p tests-e2e-rust`. Verify: all green.
- [ ] 6.3 Bump `compiler/compiler-version.ss` 0.31.116 → 0.31.117, `flake.nix`, regenerate `doc/ledger-adt.mdx` via `./compiler/go`, grep-sweep the old triple, add the CHANGELOG entry under `### Fixed`. Verify: zero stale embeds; `changelog-check` satisfied.
- [ ] 6.4 Update AGENT.md §5 if the fixture-registration recipe changed (dev-dependency requirement). Verify: recipe matches enforcement.
