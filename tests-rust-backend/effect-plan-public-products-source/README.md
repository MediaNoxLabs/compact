# Source-derived public products

The retained schema-20 IR is the complete pinned-frontend output for
`examples/rust_backend/effect_plan_public_products.compact`. The build script
renders current Rust and requires recorded and observed-call capabilities for
all four exports. Compiler tests separately require the actual checked algebra
owner; fixture counts do not establish language coverage.

`run` passes an owned product through a stateful helper, increments a Counter,
and conditionally invokes one bytes witness. Its result retains both the original
product and a chosen product. The selected product contains the original tuple's
bytes and the witness replacement in its two-element vector. The product combines
opaque strings/bytes, tuple, vector, nested named product, Uint<129>, and an empty
named product. Tests use a value above u128, empty and Unicode strings, and
canonical interior-zero bytes.

Four integration tests cover six canonical branch/input combinations, five
noncanonical input leaf positions, three output-only refusals, three canonical
interface boundary cases, and three tuple/vector invalid-leaf cases. They compare
native/generated/manual recording, state, effects, gas, private output and
transcript/replay; prepared public atoms and alignments are literal expectations.
Invalid observed inputs precede callbacks; invalid witness outputs follow exactly
the expected callback. Raw native/recorded values remain observable. Random
verifier carriers only support preparation tests here; these tests do not prove
transactions. The separate proof-smoke adapter owns any actual proof evidence.

`tuple11` observes one tuple field. `vector9` deliberately uses an unused public
vector-of-nine-tuples argument to qualify its ABI and recursive validation.
`empty` qualifies a unit and a zero-length vector, with a nonempty Counter
transcript and a zero-field result. These are genuine Compact source exports,
but vector9 does not establish vector indexing support.

## Separate native emitter findings

During source construction, indexing `Vector<2, Opaque<"Uint8Array">>` failed
native Rust emission with `expected tuple value, got Vector`. Nesting a witness
call directly in vector construction failed with `stateful expression requires
stateful evaluation`. The successful source uses a tuple projection and explicitly
binds/discloses the witness result inside a helper. Public interface admission does
not fix these expression-lowering gaps. Scratch inputs, logs and a reproduction note are historical external evidence
and are not distributed with this fixture; no TypeScript execution comparison
is claimed for either failure.
