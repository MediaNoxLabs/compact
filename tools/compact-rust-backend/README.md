# Compact Rust backend (ledger 8)

This backend compiles Compact through the ledger-8 Scheme frontend and renders
its typed Rust IR as native Rust source. It is independent of the TypeScript
backend. The generated code uses [`midnight-compact-runtime`](../../runtime-rs),
which delegates state, encoding, hashing, curves, and VM operations to the
ledger-8 Midnight crates.

## Compile a contract

The packaged `compactc` accepts a repeatable `--target` option. TypeScript is
the default; select Rust to generate a directly usable Cargo library:

```sh
compactc --target rust \
  examples/rust_backend/counter.compact /tmp/compact-rust-output
```

The output contains `contract/Cargo.toml`, `contract/lib.rs`,
`contract/compact-rust-ir.json`, and source copies of the matching runtime and
derive-macro crates, along with the compiler metadata, ZKIR, and proof
artifacts produced by the Scheme compiler. The Cargo library uses the bundled
runtime as a path dependency. A separate Rust
project can depend on `contract/` by path without copying generated source or
editing its manifest. Run with `--target ts --target rust` to emit both contract
languages. `--skip-zk` skips proving keys for a quicker local build.
For declared Cells and Counters, the generated `ledger_slots` module exposes
typed constants such as `ledger_slots::flag` and `ledger_slots::round`. Their
methods run the matching runtime VM operations or add them to a recording
frame; a Cell write accepts only the declared Rust value type. Generated
recorded methods call these descriptors as well. This first descriptor slice
covers Cells and Counters, including chunked ledger paths.

For applications that use several generated contracts, pass the same
`--rust-runtime-root /path/to/compact` for each compilation. The root must
contain `runtime-rs/` and `runtime-rs-macros/`. Each generated manifest then
points to that one runtime source package, allowing the contracts to share a
Cargo graph. This mode depends on the chosen local path; the default bundled
mode keeps each individual generated contract self-contained.

The packaged command keeps the ledger-8 Scheme compiler as a sibling named
`compactc-scheme`. For compiler development, set `COMPACTC_SCHEME` to a local
Scheme executable and run `cargo run -p compact-rust-backend --bin compactc --`.
The older `compact-rustc` command remains available for fixture generation; it
always uses `--skip-zk` and does not create a Cargo manifest. The crates in
[`tests-rust-backend`](../../tests-rust-backend) show the generated API and
how to execute constructors and circuits.

The public API uses `pure_circuits` and `ledger_contract` modules. Exported
stateful circuits are available as free functions and as methods on
`ledger_contract::Contract<W>`. Use `Contract::default()` without witnesses or
`Contract::from(MyWitnesses)` with an implementation of the generated
`Witnesses<Private>` trait. Methods take an explicit `CircuitContext<Private>`
and typed circuit arguments; witness bounds apply only to methods that need
them. The facade delegates to the existing functions, preserving state and
witness behavior. `ledger_slots` exposes typed Cell, Counter, and Set descriptors.
The runtime's recording frame is opt-in. When a circuit has a complete recorded
trace, the generated `contract.recording` handle exposes it as a typed method.
For witnessed root Cell circuits, `contract.recording()` borrows the contract's
witness implementation and records private outputs in execution order. A
supported Unit-returning stateful call is expanded in recording order, including
its typed arguments and nested witness operations. The separate recorded
emitter also evaluates supported Field reads, witness calls, addition, and
internal Field-returning calls in source order. Supported root Set mutation, membership, size, and emptiness calls also have replayable recorded methods. Other methods continue to
return native execution results until their full transcript coverage is proven.
Enable the runtime's `ledger-transaction` feature in a consuming Cargo graph
to use `transaction::prepare_call` with a recorded result and a `CallSpec`.
That adapter builds a ledger-8 call prototype from the trace and emitted
verifier artifact.
The generated crate forwards the feature as `ledger-transaction` and reexports
its exact runtime as `contract_crate::runtime`, so a consumer can depend only on
the generated crate for these types.
Generated modules suppress Rust's naming lint so public names retain their
Compact spelling without warning in consumer builds.

## Version compatibility

| Boundary | Current contract | Failure behavior |
|---|---|---|
| Compact compiler | Toolchain 0.31.128, language 0.23.105 | Versions are recorded in `compiler/contract-manifest.json`. |
| Rust IR | Schema 6, private to this backend | The renderer rejects any other schema before writing `lib.rs`. |
| Generated code and Rust runtime | ABI 3 | Generated modules assert the ABI at Rust compile time. |
| Rust runtime source | Bundled runtime crates or an explicit shared source root | Cargo resolves the matching runtime and its pinned Midnight crates. |

`--runtime-version` reports the TypeScript runtime version; the Rust runtime
compatibility contract is the ABI assertion and matching source packages. The
generated `Cargo.toml` has `publish = false` because it is a contract-specific
artifact. Change the runtime source only alongside an ABI and consumer test review.

The backend directory has its own `Cargo.lock` for the isolated Nix
`compact-rust-cli` package. The repository root lockfile governs workspace
tests. A dependency change to this package must update both lockfiles and pass
the Nix package build and workspace tests.

## Source model

The Scheme frontend lowers its analyzed program into a versioned JSON IR.
[`src/ir.rs`](src/ir.rs) defines closed Rust types for contract declarations,
expressions, effects, and ledger actions. The renderer builds `syn` syntax
nodes with `quote` and `proc-macro2`, then formats the file with
`prettyplease`. A new language operation must have an IR variant and a
renderer branch; it should not inject Rust source strings through the
compiler frontend. The generated library records its runtime ABI version,
and the renderer rejects an unknown IR schema.

The runtime owns `Field`, `BoundedUint`, fixed bytes and vectors, user type
representation derives, native crypto, ledger views, and the stateful circuit
context. Most ledger behavior comes from the matching
`midnight-{base-crypto,onchain-state,onchain-vm,transient-crypto,...}` releases
pinned in [`runtime-rs/Cargo.toml`](../../runtime-rs/Cargo.toml).

## Verify a backend change

```sh
cargo fmt --all -- --check
nix develop .#compiler --command env COMPACTC=compactc \
  python3 tools/compact-rust-backend/check_compactc_target.py --consumer --proof
COMPACTC=/path/to/ledger-8/compactc \
  python3 tools/compact-rust-backend/check_fixture_outputs.py
COMPACTC=/path/to/ledger-8/compactc \
  python3 tools/compact-rust-backend/check_rejections.py
cargo test --workspace --exclude compact
```

The fixture checker compiles every source in
[`examples/rust_backend`](../../examples/rust_backend), formats the output,
and compares it with the checked-in generated Rust. Use `--update` after an
intentional renderer change, then inspect the diff. The fixture crates also
compare native results and serialized state with captures from the ledger-8
TypeScript runtime. Capture programs are in [`oracles`](oracles).
When capturing from a local runtime package, copy generated `index.js` into a
separate temporary harness before adding a `node_modules` link. The compiler
cleans its output directories on each run, so links must stay outside them.
The rejection checker verifies source-located failures for unsupported
constructs and that no generated Rust library survives a rejected compile.
The `--proof` target check derives the Counter increment statement from the
generated recorded trace, proves it against emitted ZKIR and keys, and rejects
a changed binding input. It validates offline ledger-8 deployments, then
proves, validates, and applies Counter increment, Boolean Cell write, and
Boolean Cell read calls.
Other circuit operations still need recording coverage before wallet submission.

The fixture suite covers all 37 top-level `*_fixture.compact` contracts from
the `codegen-rust` oracle branch, alongside smaller source contracts used to
exercise individual operations. This is source and execution coverage for
those fixtures, not a claim that every valid Compact program or proving path
is supported.

## Current boundary

The backend executes constructors, pure and stateful circuits, typed witnesses,
ledger Cells and collections, user structs and enums, bounded arithmetic,
control flow, hashing, and the curve operations used by the oracle fixtures.
The oracle `tiny`, `election`, `zerocash`, and digital passport contracts also
run with TypeScript result or serialized state comparisons. New Compact shapes
must be added to the typed IR and renderer before they can be emitted.

The public command preserves the Scheme compiler's ZKIR and proving-key
behavior; the legacy fixture command uses `--skip-zk`. Unsupported language
operations produce a Compact source diagnostic; examples include unknown
`Opaque` tags and Field-to-Uint narrowing. The rejection gate checks that a
failed compile does not leave a generated Rust library. The Rust API and IR
schema are local to this branch and may change as support expands.
