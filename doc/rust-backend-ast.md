# Rust backend: typed syntax pipeline

This branch starts from the plain `ledger-8` compiler. The Rust backend reads
`Lnodisclose`, the output of shared analysis, before TypeScript lowering.
`compiler/rust-ir-passes.ss` converts supported Compact constructs into a
versioned JSON domain model. `compact-rust-backend` validates that model,
constructs `syn` syntax, and formats the result with `prettyplease`.
`compact-rustc` joins the two processes without a shell command: it invokes
`compactc --skip-zk --emit-rust-ir` and writes `contract/lib.rs`.

The JSON schema is defined in
`tools/compact-rust-backend/src/ir.rs`. It has no Rust-source escape hatch.
Adding an expression or type requires an explicit IR variant, conversion in
the Scheme pass, type validation in the renderer, and an executing fixture.
Unsupported Compact constructs fail with a compiler error; they never become
placeholder Rust.

## Current slice

The backend supports exported pure circuits with `Field`, `Boolean`,
`Bytes<N>`, `Uint<N>`, unit, tuple, and vector types. Bodies currently support parameter references,
Boolean literals, unit, tuple construction, and field addition. The fixtures
in `examples/rust_backend/` run from Compact source through the Scheme
compiler, JSON bridge, `syn` renderer, native runtime, and executing Rust
tests. The compiler rejects ledger fields, witnesses, and bounded unsigned
arithmetic until their runtime semantics are implemented. `Uint<N>` uses a
bounded runtime type with a checked inclusive maximum and Compact's byte
alignment; `Bytes<N>` uses the ledger's fixed byte array representation.

The runtime facade in `runtime-rs` reexports `Fr` from
`midnight-transient-crypto` 2.0.1 as Compact `Field`. It also reexports the
ledger's `FieldRepr` and `FromFieldRepr` traits and derive macros, and the
`MemWrite` trait they use. This preserves the ledger-8.0.2 field and encoding
line instead of introducing another field implementation. The transitive
midnight-zk crates supply curves and proof primitives.

The runtime also exposes ledger-owned `ChargedState`, `QueryContext`, and
Zswap state through constructor and circuit context envelopes. Its FAB tests
compare Field, Boolean, Bytes, and Uint alignment and normalized values with
the ledger-8 implementations. The ledger-8.0.2 lock references
`midnight-zswap` 8.0.1, which is no longer published; the runtime pins 8.0.0
from the same 8.0 line.

The runtime can now construct and decode ledger Cells and Counters, and it
runs Cell writes plus Counter increments/decrements through the ledger VM.
The Counter test serializes the full post-increment `ContractState` and matches
the TypeScript oracle fixture byte for byte. Compiler emission for these
ledger operations is still pending.

## Run locally

With the base compiler's Chez and nanopass dependencies available, build the
Rust renderer and runtime:

```sh
cargo test -p compact-rust-backend -p midnight-compact-runtime \
  -p compact-rust-identity-fixture -p compact-rust-truth-fixture \
  -p compact-rust-one-tuple-fixture -p compact-rust-field-add-fixture \
  -p compact-rust-uint-identity-fixture -p compact-rust-bytes-identity-fixture
cargo build -p compact-rust-backend --bin compact-rustc
```

Set `COMPACTC` to a compiler executable built from **this branch**, then run:

```sh
COMPACTC=/path/to/compactc target/debug/compact-rustc \
  examples/rust_backend/identity.compact /tmp/compact-identity
```

The output contains `contract/compact-rust-ir.json` and `contract/lib.rs`.
For direct frontend debugging, `compactc --skip-zk --emit-rust-ir` emits the
JSON alongside the usual TypeScript artifacts.

## Next slices

1. Add literals and typed primitive operations using ledger and ZK crate
   semantics, with checks for Compact field and bounded unsigned behavior.
2. Add user structs using the ledger derives, then enums and recursive
   composite types with explicit encoding tests.
3. Add the minimal ledger state envelope, Cell, and Counter. Compare state
   bytes and behavior with TypeScript fixtures from the oracle branch.
4. Extend witnesses, calls, control flow, ledger ADTs, and cryptographic
   natives in small executing fixtures until the oracle matrix is covered.

The detailed plan and research log are in the `midnight` Obsidian vault under
`Initiatives/02 Compact Rust emission/AST backend rebuild — 2026-10-01.md`.
