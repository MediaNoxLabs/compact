# Compact Rust backend (ledger 8)

This backend compiles Compact through the ledger-8 Scheme frontend and renders
its typed Rust IR as native Rust source. It is independent of the TypeScript
backend. The generated code uses [`midnight-compact-runtime`](../../runtime-rs),
which delegates state, encoding, hashing, curves, and VM operations to the
ledger-8 Midnight crates.
Generated Merkle witness views expose local `root`, `first_free`, path, and
historic history methods alongside fallible `is_full` and `check_root` methods.
The latter two execute canonical ledger-8 VM queries and contribute their cost
to Rust circuit results. TypeScript currently reports zero wrapper gas for
witness-only VM queries; the Rust result sums the observed query costs.

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
The generated `ledger_slots` module exposes named typed descriptors for Cell,
Counter, Set, Map, List, and Merkle declarations. For example,
`ledger_slots::tree.insert(context, value)` accepts the declared Merkle leaf
type and uses its declared path and depth. Plain and historic Merkle slots
share the native API; only historic slots expose `reset_history`. These slot
constructors are public typed conveniences, not access-control boundaries.

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
witness behavior. `ledger_slots` exposes typed Cell, Counter, Set, scalar Map,
root List, and plain/historic Merkle descriptors. Nested Maps have a typed structural slot with
`MapNode<K, V>` values. Its `is_empty`, `size`, and `member` shape reads are
available for native and recorded calls; scalar lookup and value mutations
remain unavailable until nested value semantics are proven.
The runtime's recording frame is opt-in. When a circuit has a complete recorded
trace, the generated `contract.recording` handle exposes it as a typed method.
For witnessed root Cell circuits, `contract.recording()` borrows the contract's
witness implementation and records private outputs in execution order. A
supported Unit-returning stateful call uses one private, frame-taking Rust
helper for each referenced callee. Calls evaluate typed arguments in source
order and share the frame across nested witness and ledger operations. The
separate recorded emitter still evaluates supported Field reads, witness
calls, addition, and internal Field-returning calls in source order. Supported
root Set and Map mutation, membership, lookup, size, and emptiness calls have
replayable recorded methods. Root typed List push, pop, reset, length, emptiness, and head
calls also have recorded methods. Merkle calls remain native until a complete
recorded trace is supported. Other methods continue to
return native execution results until their full transcript coverage is proven.
The `tiny` fixture also records typed enum/Bytes Cell reads and writes,
state-backed assertions, Bytes witnesses and pure calls, and a conditional
`Maybe<Field>` return. Its generated `clear`, `set`, and `get` methods keep
ordinary Rust control flow over the recording frame. For witnessed calls use
`Contract::from(witness).recording().clear(context)`; the generated method
borrows the witness implementation. Recorded entry points are emitted only
when their complete circuit shape is supported.
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
| Compact compiler | Toolchain 0.31.133, language 0.23.105 | Versions are recorded in `compiler/contract-manifest.json`. |
| Rust IR | Schema 8, private to this backend | The renderer rejects any other schema before writing `lib.rs`. Ledger, circuit, witness, constructor, and exported alias declarations carry optional Compact source locations for diagnostics. |
| Generated code and Rust runtime | ABI 14 | Generated modules assert the ABI at Rust compile time. ABI 14 records historic Merkle append with root-history VM semantics; ABI 13 records plain Merkle append through the typed slot and shared VM builder; ABI 12 adds typed plain/historic Merkle slots for native calls; ABI 11 meters Merkle witness VM reads while keeping local projections uncharged; ABI 10 adds fallible `TryWitnesses` and adapts existing pair-returning `Witnesses` implementations; ABI 9 meters List witness reads, ABI 8 Map, ABI 7 Set, ABI 6 Cell/Counter; ABI 5 added structural nested Map slots. |
| Rust runtime source | Bundled runtime crates or an explicit shared source root | Cargo resolves the matching runtime and its pinned Midnight crates. |

`--runtime-version` reports the TypeScript runtime version; the Rust runtime
compatibility contract is the ABI assertion and matching source packages. The
generated `Cargo.toml` has `publish = false` because it is a contract-specific
artifact. Change the runtime source only alongside an ABI and consumer test review.

### Fallible witnesses

Generated crates expose `TryWitnesses<Private>` for witnesses that read the
ledger. Implement its methods with `Result<(Private, T), CompactError>` so a
projection read can use `?`:

```rust
impl TryWitnesses<u64> for ReadFlag {
    fn read_flag(&self, context: WitnessContext<'_, u64, LedgerView<'_>>)
        -> Result<(u64, bool), CompactError> {
        let flag = context.ledger.flag()?;
        Ok((*context.private_state + 1, flag))
    }
}
```

Existing pair-returning `Witnesses<Private>` implementations continue to work
through a generated adapter. Choose one trait for each witness type; the
adapter prevents implementing both traits on the same type. A rejected read
returns `CompactError` from the generated circuit without a partial result.

The backend directory has its own `Cargo.lock` for the isolated Nix
`compact-rust-cli` package. The repository root lockfile governs workspace
tests. A dependency change to this package must update both lockfiles and pass
the Nix package build and workspace tests.

For the local runtime crate release rehearsal, run
`python3 tools/compact-rust-backend/check_release_packages.py` from the
repository root. It verifies the macro and runtime crate archives, including
their license files, and compiles each unpacked package. Until the exact macro
version is published, the runtime check supplies the local macro source through
a temporary Cargo patch. Publication, remote CI, and an unpatched consumer are
separate release gates tracked in [issue #106](https://github.com/MediaNoxLabs/compact/issues/106).

To retain the exact local archives and their inputs for review, pass
`--manifest target/rust-runtime-release.json`. The manifest records Git commit,
tree and dirty state; package hashes, sizes and versions; Rust/Cargo versions;
and hashes of both Cargo locks and `flake.lock`. Run the same script with
`--verify-manifest target/rust-runtime-release.json` to repackage and compare
the result. CI uploads the JSON and both `.crate` files as one artifact. For a
release candidate, add `--candidate-tag <tag>` to require a clean checkout at
a signed annotated tag. This gate does not publish crates or prove that an
unpatched registry consumer can build; the manifest says that the local macro
patch was used during archive verification. The `midnight` vault's ADR-0013
records the decision; [issue #114](https://github.com/MediaNoxLabs/compact/issues/114)
tracks the remaining gates.

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
python3 tools/compact-rust-backend/check_oracle_acceptance.py
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
The [oracle acceptance inventory](oracle_acceptance.json) pins the 37
`codegen-rust` fixture sources at the recorded commit. Its checker verifies
local source bytes, executable lines after excluding full-line comments, and
Rust test/TypeScript fixture links. This is a provenance and test-inventory
gate; individual executing tests establish result and state parity, while
negative cases, gas, and VM transcript coverage remain separate M2 work.
When capturing from a local runtime package, copy generated `index.js` into a
separate temporary harness before adding a `node_modules` link. The compiler
cleans its output directories on each run, so links must stay outside them.
The rejection checker verifies source-located failures for unsupported
constructs and that no generated Rust library survives a rejected compile.
The `tiny` gas fixture is captured with
[`tiny_gas_capture.mjs`](oracles/tiny_gas_capture.mjs) from a TypeScript
compilation with the matching runtime package available to Node. It records
each VM query cost and the public transcript for `clear`, `set`, and `get`.
The Rust fixture compares its generated circuit's total gas with the sum of
those TypeScript query costs. The current TypeScript wrapper reports only
the final query's cost in its `gasCost` field; compare the query sum when
checking whole-circuit work. The generated `tiny` test compares each
normalized public Verify operation with this TypeScript transcript for
`clear`, `set`, and `get`, as well as total gas, state, results, private
outputs, and assertion failures. A single replay query has different gas
from the sum of the compiler's separate queries; replay checks state and
the query sums check gas.
The plain and historic Merkle captures also retain labeled native VM query
costs for initial fullness, root checks, and insertion; the historic capture
adds history reset. Their Rust fixture tests compare all four gas dimensions
against the TypeScript query sums while preserving the existing state and
result oracles. A runtime unit test compares the complete serialized VM
programs for those seven captured queries, including ordered operations,
path keys, cache flags, pushed values, and the inserted leaf hash. These
native circuits do not yet emit recorded proof traces.
The `--proof` target check derives the Counter increment statement from the
generated recorded trace, proves it against emitted ZKIR and keys, and rejects
a changed binding input. It also validates offline ledger-8 deployments and
proves, verifies, validates, and applies the supported Counter, Cell, Set,
Map, List, enum Cell, and `tiny` call fixtures. `tiny` is proved in both
present and absent `get` branches. Other circuit operations still need
recording coverage before wallet submission.

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
