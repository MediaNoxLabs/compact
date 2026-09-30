# Rust backend: typed syntax pipeline

This branch starts from the plain `ledger-8` compiler. The Rust backend reads
`Lnodisclose`, the output of shared analysis, before TypeScript lowering.
`compiler/rust-ir-passes.ss` converts supported Compact constructs into a
versioned JSON domain model. `compact-rust-backend` validates that model,
constructs `syn` syntax, and formats the result with `prettyplease`.
`compact-rustc` joins the two processes without a shell command: it invokes
`compactc --skip-zk --emit-rust-ir` and writes `contract/lib.rs`.

The version 3 JSON schema is defined in
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
tests. The compiler supports the initial Counter and Boolean Cell state
actions and rejects other ledger shapes, witnesses, and bounded unsigned
arithmetic until their runtime semantics are implemented. `Uint<N>` uses a
bounded runtime type with a checked inclusive maximum and Compact's byte
alignment. `Bytes<N>` uses a small fixed bytes wrapper: it delegates field
encoding and FAB alignment to ledger-8's `[u8; N]` support, and calls the
ledger's byte decoder to fill the `FromFieldRepr` gap for widths other than 32.

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
the TypeScript oracle fixture byte for byte. The typed IR includes ordered
ledger declarations and state actions. The compiler and renderer generate
the minimal Counter increment and Boolean Cell write contracts end to end.
Other ledger operations remain explicit compiler errors. Stateful circuits
can accept typed parameters for Cell writes and Counter increments. Counter
amounts have a closed literal-or-parameter IR variant; a parameter must be
`Uint<16>`. The frontend inserts Compact's required `disclose` boundary before
public ledger operations, and the renderer checks parameter types before
emitting Rust.

The user type slice emits native Rust structs from the typed Compact type
shape and derives the ledger's `BinaryHashRepr`, `FieldRepr`, and
`FromFieldRepr` macros. It emits closed enum ordinal conversions with the
same field and declared byte width. Struct, nested struct, enum, and
vector/tuple composite fixtures execute. `FixedVector<T, N>` delegates
element encoding to ledger primitives and fills the missing generic array
field decoder. Runtime tests cover recursive vector/tuple values in Cells.
The separate `runtime-rs-macros` crate derives FAB alignment, value encoding,
and Cell decoding for generated structs and unit enums. Its input is the
renderer-built Rust syntax, and its output composes the ledger's FAB types.
Source-level `cell_struct` and `cell_enum` fixtures test constructors, exact
Cell alignment, value round trips, and writes through the ledger VM.

## Run locally

With the base compiler's Chez and nanopass dependencies available, build the
Rust renderer and runtime:

```sh
cargo test -p compact-rust-backend -p midnight-compact-runtime \
  -p compact-rust-identity-fixture -p compact-rust-truth-fixture \
  -p compact-rust-one-tuple-fixture -p compact-rust-field-add-fixture \
  -p compact-rust-uint-identity-fixture -p compact-rust-bytes-identity-fixture \
  -p compact-rust-counter-fixture -p compact-rust-cell-boolean-fixture \
  -p compact-rust-struct-identity-fixture -p compact-rust-nested-struct-fixture \
  -p compact-rust-composite-struct-fixture -p compact-rust-enum-identity-fixture \
  -p compact-rust-cell-struct-fixture -p compact-rust-cell-enum-fixture \
  -p compact-rust-cell-parameter-fixture -p compact-rust-counter-parameter-fixture
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
2. Expand the initial Cell and Counter path to typed read results and
   additional ledger field types while preserving state byte parity.
3. Extend witnesses, calls, control flow, ledger ADTs, and cryptographic
   natives in small executing fixtures until the oracle matrix is covered.

The detailed plan and research log are in the `midnight` Obsidian vault under
`Initiatives/02 Compact Rust emission/AST backend rebuild — 2026-10-01.md`.
