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

That rule was not free, and it was not won in one pass. Every item below
is a place that used to violate it — each a different flavour of the same
mistake, and each found by hand rather than by a gate:

- **`Field as Uint<N>`** shared an emitter clause with the `Uint`-source
  cast, so it emitted `(x) as uN` where `x: Fr`. `Fr` is a struct, so that
  is a non-primitive cast (E0605): `compactc` exited 0, the fixture
  regenerated "fine", and the breakage appeared later at `cargo build`.
  On the ledger-9 line this one is structurally impossible to reintroduce:
  0.33 split `Field as Uint<N>` into its own `cast-from-field` IR form and
  made `downcast-unsigned` require an unsigned source, so the two casts no
  longer share a clause at all.
- **A constructor body that no walker shape matched** emitted the *default
  scaffold* — every ledger write in the constructor silently discarded,
  from a compile that exited 0. Not a build break at all: well-formed Rust
  that compiled, ran, and deployed a contract with none of its initial
  state. The worst of the set, and the last found.
- **A ledger field with no decoder** emitted `decode_u64` behind a `TODO`
  comment, so a `Vector<3, Bytes<32>>` accessor returned a `u64` in tail
  position of `Result<[[u8; 32]; 3], _>`. Reachable with one ledger field,
  no circuits, and no constructor.
- **A constructor `assert` whose condition was a non-inlinable circuit
  call** emitted `assert!(true)`. A precondition silently disappeared, in
  code that compiles and reads correctly.
- **A constructor `if` in the same situation** emitted `if true`, making
  the then-branch unconditional and the else-branch dead — so a
  conditional ledger write became unconditional.
- **Four natives with no Rust binding** (`keccak256`, `ownPublicKey`,
  `createZswapInput`, `createZswapOutput`) emitted `unimplemented!()` into
  the generated crate. Those compiled cleanly and panicked at run time.

All of them now reject at compile time, and
`tests-e2e-rust/tests/rejection_corpus.rs` keeps them rejecting. If you are
adding a lowering and find yourself unsure what the correct output is,
raise a `rust-feature-error` rather than emit a guess — the guess is much
more expensive to find later than the error is to hit now.

**Note how each was found: by reading the emitter, not by a failing
test.** Byte parity structurally cannot catch this class. It compares
committed output against regenerated output, so a construct that emits bad
Rust agrees with itself perfectly and the fixture stays green forever.
Every bug above shipped behind a green byte-parity run. That is what the
negative corpus exists to cover, and why a new lowering should add an entry
there rather than only a fixture.

Diagnostics carry the source location and a kind symbol:

```
Field-to-Uint cast (`as Uint<8>`) has no Rust lowering;
a range-checking midnight_compact_runtime helper is needed
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

At the time of writing that is **34 call sites** across 4 passes
(`rust-passes-emit.ss` 30, `rust-passes-walker.ss` 2,
`rust-passes-helpers.ss` 1, `rust-passes-prelude.ss` 1), spanning **27
distinct kinds**.

One caveat when reading a diagnostic: several emitters probe alternative
shapes under a catch-all `(guard (c [#t #f]) …)`, which swallows a specific
`rust-feature-error` and reports the enclosing body's generic kind instead
(`pure-circuit-body-emission`, `ctor-body-emission`). The refusal is still
correct — nothing is emitted — but the message can be coarser than the
kind that actually fired. `Field as Uint<N>` is the current example: it
reports `pure-circuit-body-emission`, not `cast-from-field`.

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

### Constructor bodies past the walker's shapes

`ctor-body-emission`. The constructor is lowered by shape-matching, not by
a general statement compiler, so a body that combines collection seeding
with control flow can fall outside every shape:

```compact
constructor() {
  names.insert(1, 100);
  for (const i of 0..3) { total = (total + 7) as Uint<64>; }   // rejected
}
```

**Workaround:** move the loop into an exported circuit called once after
deploy, or unroll it in the constructor.

This is the one limitation worth knowing about even if you never hit it,
because until recently it was not a limitation at all — it emitted the
default scaffold and threw the constructor away. If you are on an older
build, check that your deployed initial state is what you wrote.

### `Field as Uint<N>` — no lowering

`cast-from-field`. Narrowing a `Field` to a bounded unsigned integer needs
a `midnight_compact_runtime` helper that range-checks an `Fr` and narrows it to
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
(`midnight_compact_runtime::schnorr_verify_jubjub`) rather than by lowering the
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
