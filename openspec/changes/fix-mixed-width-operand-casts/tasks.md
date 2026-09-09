# Tasks: fix-mixed-width-operand-casts

All work inside `nix develop`; byte-parity per AGENT.md §2.1/§3.3 is local-only and mandatory before every push. Compiler changes require the full FIXTURES corpus regenerated in the same commit.

## 1. Emitter fix

- [ ] 1.1 Locate the binary-operation rendering in `compiler/rust-passes-emit.ss` (`expr-rust`) and determine where operand widths come from (typer annotations on the IR node vs `mbits->rust-width` re-derivation). Verify: can print/trace the widths for the `/tmp/width-probe/a.compact` shape (`q * 4 <= y`, q/y `Uint<32>`).
- [ ] 1.2 Implement the widening rule: when a binary operation's operands have different minimal Rust widths, cast the narrower operand losslessly (`as <wider>`) at the boundary — for `<= >= < > == !=` and `+ - *`, including the guarded-subtraction route (guard comparison and subtraction operands widened consistently). Verify: probe `a.compact` and `b.compact` emit comparisons with same-width operands; `q * 4` still renders `u64` (no narrowing); `nix build .#compactc` succeeds.
- [ ] 1.3 Confirm no uniform-width emission changed: regenerate `tiny`, `widening-arith-fixture`, and `ternary-cond-fixture` crates and diff — byte-identical except (if any) genuine mixed-width sites. Verify: `git diff tests-e2e-rust/contracts/` empty for uniform-width fixtures.

## 2. Repro fixture + registration

- [ ] 2.1 Author `examples/mixed_width_operand_fixture.compact` (Apache header): per-operator mixed-width circuits — each comparison operator with one range-widened operand; mixed-width `+ - *` incl. the guarded-subtraction shape; the exact dogfood shape (narrow ternary join vs widened product inside an assert chain); cascading `const` bindings; at least one impure circuit and one constructor site. Verify: `compactc --target ts --skip-zk` compiles it.
- [ ] 2.2 Generate `tests-e2e-rust/contracts/mixed-width-operand-fixture/` (`--target rust --skip-zk`), author the crate `Cargo.toml` per sibling convention, register: root `Cargo.toml` workspace member, `tests-e2e-rust/Cargo.toml` dev-dep, FIXTURES row `("mixed_width_operand_fixture.compact", "mixed-width-operand-fixture")`. Verify: emitted crate compiles standalone (`cargo build -p compact-contract-mixed-width-operand-fixture`).
- [ ] 2.3 Full corpus regen sweep: regenerate every FIXTURES crate with the fixed compiler, `git diff` categorised per AGENT.md codegen discipline (byte-changes only at mixed-width source sites; expected suspects: widening-arith-fixture). Verify: `cargo test -p tests-e2e-rust rust_codegen_byte_parity` green.

## 3. Executing parity gate

- [ ] 3.1 Author `tests-e2e-rust/fixtures/capture-mixed-width-operand-fixture.mjs` (Apache header) + committed JSON per the existing capture pattern: values crossing the 2³² boundary (e.g. `q32` near max so `q32 * 4` exceeds 2³²) so a truncating or wrongly-directed cast produces observably wrong bytes. Verify: script runs, emits committed JSON.
- [ ] 3.2 Write `tests-e2e-rust/tests/mixed_width_operand_fixture.rs` (Apache header) asserting Rust outcomes byte-equal the TS reference at each step. Verify: `cargo test -p tests-e2e-rust mixed_width` green.

## 4. Acceptance + release

- [ ] 4.1 Dogfood acceptance gate: `cargo build -p tests-e2e-rust --tests --locked` green (the 13 `E0308`s in `compact-contract-digital-passport-credential` gone); regenerate the dogfood crate and confirm the byte-parity row passes with the updated `lib.rs`. Verify: zero `E0308` in the dogfood crate; full FIXTURES table green.
- [ ] 4.2 Version bump 0.31.117 → 0.31.118 with full embed-site sweep (`compiler-version.ss`, `flake.nix`, `doc/ledger-adt.mdx` regen, grep old triple) + CHANGELOG entry (mixed-width widening casts; dogfood-surfaced). Verify: zero stale embeds; `changelog-check` satisfied.
- [ ] 4.3 Full local gates: fmt, clippy (incl. new crate), `cargo test -p midnight-compact-runtime -p tests-e2e-rust`, `python add_headers.py --validate`. Verify: all green under `nix develop`.
- [ ] 4.4 Commit signed+DCO on `feature/fix-mixed-width-operand-casts` (cut from `feature/add-digital-passport-dogfood-fixture`; merges into `digital-passport-patch`). Verify: `git log --show-signature` clean (configure `gpg.ssh.allowedSignersFile` locally for verification).
- [ ] 4.5 On resume of `add-digital-passport-dogfood-fixture`: renumber its task 6.3 version bump to 0.31.118 → 0.31.119 (this change owns 0.31.118) via openspec-update-change. Verify: dogfood tasks.md reflects the renumbered bump.
