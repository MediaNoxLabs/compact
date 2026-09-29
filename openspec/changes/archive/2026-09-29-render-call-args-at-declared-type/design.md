# Design

## Context

See proposal.md - Why. How the defect arises:

```
typechecker: maybe-safecast(declared, actual, e)       (analysis-passes.ss ~L1827)
                 |
     sametype? --+-- yes --> e unwrapped   <-- no expected type reaches the emitter
                 |
                 +-- no  --> (safe-cast declared actual e)
                                 |
                                 v
emitter: expected = expr-expected-type(arg)   (#f when unwrapped)
         ctor-expr-rust / expr-rust with expected #f
           --> expr-strip-cast on inner nodes  --> casts inside the aggregate are dropped
```

An argument that is left unwrapped has the formal type by construction, but "same type" is modulo `sametype?`: `Vector<2, Field>` and `[Field, Field]` count as the same type yet lower to different Rust types (the compact#83 bridge). What is missing is the Rust type the callee's signature uses, which the declared formal type supplies.

Probe on 0.31.121 (`[0 as Field, 1 as Field]` into `Vector<2, Field>`):

| Call site | Emitted | Effect |
|---|---|---|
| pure circuit (pure / impure body) | `pure_circuits::pv([0, 1])?` | build fails |
| witness | `self.witnesses.wv(&ctx, [0, 1])` | build fails |
| `transientHash` / `persistentHash` | `AlignedValue::from(0)` | build fails: `AlignedValue` needs `Aligned` plus `Value: From<_>`, which exist for `u8..u128`, `bool` and `HashOutput` only |
| `persistentCommit<Field>(0 as Field, o)` | `persistent_commit(&0, …)` | **builds, wrong value**: `T: BinaryHashRepr` holds for all integers (`midnight-base-crypto` `repr.rs:128`), so Rust infers `i32` and hashes 4 LE bytes |
| ternary arms, ledger write | `Fr::from(0u64)` | correct (context supplies the type) |

Argument render sites. 31 were inventoried; all have the callee's function-name id in scope, and all except `persistentCommit` and `inline-circuit-call` take the expected type from the argument's own `safe-cast`:

- `rust-passes-emit.ss` `call-rust`: `some`/`none`, `persistent_hash`, `transient_hash`, `persistent_commit` (plain `expr-rust`, no expected type at all), 1:1 natives, user pure circuits.
- `rust-passes-walker.ss`: `ctor-call-rust` (stdlib, pure), `inline-circuit-call` (plain `ctor-expr-rust`; the formal type is in scope but used only for enums), flat-body `const =` / bare witness, pure and impure calls, `emit-hoisted-impure-calls`, `emit-hoisted-witnesses`.
- `rust-passes-streaming.ss`: `const =` / bare witness, pure and impure calls, mid-branch and terminal impure calls (A17/A26).
- Only the pure-circuit `const` binding in both walkers looks up formals (`circuit-formal-arg-types`), and only to decode an enum read from the ledger.

Tables: `build-circuit-id-ht` and `build-witness-id-ht` map id → declaration; `build-native-id-ht` (`rust-passes-prelude.ss:182`) maps id → native-entry record only, dropping the monomorphised `(native src fn native-entry (arg* ...) type)` declaration (`langs.ss:540`).

## Goals / Non-Goals

**Goals:**
- One decision point that pairs each call argument with its callee's formal type, used by every call site and enforced by a test.
- No byte change for any argument that renders correctly today.

**Non-Goals:**
- Changing `arg-rust-clone-if-var` / `pure-call-arg-rust` behaviour for their non-call users (ledger ADT operation arguments, cell-write values).
- Adding clone behaviour where a site has none today (`inline-circuit-call`).

## Decisions

### D1. Fix at the call site, not at the leaf

Render each argument with expected type `(or (expr-expected-type e) formal)`.
- When a wrapper exists, its target is the declared type (`maybe-safecast` wraps with `declared-type`), so putting the wrapper first is equivalent and keeps wrapped arguments byte-identical by construction.
- Alternative, rejected: make an unwrapped-context `safe-cast` render at its own target. That fixes #21 everywhere but not #90 (that literal has no wrapper), collides with the rule that unsigned binary operands are not pushed an expected type (double cast), and has a far wider byte-stability surface.

### D2. A new entry point, `call-args-rust fn-id expr*`

It looks up the formals and, for each argument, binds the expected type as in D1, then applies the existing clone decision, on both the `expr-rust` side and the walker side. The two copies of that decision (`pure-call-arg-rust` and the body of `arg-rust-clone-if-var`) are merged into one shared `clone-if-var-rust`. Every site in the inventory switches to the entry point. `render-pure-circuit-arg`'s enum ledger-read decode is kept as a case inside the walker variant.
- As implemented, `pure-call-arg-rust`, `render-pure-circuit-arg` and `circuit-formal-arg-types` lose their last callers and are removed. `arg-rust-clone-if-var` remains, for the non-call users only.
- Alternative, rejected: add a `formal-type` parameter to `arg-rust-clone-if-var` / `pure-call-arg-rust`. That leaves the call sites free to omit it, and changes a helper shared with ADT and cell-write sites.

### D3. One `callee-formal-types` table

id → list of formal types, built once next to the existing tables (`build-callee-formal-types`, `rust-passes-prelude.ss`, bound in `rust-passes.ss` before anything renders) from native, witness and circuit declarations, and exposed as a parameter like `current-circuit-id-ht`. Native declarations are monomorphised, so generic natives (`persistentHash<A>`, `hashToCurve<A>`) get concrete types with no instantiation logic.
- An id with no entry, or an arity mismatch, falls back to `expr-expected-type` alone (today's behaviour). It never refuses: a refusal at a TS-accepted position is a defect under the parity requirement.
- Alternative, rejected: store the declaration in `native-id-ht` and add per-kind accessors. That means three lookups and touches `native-id-ht`'s existing users.

### D4. Special-case natives

- `persistentCommit`: value rendered at its formal type (the instantiated `A`), keeping the `&` borrow; the opening rendered at `Bytes<32>` inside `HashOutput(...)`. The current plain `expr-rust` also inherits any enclosing expected type; the typed render clears that.
- `persistentHash` / `transientHash`: `native-vector-aligned-slice` passes the formal type as `native-vector-atoms`' target when the argument has no wrapper, so a same-type vector's leaves are coerced element-wise. The existing `expr-value-type` fallback stays.

### D5. `inline-circuit-call` gets the expected type only

Each actual is rendered with the formal as its expected type; the enum toggle (`current-enum-ref-typed?`) is unchanged. The site gets no `.clone()` logic: adding it would change bytes with no bug behind it.

### D6. Byte stability is enforced, not assumed

A same-type argument now gets an expected type equal to its own type. Only three renderers act differently when an expected type is set:
- the `quote` clause (width suffix, only when the formal is exactly `Uint<0..N>`);
- the `tuple` clause (the #21 fix);
- `aggregate-kind-bridge` (the #83 fix).

The whole `FIXTURES` corpus is regenerated and every diff is classified. A previously correct fixture that changes is a defect in this change: the renderer is adjusted to keep its bytes, for example with a peel like the one in `coerce-cmp-operand-rust`.

### D7. The #90 fallback stays; its redundancy is shown once

During implementation the `(> datum (max-unsigned))` branch is removed temporarily, the #90 fixtures are regenerated and must be byte-identical, then the branch is restored. The result is recorded in tasks.md.
- Alternative, rejected: a permanent test-mode switch. It would guard only Field-only literals, not the common #21 regression; it would need plumbing to tell call positions from other positions without an expected type; and it would add a test-only branch to the emitter.

### D8. Drift is guarded structurally

A Rust test reads `compiler/rust-passes-*.ss` and fails if `arg-rust-clone-if-var`, `clone-if-var-rust` or `pure-call-arg-rust` is called outside `call-args-rust` and a named allowlist (ledger ADT operation arguments and cell-write values for `arg-rust-clone-if-var`; `arg-rust-clone-if-var` itself for `clone-if-var-rust`). `pure-call-arg-rust` is removed, and it stays guarded so it cannot come back. A new call site that bypasses the entry point then fails `cargo test`, whatever literal it happens to receive.
- Limit: the guard sees only these renderers. A site that renders arguments with `expr-rust-typed` / `ctor-expr-rust` directly, and does no clone decision, is not detected. Such a site is caught behaviourally instead, by the fixture's build and parity tests, when its call kind is covered.

### D9. Tests are behavioural, through TS state parity

The existing `persistentCommit` parity (the dogfood contract) passes only typed locals (`Bytes<32>`, `Bytes<64>`, `Uint<32>`), whose Rust type is fixed, so it cannot see this bug. The new fixture has exported impure circuits that write each result to the ledger, covering:
- the #21 vector at each call kind;
- `persistentCommit<Field>` with an `N as Field` value at or below and above `max-unsigned`;
- the hash natives;
- the #90 literal as an argument;
- the tuple/vector bridge.

A `capture-*.mjs` script records the TS state, and the Rust test compares state bytes, so a wrong commitment fails even though it builds.

## Risks / Trade-offs

- [The regen changes a previously correct fixture (e.g. a width suffix on an exact-`Uint<0..N>` literal argument)] → D6: treated as a defect and fixed in the renderer; blocks the change until resolved.
- [The text-level guard (D8) is brittle to renames or formatting] → It fails loudly, never silently; the allowlist names functions, not line numbers.
- [A callee missing from the table silently keeps today's broken rendering] → The new fixture covers every call kind, so a missing kind shows up as a build or parity failure there.
- [Ledger ADT / cell-write arguments have the same same-type gap] → Out of scope. They are probed during implementation and a follow-up issue is filed if they break.
- [The TS capture drifts with the TS runtime version] → Same exposure as the existing captures; regenerated with the pinned toolchain.
