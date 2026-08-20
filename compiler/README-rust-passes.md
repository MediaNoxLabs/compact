# `compactc --rust` codegen — module map

This document is a roadmap of the Scheme pass that emits Rust crates from
Compact source. It's for engineers landing changes in `rust-passes-*.ss`.

Read this before opening `rust-passes-walker.ss`, which is 3,700 lines and
will not orient you on its own. For what the backend *refuses* to lower and
why, see [`../docs/rust-backend-limitations.md`](../docs/rust-backend-limitations.md).

## Where in the pipeline

`compactc --rust` runs the standard frontend (parse → expand → type
infer → desugar modules → monomorphise → resolve natives → lower for
the codegen target) and then hands an `Ltypescript` IR program to the
`print-rust` pass defined in [`rust-passes.ss`](./rust-passes.ss). That
pass walks the IR top-down and writes a single `contract/lib.rs` plus
a `contract/Cargo.toml` to the output directory.

`Ltypescript` is the same IR the TypeScript backend consumes — see
[`langs.ss`](./langs.ss) line 846. Reading the TS emitter
([`typescript-passes.ss`](./typescript-passes.ss)) is often the fastest
way to understand what a given IR node should produce, because the TS
codegen has full coverage and the Rust codegen is catching up.

## Module map

The `print-rust` pass is split into **nine** Scheme files included from
[`rust-passes.ss`](./rust-passes.ss). All nine are `include`d into a single
`definitions` block, so **every definition is lexically visible to every
other file** — there are no module boundaries between them and the split is
for human navigation only. The include order is the loading order, so a
helper defined later may be referenced by an earlier file as long as the
call site isn't reached during macro expansion.

Each file now carries its own header comment stating what it owns; this
table is the index.

| File | LOC | Purpose | Touch when... |
|---|---:|---|---|
| `rust-passes.ss` | ~190 | Entry point. Defines `print-rust` + the `Program` clause that drives top-level emission. | Reordering top-level emission stages (typedefs → witnesses → contract struct → initial-state → circuits → ledger view → Cargo.toml). |
| `rust-passes-helpers.ss` | ~580 | Identifier helpers (casing, sym ⇄ rust id), parameters/fluids, stdlib lookup tables. | Renaming conventions, adding a new fluid/parameter, registering a new stdlib mapping. |
| `rust-passes-types.ss` | ~340 | `type-rust` and per-field encoding helpers (`Aligned` / `FieldRepr` / `FromFieldRepr` field emission). | Adding support for a new Compact type variant (e.g. `Uint<L..U>`, `List<T>`, generic structs). |
| `rust-passes-prelude.ss` | ~300 | File header, `emit-witnesses`, `emit-contract-struct`, `program-*` IR collectors. | Changing the file preamble, witness trait shape, or contract-struct layout. |
| `rust-passes-decls.ss` | ~450 | `collect-pure-circuit-tdefns` + `emit-type-decls` (user enums and structs with their `Aligned`/`FieldRepr`/`FromFieldRepr` impls). | Adding a new declaration kind (e.g. nominal aliases get rust newtype wrappers) or fixing user-type encoding. |
| `rust-passes-walker.ss` | ~3750 | Body walker. `body-walkable?` predicates and `emit-body-or-fallback` central dispatch — translates Compact statements to Rust expressions on the `OpProgramVerify` / `OpProgramGather` builder chain. Also: constructor body emission, for-range / for-iter / fold expansion, map() unrolling, loop-var substitution. | Adding a new statement shape, a new RHS form for `const` bindings, a new ledger op, or extending control flow. **The main hub** — most circuit-body features land here. |
| `rust-passes-streaming.ss` | ~1190 | Multi-stage body walker — handles bodies that interleave gather (ledger read) and verify (ledger write) ops, plus mid-body `if` statements that mix the two. | Touching bodies where impure `if`s appear between ledger reads and writes. Otherwise the simpler walker is enough. |
| `rust-passes-emit.ss` | ~3170 | `emit-initial-state` + circuit body assembly + ledger-view emission. Renders the actual Rust text — VM-instruction → builder-method translations live here. | Changing how an ADT seeds its initial StateValue (`new_map` / `new_merkle_tree` / `new_list`), or how a VM op renders as a builder call. |
| `rust-passes-naming.ss` | ~245 | Disambiguation tables for prefix-instantiated modules, threaded via `current-id-rust-name-ht` / `current-struct-rust-name-ht`. | Two module instantiations collide on a struct or circuit name in the flat Rust namespace. Rationale: [ADR 0001](../docs/adr/0001-rust-struct-name-disambiguation.md). |

## The walker/emit split

Not obvious from the file names, and the most common source of confusion.

**The walker answers "can this be lowered?" The emitters answer "what is the
Rust?"**

Before emitting a body, the walker walks its shape and decides whether every
construct in it has a lowering. That sounds redundant — why not try to emit
and fail? — but it buys two things:

1. **A body is emitted or refused as a whole.** Partial emission would leave
   a half-written circuit in the output stream.
2. **Alternative routes.** A body that one path cannot lower may be
   lowerable another way — notably the streamed constructor form in
   `streaming.ss`. The support check is what chooses.

The cost is real: the walker and the emitters both have to understand the
same IR shapes, so **a new construct usually needs handling in BOTH.** A
shape the emitter can render but the walker rejects is silently unsupported;
a shape the walker admits but the emitter cannot render is a crash.

## The three body routes

A circuit body takes exactly one of three routes, and conflating them causes
subtle bugs:

- **Constructor (`'ctor` mode).** Runs once against a *known* initial state,
  so ledger reads can be folded against it and conditions are often
  resolvable at emission time. Writes accumulate and must be flushed to the
  query context before any impure call that reads them back (A28
  read-your-writes). Rendered via `ctor-expr-rust`, or `streaming.ss` when
  the body branches.
- **Impure circuit.** A method taking and returning a `CircuitContext`.
  Ledger operations become op-builder programs; `ctx` is *moved* into each
  call, which is why arguments that read from it must be hoisted into
  temporaries first (A-05).
- **Pure circuit.** No context at all — a plain function over its arguments,
  in `mod pure_circuits`.

## What each test layer can and cannot catch

Three layers, and the difference between the first two matters when you
decide what evidence a change needs:

| Layer | Checks | Cannot catch |
|---|---|---|
| **Byte-parity** (`tests-e2e-rust/tests/codegen_regression.rs`) | regenerates every fixture with the real compiler, asserts byte-identity with the committed `contracts/*/lib.rs` | whether the committed output is *correct* — it pins text, so a plausible-but-wrong lowering stays pinned once committed |
| **Executing tests** (`tests-e2e-rust/tests/*.rs`) | runs generated code against the real ledger runtime; asserts behaviour and state bytes | shapes no fixture exercises |
| **`print-rust` snapshots** (`test.ss`, `snapshots/`) | emission from inside the compiler's own suite — no cargo, no built binary | only 2 cases today (MediaNoxLabs/compact#38) |

**Byte-parity locks the text; executing tests lock the meaning.** A wrong
lowering that happens to serialise identically passes byte-parity. A correct
lowering that is merely reformatted fails it. An emitter change should say
which layer covers it.

`contracts/*/lib.rs` is **generated output, not source** — never hand-edit
it.

## How a new feature lands

For most upstream-parity gaps, the path is:

1. **Reproduce the IR shape.** Run `./result/bin/compactc --trace-passes
   --skip-zk examples/<fixture>.compact /tmp/<out>/ 2>&1 | grep -A5
   "after print-rust"` (or earlier passes) to see what `Ltypescript`
   shape the construct lowers to. The IR shape determines which file
   you touch.
2. **Add the dispatch.** Most features mean a new `[(...) ...]` clause
   in either `body-walkable?` or `emit-body-or-fallback` in
   `rust-passes-walker.ss`. Mirror an existing clause that does
   something similar.
3. **Add runtime support if needed.** New types or builders go in
   `runtime-rs/src/{builders,std_lib}.rs` and get re-exported from
   `lib.rs`.
4. **Add a fixture.** Drop a small `.compact` source in `examples/`,
   wire a new crate under `tests-e2e-rust/contracts/<name>-fixture/`,
   capture the TS reference state, and add a Rust byte-parity test.
   The recipe is in [`tests-e2e-rust/README.md`](../tests-e2e-rust/README.md).
5. **Verify regen.** `cargo test -p tests-e2e-rust --test
   codegen_regression` re-runs `compactc --rust` against every example
   and asserts the emitted `lib.rs` is byte-identical to the committed
   one. This is the regression guard that protects against the
   "Scheme doesn't compile but tests pass" failure mode.

## Conventions

- Every new pattern match begins with the cheapest predicate.
- **Unsupported constructs raise `rust-feature-error`. They never emit a
  placeholder.** This is the backend's central safety property, and it
  replaces an older convention that said walkers should "bail to
  `unimplemented!()` rather than producing wrong code". That turned out to
  be the same mistake in a different costume — it defers the failure to
  run time instead of preventing it. Four places violated the rule and all
  four produced compiling, wrong output: an `assert!(true)` that never
  fired, an `if true` that made a branch unconditional, a catch-all
  `guard` that swallowed rejections, and four natives that panicked at run
  time. Worked examples are in
  [`../docs/rust-backend-limitations.md`](../docs/rust-backend-limitations.md).
  If you are unsure what the right output is, raise the error.
- Defensive `.clone()` is preferred over borrow gymnastics on
  generated code — the cost is paid once at compile time, never at
  runtime.
- Stdlib symbols carry `(rust "name")` annotations in
  [`midnight-natives.ss`](./midnight-natives.ss); the codegen routes
  them through `runtime-rs::std_lib`. Don't hard-code stdlib names in
  the walker — extend the lookup table in `rust-passes-helpers.ss`.
- Section comments inside the large modules (`walker.ss`, `emit.ss`)
  follow a `;; --- NAME ----------------------------------------` form
  so they show up in editor outlines. Add one whenever you introduce
  a new logical section.

## When something breaks

- **Codegen builds but emits malformed Rust.** Run `./result/bin/compactc
  --rust --skip-zk examples/<failing>.compact /tmp/regen/` and read
  `/tmp/regen/contract/lib.rs`. Diff against a known-good fixture.
- **Codegen Scheme doesn't compile.** Most often: pattern-matching against
  a non-terminal that's been removed at the current language layer.
  Check [`langs.ss`](./langs.ss) for `(- (form ...))` markers that
  remove forms in the language hierarchy.
- **`codegen_regression` test fails.** The committed `lib.rs` in
  `tests-e2e-rust/contracts/<name>-fixture/lib.rs` has drifted from
  what `compactc` now emits. Regenerate and inspect the diff — either
  fix the Scheme or update the committed `lib.rs` (whichever reflects
  the intended change).
