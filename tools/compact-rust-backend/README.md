# Compact Rust backend (ledger 8)

This backend compiles Compact through the ledger-8 Scheme frontend and renders
its typed Rust IR as native Rust source. It is independent of the TypeScript
backend. The generated code uses [`midnight-compact-runtime`](../../runtime-rs),
which delegates state, encoding, hashing, curves, and VM operations to the
ledger-8 Midnight crates.

## Compile a contract

From the repository root, make a ledger-8 `compactc` available on `PATH`, or
set `COMPACTC` to its executable path. Then run:

```sh
cargo run -p compact-rust-backend --bin compact-rustc -- \
  examples/rust_backend/counter.compact /tmp/compact-rust-output
```

The output directory contains `contract/compact-rust-ir.json` and
`contract/lib.rs`. The command invokes `compactc --skip-zk --emit-rust-ir`;
it does not build proof artifacts. To use the generated source, place `lib.rs`
in a Rust crate that depends on `midnight-compact-runtime` at the same source
revision. The crates in [`tests-rust-backend`](../../tests-rust-backend) show
this layout and how to execute constructors and circuits.

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
The rejection checker verifies source-located failures for unsupported
constructs and that no generated Rust library survives a rejected compile.

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

The command uses `--skip-zk`: it generates a native execution library, not
proving keys or a deployable ZK artifact. Unsupported language operations
produce a Compact source diagnostic; examples include unknown `Opaque` tags
and Field-to-Uint narrowing. The rejection gate checks that a failed compile
does not leave a generated Rust library. The Rust API and IR schema are local
to this branch and may change as support expands.
