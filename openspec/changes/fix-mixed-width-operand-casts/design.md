# Design: fix-mixed-width-operand-casts

## Context

Probe evidence (all against `result/bin/compactc` at the merge of the ternary fix into `feature/add-digital-passport-dogfood-fixture`):

- 3-line repro (`/tmp/width-probe/a.compact`): `pure circuit a(q: Uint<32>, y: Uint<32>) { assert(q * 4 <= y, "msg"); }` emits
  ```rust
  let t = ((q) as u64).wrapping_mul((4) as u64);
  compact_assert!((t <= y), "msg");          // u64 vs u32 → E0308
  ```
- The typer rejects `const z = q * 4; return z;` against a declared `Uint<32>` return with
  `mismatch between actual return type Uint<0..17179869181> and declared return type Uint<32>` — proving the typer assigns **range types** to arithmetic results (17179869181 = 4·(2³²−1)), and the emitter's wider rendering is deliberate range-fidelity, not a bug.
- Ternary/subtraction chains keep narrow renderings: `const z = q <= 2 ? q - 1 : q` emits `((q) as u32).wrapping_sub((1) as u32)` — `Uint<32>`-ranged, correct.
- Dogfood contract (13 × `E0308`, `tests-e2e-rust/contracts/digital-passport-credential/lib.rs` 3333–3371): `yearAdjusted` (ternary join, u32) meets `yearAdjustedQuotient4 * 4` (range-widened, u64) in `<=`, `<`, and guarded `-`; all `DigitalPassportCivilDate` fields are `Uint<32>` (helpers.compact 243–266), so every mismatch is typer-legal mixed-width, never a source error.

So the defect is exactly: the shared `expr-rust` renderer renders each binary-operation operand at its own minimal width and inserts no widening where they meet.

## Goals / Non-Goals

**Goals:**
- Every expression the typer accepts emits Rust that compiles (comparison + arithmetic routes, pure/impure/constructor bodies).
- Range-driven widths are preserved — no fix-by-narrowing.
- The dogfood crate builds (`cargo build -p tests-e2e-rust --tests` green) — the acceptance gate.
- Corpus byte-stability outside mixed-width sites.

**Non-Goals:**
- No typer changes (mixed-range operations stay legal; this is purely an emission gap).
- No TS-backend changes (JS numbers have no widths; parity is unaffected).
- No new width-ladder rungs (the `mbits->rust-width` ladder from the widening-arith work stays as-is).

## Decisions

1. **Cast the narrower operand at the binary-op boundary** (`(x) as <wider>`), rather than threading an expected/joined type down through the renderer. Rationale: the operand boundary is the only place the mismatch is observable; a lossless upcast (narrower range ⊆ wider range, so the cast is always value-preserving zero-extension) fixes every affected operator with one rule and no renderer signature changes. Threading types down risks perturbing uniform-width emission (byte-stability goal) and touches far more code.
2. **Apply to every Rust same-operand-type operator** the emitter renders from binary IR nodes: `<= >= < > == !=`, `+ - *`. The guarded-subtraction route (`compact_assert!` + `wrapping_sub`) must widen its *guard comparison* and its *subtraction operands* consistently.
3. **Dedicated fixture** `examples/mixed_width_operand_fixture.compact` (never the dogfood as repro — the dogfood is the acceptance test, the fixture is the minimal proof), registered per the full recipe, plus an executing parity test with values crossing the 2³² boundary so a wrongly-directed or truncating cast cannot pass by merely compiling.
4. **Own version bump 0.31.117 → 0.31.118** (changelog-check requires CHANGELOG + compiler-version in the diff; the ternary fix set the precedent of per-fix bumps). Consequence recorded in the proposal: `add-digital-passport-dogfood-fixture` task 6.3 renumbers to 0.31.118 → 0.31.119 when that change resumes.
5. **Branch cut from `feature/add-digital-passport-dogfood-fixture`**, not `codegen-rust`: the dogfood vendor tree + staged registrations (workspace member, dev-dep, FIXTURES row, `Cargo.lock`) then make `cargo build -p tests-e2e-rust --tests` and the dogfood row of the byte-parity sweep this change's own gates — mirroring how the ternary fix used its task 5.1 against the dogfood sources.

## Risks / Trade-offs

- [Regen sweep changes unexpected fixtures] — suspected: `widening-arith-fixture` (built to stress the width ladder); possible: others with range-widened arithmetic compared against narrow bindings. Mitigation: the sweep is a task with explicit diff categorization (AGENT.md codegen-change discipline), and byte-changes must be traceable to mixed-width source sites.
- [Cast placement changes uniform-width bytes] — rejected by construction: casts are inserted only when operand widths differ; uniform-width sites emit identically.
- [`as` casts interact with `wrapping_*` lint expectations] — clippy gate (`-D warnings`) runs on the new crate; `as u64` upcasts do not trip `clippy::cast_lossless` at `-D warnings` in this codebase's configuration, but the gates task catches any surprise.
- [Comparison-only fix leaves arithmetic broken] — the fixture enumerates operators explicitly so the gap cannot close for `<=` and reopen for `-`.

## Migration Plan

Cut `feature/fix-mixed-width-operand-casts` from `feature/add-digital-passport-dogfood-fixture`; land (merge into `digital-passport-patch` per the fork's flow) before the dogfood change's group 3 resumes. No migration of existing artifacts; rollback = revert commit.

## Open Questions

None blocking. One implementation-time check: whether the binary-op renderer sees operand widths as typer annotations on the IR node or must re-derive them via the `mbits->rust-width` ladder — the fix task starts by locating the exact clause in `rust-passes-emit.ss` and may adjust Decision 1's mechanics (cast vs joined-width rendering) without changing the contract above.

**Resolution (recorded at implementation):** widths are typer annotations — `relational-operator` / `equality-operator` (analysis-passes.ss) wrap the narrower operand in `(safe-cast <joined-range> <own-range> expr)` exactly when the ranges map to different minimal Rust widths, verified via `--trace-passes` on the probe shapes; the fix materialises that wrapper as `((operand) as <wider>)` at the comparison, equality, and conditional-branch boundaries (the cast must be parenthesised as a whole — `x as u64 < y` parses `<` as generics). Three extensions fell out of the same mechanics, all required by the dogfood acceptance scenario (zero E0308) without changing the contract: (1) the ternary **branch join** (`month >= 3 ? month - 3 : month + 9`) also wraps its narrower branch and needed the same widening in `expr-rust`'s `if` clause — that was one of the original 13 errors; (2) the constructor body walker needed a `const-decl-only?` skip clause for the declaration-only statements the typer emits when a ctor const RHS lifts its own temps (without it, `constructor { const diff = base - q * 4; … }` was wholly unwalkable); (3) the `+ - *` routes needed NO change — `arith-binop-rust` already casts both operands to the `mbits` result width. Bare integer literals still peel silently (Rust infers their width), which is what keeps uniform-width emission byte-stable.
