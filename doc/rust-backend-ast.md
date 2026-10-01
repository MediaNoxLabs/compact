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
Nonempty or parameterized ledger constructors currently receive a source
diagnostic. The backend must preserve their initialization actions before
allowing them to emit a default Rust state.

## Current slice

The backend supports exported pure circuits with `Field`, `Boolean`,
`Bytes<N>`, `Uint<N>`, unit, tuple, and vector types. Bodies currently support parameter references,
Boolean, Field, and Uint literals, unit, tuple construction, typed conditionals, sequential
local bindings and assertion statements, pure circuit calls, Field addition/subtraction/multiplication,
typed equality and inequality,
ordered `Uint` comparisons,
checked unsigned addition, subtraction, and multiplication, and the
`transientHash`, `transientCommit`, `persistentHash`, `persistentCommit`, `keccak256`,
`degradeToTransient`, `upgradeFromTransient`, `hashToCurve`, Jubjub
coordinate natives, checked point construction, point addition/negation/multiplication, and native-to-Jubjub
scalar reduction. The fixtures
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
Pure assertion sequences are typed IR nodes. The fixture checks that two
assertions run in source order, that a successful circuit returns its value,
and that both failure messages match generated TypeScript.
Stateful assertions are typed action nodes. They evaluate a witnessed Boolean
before deciding whether the following action runs. The fixture compares two
witnessed assertions and an assertion before a Cell write with generated
TypeScript, including short circuiting, witness order, private state,
transcript alignment, and the successful ledger read.
Typed equality and inequality reuse the Rust value types' structural
`PartialEq`, after the renderer checks both operand types. The source fixture
covers Field, Bytes<4>, Vector<2, Field>, and two witnessed Field comparisons;
generated Rust matches TypeScript results and witnessed FAB transcript order.
The compiler normalizes Boolean `!`, `&&`, and `||` to typed conditionals
before Rust lowering. A source fixture checks pure results and witnessed
short circuit behavior: the unused right branch makes no witness call and
adds no transcript value. Rust and TypeScript agree on result, private state,
and each emitted Boolean FAB value.
The comparison IR carries one closed operator for `<`, `<=`, `>`, and `>=`.
The renderer requires Uint operands and compares their checked numeric values,
including an upstream widening cast for mixed widths. The fixture compares
all four operators, equal boundaries, mixed widths, and two witnessed operands
with generated TypeScript, including private state and transcript order.

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
`keccak256` uses Compact's separate byte encoding: it concatenates normalized
FAB value atoms and hashes those bytes with Keccak-256. The Field, Bytes<4>,
and vector fixture compares generated Rust with generated TypeScript circuits
and direct TypeScript native calls. A witnessed case checks private state and
FAB transcript output.
Field, vector, and bytes fixtures compare generated Rust results to generated
TypeScript circuits and the TypeScript native calls, including nested
hash-to-field conversion.
The first curve slice adds `JubjubPoint` as a distinct IR type backed by
ledger-8's `EmbeddedGroupAffine`, with `hashToCurve` and affine X/Y accessors.
The Field and vector curve-hash fixture compares both coordinates with the
ledger WASM runtime through generated TypeScript. A local wrapper supplies
ledger FAB and field representation traits for points inside user structs and
Cells. The default point uses the same `(0, 1)` encoding as the pinned ledger
native output; the ledger decoder also accepts a `(0, 0)` identity sentinel
and normalizes it to `(0, 1)`. A generated point and struct
Cell fixture compares the default and post-write reads with TypeScript and
round-trips the derived representations. The wrapper checks compressed
coordinates before calling the pinned ledger constructor, avoiding its panic
on malformed points. Witnessed points also compose through the supported
curve natives. Their typed FAB transcript values and source order match the
generated TypeScript fixture for direct returns, coordinates, point arithmetic,
and curve hashing.
The group arithmetic fixture compares addition, negation, point and generator
multiplication, and scalar reduction with the generated TypeScript and ledger
WASM runtime. `ecMul` and `ecMulGenerator` require a canonical embedded scalar
in Rust, matching WASM rejection at the native Field maximum. The separate
`jubjubScalarFromNative` operation reduces that value before multiplication.
`constructJubjubPoint` checks coordinates with the midnight-zk curve decoder
before using the pinned ledger point constructor. This prevents an upstream
panic for malformed points. Valid and identity points, a nested Y accessor,
and two witnessed coordinates match generated TypeScript. The canonical
identity is `(0, 1)` in FAB, field, binary, and coordinate representations.
The TypeScript pure helper can return unchecked coordinate pairs; Rust rejects
those because its point type represents a valid ledger point.

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
Witness calls also compose through the six supported crypto natives. Hashes
and conversions evaluate their input witness before the native call;
commitments evaluate the value before the opening. The witness crypto fixture
checks private state and FAB transcript order against TypeScript, including a
commitment with two different witnesses and nested persistent hash conversion.
Stateful emission now resolves calls to exported pure circuits through the same
typed signature table as pure emission. The fixture calls a pure helper in a
ledger Cell write and in the stateful return, then compares the returned value,
Cell read, and serialized ledger state with generated TypeScript.
Local pure helpers also enter the typed IR with an `internal` visibility flag.
Rust emits them as crate-visible functions, so exported pure and stateful
circuits can call them without publishing them as external entry points. A
source fixture compares nested helper calls and a helper-fed Cell write with
TypeScript, including the full serialized state.
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
Vector and tuple Cells now pass constructor validation because the runtime
already provides recursive `CellValue` implementations for both. A source
fixture writes and reads `Vector<3, Field>` and `[Field, Boolean]`; initial
and post-write full `ContractState` bytes match generated TypeScript.
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
  -p compact-rust-keccak-fixture \
  -p compact-rust-assert-pure-fixture \
  -p compact-rust-jubjub-hash-fixture \
  -p compact-rust-jubjub-arithmetic-fixture \
  -p compact-rust-jubjub-construct-fixture \
  -p compact-rust-jubjub-cell-fixture \
  -p compact-rust-witness-jubjub-fixture \
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
  -p compact-rust-witness-hash-fixture \
  -p compact-rust-assert-witness-fixture \
  -p compact-rust-equality-fixture \
  -p compact-rust-boolean-logic-fixture \
  -p compact-rust-uint-compare-fixture \
  -p compact-rust-vector-tuple-cell-fixture \
  -p compact-rust-stateful-pure-call-fixture \
  -p compact-rust-internal-pure-call-fixture \
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
