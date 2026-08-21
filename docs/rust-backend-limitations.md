# `compactc --target rust` — known limitations

What the Rust backend does *not* lower, and what happens when you hit it.

The TypeScript backend is the reference; the Rust backend targets the same
`ContractState` bytes but covers a subset of the language. This page exists
so that subset is discoverable before you write a contract, rather than at
`compactc` time — or, worse, at `cargo build` time.

## The contract: unsupported means a compile error, never bad output

Every construct the backend cannot lower raises a `rust-feature-error`,
which fails the compile with a named diagnostic and a source location.
This is the single most important property of the backend, and the one to
preserve when adding lowerings:

> A construct the emitter does not understand must **FAIL**, not emit
> something plausible.

That rule was not free. Four places used to violate it, and each was a
different flavour of the same mistake:

- **`Field as Uint<N>`** shared an emitter clause with the `Uint`-source
  cast, so it emitted Rust referencing a conversion that does not exist:
  `compactc` exited 0, the fixture regenerated "fine", and the breakage
  appeared later at `cargo build`.
- **A constructor `assert` whose condition was a non-inlinable circuit
  call** emitted `assert!(true)`. A precondition silently disappeared, in
  code that compiles and reads correctly.
- **A constructor `if` in the same situation** emitted `if true`, making
  the then-branch unconditional and the else-branch dead — so a
  conditional ledger write became unconditional.
- **Four natives with no Rust binding** (`keccak256`, `ownPublicKey`,
  `createZswapInput`, `createZswapOutput`) emitted `unimplemented!()` into
  the generated crate. Those compiled cleanly and panicked at run time.

All four now reject at compile time. If you are adding a lowering and find
yourself unsure what the correct output is, raise a `rust-feature-error`
rather than emit a guess — the guess is much more expensive to find later
than the error is to hit now.

Diagnostics carry the source location and a kind symbol:

```
probe.compact line 11 char 10:
  compactc --target rust: unsupported Compact construct (ctor-assert-condition-inline):
  cannot inline the call to `readFlag` used as an `assert` condition in a constructor
```

## Enumerating the current set

The authoritative list is the code, not this page — it moves as lowerings
land. To count the rejection sites the same way:

```bash
grep -rn "(rust-feature-error" compiler/rust-passes*.ss \
  | grep -v "define (rust-feature-error" | wc -l
```

At the time of writing that is **28 call sites** across 4 passes
(`rust-passes-emit.ss` 22, `rust-passes-walker.ss` 4,
`rust-passes-helpers.ss` 1, `rust-passes-prelude.ss` 1), spanning **24
distinct kinds**.

## The limitations most likely to affect a contract

### Natives with no Rust binding

`native-binding-missing`. Four natives have no Rust implementation to bind
to, and calling any of them fails the compile:

| Native | Why |
|---|---|
| `keccak256` | no upstream Rust binding — midnight crypto exposes keccak only as a ZK circuit chip |
| `ownPublicKey` | host-side zswap witness; needs a `WitnessContext`-mediated binding |
| `createZswapInput` | as above |
| `createZswapOutput` | as above |

These are represented in `compiler/midnight-natives.ss` by the *absence*
of a `(rust …)` clause, which leaves the field `#f` and makes
`native-call-site-rust` reject. That is deliberate: the absence of a
binding is now representable as absence, rather than as a string that
happens to contain `unimplemented!()`.

Note `keccak256` is also gated earlier under ZKIR v2 — you will see the
ZKIR diagnostic first unless you pass `--feature-zkir-v3`.

### Constructor conditions that need inlining

`ctor-assert-condition-inline`, `ctor-if-condition-inline`. A constructor's
`assert` or `if` condition must be resolvable at emission time — either a
directly renderable expression, or a circuit call the emitter can inline.
An impure, non-exported circuit call generally cannot be inlined, and is
rejected:

```compact
circuit readFlag(): Boolean { return count.read() > 0; }

constructor() {
  assert(readFlag(), "flag must be set");   // rejected
}
```

**Workaround:** inline the condition yourself (`assert(count.read() > 0, …)`),
or move the check out of the constructor into an exported circuit.

Many conditions that *look* like this are fine, because the constructor
evaluates ledger reads against the known initial state and folds the
condition. Reach for the workaround only when you actually see the
diagnostic.

### `Field as Uint<N>` — no lowering

`cast-from-field`. Narrowing a `Field` to a bounded unsigned integer needs
a runtime helper that range-checks an `Fr` and narrows it to `uN`; the TS
runtime does the equivalent with a bigint bounds check. Until that helper
exists, the cast is rejected.

**Workaround:** keep the value in `Field` where possible, or perform the
narrowing in a witness (off-circuit) and pass the `Uint<N>` in.

### Generic impure circuits — body not lowered

A generic *impure* circuit's body is not lowered. The case that matters in
practice — the Schnorr-on-Jubjub verifier — is handled by rewriting the
call to an orphan-safe runtime wrapper
(`midnight_compact_runtime::schnorr_verify_jubjub`) rather than by lowering
the generic body; see `runtime-rs/src/std_lib/schnorr.rs` and
`schnorr_attest_fixture`. Generic *pure* circuits are supported.

### Fixed-arity hash and commit natives

`transient-hash-arity`, `persistent-hash-arity`,
`persistent-commit-arity`. Lowered for the arities the corpus exercises;
other arities reject.

### Ledger-read shapes

`ledger-read-decoder-missing`, `ledger-read-non-index-path`,
`ledger-op-non-read`, `adt-read-with-arg-lowering`. Reads are lowered
per-ADT with alignment-aware decoding; an ADT or access path without a
decoder rejects rather than guessing a layout. This is the class most
likely to meet a new contract shape, and the cheapest to fix — usually a
new decoder arm plus a fixture.

### `Map` MVP shape

`map-mvp-shape`. `map()` is lowered for the shapes the fixtures cover
(identity body, non-identity lambda, named function); other shapes reject.

## Verifying a claim on this page

Every limitation above is a `rust-feature-error` site, so it can be
confirmed by compiling a contract that uses the construct:

```bash
nix build .#compactc
./result/bin/compactc --target rust --skip-zk path/to/probe.compact /tmp/out
```

A limitation that does *not* produce a diagnostic is itself a bug — that is
the silent-bad-output path this page's contract exists to forbid. If you
find one, it belongs in the same category as the four listed above, not in
a TODO comment.
