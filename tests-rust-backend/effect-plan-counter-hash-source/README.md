# Counter-derived hashes and fixed-byte products

The retained schema-20 IR is produced from
`examples/rust_backend/effect_plan_counter_hash.compact`. `build.rs` uses the
current emitter and requires the exact two public exports (`run`, `products`)
to provide recording and observation. This fixture changes no selector priority.

`run` increments a Counter, conditionally calls a witness, and invokes an
actionless stateful helper whose triple hash reads the Counter. It writes and
reads a digest Cell and reuses one encoded Field in a pair hash. The witness
reads the Counter through the official metered view, so its history observes
the increment even when it refuses. An unselected refusing witness is skipped.

`products` hashes Bytes<3>, a fixed vector, a named Struct containing bytes and
a vector, and the empty Bytes<0>, Vector<0, Bytes<3>>, and empty Struct products.
The manual program uses official runtime slots and hashing, with a tuple-based
FAB representation for the Struct so it does not reuse its generated helper.

## Evidence and limits

Three tests cover 18 successful `run` combinations (Field zero, 257, and the
canonical maximum; Counter increment 0, 1, and 65535; both branches), three
product inputs, and two witness refusal/skip controls. They compare native,
recorded, and manual result/state/effects/private state/private outputs/execution
gas; public operations and replay gas are checked against the manual program.
Failure checks observe witness history, not a returned partial public frame.

`oracles/ts-primitives.json` comes from official compact-runtime **0.16.101**.
Its independently specified little-endian Field bytes and typed hash inputs
check digest and conversion values. This is a **primitive TS oracle**, not a
claim of full TS contract execution. Reproduce without downloading dependencies:

```sh
node oracles/capture.mjs /path/to/node_modules/@midnight-ntwrk/compact-runtime > oracles/ts-primitives.json
```

The same ordered FAB atoms are used for tuple/vector product hash expectations.
The Rust test also checks the actual generated named Struct and empty product
representations. No proof was added; no new public/witness ABI or broad hash
support for Field, unsigned, point, or opaque values is claimed. Fields must
first convert explicitly to Bytes<32> in this fixture.

Source recompilation (from repository root; use the pinned ledger-8 frontend):

```sh
COMPACTC_SCHEME=/path/to/compactc-scheme compactc --target rust --skip-zk \
  --rust-runtime-root "$PWD" examples/rust_backend/effect_plan_counter_hash.compact /tmp/counter-hash
cmp /tmp/counter-hash/contract/compact-rust-ir.json \
  tools/compact-rust-backend/tests/effect-plan-counter-hash-schema20-ir.json
cargo test --offline --locked -p compact-rust-effect-plan-counter-hash-source-fixture --all-features
```
