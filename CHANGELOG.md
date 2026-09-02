# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

_No changes yet._

## [Toolchain 0.34.111, language 0.26.0, runtime 0.19.100] — split the emitter by capability (2026-09-02)

### Changed

- **`rust-passes-emit.ss` (3,397 lines) is split into ten files by capability**,
  the same treatment `rust-passes-walker.ss` got in 0.34.108 and by the same
  method: textual, into the shared `(definitions ...)` block, with byte parity
  over the fixture corpus as the proof that nothing moved.

  | File | Lines | Defines |
  |---|---:|---:|
  | `-scaffold` — initial-state scaffold, circuit arg lists | 221 | 4 |
  | `-structs` — struct/Maybe helpers, struct literals | 131 | 6 |
  | `-stmt-shapes` — statement shape extraction | 435 | 19 |
  | `-vm` — vm-value lowering, op-program builder calls | 539 | 10 |
  | `-impure` — public-ledger bodies, impure circuits | 418 | 9 |
  | `-arith` — arithmetic, casts, width selection | 279 | 8 |
  | `-expr` — the expression renderer | 376 | 1 |
  | `-calls` — ledger reads and call sites | 510 | 6 |
  | `-pure` — pure circuit bodies | 275 | 3 |
  | `-view` — ledger view, decoders, defaults, manifest | 448 | 17 |

  Unlike the walker, every one of these is PR-sized — there is no equivalent
  of `-body`'s 1,002-line outlier, because `expr-rust` at 388 lines fits a
  file on its own.
## [Toolchain 0.34.110, language 0.26.0, runtime 0.19.100] — two defects in compact-test.yml (2026-09-02)

### Fixed

- **The Cargo caches in `compact-test.yml` keyed on a file that does not
  exist.** All three cache steps used
  `hashFiles('tools/compact/Cargo.lock')`, and the target cache used
  `path: tools/compact/target`. Neither path exists — `Cargo.lock` and
  `target/` both live at the repository root, because `tools/compact` is a
  workspace member rather than a standalone crate.

  `hashFiles` returns an empty string when nothing matches, so the key was the
  constant `<os>-cargo-`: it never invalidated when the lockfile changed, and
  the target cache had nothing to save. Both now key on the root `Cargo.lock`,
  and the target cache points at the root `target/`.

- **The `paths:` filter could not catch workspace regressions.** It listed only
  `tools/compact/**`, but that crate is a member of the root workspace and
  inherits `version` / `edition` / `rust-version` from `[workspace.package]`,
  so its build depends on files outside its own directory.

  A change to the root manifest or lockfile that breaks this crate's dependency
  resolution therefore did not trigger the job that would catch it — the
  breakage surfaces later, on an unrelated contributor's PR, where it looks
  like their fault. `Cargo.toml` and `Cargo.lock` are now in the filter.

  This is not hypothetical: it is exactly how a rustls feature conflict
  introduced by a root-workspace change went unnoticed here until it was
  tracked down by hand.

## [Toolchain 0.34.109, language 0.26.0, runtime 0.19.100] — forward `nominal?` unconditionally (2026-09-02)

### Fixed

- **`apply-type-alias` now forwards the alias's `nominal?` flag** in
  `expand-modules-and-types.ss`, instead of the hardcoded `#f` it used to pass.

  `nominal?` is in scope, bound by the enclosing `Info-type-alias` pattern, and
  the other two `apply-type-alias` call sites already forward it. This one did
  not. `apply-type-alias` puts the flag straight into
  `(talias ,alias-src ,nominal? ...)`, and `sametype?` opens its `talias`
  clause with `(assert nominal1?)` — so passing `#f` makes the IR assert a
  falsehood about the type's identity.

  It never fires today only because `already-exported?` compares `Info`s rather
  than types, so the mis-flagged node never reaches `sametype?`. That is luck,
  not design: a change routing export-typedef types through `sametype?` turns
  it into an assertion failure.

### Changed

- **The fix is no longer gated on `(emit-rust)`.** It was written as
  `(and (emit-rust) nominal?)` on the theory that changing shared IR might
  perturb the TypeScript pipeline.

  Measured rather than assumed: with the gate removed, TypeScript output is
  **byte-identical across all 37 contracts** in the two `examples/` trees,
  compared against a compiler built from pristine `upstream/main`. The gate was
  protecting against nothing, and it obscured that this is a plain bug fix that
  belongs upstream on its own merits — where the `#f` is still present.

## [Toolchain 0.34.108, language 0.26.0, runtime 0.19.100] — split the walker by capability (2026-09-02)

### Changed

- **`rust-passes-walker.ss` (3,792 lines) is split into eight files by
  capability.** No behavioural change: every one is `include`d into the same
  `(definitions ...)` block in `rust-passes.ss`, where internal defines are
  mutually recursive, so the grouping carries no ordering constraint. Byte
  parity over the fixture corpus is what proves that rather than assertion.

  | File | Lines | Defines |
  |---|---:|---:|
  | `-tables` — witness/circuit lookup, enum coercion | 207 | 11 |
  | `-ctor-expr` — constructor-context expression rendering | 516 | 12 |
  | `-support` — the "can this be lowered" predicates | 589 | 15 |
  | `-hoisting` — impure and witness call hoisting | 303 | 8 |
  | `-body` — walkability and the body/ctor dispatchers | 1,002 | 7 |
  | `-stmt` — statement classification | 404 | 15 |
  | `-branches` — if/else analysis, public-ledger call lines | 294 | 4 |
  | `-terminals` — writes, mutations, loops, if/else | 662 | 11 |

  This is a precondition for upstreaming rather than housekeeping. A
  3,792-line file cannot be sent to a project whose median merged PR is
  ~100–150 lines; seven of these eight can.

  `-body` is the exception at 1,002 lines, because it still contains
  `emit-body-or-fallback` (568 lines in one define). Decomposing that is
  MediaNoxLabs/compact#40 and is a semantic change, deliberately kept out of a
  refactor whose whole claim is that it changes nothing.

## [Toolchain 0.34.107, language 0.26.0, runtime 0.19.100] — Schnorr verification delegates to the ledger (2026-09-01)

### Changed

- **`std_lib::schnorr::verify` now calls
  `midnight_transient_crypto::schnorr::verify`** instead of repeating it.
  The module header explained that the verifier was vendored because the
  pinned transient-crypto 2.1.0 exposed no `schnorr` module, and that the copy
  could go once upstream shipped one. On the ledger-9 line it has: this crate
  resolves transient-crypto **3.0.0**, whose implementation matches ours in
  every respect that matters — same Poseidon challenge over
  `[ann_x, ann_y, pk_x, pk_y, ..msg]`, same reduction mod `r_jubjub`, same
  verification equation, and the same up-front identity rejection.

  The gain is provenance, not size: the security-critical path is upstream's
  implementation rather than our transcription of it. A copy that agrees today
  is a copy that can silently stop agreeing.

  The signature type stays local — ours declares `response: Fr` because
  Compact declares that field `Field`, upstream's declares `EmbeddedFr` — so
  the reduction is applied at the call boundary.

### Added

- **Four tests on a path that had none.** This branch carried no Schnorr
  fixture and no Schnorr test at all, so the verifier was being changed with
  zero coverage. They pin the security property directly: an identity public
  key is rejected, an identity announcement is rejected, and
  `JubjubPoint::default()` is the identity — which is what makes the guard
  reachable rather than theoretical, since an unwritten ledger key cell holds
  exactly that value.

  The forgery is constructed explicitly rather than described: with `pk = O`,
  `pk·c` is `O` for every challenge, so `s·G == R + pk·c` collapses to
  `s·G == R`, which any `(s, s·G)` pair satisfies for any message with no
  secret key.

- **A test pinning a deliberate difference.** `jubjub_schnorr_verify` mirrors
  the 0.33 standard library's circuit, which performs **no** identity
  rejection, so it *accepts* the same forgery `verify` refuses. Routing it
  through upstream would have made the Rust path disagree with the circuit it
  exists to match — a new divergence rather than a fix. The test asserts that
  weakness on purpose, so nobody removes the difference by tidying it away.

## [Toolchain 0.34.106, language 0.26.0, runtime 0.19.100] — the streaming pass no longer emits a comment as an expression (2026-09-01)

### Fixed

- **Three catch-all handlers in the streaming pass substituted a comment
  string for an expression.** `(guard (c [#t "/* TODO A24 */"]) …)` and two
  `A14` siblings caught *any* failure while lowering a constructor
  let-binding and handed back the comment text, which flowed straight into

  ```rust
  let some_name = /* TODO A24 */;
  ```

  That is not a degraded lowering, it is a syntax error — emitted from a
  compile that exited 0, so the break landed at `cargo build` in generated
  code with no pointer back to the Compact source. The same shape as
  MediaNoxLabs/compact#45, in the one pass the earlier audit had not reached.

  They now raise `ctor-lifted-binding-emission`. Nothing relied on the
  fallback: the whole suite is unchanged by the switch, which is the
  expected result, since the fallback could only ever have produced code
  that failed to build.

  The pass's other five catch-alls return `#f` rather than a string, and `#f`
  propagates to a rejection, so they were already failing closed and are left
  alone.

## [Toolchain 0.34.105, language 0.26.0, runtime 0.19.100] — pin the boundaries the #51 sweep found (2026-09-01)

### Added

- Three more entries in the negative corpus, from a systematic sweep rather
  than a lucky find. #51 turned up by comparing one lowering against
  TypeScript's; this enumerates **every** value-check the TypeScript emitter
  generates and asks what Rust does with each.

  There are three: the narrowing cast (that was #51, now fixed), and both
  directions of enum cast. **Rust refuses both enum casts**, so there is no
  second divergence there — but the corpus now says so, and anyone adding an
  enum-cast lowering has to delete an entry to do it, which is the moment to
  remember the bound.

- `Uint<128>` arithmetic, pinned on both sides, and this one is load-bearing.

  Rust lowers `a + b` as `wrapping_add` on operands widened to the next width
  up, then range-checks the narrowing back down. Below u128 that composes
  correctly: `u64 + u64` cannot lose anything at u128, so the check sees the
  true sum and agrees with TypeScript's exact bigints.

  At u128 there is no next width, so a `wrapping_add` would wrap *before* the
  check, the check would see an in-range value, and Rust would silently
  return a wrapped result where TypeScript throws — #51 again, in a form the
  narrowing fix cannot catch because the information is already gone. The
  backend refuses instead, and the corpus records that `Uint<128>` values
  themselves still work, so the guard is not widened by accident.

## [Toolchain 0.34.104, language 0.26.0, runtime 0.19.100] — narrowing casts agree with TypeScript again (2026-09-01)

### Fixed

- **`x as Uint<N>` silently returned wrong values instead of failing.**
  TypeScript lowers a narrowing cast to a bounds check that throws; Rust
  lowered it to `as`, which checks nothing. The two disagreed on every
  out-of-range value, in two different ways (MediaNoxLabs/compact#51):

  - `as` widens the accepted range to the **Rust** type's bound rather than
    the Compact one. `Uint<0..100>` becomes `u8`, so `100..=255` was returned
    unchanged — outside the declared type, and not truncated, so nothing made
    it visible.
  - past that it truncates. `shrink(300)` returned `Ok(44)`: an out-of-range
    input became a plausible in-range output, which is worse than an error and
    worse than a panic.

  TypeScript is normative, so both were Rust bugs. Narrowing now goes through
  `compact_runtime::std_lib::narrow`, which takes the Compact bound and
  reproduces TypeScript's message verbatim, so a contract running on both
  backends reports one story rather than two.

  `?` is safe in every position this renders into — circuit, pure-circuit and
  constructor bodies all return `Result<_, CompactError>`, `initial_state`
  included, and 38 existing `?` uses in constructor bodies already relied on
  that.

### Added

- `CompactError::CastFailed`, distinct from `AssertionFailed`. A failed cast
  is not a user `assert`, and reporting it as one misattributes the failure to
  contract code that does not exist.

- `examples/narrowing_fixture.compact` and its **executing** test. Byte parity
  is structurally blind to this class: it compares committed Rust against
  regenerated Rust, and `(x) as u8` agrees with itself perfectly — the
  disagreement exists only at run time, for inputs no fixture fed it. The test
  asserts on the specific wrong answers (`shrink(300)` must not be `44`,
  `shrink_wide(70000)` must not be `4464`), so a regression fails on the value
  rather than only on the `Result` shape.

  Verified by reverting the emitter to `as`: 5 of the 6 assertions fail, with
  `shrink(300) returned Ok(Some(44))`, while the in-range test keeps passing —
  so they are specific to the bug rather than failing everything.

## [Toolchain 0.34.103, language 0.26.0, runtime 0.19.100] — constructor miscompiler guard and the negative corpus (2026-09-01)

### Fixed

- **`--rust` silently discarded every constructor write** when no walker shape
  matched the constructor body. `emit-initial-state` treated "there is no
  constructor" and "there is a constructor and we could not lower it" as the
  same state, and emitted the default scaffold for both. A contract whose
  constructor seeded a `Map` and looped over a ledger cell compiled with
  **exit 0** and produced a contract that deployed with none of its initial
  state.

  That is a miscompiler, not a missing feature: the output was well-formed
  Rust that built and ran, so nothing downstream could notice. It is now a
  `ctor-body-emission` rejection.

  The discriminator is narrower than it looks. `ldecl-constructor-stmt` is
  non-`#f` even when the author wrote no constructor, because the front end
  synthesises one for any contract with ledger fields — so "a statement
  exists" does not mean "the author wrote a constructor". The guard tests
  whether the body flattens to anything; constructor-less contracts and
  explicitly empty constructors both still compile.

- **A ledger field with no decoder** fell back to `decode_u64` behind a `TODO`
  comment at the second of the two decoder sites, so a `Vector<3, Bytes<32>>`
  accessor declared `Result<[[u8; 32]; 3], _>` and returned a `u64` in tail
  position. Reachable with one ledger field, no circuits, no constructor. Both
  sites now raise `ledger-read-decoder-missing`.

- **`default-value-rust` no longer has a catch-all.** An unrecognised type fell
  through to `Default::default()`, which either fails to compile (no `Default`
  impl) or seeds a ledger cell with a value that is not the Compact default.
  `default-supported?` in the walker mirrors the handled arms and is meant to
  gate this, but it is consulted at one of the seven call sites. No contract
  reaches the arm today and removing it leaves the suite unchanged, so this
  converts defence by accident into defence by construction
  (`default-value-unsupported-type`).

### Added

- `tests-e2e-rust/tests/rejection_corpus.rs` — the negative corpus. Every other
  test in that crate pins what the backend *emits*; this pins what it must
  **refuse**, which is a distinct property and was untested. Byte parity cannot
  cover it: a construct that emits bad Rust agrees with its own committed
  fixture perfectly, so the gate stays green forever. Both bugs above shipped
  behind a green byte-parity run.

  It asserts on both sides — the constructs that must reject, and the
  neighbouring shapes that must still compile, because a rejection guard that
  is too wide is its own bug.

### Note on `Field as Uint<N>`

The ledger-8 line needed a third fix here, because `Field as Uint<N>` was
spelled `(downcast-unsigned src #f nat expr)` and shared an emitter clause with
the `Uint`-source cast, which rendered `(x) as uN` on an `Fr` — a non-primitive
cast from a compile that exited 0.

**That fix does not apply to this line and is not needed.** 0.33 split the cast
into its own `cast-from-field` production and made `downcast-unsigned` require
an unsigned source, so the two can no longer share a clause; the emitter
already raises `cast-from-field` and the walker already declines it. Ported as
a comment rather than as code, so the guard is not reintroduced as dead code.

## [Toolchain 0.34.102, language 0.26.0, runtime 0.19.100] — the Rust backend gets its own Cargo workspace (2026-08-31)

### Fixed

- **The Rust crates broke `tools/compact`'s tests by being in the same Cargo
  workspace.** `compact-runtime` depends on `midnight-base-crypto`, whose
  default features enable `reqwest/rustls` → `__rustls-aws-lc-rs` →
  `hyper-rustls/aws-lc-rs`. `tools/compact` reaches `rustls` via `ring`.
  Cargo unifies features across a workspace, so `rustls` was built with
  **both** providers, no default `CryptoProvider` could be selected, and four
  of upstream's `fetch::tests` panicked in `rustls::crypto`.

  `cargo tree -i aws-lc-rs` is misleading here — it shows only `tools/compact`
  paths, because this is feature unification rather than a package edge. The
  visible signal is the feature set: bare upstream builds `rustls` with `ring`
  alone; before this change we added `aws-lc-rs` alongside it.

  The Rust backend now lives in its own workspace rooted at `runtime-rs/`,
  with `runtime-rs-macros`, `tests-e2e-rust` and the fixture crates as
  members, and the root workspace `exclude`s all three. Members outside the
  workspace directory declare `package.workspace` so Cargo resolves them to
  the right root.

  The result is checkable rather than asserted: the root `Cargo.toml` now
  differs from upstream's by **5 lines** (the `exclude` block), the root
  `Cargo.lock` is **byte-identical to upstream's**, and `cargo test
  --workspace` at the root reproduces upstream's results exactly — including
  the one pre-existing `test_compact_check_no_param` failure, which fails the
  same way on a clean checkout of `upstream/main`.

  A narrower fix — `default-features = false` on `midnight-base-crypto` —
  was tried first and **does not work**: `cargo test -p compact --lib` passes
  under it, but `cargo test --workspace` still fails the same four tests,
  because single-package and workspace builds resolve features differently.
  Only the workspace boundary makes upstream's resolution provably untouched.

### Changed

- `compact-runtime` and `compact-runtime-macros` are versioned **0.19.100**,
  tracking the runtime version the compiler emits into
  `check_runtime_version!`. The 0.34 merge moved that pin, and the crates had
  not followed, so every generated fixture failed its own version assertion.

- The 32 committed byte-parity fixtures are repinned to runtime `0.19.100` —
  one line each, the only drift the 0.34 merge produced in generated Rust.

- `rust-runtime-test.yml` runs its cargo steps in `runtime-rs/`, and its cache
  key, cache path and `paths:` filter follow `runtime-rs/Cargo.lock`.

## [Toolchain 0.34.101, language 0.26.0, runtime 0.19.100] — upstream 0.34 merged into the Rust-codegen fork (2026-08-31)

- **Fork**: merged upstream `LFDT-Minokawa/compact` `main` (toolchain 0.34.100,
  language 0.26.0, runtime 0.19.100) into the Rust-codegen fork. The merge was
  clean in every compiler source file — the only conflicts were the version
  stamps and this changelog — which is the practical evidence that the `--rust`
  backend is a leaf pass: it adds no IR and no semantics, so upstream IR work
  does not collide with it.

  Upstream inserted two IR stages ahead of `Ltypescript` in this range
  (`Lnodisclose → Lnoserialize → Lloweredemit → Ltypescript`), changing the
  `emit` expression twice, dropping `serialize`/`deserialize`, adding
  `event-version`/`event-tag` to the `field` terminal, and changing the
  `program` production. No `rust-passes-*` clause matches any of those forms,
  so none of it needed porting.

- **Changed**: the `print-rust` snapshots are repinned to runtime `0.19.100`.
  That is the only change to emitted Rust across the entire merge — one line
  per snapshot.

## [Toolchain 0.34.100, language 0.26.0, runtime 0.19.100]

### Fixed

- `CompactTypeOpaqueUint8Array.fromValue` and `CompactTypeOpaqueString.fromValue`
  now throw a `CompactError` on exhausted input. Previously they returned
  `undefined` (laundered through an `as Uint8Array` cast) and `""` respectively.
  These were the last two descriptors that did not fail loudly on a truncated
  field-aligned binary value.

- `CompactTypeEnum.fromValue` now reports an out-of-range value as
  `expected Enum[<=N]` rather than `expected UnsignedInteger[<=N]`.

- `CompactTypeBytes.fromValue` now returns a copy rather than aliasing the atom
  it decoded, so mutating a decoded value cannot mutate the field-aligned
  binary value it came from.

### Changed

- **Breaking.** Runtime type checks on curve point arguments to exported
  circuits now bound the coordinates rather than only checking their types. A
  `Secp256k1Point` whose `x` or `y` falls outside `[0, MAX_SECP256K1_BASE]`, or
  a `JubjubPoint` whose coordinates fall outside `[0, MAX_FIELD]`, is now
  rejected at the circuit boundary rather than failing later inside `toValue`.
  This makes the check consistent with the one already applied to a bare
  `Secp256k1Base` or `Field` argument.

- **Breaking.** `CompactTypeSecp256k1Point.fromValue` now rejects an identity
  flag that is neither 0 nor 1. Previously any value other than 1 was silently
  read as `false`.

- **Breaking.** `CompactTypeUnsignedInteger.toValue`, `CompactTypeEnum.toValue`
  and `CompactTypeBytes.toValue` now reject arguments they cannot faithfully
  encode: a value outside the type's range, a non-integer enum tag, or a byte
  string longer than the type's length. Previously these descriptors validated
  on decode but not on encode, so it was possible to write a value to the ledger
  that could not be read back.

### Added

- `SECP256K1_LOW_LIMB_BOUND`, the exclusive upper bound of the low-order limb
  of a secp256k1 field value in its field-aligned binary representation. This
  replaces an unnamed decimal literal in `CompactTypeSecp256k1Base` and an
  equivalent but differently spelled expression in `CompactTypeSecp256k1Scalar`.

- `runtime/test/compact-types.test.ts`, a conformance suite for the
  `CompactType` protocol. For every descriptor it asserts that `alignment()`
  declares as many atoms as `toValue` produces, that `fromValue` consumes
  exactly its own prefix, tolerates trailing atoms and fails loudly on truncated
  input, and that values round trip. It also decodes every ordered pair of
  descriptors from one shared array, the way the generated tuple and struct
  classes do, and compares the results element-wise, so a descriptor that
  decodes correctly only as the last member of a compound fails as the first
  half of every pair.

  Descriptors are discovered from the module's exports rather than listed, and a
  coverage test names any exported descriptor that has no sample, so a new
  descriptor cannot be added without conformance coverage. No runtime unit test
  previously called any descriptor's `fromValue` or `toValue`.

  End-to-end coverage of specific Compact constructs remains in
  `compiler/test.ss`; this suite does not duplicate it.

## [Toolchain 0.34.0, language 0.26.0, runtime 0.19.0]

This release includes all changes for compiler versions in the range between
0.33.100 and 0.34.0; language versions in the range between 0.25.100 and 0.26.0;
and Compact runtime versions in the range between 0.18.100 and 0.19.0.

## [Toolchain 0.33.124, language 0.25.107, runtime 0.18.107] — Field arithmetic lowering fix (2026-08-20)

- **Fixed** (`--rust`): `Field` arithmetic emitted `wrapping_*` on `Fr`,
  which does not compile. `arith-binop-rust`'s fallback branch was reached
  whenever the result type had no unsigned width — i.e. every `Field`
  result — so `return a + b` on two `Field`s produced
  `Ok((a).wrapping_add(b))`. `Fr` implements `Add`/`Sub`/`Mul` but not the
  `wrapping_*` family, so `compactc` exited 0 and the failure surfaced at
  `cargo build`. Field arithmetic now lowers to plain `+`/`-`/`*`, which is
  the correct semantics (field arithmetic is modular in the field
  characteristic). An arithmetic result type that is neither a bounded
  unsigned nor a field now raises a `rust-feature-error` rather than
  emitting an operator the type may not have. Byte-parity neutral: no
  fixture reached the branch.
- **Changed** (`--rust`): the six `+`/`-`/`*` support guards in
  `rust-passes-walker.ss` admitted only `tfield`, a literal translation of
  the pre-0.33 `(not mbits)` condition and backwards for guards in front of
  unsigned arithmetic. They now admit bounded unsigned *and* field results,
  both of which have correct lowerings. Byte-parity neutral — the guards are
  unreachable for the current corpus, which is also why the inversion went
  unnoticed.
- **Docs**: added `docs/rust-backend-limitations.md`, a consumer-facing list
  of the constructs `--rust` refuses (27 kinds across 31 rejection sites),
  the "unsupported must fail, never emit plausible output" contract, and the
  command to enumerate the sites from the code so the page cannot drift.

## [Toolchain 0.33.123, language 0.25.107, runtime 0.18.107]

Fork-only release: integrates upstream 0.33.122 into the Rust-codegen fork.
The `codegen-rust` branch is unaffected and stays on the released ledger-8
line; this branch tracks upstream's own ledger-9 release-candidate pin
(`ledger-9.1.0.0-rc.3`).

### Fixed

- Cross-contract-call compilation crashed with
  `incorrect number of arguments 4 to #<procedure make-native-entry>`.
  Upstream's new `circuit-passes/desugar-contract-calls.ss` synthesises a
  `transientCommit` native, but the fork's `native-entry` record carries an
  extra `rust-function` field. 47 tests across `save-contract-info`,
  `print-zkir`, `print-zkir-v3` and `save-manifest` were failing.
- `Field as Uint<N>` under `--rust` emitted `(<expr>) as uN` with an `Fr`
  operand. `Fr` is a struct, so that is E0605 "non-primitive cast": compactc
  exited 0 and the failure only appeared at `cargo build`. The cast is now
  rejected with a diagnostic. (0.33 also gave it its own IR production,
  `cast-from-field`, split out of `downcast-unsigned`.)

### Changed

- The Rust backend follows upstream's reshaped IR productions: arithmetic
  `+ - *` carry a result `Type` instead of maybe-bits, `downcast-unsigned`'s
  source bound became mandatory, and `cast-from-field` / `cast-to-field` are
  new. All 32 `codegen_regression` fixtures regenerate with no codegen drift.
- `Field`/`Uint<N≤64> as JubjubScalar` lowers to
  `compact_runtime::jubjub_scalar_from_field`. `JubjubScalar as Field` has no
  runtime helper and is rejected.
- `ecMul` / `ecMulGenerator` take a `JubjubScalar` (upstream change); `ecNeg`
  is bound to `compact_runtime::ec_neg`. Existing `.compact` sources passing a
  `Field` need an explicit `as JubjubScalar`.
- `JubjubPoint` moved from `Opaque<"JubjubPoint">` to the builtin `tpoint`
  type, with the same decoder and default-seeding behaviour.
- Upstream 0.33 no longer implicitly widens an integer literal to `Field` in
  ledger-assignment position; `sealed_ledger_fixture.compact` and
  `struct_collision_fixture.compact` gained explicit `as Field` casts.
- The fork's F8 `nominal?` alias-export patch is now gated on `(emit-rust)`,
  so the TypeScript pipeline sees exactly upstream's IR (ADR 0002).
- TS byte-parity fixtures re-captured against the ledger-9 runtime:
  `contract-state[v6]` → `[v8]`. A `capture-counter.mjs` was added — counter
  was the one fixture in the corpus with no way to regenerate its reference.

### Added

- `--rust` now rejects `--feature-zkir-v3` with a clear diagnostic instead of
  emitting a `lib.rs` that cannot compile (the v3 natives have no Rust
  bindings and the secp256k1 type surface has no lowering). Enforced in both
  `compactc.ss` and `generate-everything`, so programmatic drivers are covered
  too. ZKIR v3 support in the Rust backend is tracked as follow-up work.
- `compact_runtime::std_lib::jubjub_schnorr_verify` and
  `JubjubSchnorrSignature`, mirroring the 0.33 standard library's Schnorr
  verifier, with call-level routing so the generic stdlib body is never
  lowered.

## [Toolchain 0.33.122, language 0.25.107, runtime 0.18.107]

### Fixed

- Issue #704, where nested ZKIR v3 native-typed values did not have proper
  ledger conversions in-circuit.  The underlying issue was that we give these
  types a "pseudo-alignment" of `anative` in `flatten-datatypes`, and this
  alignment cannot appear in the ledger.  Consequently, we translate this into
  ledger alignments and ZKIR encoding instructions (essentially, flattening
  further) in the ZKIR v3 backend.
  
  There were three places that had near duplicates of this code: (1) for ZKIR
  hashing instructions that need an alignment, (2) for Impact `popeq`, (3) and
  for all other Impact instructions.  Only number (1) of those correctly handled
  nested ZKIR-typed values.
  
  The fix is to reuse the correct code everywhere.
  
  This revealed a different issue with the descriptors for secp256k1 points and
  field types, they did not properly nest inside Compact types because they did
  not consume the FAB encoding in `fromValue`.

## [Toolchain 0.33.121, language 0.25.107, runtime 0.18.106]

### Changed

- The compiler now hashes manifest files in-process using Common Crypto or
  OpenSSL when available.

## [Toolchain 0.33.120, language 0.25.107, runtime 0.18.106]

### Changed

- Pull in a version of the ledger with some ZKIR 3.1 features in it.

### Internal notes

- Update the ZKIR v3 ledger dependency (on `main`, the development branch) to
  pull in a version of the ledger that has the ZKIR 3.x features secp256r1 and
  Curve25519.  The `zkir-v3` and `zkir-v3-wasm` dependencies are changed to
  include the new features.  The `onchain-runtime-v4` dependency is kept to
  track the tag `ledger-9.1.0.0-rc.3`.

  The toolchain version is bumped (if nothing else, it reports a different
  string for `--feature-zkir-v3 --ledger-version`).

## [Toolchain 0.33.119, language 0.25.107, runtime 0.18.106]

### Changed

- Cross-contract callees may now perform shielded (Zswap) coin operations.
  Previously the runtime blanked a callee's Zswap local state before invoking
  it, so `receiveShielded`, `sendShielded`, `mergeCoin` and friends failed with
  "Zswap local state is undefined for contract". That blocked the pattern the
  ledger actually requires: a shielded coin addressed to a contract is only
  credited if that contract claims the receive in the same transaction, which
  for a callee means running `receiveShielded` during the call.

  `CircuitContext` now carries `zswapLocalStates`, a per-contract-address record
  alongside `queryContexts` and `gasCosts`. Each contract in the call tree keeps
  its own state — its own `currentIndex`, `inputs` and `outputs` — sharing only
  the transaction submitter's coin public key. A callee's state is created on
  first entry, threaded back to the caller on return, and recorded on the call's
  `CallProofData` as `zswapLocalState` so transaction assembly can attribute
  every contract-owned input and output to the contract that made it.

  This covers all three Zswap natives — `ownPublicKey`, `createZswapInput` and
  `createZswapOutput`. Note that `ownPublicKey()` always names the transaction submitter,
  never the calling contract. A callee meaning to pay back its caller wants that caller's
  `ContractAddress`, not `ownPublicKey()`.

  Addresses [#658](https://github.com/LFDT-Minokawa/compact/issues/658); the
  transaction-assembly half lives in `compact-js` and `midnight-js`.

## [Toolchain 0.33.118, language 0.25.107, runtime 0.18.105]

### Changed

- The types `JubjubPoint` and `Secp256k1Point` are no longer defined as
  nominal type aliases for opaque types but rather standard-library names for
  internally handled types.  They can no longer be exported from a contract's
  top level.  This is a **breaking** change.

- Similarly, the types `JubjubScalar`, `Secp256k1Base`, and `Secp256k1Scalar`
  are no longer built-in types but rather standard-library names for
  internally handled types.  This is a **breaking** change, since programs
  must now import these types from CompactStandardLibrary to use them.

### Internal notes

- Previously, there were builtin types `Opaque<'JubjubPoint'>` and
  `Opaque<'Secp256k1Point'>` and the standard library exported nominal type
  aliases `JubjuPoint` and `Secp256k1Point` for these.  The compiler now
  injects definitions for these points from midnight-natives.ss and
  zkir-v3-natives.ss into the standard library and expands them into built-in types.
  Similarly, it injects definitions for `JubjubScalar`, `Secp256k1Base`, and
  `Secp256k1Scalar` from midnight-natives.ss and zkir-v3-natives.ss into the
  standard lbirary and expands them into built-in types.

## [Toolchain 0.33.117, language 0.25.106, runtime 0.18.105]

### Changed

- The generated JS code now has circuit argument and witness return value type
  checks for the JS opaque types `Opaque<'string'>` and `Opaque<'Uint8Array'>`.
  Before, we allowed any value at all to be passed or returned.  This is a
  **breaking change** for programs that relied on being able to store any random
  JS value as, say, an `Opaque<'string'>`.

## [Toolchain 0.33.116, language 0.25.106, runtime 0.18.105]

### Changed

- Equality of `Opaque<'Uint8Array'>` is now (1) same length and (2) element-wise
  strict equality (`===`).  It was formerly simple strict equality, which for
  typed arrays is object reference equality.

  This is a breaking change in the language, because `Uint8Arrays` that were
  formerly not equal in Compact can now compare as equal.

  This change brings the JS semantics more in line with the ZKIR semantics,
  which uses equality of the Poseidon hash of the typed array's contents.

## [Toolchain 0.33.115, language 0.25.105, runtime 0.18.105]

### Changed

- The JS implementation of the accessors secp256k1PointX and secp256k1PointY now
  fail with a `CompactError` when passed the identity (`default`) point.  This
  matches the ZKIR behavior, where these operations fail.

  Before, these accessors returned whatever was stored in the x- or y-coordinate
  of the JS object.  This was not a valid coordinate for this point, and not
  even a sentinel value like 0 because we don't currently canonicalize identity
  points.

### Internal notes

- This is a **breaking change** in the language.  Though it's a bug fix, the
  language version is still correctly incremented.

## [Toolchain 0.33.114, language 0.25.104, runtime 0.18.104]

### Changed

- Clean up the Compact runtime to reflect the intended structure: types and
  descriptors needed by the generated code are in `compact-types.ts`, but not
  redundant and unnecessary implementations; functions used by emitted code are
  in `built-ins.ts`, but not helpful utility functions; those are in `utils.ts`.

### Internal notes

- This is a breaking change because some unused and unnecessary exported types
  and descriptors have been deleted.

## [Toolchain 0.33.113, language 0.25.104, runtime 0.18.103]

### Added/Changed

- The binary arithmetic operators `+`, `-`, and `*` now work for `Secp256k1Base`
  and `Secp256k1Scalar`.  The operands must have the same type and the result
  will have that type.  There is a new runtime function to perform subtraction
  for these types.  The standard library circuits `add` and `mul` have been
  removed.

## [Toolchain 0.33.112, language 0.25.103, runtime 0.18.102]

### Fixed

- The ZKIR v3 printer now respects the --no-communications-commitment flag.

## [Toolchain 0.33.111, language 0.25.103, runtime 0.18.102]

### Changed

- The type `Uint<0>` is allowed where previously it was a compiler error.  It's
  equivalent to `Uint<0..1>` by the rule and the fact that 2^0 equals 1.
  `Uint<0..1>` is allowed so there is no reason to prohibit `Uint<0>` even
  though it's not super useful.

## [Toolchain 0.33.110, language 0.25.102, runtime 0.18.102]

### Added

- Add `secp256k1EcdsaRecover` to the Compact JavaScript runtime. Given a
  32-byte message hash, an ECDSA signature and a recovery,
  it returns the corresponding secp256k1 public key.

  Recovery runs off-circuit: the intended pattern is to recover the key here,
  pass it into a circuit as a witness or an argument, and
  constrain it there with the standard library's `secp256k1EcdsaVerify`.

  `secp256k1EcdsaVerify` accepts both low-s and high-s signatures, as
  [FIPS 186-5](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-5.pdf)
  section 6.4.2 constrains `s` only to `[1, n - 1]`.

## [Toolchain 0.33.109, language 0.25.102, runtime 0.18.101]

### Internal notes

- Each of the compiler passes now resides in its own file.  For example,
  infer-types used to reside in analysis-passes.ss along with the other
  analysis passes.  It now resides in analysis-passes/infer-types.ss, which
  analysis-passes.ss now includes.

## [Toolchain 0.33.108, language 0.25.102, runtime 0.18.101]

- Fix issue [#588](https://github.com/LFDT-Minokawa/compact/issues/588).  For
  the type `Uint<0..1>` (and enums with a single variant, which get lowered to
  `Uint<0..1>`), we used an alignment of `bytes:0`.  The ledger and ZKIR expects
  **no** values for such an alignment, but we provided one (always zero) value
  in the transcripts.
  
### Internal notes

- The fix is to use an alignment of `bytes:1` for the type `Uint<0..1>`, so that
  the ZKIR code will expect the value provided by JS.

## [Toolchain 0.33.107, language 0.25.102, runtime 0.18.101]

### Fixed

- Modify the standard library's `secp256k1EthereumAddress` circuit to `assert`
  that the input is not the secp256k1 identity point, because it does not have a
  corresponding Ethereum address.  This required two other fixes:
  - ZKIR code generation for `default<Secp256k1Point>` was not yet implemented
    and is needed, and
  - `persistentHash` and `keccak256` hashing functions need to properly handle
    alignment for `JubjubScalar`, `Secp256k1Base`, and `Secp256k1Scalar` in the
    ZKIR v3 backend.

### Internal notes

- The standard library behavior is changed (to reject the secp256k1 identity
  point) but this is deemed a bug fix and not a language version change.

## [Toolchain 0.33.106, language 0.25.102, runtime 0.18.101]

### Fixed

- Fix issue [#609](https://github.com/LFDT-Minokawa/compact/issues/609).
  Successive calls to `secp256k1EcdsaVerify` triggered a failure in the circuit
  optimizer where the secp256k1 base and scalar fields were not handled in a
  comparison predicate.

## [Toolchain 0.33.105, language 0.25.102, runtime 0.18.101]

### Fixed

- Fix issue [#608](https://github.com/LFDT-Minokawa/compact/issues/608).  The
  ZKIR v3 backend did not properly handle alignment for JubjubPoint and
  Secp256k1Point when passed to the hashing function `persistentHash`,
  `persistentCommit`, or `keccak256`.

## [Toolchain 0.33.104, language 0.25.102, runtime 0.18.101]

### Fixed

- Implement proper equality comparison for `Secp256k1Point`.  Identity points
  are equal to identity points, and non-identity points are equal if they have
  the same affine X- and Y- coordinates.

### Internal notes

- JS code for `JubjubPoint` equality is simplified, and `Uint` types now use
  direct `===` comparisons, rather than a helper that performs only `===`
  comparison.

## [Toolchain 0.33.103, language 0.25.101, runtime 0.18.101]

### Changed

- Pulls in ledger-9.1.0.0-rc.3

## [Toolchain 0.33.102, language 0.25.101, runtime 0.18.100]

### Fixed

- Add a `toBinaryRepr` to the Compact runtime that replicates the effect of the
  on-chain Rust `binary_repr`.  Use it in the runtime for the argument to the
  Noble hashes `keccak_256` function, to correctly replicate the in-circuit
  implementation.  This ensures that trailing zero bytes from byte vectors are
  preserved and hashed in JS as well as in circuit.

- Change casting of byte vectors to foreign fields so that they perform modular
  reduction by the field modulus rather than failing for byte vectors encoding
  values out of range.  The failure is kept for native fields to avoid a
  breaking change at this time.

### Internal notes

- There is a Compact runtime change, so when this change is cherry-picked to the
  0.33 release, there should be another Compact runtime release candidate
  release.

## [Toolchain 0.33.101, language 0.25.100, runtime 0.18.0]

### Changed

- The compiler now tries sha256sum first, then shasum -a 256 when looking
  for a program to compute a sha256 hash.

## [Toolchain 0.33.100, language 0.25.100, runtime 0.18.0]

### Fixed

- The `ShieldedReceive` standard event now serializes its fields in the order
  specified by CoIP-442 and MIP-0002: `commitment`, `ciphertext`,
  `contractAddress` (previously `contractAddress` preceded `ciphertext`).
  Serialized size is unchanged (578). Fixes #590.

### Changed

- The standard library ECDSA circuits `secp256k1EcdsaVerify` and
  `secp256k1EcdsaRecover` deserialize the message hash as a big endian
  secp256k1 scalar `z` internally, following the ECDSA convention (RFC 6979).

- The circuit `secp256k1EcdsaRecover` and struct
  `Secp256k1EcdsaSignatureWithRecovery` have been removed from the standard
  library.

## [Toolchain 0.33.0, language 0.25.0, runtime 0.18.0]

This release includes all changes for compiler versions in the range between
0.32.100 and 0.33.0; language versions in the range between 0.24.100 and 0.25.0;
and Compact runtime versions in the range between 0.17.100 and 0.18.0.

## [Toolchain 0.32.111, language 0.24.103, runtime 0.17.106]

### Changed

- `zkir` and `onchain-runtime-v4` use `ledger-9-rc.2`.

## [Toolchain 0.32.110, language 0.24.103, runtime 0.17.105]

### Added

- New Compact types for Jubjub and secp256k1 curves: builtin field types
  `JubjubScalar`, `Secp256k1Base`, and `Secp256k1Scalar` (the Jubjub base field
  is the native BLS12-381 Compact `Field` type), and the point type
  `Secp256k1Point`.
- The secp256k1 fields and curve points are **ONLY** supported by using the new
  ZKIR v3 backend at compile time.  This is enabled by passing the flag
  `--feature-zkir-v3` to the Compiler.  Using them with the default ZKIR v2
  backend is a compile-time error.
- The standard library has new circuits for verifying ECDSA signatures in
  Compact.

#### `JubjubScalar`
- There is a cast from `Field` to `JubjubScalar` and from `JubjubScalar` to
  `Field`.  The cast from `Field` to `JubjubScalar` will reduce the `Field`
  value modulo the Jubjub scalar modulus.  It will not fail, but do note that
  round tripping from `Field` to `JubjubScalar` and back will possibly give a
  different `Field` value.  The cast from `JubjubScalar` to `Field` will always
  succeed and give the same value (as a `Field`), because the maximum
  `JubjubScalar` value is less than the maximum `Field` value.
- There is a cast from all `Uint` types to `JubjubScalar`.  This cast behaves
  the same as the cast from `Field` to `JubjubScalar`, namely never failing but
  reducing values larger than the maximum Jubjub scalar by the Jubjub scalar
  modulus.  There is also a cast from `JubjubScalar` to all `Uint` types.  This
  cast will fail at runtime if the actual Jubjub scalar value is too large for
  the target `Uint` type.
- `default<JubjubScalar>` is zero.
- Arithmetic is not supported for the `JubjubScalar` type.  Equals and
  not-equals comparisons are supported, but other relational comparisons are not
  supported.
- The Compact runtime exports new `bigint` constants `JUBJUB_SCALAR_MODULUS` and
  `MAX_JUBJUB_SCALAR`.

#### secp256k1 fields
- There are casts to and from both `secp256k1` field types and `Bytes<32>`.  The
  `Bytes<32>` representation is little-endian.  The casts targeting `Bytes<32>`
  cannot fail (both fields' maximum values fit in 32 bytes).  The casts from
  `Bytes<32>` will fail if the resulting value would exceed the respective
  target type's maximum value.  Therefore, round tripping through `Bytes<32>`
  always succeeds and gives the same value; round-tripping through a secp256k1
  field type will only work if the original `Bytes<32>` is a valid value for
  that field.
- There are **NO** other casts to or from the secp256k1 field type from any
  other type.  Specifically, because there are no `Uint` casts and literals have
  `Uint` type, there is no way to use a literal as a secp256k1 field value (you
  can obtain one from a witness, though).
- `default<Secp256k1Base>` and `default<Secp256k1Scalar>` are zero.
- Arithmetic is supported via standard library circuits: `add` takes a pair of
  secp256k1 field values of the same type and returns a value of that type,
  `mul` takes a pair of secp256k1 field values of the same type and returns a
  value of that type, `neg` negates a secp256k1 field value and returns a value
  of the same type, `inv` returns the multiplicative inverse (for a value `x`,
  this is the value `y` such that `mul(x, y) == 1`).  All of these operations
  are performed modulo the respective field's modulus.
- Equals and not equals comparisons are supported for these same-typed values of
  these types, but other relational comparisons are not supported.
- The Compact runtime exports new functions to perform arithmetic operations in
  the respective field.  It exports constants for the field modulus and the
  maximum value.

#### secp256k1 points
- The standard library has circuits `secp256k1PointX` and `secp256k1PointY` to
  extract the affine X- and Y-coordinates of a value of type `Secp256k1Point`.
  These coordinates both have type `Secp256k1Base`.  There is no way in Compact
  to explicitly construct `Secp256k1Point`s from their coordinates (but note
  that they can be obtained from witnesses).
- `default<Secp256k1Point>` is the additive identity point (a point `y` such
  that `ecAdd(x, y) == x` for any point `x`.
- The elliptic curve operations `ecAdd`, `ecMul`, and `ecMulGenerator` are
  overloaded to work for secp256k1 types.  If `ecAdd` is given a pair of
  `Secp256k1Point`s it will return a `Secp256k1Point`.  If `ecMul` is given a
  `Secp256k1Point` and a `Secp256k1Scalar` it will return a `Secp256k1Point`.
  If `ecMulGenerator` is given a `Secp256k1Scalar` it will return a
  `Secp256k1Point`.
- Equals and not-equals comparisons between these points do not work reliably.
- The compact runtime exports new functions to perform these operations.  In the
  Compact runtime, `Secp256k1Point` is represented as an object with affine X-
  and Y-coordinates.  The additive identity point (this is the value of
  `default<Secp256k1Point>` is not representable with `x` and `y` coordinates;
  there is a property `identity` on the Compact runtime representation.  If
  `pt.identity` is true then the point is the identity point and the X- and
  Y-coordinates should be ignored.

#### ECDSA verification in Compact

- The standard library has a new circuit `secp256k1EcdsaVerify` that attempts to
  verify an ECDSA signature and returns a boolean value telling whether the
  verification succeeded.  If you want to ensure a signature verifies in
  Compact, you should `assert` that the value of `secp256k1EcdsaVerify` is true.
  This circuit takes (1) a message hash as Compact `Bytes<32>`, which must be
  produced by either `persistentHash` (SHA-256) or `keccak256` **in Compact**,
  (2) a signature as a value of a new standard library structure type
  `Secp256k1EcdsaSignature` containing a pair of `Secp256k1Scalar` values, and
  (3) the public key as a `Secp256k1Point`.
- The standard library has a new circuit `secp256k1EcdsaRecover` that recovers
  the public key from an ECDSA signature.  It returns the public key as a
  `Secp256k1Point`.  This circuit takes (1) a message hash as Compact
  `Bytes<32>`, which must be produced by either `persistentHash` (SHA-256) or
  `keccak256` **in Compact**, and (2) a signature value of a new standard
  library structure type `Secp256k1EcdsaSignatureWithRecovery`.  This has a pair
  of `Secp256k1Scalar` values (the signature) and the signing nonce point as a
  `Secp256k1Point` computed outside of Compact (e.g. as a witness return value
  or a circuit argument).  This is computed outside Compact to avoid an
  expensive square root computation in the secp256k1 base field,
  `secp256k1EcdsaRecover` asserts in circuit that the X-coordinate of this point
  matches the signature's `r` (when cast to Compact `Bytes<32>`, that is as
  32-byte little-endian representations).
- **NOTE:** these circuits use the secp256k1 curve and field types, and
  consequently they are unavailabe with the default ZKIR v2 backend.  Pass the
  flag `--feature-zkir-v3` at compile time to enable them.

### Changed

- The standard library circuit `ecMul` now requires the second argument to have
  type `JubjubScalar` when the first argument has type `JubjubPoint`.
  Previously this type was `Field`.  The standard library circuit
  `ecMulGenerator` requires its argument to have type `JubjubScalar`.
  Previously this type was `Field`.
- `Field` is no longer a supertype of `Uint` types.  This is a major **BREAKING
  CHANGE**.  `Uint` values will no longer be implicitly cast to `Field` types in
  many contexts, and an explicit `as Field` cast will be required.  This
  **specifically** affects numeric literals.  The literal `n` has exact type
  `Uint<0..n+1>`, which is no longer implicitly cast to `Field` type.
  Effectively, there are no longer any `Field` literals in Compact, and you must
  write, e.g., `7 as Field`.  **NOTE:** we might later change this behavior by
  allowing `Field` literals in some way.
- An exception is in arithmetic.  The rules for binary arithmetic operations
  (addition, subtraction, multiplication) are **unchanged**: if one operand has
  type `Field` and the other has a `Uint` type, then the `Uint` value is cast to
  a `Field` value and `Field` arithmetic is used.  Where formerly this inserted
  cast was an upcast (from a subtype to a supertype), it is no longer an upcast
  (`Field` and `Uint` types are unrelated by subtyping), but it is still
  implicit.  **NOTE:** we might later change this behavior.

## [Toolchain 0.32.109, language 0.24.102, runtime 0.17.104]

### Added

- The compiler usage page now introduces `--feature-zkir-v3`.

## [Toolchain 0.32.108, language 0.24.102, runtime 0.17.104]

### Fixed

- a bug that prevented `contract-manifest.json` from including some file hashes

## [Toolchain 0.32.107, language 0.24.102, runtime 0.17.104]

### Changed

- The generated zkir reverted back to having separate impact instructions.

## [Toolchain 0.32.106, language 0.24.102, runtime 0.17.104]

### Changed

- Adds `domainSep` to `UnshieldedSpend` and `UnshieldedReceive` events.
- Renames event fields to follow camelCase.

## [Toolchain 0.32.105, language 0.24.102, runtime 0.17.104]

### Added

- Multi-contract systems: contract types, contract references, and
  cross-contract calls (see [CoIP-2](coips/coip-0002.md)). This is the first
  stage of support for multiple contracts that work together as a system. The
  new language constructs are:
  - `contract` type declarations, naming a collection of circuit
    signatures (parameter types, return type, and purity) that another
    contract may depend on.
  - The `contract implements C;` assertion. A contract implements a contract type
    whenever it exports a matching circuit for each one the contract type declares
    -- but when the assertion is present the compiler verifies it and rejects
    the contract at compile time if any required circuit is missing or has a
    non-matching signature.
  - Contract references: a contract type is an ordinary
    program-defined type and may be used as a circuit or witness parameter, a
    struct field, or the element/value type of a ledger collection (e.g.
    `List<C>`, `Map<Field, C>`). A reference is introduced
    from application code by passing a deployed contract's address where a
    value of the contract type is expected.
  - Cross-contract calls: `reference.circuit(args...)` invokes a circuit
    named in the reference's type.
- Adds runtime support for cross-contract calls (CCC), the execution machinery
  behind contract interfaces, contract references, and one contract's circuit
  calling another's (see CoIP-2). Two new modules are added and re-exported from
  the package index:
  * `contract.ts`, exporting `crossContractCall` — invokes a circuit on
    another contract from within the executing circuit, threading the callee's
    ledger state, gas, and proof data back into the caller's context and
    emitting the `Kernel.claimContractCall` transcript that links the two.
  * `providers.ts`, exporting the `ContractStateProvider` interface — a
    user-supplied `getContractState(blockHash, address)` used to fetch a
    cross-contract callee's deployed public state at runtime. The
    `parentBlockHash` recorded on the context is passed as the `blockHash`.
- **Breaking:** `CircuitContext` is restructured to model a whole call tree
  rather than a single contract execution.
  * Per-call state moves into a new `callContext: CallContext<PS>` member
    (`circuitId`, `contractAddress`, `initialQueryContext`,
    `currentQueryContext`, `currentGasCost`, `currentPrivateState`,
    `currentZswapLocalState`, `parentBlockHash`, `time`). Fields previously at
    the top level — `currentPrivateState`, `currentZswapLocalState`, and
    `currentQueryContext` — are now reached through `callContext`.
  * New top-level members: `queryContexts` and `gasCosts` (per-contract-address
    maps spanning the call tree), `contractStates` (retained deployed states of
    resolved callees, so the verifier-key guard can run on every call),
    `callProofDataTrace` (depth-first sequence of `CallProofData` for the root
    circuit and every sub-call), `stateProvider`, `reentrancyGuard`, and
    `activeContracts`.
- **Breaking:** `createCircuitContext` gains a leading `circuitId` argument and
  new trailing `stateProvider`, `parentBlockHash`, and `reentrancyGuard`
  arguments. Its full signature is now `(circuitId, contractAddress,
  coinPublicKeyOrZswapState, contractState, privateState, stateProvider?,
  gasLimit?, costModel?, time?, parentBlockHash?, reentrancyGuard?)`. The
  `stateProvider`, `parentBlockHash`, and `reentrancyGuard` arguments are only
  needed by circuits that make cross-contract calls.
- **Breaking:** `CircuitResults` no longer carries a `proofData` field. The
  proof data for each circuit run (root and sub-calls) is now collected in
  `callProofDataTrace` on the context.
- Adds dynamic safety guards on every cross-contract call:
  * Re-entrancy guard — entering a contract already executing on the call
    stack (`A -> A`, or `A -> B -> A`) throws a `CompactError`. Enabled by
    default; pass `reentrancyGuard: false` to `createCircuitContext` to opt
    out (e.g. tests that deliberately exercise recursion).
  * Implementation-binding guard — hashes the deployed verifier key for the
    called circuit (SHA-256) and compares it to the `expectedVk` fingerprint
    the compiler emits onto the contract module; a mismatch throws the new
    `ContractInterfaceMismatchError`, rejecting a call whose target address
    points at a different contract than the interface resolves to.
  * Purity guard — a callee whose actual purity disagrees with the interface's
    `pure` declaration is rejected.
  * Witness guard — a cross-contract callee that invokes a witness throws a
    `CompactError`; called contracts must have no private state.
  * Calling the default (dummy) contract address throws a `CompactError`.
- Error module (`error.ts`):
  * `CompactError` now carries a readonly `isCompactError` brand so consumers
    can reliably distinguish compiler-originated errors from other failures.
  * Adds `ContractInterfaceMismatchError` (extends `CompactError`).
  * Adds internal `assertDefined` / `assertUndefined` assertion helpers.
- Utilities (`utils.ts`): adds `assertIsContractAddress`, which throws a
  `CompactError` for values that are not contract addresses.
- Zswap (`zswap.ts`): `createZswapInput`, `createZswapOutput`, `ownPublicKey`,
  and `hasCoinCommitment` now read and write Zswap local state and the query
  context through `circuitContext.callContext`, following the context
  restructure. A new `assertHasCurrentZswapLocalState` check makes these
  operations throw a `CompactError` when there is no Zswap local state — for
  example inside a cross-contract callee, which has none.
- Adds new exported types and helpers used by the above: `CircuitId`,
  `CallContext`, `CallProofData`, `CallProofDataTrace`,
  `CommunicationCommitmentData`, `createCallContext`, `copyCircuitContext`,
  `finalizeCallProofData`, and a now-exported `createInitialQueryContext` (which
  gains `parentBlockHash` and `caller` parameters and a required `time`).

## [Toolchain 0.32.104, language 0.24.101, runtime 0.17.103]

### Added

- The new language form `emit(expr)` emits a an event.  `expr` must have a
  standard event type.  Constructors cannot emit an event, either directly
  or indirectly via calls to circuits that use `emit`.  Calling `emit` makes
  a circuit impure.
- The onchain-runtime requires the argument passed to `emit` to be serialized.
  Compact's standard library provides a generic `serialize<T,n>` and
  `deserialize<T,n>`.  The end user cannot see their definition,
  but they can see their type signature in Compact's standard library. 

### Changed

- The generic `serialize` and `deserialize` are defined in `midnight-inlines.ss`
  and the macro expansion is defined in `inlines.ss` and they are inserted during
  `expand-modules-and-types`.
- The TypeScript wrapper for each impure exported circuit now exposes an
  `events` field on the wrapped return value, and a corresponding `events`
  field on `CircuitContext`.  Both refer to the same array; events emitted
  by `emit` expressions during the circuit's execution are appended in order
  of evaluation.  Pure circuits' wrapped return values are unaffected.
- Updates the compact-runtime: adds the required `events` field
  to `CircuitContext` and `CircuitResults`.  This is a **breaking** change
  for TypeScript code that constructs these types by hand; code that uses
  the runtime's `createCircuitContext` helper is unaffected.

### Internal notes

- The standard event types are defined in `compiler/midnight-events.ss` in
  a DSL that is defined in `compiler/events.ss`.  Events are injected into
  CompactStandardLibrary during `expand-modules-and-types`.
- Some of the downstream type checkers did not handle `let*` forms with
  multiple bindings properly but now do.  This was not previously a problem
  because the upstream passes did not produce such `let*` forms.

## [Toolchain 0.32.103, language 0.24.0, runtime 0.17.102]

### Changed

- Pulls in ledger-9.1.0.0-rc.3

## [Toolchain 0.32.102, language 0.24.0, runtime 0.17.101]

### Fixed

- Code generation for certain casts.

## [Toolchain 0.32.101, language 0.24.0, runtime 0.17.101]

### Changed

- Pulls in ledger-9.1.0.0-rc.2. Note for pulling in alpha versions of the ledger:
  in `runtime/package.json` remove the onchain-runtime dependency and update the
  onchain-runtime nixDependency, in `runtime` run
  `npm install --package-lock-only --ignore-scripts`, in `compact` run `nix build`
- The runtime pulls in onchain-runtime-v4.

## [Toolchain 0.32.0, language 0.24.0, runtime 0.17.0]

This release includes all changes for compiler versions in the range between
0.31.100 and 0.32.0; language versions in the range between 0.23.100 and 0.24.0;
and Compact runtime versions in the range between 0.16.100 and 0.17.0.

## [Toolchain 0.31.108, language 0.23.105, runtime 0.16.101]

### Added

- Add `ecNeg` to the standard library for JubJub point negation.

## [Toolchain 0.31.107, language 0.23.104, runtime 0.16.101]

### Fixed

- Fix [issue #456](https://github.com/LFDT-Minokawa/compact/issues/456), a ZKIR
  v2 bug in Schnorr signature verification.  This change also fixes a bug in
  Schnorr signature verification for the experimental ZKIR v3 backend.

### Internal notes

- The Schnorr signature verification feature is unreleased (added in toolchain
  0.31.104).

## [Toolchain 0.31.106, language 0.23.104, runtime 0.16.101]

### Added

- The compiler now writes a manifest to the file `contract-manifest.json` in the
  `compiler` subdirectory of the target directory.  The manifest contains sizes
  and sha256 sums for each of the generated files except `contract-manifest.json`
  itself.

### Changed

- The compiler now removes and recreates the `contract` subdirectory of target
  directory.  While previous versions removed and recreated the `compiler`, `zkir`,
  and `keys` directory they left the `contract` subdirectory _and its contents_
  in place and instead replaced only the target files `index.dts`, `index.js`,
  and `index.js.map`.

- The compiler now always creates the `keys` subdirectory of the target
  directory.  The `keys` directory will be empty, however, if the --skip-zk
  flag is used, the zkir binary isn't found, or none of the contracts circuits
  require proofs.

## [Toolchain 0.31.105, language 0.23.104, runtime 0.16.101]

- The ZKIR v3 format, behind the feature flag `--feature-zkir-v3`, has changed
  so that:
    - it contains an `outputs` field that is a list containing the type of each
      output of the circuit
    - it contains a single `output` instruction that specifies a list of output
      operands and is the last instruction

## [Toolchain 0.31.104, language 0.23.104, runtime 0.16.101]

### Added

- Schnorr signature verification over the JubJub embedded curve, via the new
  `JubjubSchnorrSignature` struct and `jubjubSchnorrVerify` circuit in the
  standard library.

## Fork history — MediaNoxLabs Rust-codegen fork (pre-0.33 merge)

> The entries below are MediaNoxLabs fork releases (toolchain
> 0.31.104–0.31.111) made on top of upstream 0.31.103, before this fork
> merged upstream 0.33.x. Version numbers in this block are fork-local and
> do not correspond to upstream releases that may carry the same numbers.

## [Toolchain 0.31.111, language 0.23.103, runtime 0.16.100] — struct-field projections in trapping arithmetic (2026-08-10)

### Fixed

- **Struct-field projection as an operand of trapping arithmetic (G1)** —
  a pure circuit whose assert subtracted a struct-field projection failed to
  compile with `unsupported Compact construct (pure-circuit-body-emission): no
  walker shape matched pure circuit body`, e.g.
  `if (policy.enforceMaxAge) { assert(currentTime -
  attestation.proof.createdAt <= policy.maxAge, "..."); }`
  (MediaNoxLabs/compact#5). The typer wraps trapping unsigned arithmetic in an
  underflow guard `(seq (assert (>= a b)) (- a b))` and let\*-lifts any
  operand that isn't already a simple local into its own temp, so a projection
  operand nests TWO levels of lifted assignment:
  `(= %t.0 (seq (= %t.1 (elt-ref attestation.proof createdAt)) (seq (assert
  (>= currentTime %t.1) ...) (- currentTime %t.1))))`. `stmt-flatten`'s
  `lift-seq-prefix-exprs` hoists the OUTER level to a statement, where
  `stmt-pure-body-rust`'s `stmt->assignment` clause lowers it as
  `let <temp> = <rhs>;` — but the inner level stayed inside the RHS and reached
  `seq-stmt-rust`, which handled only `assert` and fell through to `expr-rust`,
  which has no `(=)` clause at all. Plain scalar operands need only one level,
  and the same projection under `==` rather than `<=` also stays single-level,
  which is why every ingredient compiled in isolation and the `if` guard turned
  out to be incidental (the unguarded assert failed identically).
  `seq-stmt-rust` (`compiler/rust-passes-emit.ss`) now renders a nested
  assignment through the same helpers the statement-level path uses —
  `uniquify-rust-name` over `current-var-substitution`, then `expr-rust` for
  the RHS — rather than gaining a second projection-aware renderer, and
  `expr-rust`'s `seq` clause folds its prefix statements instead of mapping
  them so a binding one introduces is in scope for the statements after it and
  for the tail. The two `let`-binding clauses in `stmt-pure-body-rust` reserve
  their own Rust name for the duration of the RHS render, so a temp lifted from
  inside it uniquifies instead of shadowing the binder. Unblocks
  midnight-verifiable-credentials' `status-proof-protocol.compact`
  (`assertAuthorityAttestedStatusProofFreshEnough`, reached through
  `revocation-registry.compact`) and `secret-birth-credential.compact`, both of
  which previously died at `status-proof-protocol.compact:522`. Every existing
  byte-parity fixture is UNCHANGED — bodies without a nested assignment take
  the identical code path, and the digital-passport fixture's single-level
  `let t = { assert!(...); ... }` block form is preserved. Guarded by the new
  `examples/guarded_assert_arith_fixture.compact` fixture (byte parity via
  `tests-e2e-rust/tests/codegen_regression.rs`) plus the executing assert gate
  `tests-e2e-rust/tests/guarded_assert_arith_fixture.rs`, which calls the
  generated pure circuits with values that satisfy and values that trip each
  assert — pinning the inclusive boundary, the underflow trap (a dropped guard
  would wrap on `u64` and silently pass), the skipped-guard path, and a
  two-projection subtraction whose returned difference proves the two lifted
  temps stayed distinct. Emitter-only fix: the runtime crate version stays
  0.16.100 because generated contracts pin it via `check_runtime_version!`,
  which requires exact equality — only the toolchain version advances.

## [Toolchain 0.31.110, language 0.23.103, runtime 0.16.100] — alignment-aware ledger-read decode (2026-08-10)

### Fixed

- **Field-repr-arity ledger-read decoding of alignment-encoded cells (A30)** —
  `compact_runtime::std_lib::decode_via_field_repr<T>` converted the
  `AlignedValue`'s atoms to `Fr`s 1:1 (one `Fr::try_from(atom)` per atom) and
  fed that to `T::from_field_repr`. But cells are ALIGNMENT-encoded — one atom
  per leaf value — and a single leaf may span multiple field-repr `Fr`s: a
  32-byte address cell is ONE atom but `[u8; 32]::FIELD_SIZE == 2` `Fr`s
  (1-byte stray chunk + 31-byte chunk, packed from the end per upstream
  `impl FieldRepr for [u8]`). Every `ContractAddress` read —
  did.compact 0.5.0's `ledger().id()` accessor and the constructor's
  `id = kernel.self()` readback — could therefore NEVER decode, and
  multi-leaf struct reads (did-05's `VerificationMethod` map lookups: 6 atoms
  vs `FIELD_SIZE` 3) hit the same arity mismatch. Found empirically in the
  MediaNoxLabs/midnight-identity port (the "second decode-path finding" in the
  MediaNoxLabs/compact#3 comment thread; MediaNoxLabs/compact#4 landed the
  did-05 readback gate `#[ignore]`d on exactly this bug). The decoder now
  walks `av.alignment` in lockstep with the atoms and expands each leaf into
  exactly the `Fr` chunks its field-repr occupies: `Field` atoms as one `Fr`;
  `Bytes { length }` atoms as `ceil(length/31)` 31-byte little-endian chunks
  in reverse chunk order, with leading zero-`Fr`s re-padding the trailing
  zero bytes stripped by `ValueAtom::normalize`; `Compress` atoms (opaque
  strings / `Vec<u8>`) as raw-byte chunks of the actual atom — `ceil(n/31)`
  `Fr`s, zero `Fr`s for the empty value — matching this runtime's
  `OpaqueString`/`Vec<u8>` `FieldRepr` convention.
  `OpaqueString::from_field_repr` now strips the 31-byte-chunk zero padding
  (on-chain `Compress` atoms are normal-form, so trailing NULs are
  unrepresentable — stripping is the faithful inverse). For fixed-size
  targets the expanded stream must match `T::FIELD_SIZE` exactly, so a
  NON-empty variable-length leaf inside a fixed-slicing struct fails loudly
  instead of silently mis-slicing every following field
  (`OpaqueString::FIELD_SIZE == 0` gives the generated `from_field_repr` no
  slot for the bytes; variable-length struct leaves round-trip only while
  empty — tracked as a codegen follow-up). Runtime-only fix: NO generated
  code changes and byte-parity fixtures untouched; the runtime crate version
  stays 0.16.100 because generated contracts pin it via
  `check_runtime_version!`, which requires exact equality — only the
  toolchain version advances. Guarded by round-trip unit tests in
  `runtime-rs/src/std_lib/adts.rs` (`new_cell(T)` →
  `decode_via_field_repr::<T>` for u8/u32/u64/bool/Fr, tuples,
  `ContractAddress` incl. the all-zero normalised-empty-atom edge,
  `[u8; 32]`, a `[u8; 32]`-bearing struct, a JWK-shaped struct with empty
  string leaves, plain enums, opaque strings empty/short/31-byte/multi-chunk,
  and the loud non-empty-string-leaf rejection) and by un-ignoring the
  did-05 executing readback gate
  (`tests-e2e-rust/tests/did05_constructor_scaffold.rs`), which now asserts
  the full `initial_state` → `ledger()` accessor readback INCLUDING `id`
  (the pre-A30 failure-mode pin test is removed).

## [Toolchain 0.31.109, language 0.23.103, runtime 0.16.100] — initial-state chunked scaffold (2026-08-07)

### Fixed

- **Flat initial-state scaffold for >16-field ledgers (A29)** — contracts with
  more than 16 ledger fields (did.compact 0.5.0 has 19) generated an
  `initial_state` that seeded a *flat* n-element `new_array(vec![...])`
  scaffold, while every read/write emission site — including the constructor's
  own writes — used the front end's chunked nested shape
  (`StateValue::Array` caps at 16; did-05 chunks as an outer 2-slot array of
  4 + 15 fields). Executing the Rust constructor therefore wrote nested
  `idx_at_index` paths into a flat scaffold, producing state the generated
  `ledger()` accessors (and the chain shape) could not read. The scaffold
  emission (`emit-scaffold-elements` in `compiler/rust-passes-emit.ss`) now
  walks the IR's `public-ledger-array` structure recursively — the SAME
  nested structure `binding-path-indices` and all read/write emitters derive
  their paths from, mirroring `typescript-passes.ss::ledger-initializers` —
  so the shapes cannot diverge. Contracts with <=16 fields have no nested
  pl-arrays and stay byte-identical. Found while building the
  indexer-backed snapshot decoder in midnight-identity (tracked in
  MediaNoxLabs/compact#3 comments). Guarded by a new EXECUTING constructor
  readback gate: `tests-e2e-rust/tests/chunked_ledger_fixture.rs` runs the
  new 18-field `examples/chunked_ledger_fixture.compact`'s `initial_state`
  and reads every field back via the generated `ledger()` accessors. The
  did-05 equivalent (`tests-e2e-rust/tests/did05_constructor_scaffold.rs`)
  pins the current failure mode and carries the full readback `#[ignore]`d —
  did-05's constructor does `id = kernel.self()`, whose generated
  `decode_via_field_repr::<ContractAddress>` read hits the separate
  field-repr-vs-alignment decode bug (also tracked in the #3 comment
  thread); un-ignore when that follow-up lands.

## [Toolchain 0.31.108, language 0.23.103, runtime 0.16.100] — constructor read-your-writes (2026-08-05)

### Fixed

- **Constructor read-before-write for impure-call reads (A28)** — a constructor
  that writes a ledger field and then calls an impure circuit reading that
  field (did.compact 0.5.0's `controllerPublicKey` / `recoveryAuthorityPublicKey`
  writes followed by `assertControllerPublicKeyDistinctFromRecoveryAuthority`,
  which reads both) generated the read against the *unmodified initial ledger*,
  because all writes were batched into a single `OpProgramVerify` applied at the
  end. Both the argument read and the callee's own ledger reads saw
  `JubjubPoint::default()`, silently defeating the distinctness invariant. The
  codegen now flushes the pending cell-writes/pl-calls to `qctx`
  (`ctor-write-flush-lines`) before an impure call in a constructor, giving
  read-your-writes; the impure call and its args then observe the witnessed
  values. Surfaced by Codex review (P1). Covered by a structural regression
  test (the write-flush must precede the distinctness assert) plus the did-05
  byte-parity lock; other constructors (no impure call) are byte-identical.

## [Toolchain 0.31.107, language 0.23.103, runtime 0.16.100] — impure-call gas accounting (2026-08-05)

### Fixed

- **Circuit gas under-reporting for impure-helper calls (A27)** — a circuit
  that calls an impure circuit (e.g. `recordUpdate()` after a mutation, or a
  cross-circuit helper) rebound `ctx` from the callee's result but discarded
  the callee's `gas_cost`, so the generated function returned only the terminal
  write's gas and under-reported successful transactions. Both walkers now
  accumulate callee gas: the streaming walker adds `__gas_acc += _cr.gas_cost`
  for bare/const impure calls, and non-streamed circuit bodies with impure
  calls seed a `__gas_acc` and return `__gas_acc + results.gas_cost`. Surfaced
  by Codex review of the did.compact 0.5.0 circuits (setVerificationMethod /
  rotateControllerKey / …); also corrected the latent same-shape gap in
  `cross-circuit-fixture`. Covered by a new semantic gas test
  (`reset_and_set` gas strictly exceeds `reset` gas) plus byte-parity locks.

## [Toolchain 0.31.106, language 0.23.103, runtime 0.16.100] — did.compact 0.5.0 codegen support (2026-08-05)

### Added

- **did.compact 0.5.0 codegen support** — the `compactc --rust` backend now
  generates and compiles the midnight-did 0.5.0 contract (controller-
  authorization + recovery). New closures: a JubjubPoint ledger-read decoder
  and typed initial-cell default; constructor-mode impure-circuit context
  threading (query, private, and zswap-local state); multi-assert if/else
  branches emitted in source order; and non-terminal branch calls threaded in
  source order with their returned context carried into the terminal op
  (A22–A26). Added the `did-05` regression fixture (vendored contract + its
  jubjub-schnorr dependency under `examples/did-05/`) to the
  `codegen_regression` byte-parity table and as a workspace compile gate. All
  pre-existing fixtures regenerate byte-identically.

## [Toolchain 0.31.105, language 0.23.103, runtime 0.16.100] — codegen correctness sweep + digital-passport (2026-07-10)

### Fixed

- **export-typedef promotion gated to `--rust`** — the M3.5-E2 pass that
  synthesises `export-typedef` entries for user structs/enums (so the Rust
  H5-H7 emitter can declare them) ran unconditionally and mutated the shared
  `Lexpanded` IR, drifting ~64 `compiler/test.ss` goldens across
  expand-modules-and-types / infer-types / reject-recursive-circuits /
  track-witness-data / combine-ledger-declarations. It is now
  `(when (emit-rust) …)`, so the TS-backend IR is unchanged and the Rust
  fixtures still get their typedefs. This let the CI drop all fork-scoped
  `if: github.repository != …` skips. See
  [ADR 0002](docs/adr/0002-gate-export-typedef-promotion-to-rust.md).

- **Struct-name disambiguation** — same-named structs from distinct module
  imports (e.g. a generic protocol module instantiated twice, or two modules
  each exporting `struct Rec`) are now resolved by structural fingerprint,
  keyed on field **names** as well as types, and the disambiguated name
  (`Name` / `Name_1`) is applied at **all** emission sites — struct
  literals, decoder turbofish, and default expressions — not only in type
  positions. Previously the fingerprint ignored field names (merging
  distinct structs → `E0609`) and value sites emitted the raw name
  (mismatch vs the disambiguated signature → `E0308`/`E0422`). Stdlib
  structs (`Maybe`, `MerkleTreePath*`, `ContractAddress`) are excluded from
  disambiguation. New `struct_collision_fixture` byte-parity gate; see
  [ADR 0001](docs/adr/0001-rust-struct-name-disambiguation.md).

- **A20** — read-no-arg adt-op vm-code lowering. `emit-ledger-read-expr`
  no-arg branch in `compiler/rust-passes-emit.ss` was hardcoded to emit
  `dup → idx → popeq` and silently discarded the adt-op's vm-code, so
  any contract calling `Set.size`, `Set.isEmpty`, `Map.size`, `Map.isEmpty`,
  `List.isEmpty`, `List.length`, or `HistoricMerkleTree.isFull` compiled
  to misencoded gather chains that decoded the container as a raw `Cell`.
  Fix routes these ops through `expand-vm-code` (same machinery as A8's
  read-with-arg path). New `set_size_fixture` byte-parity gate.
  Commits: [`0916b28`](../../commit/0916b28), [`c15af41`](../../commit/c15af41).

- **A21** — `HistoricMerkleTree.insertIndexDefault` circuit-body shape.
  Walker rejected the IR with `circuit-body-emission: no walker shape
  matched`. Added the body-shape and op-builder support
  (`lt`/`branch`/`jmp`/`swap`/`pop`). New `hmt_default_fixture`
  byte-parity gate. Commits: [`6aa3cdc`](../../commit/6aa3cdc), [`776d83e`](../../commit/776d83e).

- **Bug-8** — `==`/`!=` walker forced typed enum rendering when the
  comparison's IR `type` was a tenum (Bug-4's optimisation), but the IR
  type is the tenum even when one operand is a ledger-read decoded as
  `u8`. Election's `state.read() == PublicState.commit` generated
  invalid Rust `u8 == PublicState::commit`. Added `is-ledger-read-expr?`
  predicate; in `==`/`!=` clauses, short-circuit `typed?` to `#f` when
  either operand is a ledger-read. Commit: [`0fffe67`](../../commit/0fffe67).

- **Bug-9** — non-exported tenum decls referenced by emitted impure
  circuits. A18+A19 (`f9b509f`) taught the emitter to render
  non-exported impure circuits as inherent methods on `Contract<PS, W>`,
  but the type-declarations pass didn't follow the new type-reference
  graph. Tiny's `circuit in_state(s: STATE): Boolean` was emitted as a
  method, but the `STATE` enum was never declared. Extended
  `collect-pure-circuit-tdefns` in `compiler/rust-passes-decls.ss` to
  walk non-exported impure circuit signatures too. Commit:
  [`a45d68d`](../../commit/a45d68d).

- **Bug-10** — typed decoder for `tenum` ledger reads (Option A).
  Previously `decoder-for-type` lowered tenum-typed ledger fields via
  `decode_u8`, which broke `state == s` (where `s: STATE` is a
  tenum-typed formal arg) because `u8 == STATE` doesn't compile.
  Bug-8's `is-ledger-read-expr?` short-circuit fixed the case where the
  RHS is an `enum-ref` literal (drops to u8 via `enum-ref->u8`), but had
  no fallback for typed var-ref RHS. Option A's fix flips the LHS to
  decode as the typed enum (`decode_via_field_repr::<EnumName>`),
  eliminating the u8/tenum mismatch entirely. Bug-8's special-case
  becomes redundant for tenum-typed reads. Commit: [`62c81be`](../../commit/62c81be).

### Changed

- **Rust codegen: `assert` is now a handleable error, not a panic
  (parity with the TS backend).** `compactc --rust` now lowers every
  `assert(cond, "msg")` inside a pure circuit to the `compact_assert!`
  macro (returning `Err(CompactError::AssertionFailed(msg))`) and emits
  pure circuits with a `Result<T, CompactError>` signature, appending
  `?` at every pure-circuit call site so the error propagates through
  impure callers. Previously pure-circuit asserts lowered to a
  panicking `assert!`, aborting the host process — fundamentally
  different from the TS backend's catchable `CompactError` throw. The
  5 fixtures with pure circuits (tiny, zerocash, election,
  if-stmt-fixture, pure-circuit-fixture) were regenerated; the on-chain
  op-program/witness bytes are unchanged, so all 51 byte-parity tests
  stay green. A new dedicated `assert_parity` fixture + test proves a
  failing pure-circuit assert yields `Err(AssertionFailed)`, not a
  panic. Note: the hand-imported `digital-passport` crate (no `.compact`
  source in this repo) is not regenerated by the pipeline and keeps its
  123 panicking `assert!` until re-emitted from upstream.

- **24 fixture `lib.rs` files regenerated** against the post-Bug-10
  compactc (commit [`4e322bc`](../../commit/4e322bc)). Drift categories:
  `PS: Clone` widening (Bug-3, all 24); `.popeq(true)` → `.popeq(false)`
  honouring the vm-code's `cached` flag (election, tiny); `tiny.in_state`
  inherent method emission (Bug-9); typed tenum ledger decoders
  (Bug-10). All 47 byte-parity tests + `codegen_regression` green.

### Process notes

The 5-bug cascade was surfaced by manually running `codegen_regression`
against a fresh `compactc --rust` regen. The standing byte-parity test
corpus is structurally blind to source-level codegen drift — bugs that
change generated Rust without changing `ContractState::serialize()`
output (Bug-8's `u8 == EnumName::variant` compile error; Bug-9's missing
enum decl; Bug-10's wrong decoder) don't surface without an explicit
regen-and-diff step. Treating `codegen_regression` as a CI gate, not
just a test result, is now the standing policy.

## [Toolchain 0.31.104, language 0.23.103, runtime 0.16.100]

### Added

- Adds `--rust` to `compactc`: lowers a `.compact` contract to a native
  Rust crate (`contract/lib.rs`) that depends on the new `compact-runtime`
  crate. The Rust crate exposes `Contract::new(...)`, `initial_state(...)`,
  each impure circuit as a method on the contract, and a `Ledger<'a, D>`
  view for reading on-chain state — parallel to the TypeScript backend's
  surface, with byte-identical `ContractState.serialize()` output.
  - **Runtime crate** (`runtime-rs/`): curated prelude over the Midnight
    Rust crates (`midnight-base-crypto`, `midnight-transient-crypto`,
    `midnight-storage`, `midnight-onchain-state` / `-vm` / `-runtime`,
    `midnight-coin-structure`, `midnight-zswap`); facade aggregates
    (`ConstructorContext`, `CircuitContext`, `ConstructorResult`,
    `CircuitResults`, `WitnessContext`, `CompactError`); Compact stdlib
    helpers (`Counter`, `Maybe<T>`, `OpaqueString`, `pad`, `disclose`,
    `persistent_hash_aligned`, Jubjub/EC native shims, Merkle path
    helpers); `OpProgramVerify` / `OpProgramGather` builders.
  - **Codegen coverage**: scalars, ADTs as ledger fields
    (`Counter`/`Cell`/`Map`/`Set`/`MerkleTree`/`HistoricMerkleTree`/`List`),
    user structs and enums, transparent + nominal type aliases, witnesses,
    native hashes, `if`-statement bodies, ADT method calls (`insert` /
    `lookup` / `member` / `checkRoot`), cross-circuit calls, `for`-range
    and `for`-iterable loops with compile-time unrolling, basic `fold`
    with loop-var substitution, bounded `Uint<L..U>`, sealed ledger fields.
  - **Compile-time safety**: unsupported Compact constructs now produce a
    `compactc --rust: unsupported Compact construct (...)` error at
    codegen time rather than emitting `unimplemented!()` Rust that
    compiles but panics at runtime.
  - **Test coverage**: 32 cross-language byte-parity tests under
    `tests-e2e-rust/`, plus a codegen-regression guard asserting all
    committed `lib.rs` files regenerate byte-identical via
    `compactc --rust`.
  - **Docs**: an end-user guide at `doc/rust-codegen-user-guide.md`,
    contributor READMEs under `compiler/README-rust-passes.md`,
    `runtime-rs/README.md`, and `tests-e2e-rust/README.md`, plus a
    parity gap report under
    `docs/superpowers/research/2026-06-02-upstream-parity-gap-report.md`.

## [Toolchain 0.31.103, language 0.23.103, runtime 0.16.100]

### Added

- Adds `keccak256` to the standard library, with the same signature as
  `persistentHash`.  Adds `keccak256` to the Compact runtime with the same
  signature as `persistentHash`.  `keccak256` requires the experimental feature
  flag `--feature-zkir-v3` to work in a circuit that directly or indirectly uses
  the public ledger state.  It is a compiler error to use it in a such a circuit
  using the ZKIR v2 backend.

## [Toolchain 0.31.102, language 0.23.102, runtime 0.16.0]

### Added

- `eval` and `arguments` which are reserved words in strict mode of JavaScript are now
  added as future reserved words in Compact. Previously Compact accepted a contract using
  these as an identifier, resulting in producing an invalid JavaScript output.

### Fixed

- Lexer matches the convention used by
  ECMAScript (https://tc39.es/ecma262/#sec-names-and-keywords) and
  UAX #31 (https://www.unicode.org/reports/tr31/#Table_Lexical_Classes_for_Identifiers):
  the lexer accepts Unicode `ID_Start` (`Lu Ll Lt Lm Lo Nl`) plus `_` and `$`.
  Previously it accepted all alphabetic charactes which includes some non-`ID-Start`
  characters which are invalid in JavaScript.
  `identifier-subsequent?` now follows Unicode `ID_Continue` (`Lu Ll Lt Lm Lo Nl Mn Mc Nd Pc`).
  Previously it included som non-`ID-Continue` characters.

## [Toolchain 0.31.101, language 0.23.101, runtime 0.16.0]

### Added

Adds `event` and `log` as future keywords that are reserved.

## [Toolchain 0.31.0, language 0.23.0, runtime 0.16.0]

This release includes all changes for compiler versions in the range between
0.30.100 and 0.31.0; language versions in the range between 0.22.100 and 0.23.0;
and Compact runtime versions in the range between 0.15.100 and 0.16.0.

## [Toolchain 0.30.107, language 0.22.101, runtime 0.15.101]

### Fixed

Various zkir operators that can result in assertion failures and thus should
be executed conditionally do not have guards are thus actually executed
unconditionally.  This can result in proof failures for correct transactions.
For example, casting an unsigned integer value to a smaller unsigned type will
always cause the proof to fail when the value is too big for that type, even if
the cast occurs in a branch that is not taken in the Compact code.

The intent is to add guards to these operators in the next version of zkir.
In the meantime, the compiler implements workarounds that arrange to invoke
these operators with inputs that cannot cause assertion failures when the
guard would be false.

The downside of these workarounds is that they can increase the size of the
generated circuit.
The size increase arises from conditional use (i.e., use in the `then` or
`else` part of an `if` statement or expression) of:

- downcasts from Uint types to smaller Uint types,
- downcasts of Field to Uint types,
- conversions of byte vectors to and from fields or unsigned integers,
- conversions of vectors to byte vectors, and
- uses of relational comparison expressions (<, <=, >=, and >) with inputs
  that might be unknown.

If the increase in circuit size is problematic for a particular contract, developers
should consider moving downcasts, conversions, and relational comparisons outside
of `if` expressions where possible until zkir supports the required guards and the
compiler workarounds have been removed.

## [Toolchain 0.30.106, language 0.22.101, runtime 0.15.101]

### Added

- Adds a `ledger` key to `contract-info.json` listing the contract's
  ledger fields. Each entry contains the field name, path index,
  export status, storage kind (Cell, Counter, Map, Set, List,
  MerkleTree, HistoricMerkleTree), and fully-resolved type tree.
  This enables language-agnostic tooling to discover a contract's
  ledger layout from the compiler output alone. Both exported and
  non-exported fields are included since the full layout is required
  to navigate the on-chain state tree and construct initial states.

## [Toolchain 0.30.105, language 0.22.101, runtime 0.15.101]

### Added

- Adds `--line-length` flag to fixup.

### Fixed

- JubjubPoint equality is now component-wise; it previously was reference
  equality.

## [Toolchain 0.30.104, language 0.22.101, runtime 0.15.101]

### Changed

- Renames `doc/lang-ref.mdx` and `compiler/lang-ref-proto.mdx` to
  `doc/compact-reference.mdx` and `compiler/compact-reference-proto.mdx`,
  respectively.  It also adopts some changes from midnight-docs PR changes
  for lang-ref 1.0.

## [Toolchain 0.30.103, language 0.22.101, runtime 0.15.101]

### Changed

- The language reference `doc/lang-ref.mdx` is now been fully revised and
  is completely up-to-date with the Compact Version 1.0 language.  Grammar
  snippets are automatically inserted into the document directly from parser.ss,
  and several changes have been made to the presentation of the grammar to
  make it more readable.

## [Toolchain 0.30.102, language 0.22.101, runtime 0.15.101]

### Changed

- Extends the `for (const i of start..end) stmt` syntax to allow `start` and
  `end` to be references to generic parameters.

## [Toolchain 0.30.101, language 0.22.0, runtime 0.15.101]

- Changes the format of the first argument passed to `convertBytesToUint` in `print-typescript` 
- Improves format of error messages for `convertBytesToUint` and `convertBytesToField`
- Changes the type of `maxval` to `bigint` to avoid JavaScript silently losing precision
  when comparing `x > maxval` for larg `Uint`s

## [Toolchain 0.30.0, language 0.22.0, runtime 0.15.0]

This release includes all changes for compiler versions in the range between
0.29.100 and 0.30.0; language versions in the range between 0.21.100 and 0.22.0;
and Compact runtime versions in the range between 0.14.100 and 0.15.0.

## [Unreleased toolchain 0.29.114, language 0.21.101, runtime 0.14.102]

### Changed

- The language reference `doc/lang-ref.mdx` is now largely up-to-date with
  the Compact 0.21.0 language.
- The HTML version of the formal grammar in `doc/Compact.html` has been
  replaced with a markdown (mdx) version in `doc/compact-grammar.mdx`.

### Added

- A list of Compact's keywords and reserved words, including those reserved
  for future use, is given in `doc/compact-keywords.mdx`.

## [Unreleased toolchain 0.29.113, language 0.21.101, runtime 0.14.102]

### Changed

- It is now a compiler error to pass Compact values containing opaque JS values
  (`Opaque<'string'>` or `Opaque<'Uint8Array'>`) to the standard library
  circuits `persistentHash` and `persistentCommit`.  Hashing such values does
  not work in circuit due to the representation of these types.  Previously,
  such code would crash the `zkir` process if it tried to generate prover and
  verifier keys.  Now it is a compiler error instead.
  
  This also affects the standard library operation `merkleTreePathRoot` (because
  it calls `persistentHash` in its implementation), and ledger `MerkleTree`
  insertion operations, because they implicitly use `persistentHash`.
  
  This is a **breaking** change because the error is signaled early, and so it
  is now an error to use any of these circuits or ADT operations, even for
  circuits that don't need prover and verifier key generation which would
  compile successfully before.

## [Unreleased toolchain 0.29.112, language 0.21.101, runtime 0.14.102]

### Changed

- The fixup tool now replaces references to the old standard-library type names
  `CurvePoint` and `NativePoint` with `JubJubPoint`.  It also does a better job
  of renaming standard-library circuits when it is safe to do so and explaining
  why when it is not safe to do so.

### Internal notes

- The expand-modules-and-types code for function lookup is more modular and
  easier to read.

## [Unreleased toolchain 0.29.111, language 0.21.101, runtime 0.14.102]

### Fixed

- The `<=` and `>` operand evaluation order in the proof circuit is incorrect
  (right-to-left rather than left-to-right).  It also differs from the evaluation
  order in the generated JavaScript code, which can result in proof failures
  when the operands are non-trivial.  This fix modifies the common upstream path
  `infer-types` to enforce the correct evaluation order.

## [Unreleased toolchain 0.29.110, language 0.21.101, runtime 0.14.102]

### Fixed

- There was an unreleased bug in ZKIR circuits (not in JS) where the
  representation of the default `JubjubPoint` was wrong.  Fixing this entailed
  allowing `default` in compiler IR from `Lflattened` and downstream in both
  ZKIR v2 and v3 backends.

## [Unreleased toolchain 0.29.109, language 0.21.101, runtime 0.14.102]

### Changed

- The compiler binary can now report `--ledger-version` (and
  `--feature-zkir-v3 --ledger-version`).  This is the version of the ledger that
  is targeted by the generated code and used to produce the generated prover and
  verifier keys.

## [Unreleased toolchain 0.29.108, language 0.21.101, runtime 0.14.102]

### Fixed

- Type declarations of `Uint<n>` and `Uint<0..n>` where `n` is a free type variable
  are now accepted by the compiler.

## [Unreleased toolchain 0.29.107, language 0.21.101, runtime 0.14.102]

### Changed

- The ZKIR v3 format, behind the feature flag `--feature-zkir-v3`, has changed
  so that:
  - circuit inputs are correctly typed as either `Scalar<BLS12-381>` or
    `Point<Jubjub>` (before they were always scalars, with `encode` instructions
    for curve points), and
  - `private_input` and `public_input` instructions are typed (before they
    always read scalars, with `encode` instructions for curve points)

## [Unreleased toolchain 0.29.106, language 0.21.101, runtime 0.14.102]

### Changed

- The Compact compiler now targets `midnight-ledger` version 8.0.0.  The Compact
  runtime now imports `onchain-runtime-v3` (instead of `-v2`) at version
  compatible with 3.0.0-rc.2.

## [Unreleased toolchain 0.29.105, language 0.21.101, runtime 0.14.101]

### Fixed

- [Breaking Change] The search order for include and external module files
  specified with non-absolute paths has been fixed so that (a) the compiler looks
  first relative to the directory of the including or importing file, and (b)
  the compiler does not automatically look in the directory where the compiler
  was invoked.

### Added

- compactc and fixup-compact support two new options: --compact-path to
  set the compact path and --trace-search to cause the compiler to say where
  it looks for include and external module files.  If the `--compact-path`
  command-line option is present, the environment variable `COMPACT_PATH`
  is ignored.

## [Unreleased toolchain 0.29.104, language 0.21.101, runtime 0.14.101]

### Added

- The generated TypeScript now includes a `ProvableCircuits<PS>` type and a
  `provableCircuits` field on the `Contract` class.  `ProvableCircuits` contains
  only the circuits that have verifier keys (i.e., circuits that appear in the
  flattened circuit IR and produce ZKIR files).  This distinguishes them from
  impure circuits that only call witnesses without touching the ledger.

### Fixed

- `setOperation` is now emitted only for provable circuits (those in
  `proof-circuit-name*`) rather than for all impure circuits.  Previously,
  witness-only impure circuits caused the runtime to look
  for a verifier key that does not exist.

## [Unreleased toolchain 0.29.103, language 0.21.101, runtime 0.14.101]

### Changed

- The standard library type `NativePoint` has been removed.  The standard
  library type `JubjubPoint` is now a `new type` alias for
  `Opaque<'JubjubPoint'>`.  This way `Opaque<'JubjubPoint'>` isn't really
  hidden, but it's not shown in error messages.
- `NativePoint` circuits in the standard library and the corresponding
  same-named functions in the Compact runtime have been renamed, and they now
  take or produce `JubjubPoint` values.
  - `nativePointX` -> `jubjubPointX`
  - `nativePointY` -> `jubjubPointY`
  - `constructNativePoint` -> `constructJubjubPoint`
- Signatures of elliptic curve operations in the standard library now use
  `JubjubPoint` in place of `NativePoint`.

### Internal notes

- The `compact fixup` tool can do these renamings except it cannot currently
  rename types (e.g. `NativePoint` to `JubjubPoint`).

## [Unreleased toolchain 0.29.102, language 0.21.100, runtime 0.14.100]

### Added

- There is a new builtin type `Opaque<'JubjubPoint'>`.  Unlike the other opaque
  types, this is intended to be a crypto backend (ZKIR) native type (not a JS
  type).  The standard library exports the type `JubjubPoint` which is a
  (transparent) `type` alias for the opaque type.

### Changed

- The standard library's (opaque) `new type` alias `NativePoint` now has
  underlying type `Opaque<'JubjubPoint'>`.
- The Compact runtime's types `CompactTypeNativePoint` and `NativePoint` are
  renamed to `CompactTypeJubjubPoint` and `JubjubPoint`.
- The runtime has TS (instead of Compact) implementations of the now-builtin
  `NativePointX` and `NativePointY` circuits.
- The feature flag `--zkir-v3` is changed to `--feature-zkir-v3` to fit a
  proposed standard naming convention, and to make crystal clear that it is
  still an experimental feature.

### Internal notes

- When the flag `--feature-zkir-v3` is enabled, `Opaque<'JubjubPoint'>` is
  represented natively in ZKIR v3.  Without the flag, it is still represented as
  a pair of field elements in ZKIR v2.
- This is implemented as a "pseudo"-alignment tag after flattening.  The tag
  looks like `(anative "JubjubPoint")` and it's interpreted as a `midnight-zk`
  JubjubPoint for ZKIR operations, converted to a pair of field values for
  the Impact code embedded in the ZKIR circuit.
- ZKIR v3 has new `encode` and `decode` gates for converting from ZKIR
  representations to Impact representations and back.
- ZKIR v3's `ec_add` has been eliminated; regular `add` is polymorphic,
  operating on either a pair of scalars or a pair of Jubjub curve points.
- ZKIR v3 has type annotations on circuit inputs and on `decode` instructions.
- ZKIR v3 has two types: `Scalar<BLS12-381>` and `Point<Jubjub>`.
- For both ZKIR v3 and ZKIR v2 modes, the JS representation of is still as a pair
  of field elements.

## [Unreleased toolchain version 0.29.101, language version 0.21.0]

### Changed

- In the formal grammar, the `stmt0` grammar production for one-armed
  `if` expressions has been removed.  It was unnecessary and made the grammar
  ambiguous.

## [Unreleased toolchain 0.29.100, language 0.21.0]

### Changed

The compiler binary can now report `--runtime-version`, the version of the
Compact runtime JS package that it will import in generated contract code.

## [Toolchain version 0.29.0, language version 0.21.0]

This release includes all changes for compiler versions in the range
0.28.100 and 0.29.0; and language versions in the range 0.20.100 and
0.21.0.  It uses Compact runtime 0.14.0 and on-chain runtime
compatible with 2.0.0.

## [Unreleased compiler 0.28.109, language 0.20.102]

### Fixed

- The fixup tool fixup-compact.ss failed to look for include files and modules
  relative to the directory of the source pathname.

## [Unreleased compiler 0.28.108, language 0.20.102]

### Removed

- The syntax for external circuits, i.e., circuit definitions with no body,
  has been removed.  This syntax was used exclusively for declaring built-in
  natives and was not useful outside of the compiler.

### Internal notes

- The compiler now injects natives directly into the standard library module.
  This is simpler and gives us a single source of truth for natives.

## [Unreleased compiler 0.28.107, language 0.20.101]

### Fixed

- An issue that caused transactions involving `mintShieldedToken`, `sendShielded`, `mintUnshieldedToken`, or
  `sendUnshielded` to fail validation with `RealUnshieldedSpendsSubsetCheckFailure` when the caller was also the 
  recipient of the newly minted token.

## [Unreleased compiler 0.28.106, language 0.20.100]

### Fixed

- An issue that caused the compiler to take an excessive amount of time to compile
  certain `for` loops, `fold` expression, and `map` expressions.

- An bug that caused the compiler to miss some of certain repeated disclosures
  of a witness value and to overstate the nature of certain other disclosures.

### Changed

- Messages about undeclared witness-value disclosures are now produced in an order
  that attempts, for each disclosure point and witness value, to put the most severe
  disclosures along the shortest paths first, since understanding these is easier
  and properly declaring them often addresses the others.

### Internal notes

- The underlying issue was the representation and maintenance of paths in the
  witness-protection program, and this has been replaced by a simpler mechanism
  with some careful crafting of the code to reduce computational complexity and
  generally make the compiler more efficient.

## [Unreleased compiler 0.28.105, language 0.20.100]

### Added

- The file compiler/contract-info.json that compactc generates in the output
  directory now includes some extra information: (1) version strings for the
  compiler, language, and runtime, and (2) for each circuit, a flag saying whether
  the circuit requires a proof (and therefore whether compactc has produced zkir
  code and prooving keys for it in the zkir and keys subdirectories of the output
  directory).

- ARM Linux artifact is added.

### Internal notes

- Adding the proof flag involved moving the pass that saves the contract-info file
  later in the compiler.  This in turn uncovered a couple of bugs in the preliminary
  handling of (as yet unsupported) cross-contract calls.  These have been fixed,
  though the code remains largely untested.  The zkir passes now recognize
  cross-contract calls and explicitly reject them as unsupported.

## [Unreleased compiler 0.28.104, language 0.20.100]

### Fixed

- A bug reported in issue [#34](https://github.com/LFDT-Minokawa/compact/issues/34) in which 
  `ChargedState` was not properly copied resulting in junk metadata being passed to contract deployments.

## [Unreleased compiler 0.28.103, language 0.20.100]

### Fixed

- A bug in the experimental `--zkir-v3` feature.  The on-chain representation of
  coin commitments changed between ledger version 6.1 and 6.2.  The domain
  separator string is changed, and the inputs to `persistentHash` are in a
  different order.

  This was already implemented for the default ZKIR v2, but the corresponding
  change was not implemented in the ZKIR v3 compiler passes.

## [Unreleased compiler 0.28.102, language 0.20.100]

### Changed

- For any circuit that returns something other than `[]` and for which some path through
  the circuit does not end in `return` form or ends in a `return` form without
  a return-value expression, the resulting error message now clearly states that
  this is the problem.

## [Unreleased compiler 0.28.101, language 0.20.100]

### Added

- Added a constructor, `constructNativePoint`, for `NativePoint` values

### Changed

- Renamed the existing accessors `NativePointX` and `NativePointY` to `nativePointX`
  and `nativePointY` for consistency with our conventions for circuit names.

## [Unreleased compiler 0.28.100, language 0.20.0]

There are no user-visible changes.

### Internal notes

- Instead of pulling test contracts from the separate (private) repository
  `midnight-contracts`, they are added to this repository under
  `test-center/test-contracts`.
  
## [Unreleased compiler 0.28.100, language 0.20.0]

### Changed

- The informal parser rule that "else" clauses belong to the innermost "if"
  expression is now explicit in the grammar.  Previously, we were relying on a
  shaky assumption about how the parser generator treats grammar ambiguities.
  This change is reflected in the formal grammar specification in doc/Compact.html
  but has no impact on how programs are compiled.

## [Compiler version 0.28.0, language version 0.20.0]

This release includes all changes for compiler versions in the range 0.27.100
(inclusive) and 0.28.0 (exclusive); and language versions in the range 0.19.100
(inclusive) and 0.20.0.  It uses compact-runtime 0.14.0-rc.0 and 
on-chain runtime 2.0.0-alpha.1.

## [Unreleased compiler version 0.27.113, language version 0.19.103]

### Changed

- The formatter's handling of several forms has been improved:
  - When the signature of a function needs to be broken up into multiple lines,
    the parameter list is also broken up into multiple lines (even if it would itself
    fit on one line), and the return-type declaration appears on a line following
    the last parameter declaration. This change applies to circuit definitions,
    external declarations, witness declarations, the constructor, and anonymous
    circuit definitions.
  - When a call expression needs to be broken up into multiple lines, the argument
    list is also broken up into multiple lines (even if it would itself
    fit on one line), and the closing parenthesis of the call appears on a line
    following the last argument expression.
  - When an anonymous circuit needs to be broken up into multiple lines, the body
    of the circuit is indented a few spaces in from the start of the parameter
    list rather than all the way out beyond the circuit's signature.
  - When the "else" expression of an "if" expression is itself an "if" expression,
    the inner "if" expression begins on the same line as the "else" and appears at
    at the same level of indentation as the outer "if" expression, in a case-like
    structure.  This special treatment is inhibited by end-of-line comments between
    the outer "else" keyword and the inner "if" keyword.

- The formatter now accepts a --line-length <n> parameter that sets the target line
  length to <n>.  The default line length currently defaults to 100.  The target line
  length can be exceeded in cases where the formatter considers the portion of input
  to be fit on a line to be unbreakable.

### Internal notes

- Configuration parameters have been collected into a single new library, (config-params)

- The formatter line length is now a configuration parameter, set to 100 by default.

- compiler/go now catches keyboard interrupts while running the tests and aborts the tests.

- compiler.md now more accurately describes the composition of the token stream.

- The formatter improvements are supported by the following changes:
  - add-block (appropriately renamed make-Qblock, since it returns a block)
    has been simplified to take a header rather than a proc that produces a header
  - make-Qsep has been split into two routines, one that expects a closer and one
    that doesn't.
  - make-Qsep and make-Qconcat now take an inherit-break? flag whose value is
    recorded in the resulting Qconcat record.  Processing a Qconcat with this
    flag set in the context in which lines are being broken causes the contents
    of the Qconcat itself to be broken into multiple lines.  The contents of a
    Qconcat q with this flag set are still indented relative to q.
  - The code for handling function signatures is now commonized into a single
    constructor make-Qsignature.

## [Unreleased compiler version 0.27.112, language version 0.19.103]

### Changed

- The Compact standard library structure type NativePoint (nee CurvePoint)
  is now a nominal type alias for an unexported internal type.  The standard
  library also now exports two new circuits, NativePointX and NativePointY,
  that can be used to access the x and y coordinates of a native point as Fields.
  This is a breaking change because the internal representation of NativePoint
  is no longer exposed.

- In type errors produced by the Compact compiler, Nominal type aliases are
  now shown simply as TypeName rather than as TypeName=Type.

## [Unreleased compiler version 0.27.111, language version 0.19.102]

### Changed

- Changes `CurvePoint` to `NativePoint`

## [Unreleased compiler version 0.27.110, language version 0.19.101]

### Changed

- Fixes PM-19299 by having `createZswapInput` and `createZswapOutput` return
  an empty array to represent the `[]` type in Compact.

## [Unreleased compiler version 0.27.109, language version 0.19.101]

### Changed

- The compiler now targets ledger version 7.0 instead of 6.2.  There are no API
  changes between 6.2 and 7.0 so it is only necessary to pull in a new
  implementation of the on-chain runtime and bump version numbers.  This is
  **not** a breaking change.

## [Unreleased compiler version 0.27.108, language version 0.19.101]

### Added

- The reserved words from TypeScript and JavaScript are now included in our
  future reserved words.

## [Unreleased compiler version 0.27.107, language version 0.19.100]

### Changed

- The compiler now targets ledger version 6.2 instead of 6.1.  This ledger
  version has changes to Zswap hashing made in response to ledger audit
  feedback.

- There are standard library changes to **non-exported** structs and circuits,
  so this is **not** a breaking change.

## [Unreleased compiler version 0.27.106, language version 0.19.100]

### Fixed

- Bugs in unreleased code preventing proper behavior of type aliases for certain
  uses of ADT types, including ledger operations that treat parameters of type
  QualifiedCoinInfo differently and the += and -= operators for incrementing
  Counters.

## [Unreleased compiler version 0.27.105, language version 0.19.100]

### Changed

- The compiler no longer generates zkir code or proving keys for circuits that
  do not directly touch the ledger.  Previously, it generated zkir code and
  proving keys for all impure circuits, so merely calling a witness or invoking
  one of the witness-like external circuits (`ownPublicKey`, `createZswapInput`,
  `createZswapOutput`) would also trigger zkir and proving-key generation.

## [Unreleased compiler version 0.27.104, language version 0.19.100]

### Fixed

- The compiler now rejects programs whose constructors contain array-reference,
  and bytes-reference, and slice expressions with out-of-bounds indices.
  Previously, such errors could lead to these expressions producing undefined
  values at run time.

## [Unreleased compiler version 0.27.103 language version 0.19.100]

### Added

- Compact now supports the definition of type aliases:
  Structually typed aliases:
    `type Name = Type;` defines `Name` to be an alias for `Type`.  For example,
    `type U32 = Uint<32>` defines `U32` to be the equivalent of and interchangeable
    with `Uint<32>`.

  Nominally typed aliases:
    `new type Name = Type;` is similar, but `Name` is defined as a distinct type
    compatible with `Type` but neither a subtype of nor a supertype of `Type` or
    any other type.  It is compatible in the senses that (a) values of type `Name`
    can be used by primitive operations that require a value of type `Type`, and
    (b) values of type `Name` can be explicitly cast to and from type `Type`.
    For example, within the scope of `type V3U16 = Vector<3, Uint<16>>`, a value
    of type `V3U16` can be referenced or sliced just like a vector of type
    `Vector<3, Uint<16>>`, but it cannot, for example, be passed to a function
    that expects a value of type `Vector<3, Uint<16>>` without an explicit cast.

    When one operand of an arithmetic operations (e.g., `+`) receives a value
    of some nominally typed alias T, the other operand must also be of type T,
    and the result is cast to type T, which might cause a run-time error if the
    result cannot be represented by type T.

    Values of some nominally typed alias T cannot be directly compared (using,
    e.g., `<`, or `==`) with values of any other type without an explicit cast.

  Both types of aliases can take type parameters, e.g.:
  `type V3<T> = Vector<3, T>`
  `new type VField<#N> = Vector<N, Field>`

  This is a breaking change due to the reservation of the `new` and `type` keywords.

### Changed

- Out-of-range constant Bytes value indices are now detected earlier in the
  compiler, which means that additional such errors might be caught, specifically
  those in code that is later discarded.  This is a breaking change.

- Upward casts no longer prevent tuple references and slices from recognizing
  constant indices, which allows more programs with references to non-vector tuple
  types to pass type checking.

### Fixed

- A bug that caused a misleading source location to be reported for some type
  errors, e.g., for invalid arguments to some calls to `map` and `fold`.

### Internal notes

- The Public-ledger ADT (`public-adt`) form, which describes the type of a
  public-ledger ADT, has been replaced by a new Type `tadt` throughout the compiler.
  This simplifies and regularizes the representation of types and allows type
  aliases to be used for ADT types as well as for non-ADT types.

- Equality testing in the unit-test framework has been tightened up to avoid
  false positives when the expected output uses different symbols to represent
  what turns out to be the same id or gensym in the actual output.  This can
  occur when the expected output is wrong or the compiler actually generates
  code that uses the same id or gensym for different purposes.  Several instances
  of the first have been fixed in the unit tests.

- A new checker, `pass-returns`, as been added to the unit-test framework.  It
  is like `returns` but checks the output of a specific pass.  This is intended
  to allow us to move toward having a single occurrence with checks for multiple
  passes rather than having to put multiple copies of the same test in different
  test groups.

- A new form `(assertf expr format-string arg ...)` has been added to utils.ss.
  Like `(assert expr)`, it returns the value of `expr` if `expr` evaluates to a
  true value and raises an exception if `expr` evaluates to #f.  Its error message
  includes the source location of the `assertf` form, as with `assert`, and also
  the result of applying `format` to `format-string` and `arg ...`.  `assertf`
  is useful in preference to `assert` when the assertion expression does not
  already indicate the problem and the problem is not otherwise obvious from the
  context.

- internal-errorf now also includes the source location in the error message.

## [Unreleased compiler version 0.27.102, language version 0.19.0]

### Changed

- The unique variable names in the ZKIR v3 output are now produced in such a way
  that they are stable in the face of changes in the order or set of circuits
  generated.  That is, if the generated zkir for a circuit doesn't otherwise
  change, the variable names should also be identical.

### Internal notes

- Running the unit tests in test.ss now produces the file replacement-results.ss
  containing one entry for each result that differs from the expected result,
  e.g., each returns form when the returned result is different, each oops
  form when the condition is different, each output-file result with the
  output is different, etc.  No entry is included for unexpected exceptions,
  e.g., no entry is included for a return form if an exception occurs instead.
  If replacement-results.ss would be empty, it is deleted and not created.
  The new program compiler/update-test.ss takes as input the pathname of the
  test file (usually compiler/test.ss), the pathname of the replacements file
  (usually replacement-results.ss), and the pathname of an output file (e.g.,
  /tmp/test.ss).  Bad things will happen if the output pathname identifies that
  same file as the input pathname.  update-test.ss applies the replacements in
  the replacements file to the input file and puts the result in the output file.
  The output file can then be manually copied over the input file.  This is useful
  primarily when making cosmetic changes that affect a large number of tests and
  only after spot-checking to make sure that the cosmetic change is doing no harm.

## [Unreleased compiler version 0.27.101, language version 0.19.0]

### Changed

- The ZKIR v3 format (behind the feature flag --zkir-v3) is changed to coalesce
  an Impact instructions encoding into a guarded array.  Previously they were
  multiple unguarded instructions followed by a guarded "skip" instruction.

## [Unreleased compiler version 0.27.100, language version 0.19.0]

### Fixed

- Use of `return` statements among the statements comprising the body of a `for`
  loop are not supported.  Previously, such uses resulted in strange run-time
  behavior or confusing compile-time error messages.  The compiler now explicitly
  flags such uses as static errors with an appropriate error message.

## [Compiler version 0.27.0, language version 0.19.0] - Branched 2025-11-19

This release includes all changes for compiler versions in the range 0.26.100
(inclusive) and 0.27.0 (exclusive); and language versions in the range 0.18.100
(inclusive) and 0.19.0.

## [Unreleased compiler version 0.26.121 language version 0.18.103]

### Changed

- Changed the intermediate languages leading up to Lexpr to reflect that circuit
  and constructor bodies must be blocks rather than arbitrary statements.  reworked
  hoist-local-variables to avoid a dependency on a fluid variable.  These are not
  user-visible changes.

## [Unreleased compiler version 0.26.120 language version 0.18.103]

### Changed

- Changed the (experimental, not yet announced) ZKIR v3 format to use symbolic
  names instead of indexes for instruction inputs and ouputs.

## [Unreleased compiler version 0.26.119 language version 0.18.103]

### Fixed

- The type checker was not raising an exception for casts from Bytes<0> values
  to Field or Uint values and vice versa, which led to confusing downstream errors
  in some cases.

## [Unreleased compiler version 0.26.118 language version 0.18.103]

### Added

- Four new kernel operations, `mintUnshielded`, `claimUnshieldedCoinSpend`, `incUnshieldedOutputs`, and
  `incUnshieldedInputs`.
- Eight new standard library functions, `mintUnshieldedToken`, `sendUnshielded`, `receiveUnshielded`,
  `unshieldedBalance`, `unshieldedBalanceLt`, `unshieldedBalanceGte`, `unshieldedBalanceGt`, `unshieldedBalanceLte`.

### Changed

- Updates the repository to use ledger `6.1.0-alpha.5`, i.e., `@midnight-ntwrk/onchain-runtime-v1` version `1.0.0-alpha.5`.
- Changes names like `QualifiedCoinInfo` and `CoinInfo` to be `QualifiedShieldedCoinInfo` and `ShieldedCoinInfo` to
  match the names in the new on-chain runtime.
- Renames standard library functions to distinguish between shielded and unshielded token utilities.

## [Unreleased compiler version 0.26.117 language version 0.18.102]

### Fixed

- A bug in which types other than tuple, vector, and bytes do not result in an internal
  error when checking the bounds of an index.  This was an unreleased bug, that is,
  the bug was created in an unreleased version of the compiler.

## [Unreleased compiler version 0.26.116 language version 0.18.102]

### Fixed

- A bug in which unimported modules enclosed in unimported modules are not processed
  to detect and report certain errors, including type errors.  While it is
  essentially harmless not to process unimported modules since code in unimported
  modules is never run, this fix potentially allows some issues to be detected
  earlier in the application development process.

## [Unreleased compiler version 0.26.115 language version 0.18.102]

### Fixed

- A bug in which the compiler sometimes mentioned the same incompatible function
  more than once in the error message produced when no function with compatible
  generic or run-time parameters is found at a call site.

## [Unreleased compiler version 0.26.114 language version 0.18.102]

### Changed

- The maximum representable unsigned integer has been reduced from the maximum value
  that fits in the number of _bits_ in a field to the maximum value that fits in the
  number of _bytes_ in a field.  This change is necessary because values that do not
  fit in the number of bytes in a field do not have a valid representation in the
  ledger.  Given that the maximum field value at present is between 2^254 and 2^255,
  the number of whole bytes representable by a field is 31, and the maximum unsigned
  value is (2^8)^31-1 = 2^248-1.

  This is a breaking change because programs that used unsigned integers between
  2^248 (inclusive) and 2^254 (exclusive) will no longer compile.  Though while they
  would previously have compiled, they would not necessarily have worked properly.

## [Unreleased compiler version 0.26.113 language version 0.18.101]

### Fixed

- A bug in which some obviously unreachable statements were not being reported as such.
  This should be considered a breaking change since some programs that previously compiled
  will no longer compile due to this fix.

## [Unreleased compiler version 0.26.112 language version 0.18.101]

### Changed

- `Uint` range end points are now exclusive rather than inclusive to match the
  range syntax for `for` ranges.  That is, `Uint<0..n>` is now interpreted as the
  set of all unsigned integers in the range 0 through `n-1`, e.g., `Uint<0..3>`
  represents the set {0, 1, 2} rather than the set {0, 1, 2, 3}.

- The runtime version has been bumped to 0.10.2.

- when passed the `--update-Uint-ranges` flag, `fixup-compact` now adjusts the
  end point of each Uint whose size is given by a range with a constant end point
  and issues a warning for each Uint whose size is given by a range when the end
  point is a generic-variable reference.

## [Unreleased compiler version 0.26.111 language version 0.18.100]

### Fixed
- A bug in which Compact enums were generated as CJS enums instead of ESM enums. Previously, `index.js` might contain:

  ```javascript
  var Status;
  (function (Status) {
  Status[Status['Pending'] = 0] = 'Pending';
  // ...
  })(Status = exports.Status || (exports.Status = {}));
  ```

  for an enum `Status`. Now, `index.js` contains:

  ```javascript
  export var Status;
  (function (Status) {
    Status[Status['Pending'] = 0] = 'Pending';
    // ...
  })(Status || (Status = {}));
  ```

## [Unreleased compiler version 0.26.110 language version 0.18.100]

### Fixed
- An unreleased bug that was created during putting bounds on vectors/tuples/bytes

## [Unreleased compiler version 0.26.109 language version 0.18.100]

### Fixed

- A bug that could cause ledger operations or witness calls occurring
  in the test part of an `if` expresssion not to be reflected in the
  generated zkir circuit.

## [Unreleased compiler version 0.26.108 language version 0.18.100]

### Fixed

- A bug in unreleased code that caused an internal error message
  about an invalid source object.
- Internal language version is now properly bumped to 0.18.100.

## [Unreleased compiler version 0.26.107 language version 0.18.1]

### Fixed

- A bug that allowed const statements binding patterns or multiple variables
  to appear in a single-statement context, e.g., the consequent or alternative
  of an `if` statement.

## [Unreleased compiler version 0.26.106 language version 0.18.1]

### Added

- Selective module import and renaming, e.g.:
    `import { getMatch, putMatch as $putMatch } from Matching;`
      imports `getMatch` as `getMatch`, `putMatch` as `$putMatch`
    `import { getMatch, putMatch as originalPutMatch } from Matching prefix M$;`
      imports `getMatch` as `M$getMatch`, `putMatch` as `M$originalPutMatch`
  The original form of import is still supported:
    `import Matching;`
      imports everything from `Matching` under their unchanged export names
    `import Matching prefix M$;`
      imports everything from `Matching` with prefix M$

### Fixed

- A bug that sometimes caused impure circuits to be identified as pure
