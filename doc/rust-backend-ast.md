# Rust backend: typed syntax pipeline

This branch starts from the plain `ledger-8` compiler. The Rust backend reads
`Lnodisclose`, the output of shared analysis, before TypeScript lowering.
`compiler/rust-ir-passes.ss` converts supported Compact constructs into a
versioned JSON domain model. `compact-rust-backend` validates that model,
constructs `syn` syntax, and formats the result with `prettyplease`.
`compact-rustc` joins the two processes without a shell command: it invokes
`compactc --skip-zk --emit-rust-ir` and writes `contract/lib.rs`.
Stateful circuit emission lives in `tools/compact-rust-backend/src/stateful.rs`
so ledger action and return variants are handled away from the top-level
contract assembly.

The version 4 JSON schema is defined in
`tools/compact-rust-backend/src/ir.rs`. It has no Rust-source escape hatch.
Adding an expression or type requires an explicit IR variant, conversion in
the Scheme pass, type validation in the renderer, and an executing fixture.
Unsupported Compact constructs fail with a compiler error; they never become
placeholder Rust.

## Current slice

The backend supports exported pure circuits with `Field`, `Boolean`,
`Bytes<N>`, `Uint<N>`, unit, tuple, and vector types. Bodies currently support parameter references,
Boolean, Field, and Uint literals, unit, tuple construction, typed conditionals, sequential
local bindings, pure circuit calls, Field addition/subtraction/multiplication,
checked unsigned addition, subtraction, and multiplication, and the
`transientHash`, `transientCommit`, `persistentHash`, `persistentCommit`,
`degradeToTransient`, and `upgradeFromTransient` natives. The fixtures
in `examples/rust_backend/` run from Compact source through the Scheme
compiler, JSON bridge, `syn` renderer, native runtime, and executing Rust
tests. The compiler supports the Cell, Counter, Set, Map, and List slices
described below. Witness calls can compose with the supported stateful
expressions and typed action bindings described below. `Uint<N>` uses a
bounded runtime type with a checked inclusive maximum and Compact's byte
alignment. `Bytes<N>` uses a small fixed bytes wrapper: it delegates field
encoding and FAB alignment to ledger-8's `[u8; N]` support, and calls the
ledger's byte decoder to fill the `FromFieldRepr` gap for widths other than 32.
The compiler's inserted Uint subtraction guard is recognized only when its
comparison matches the subtraction operands; the Rust runtime enforces the
same underflow check. Multiplication checks both host overflow and the
result's declared Compact maximum.

The runtime facade in `runtime-rs` reexports `Fr` from
`midnight-transient-crypto` 2.0.1 as Compact `Field`. It also reexports the
ledger's `FieldRepr` and `FromFieldRepr` traits and derive macros, and the
`MemWrite` trait they use. This preserves the ledger-8.0.2 field and encoding
line instead of introducing another field implementation. The transitive
midnight-zk crates supply curves and proof primitives.
The transient natives take a typed Compact value, encode only its value through
ledger-8's `ValueReprAlignedValue`, and call the ledger's Poseidon hash or
commitment primitive. This matches the ledger WASM entry points used by the
TypeScript runtime. The persistent natives similarly reuse ledger-8's SHA-256
writer and persistent commitment over the value-only FAB binary encoding.
Field, vector, and bytes fixtures compare generated Rust results to generated
TypeScript circuits and the TypeScript native calls, including nested
hash-to-field conversion.

The runtime also exposes ledger-owned `ChargedState`, `QueryContext`, and
Zswap state through constructor and circuit context envelopes. Its FAB tests
compare Field, Boolean, Bytes, and Uint alignment and normalized values with
the ledger-8 implementations. The ledger-8.0.2 lock references
`midnight-zswap` 8.0.1, which is no longer published; the runtime pins 8.0.0
from the same 8.0 line.

A direct witness return now has a typed declaration and call in the IR. The
generated `Witnesses<Private>` trait receives a borrowed context with a
contract-specific ledger view, private state, and contract address. Its method
returns the next private state and a typed value. The generated circuit records
that value as a ledger `AlignedValue` in its own private transcript outputs.
The Field witness fixtures cover zero and one typed arguments; the Boolean
Cell fixture reads current ledger state before and after a write. A Counter
fixture reads `Uint<64>` before and after increment. Set and Map witnesses
read membership, size, and emptiness; the Map witness also checks typed lookup.
The List witness reads its optional typed head, length, and emptiness before
and after prepend and pop operations.
Their results, private states, and FAB transcript values match the pinned
TypeScript runtime. The ledger view decodes Cell and Counter fields through
the ledger's FAB types and exposes Set and Map through typed, read-only views
of the ledger's own Map storage. List projection reads the ledger's
head/tail/length array. The first composable witness expression supports
Field addition, subtraction, and multiplication in a stateful return. Each
witness call updates private state and appends its own FAB transcript value in
source order; a two-call fixture compares both outputs with TypeScript.
Conditional returns execute only the selected witness branch. Tuple elements,
nested witness arguments, and sequential local bindings preserve evaluation
and transcript order in their TypeScript parity fixtures. Direct witness
returns use this same expression path. Witnessed `Uint<16>` addition,
subtraction, multiplication, and explicit downcasts use the runtime's checked
bounded types. Their results, FAB transcript values, and overflow or underflow
behavior match the TypeScript oracle fixture.
The action IR also carries typed sequential local bindings. A witnessed Cell
write evaluates its witness before the ledger VM write; two writes in one
circuit preserve the intervening ledger state, private state, and transcript
order against TypeScript. The same typed binding IR also carries a literal
Field into `Map.insert`, covering the compiler's generated temporary rather
than treating it as an undeclared circuit parameter. Witness calls in the
remaining primitive expression forms and other ledger action values, and
complete proof data remain future slices.
The generated runtime ABI is 3.

The runtime can now construct and decode ledger Cells and Counters, and it
runs Cell writes plus Counter increments/decrements through the ledger VM.
The Counter test serializes the full post-increment `ContractState` and matches
the TypeScript oracle fixture byte for byte.
Set, Map, and List initial `ContractState` bytes also match the oracle's
ledger-8 TypeScript fixtures, including operation metadata. The mutation
sequence test compares full serialized state after Set add/remove/reset, Map
put/remove/default/reset, and List prepend/pop/reset operations.

The typed IR includes ordered
ledger declarations and state actions. The compiler and renderer generate
the minimal Counter increment and Boolean Cell write contracts end to end.
Other ledger operations remain explicit compiler errors. Stateful circuits
can accept typed parameters for Cell writes and Counter increments. Counter
amounts have a closed literal-or-parameter IR variant; a parameter must be
`Uint<16>`. The frontend inserts Compact's required `disclose` boundary before
public ledger operations, and the renderer checks parameter types before
emitting Rust. Stateful `Cell.read()` returns a typed value through the ledger
VM's gather mode and its read event. Generated Boolean and struct read
fixtures execute after ledger writes; the renderer checks the requested field
index and return type against the declaration. `Counter.read()` uses the same
ledger query, then converts the ledger's `u64` Cell into Compact's checked
`Uint<64>` representation. The generated Counter fixture reads before and
after an increment. Counter increments and decrements accept checked
`Uint<16>` parameters; reset writes the ledger's zero-valued Cell through the
VM. The parameterized fixture verifies increment, decrement, reset, and
underflow behavior.

The Set slice supports `Set<Boolean>` and `Set<Field>` construction, insert,
remove, member, size, isEmpty, and reset. The runtime stores the ledger's Map
value and executes the Set VM operations; reads decode gathered events.
The generated fixture checks membership, uniqueness after a duplicate insert,
size and emptiness around removal, reset, and independent roots at indices 0
and 1. The typed IR carries the Set element type for renderer validation.
The initial state and mutation sequence have full serialized `ContractState`
byte parity against the pinned TypeScript runtime.

The Map slice supports `Map<Boolean, Field>` construction, insertion,
insertDefault, removal, membership, lookup, size, isEmpty, and reset. It
shares the ledger Map representation and key FAB semantics with Set, while
storing a typed Cell value. Direct VM and generated contract tests cover
missing keys, insertion, replacement, default insertion, Field decoding,
removal, and reset. The renderer checks both key and value types against the
declaration.
The initial state and mutation sequence also have full serialized
`ContractState` byte parity.

The first List slice supports `List<Field>` construction, pushFront,
popFront, head, length, isEmpty, and reset. The runtime constructs the ledger's
three-element head/tail/length array and executes its List VM programs. The
generated fixture checks typed `Maybe<Field>` heads, length, and emptiness
after two pushes, a pop, reset, and another push. Coin-specific List
operations remain future work.
The initial state and mutation sequence have full serialized `ContractState`
byte parity.

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
  -p compact-rust-field-arithmetic-fixture -p compact-rust-transient-hash-fixture \
  -p compact-rust-persistent-hash-fixture \
  -p compact-rust-uint-arithmetic-fixture \
  -p compact-rust-witness-minimal-fixture \
  -p compact-rust-witness-argument-fixture \
  -p compact-rust-witness-ledger-cell-fixture \
  -p compact-rust-witness-ledger-counter-fixture \
  -p compact-rust-witness-ledger-set-fixture \
  -p compact-rust-witness-ledger-map-fixture \
  -p compact-rust-witness-ledger-list-fixture \
  -p compact-rust-witness-field-expression-fixture \
  -p compact-rust-witness-cell-write-fixture \
  -p compact-rust-witness-conditional-fixture \
  -p compact-rust-witness-uint-arithmetic-fixture \
  -p compact-rust-uint-identity-fixture -p compact-rust-bytes-identity-fixture \
  -p compact-rust-counter-fixture -p compact-rust-cell-boolean-fixture \
  -p compact-rust-struct-identity-fixture -p compact-rust-nested-struct-fixture \
  -p compact-rust-composite-struct-fixture -p compact-rust-enum-identity-fixture \
  -p compact-rust-cell-struct-fixture -p compact-rust-cell-enum-fixture \
  -p compact-rust-cell-parameter-fixture -p compact-rust-counter-parameter-fixture \
  -p compact-rust-cell-read-fixture -p compact-rust-set-boolean-fixture \
  -p compact-rust-map-boolean-field-fixture \
  -p compact-rust-map-literal-fixture \
  -p compact-rust-list-field-fixture -p compact-rust-choose-field-fixture
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

1. Expand witness calls through the remaining expression forms and state
   actions, including full proof data.
2. Extend remaining primitive operations and ledger ADTs using ledger and ZK
   crate semantics while preserving oracle parity.
3. Cover control flow, remaining cryptographic natives, and the oracle contract matrix
   in small executing fixtures.

The detailed plan and research log are in the `midnight` Obsidian vault under
`Initiatives/02 Compact Rust emission/AST backend rebuild — 2026-10-01.md`.
