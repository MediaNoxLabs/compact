# `compactc --rust` — known limitations

What the Rust backend does *not* lower, and what happens when you hit it.

The TypeScript backend is the reference; the Rust backend targets the same
`ContractState` bytes but covers a subset of the language. This page exists
so that subset is discoverable before you write a contract, rather than at
`compactc` time — or, worse, at `cargo build` time.

## The contract: unsupported means a compile error, never bad output

Every construct the backend cannot lower raises a `rust-feature-error`,
which fails the compile with a named diagnostic. This is deliberate and it
is the property to preserve when adding lowerings:

> A construct the emitter does not understand must FAIL, not emit something
> plausible.

That rule was learned the hard way. `Field as Uint<N>` used to share an
emitter clause with the `Uint`-source cast, so it emitted Rust that
referenced a nonexistent conversion: `compactc` exited 0, the fixture
regenerated "fine", and the breakage only appeared later at `cargo build`.
It now rejects at compile time. If you are adding a lowering and find
yourself unsure what the right output is, emit a `rust-feature-error`
instead of a guess.

Diagnostics are raised with the source location and a kind symbol, e.g.:

```
Field-to-Uint cast (`as Uint<8>`) has no Rust lowering;
a range-checking compact_runtime helper is needed
```

## Enumerating the current set

The authoritative list is the code, not this page — it moves as lowerings
land. To enumerate every rejection site and its kind:

```bash
grep -rn "rust-feature-error" -A3 compiler/rust-passes*.ss
```

At the time of writing that is **31 call sites** across 4 passes
(`rust-passes-emit.ss` 27, `rust-passes-walker.ss` 2,
`rust-passes-helpers.ss` 1, `rust-passes-prelude.ss` 1), spanning 27
distinct kinds. To count them the same way:

```bash
grep -rn "(rust-feature-error" compiler/rust-passes*.ss \
  | grep -v "define (rust-feature-error" | wc -l
```

## The limitations most likely to affect a contract

### `Field as Uint<N>` — no lowering

`cast-from-field`. Narrowing a `Field` to a bounded unsigned integer needs
a `compact_runtime` helper that range-checks an `Fr` and narrows it to
`uN`; the TS runtime does the equivalent with a bigint bounds check. Until
that helper exists the cast is rejected.

**Workaround:** keep the value in `Field` where possible, or perform the
narrowing in a witness (off-circuit) and pass the `Uint<N>` in.

No fixture exercises the shape, so adding the helper is expected to be
byte-parity-neutral for the existing corpus.

### Generic impure circuits — body not lowered

A generic *impure* circuit's body is not lowered. The one case that matters
in practice, the Schnorr-on-Jubjub verifier, is handled by rewriting the
call to an orphan-safe runtime wrapper
(`compact_runtime::schnorr_verify_jubjub`) rather than by lowering the
generic body — see `runtime-rs/src/std_lib/schnorr.rs` and the
`schnorr_attest_fixture`. Generic *pure* circuits are supported.

### Non-native / unbound calls

`native-binding-missing`, `non-native-call`. A call to a standard-library
native with no Rust binding is rejected rather than stubbed. Some natives
are deliberately bound to `unimplemented!()` in `midnight-natives.ss`,
which shifts the failure to runtime rather than compile time — a wart
tracked as part of the upstreaming hygiene review (#22).

### Fixed-arity hash and commit natives

`transient-hash-arity`, `persistent-hash-arity`, `persistent-commit-arity`.
These are lowered for the arities the corpus exercises; other arities
reject.

### Ledger-read shapes

`ledger-read-decoder-missing`, `ledger-read-non-index-path`,
`ledger-op-non-read`, `adt-read-with-arg-lowering`. Reads are lowered
per-ADT with alignment-aware decoding; an ADT or access path without a
decoder rejects rather than guessing a layout. This is the class most
likely to bite a new contract shape, and the one most cheaply fixed —
usually a new decoder arm plus a fixture.

### Arithmetic on an unsupported result type

`arith-result-type`, `field-arith-operator`. `+`/`-`/`*` are lowered for
bounded unsigned results (cast to the result width, then `wrapping_*`) and
for `Field` results (plain `+`/`-`/`*`, since field arithmetic is modular
in the field characteristic and `Fr` implements those operators but not
`wrapping_*`). Any other result type rejects.

This branch used to be the backend's clearest example of the silent-bad-
output failure mode: field arithmetic fell through to `wrapping_*` on two
`Fr`s, so `compactc` exited 0 and `cargo build` failed later, from Compact
as ordinary as `return a + b` on two `Field`s. No fixture covered the
shape. It is now lowered correctly, and the residual unknown-type case
refuses instead of guessing.

### `Map` MVP shape

`map-mvp-shape`. `map()` is lowered for the shapes the fixtures cover
(identity body, non-identity lambda, named function); other shapes reject.

## Verifying a claim on this page

Every limitation above is a `rust-feature-error` site, so it can be
confirmed by compiling a contract that uses the construct:

```bash
nix build .#compactc
./result/bin/compactc --rust --skip-zk path/to/probe.compact /tmp/out
```

A limitation that does *not* produce a diagnostic is a bug in itself — that
is the silent-bad-output path this page's contract exists to forbid.
