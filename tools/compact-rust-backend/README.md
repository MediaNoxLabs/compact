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
`contract/compact-rust-ir.json`, `contract/rust-capabilities.json`, and source copies of the matching runtime and
derive-macro crates, along with the compiler metadata, ZKIR, and proof
artifacts produced by the Scheme compiler. The Cargo library uses the bundled
runtime as a path dependency. A separate Rust
project can depend on `contract/` by path without copying generated source or
editing its manifest. Run with `--target ts --target rust` to emit both contract
languages. `--skip-zk` skips proving keys for a quicker local build.
Rust-target output is assembled in a sibling staging directory and published
only after the compiler, Rust renderer, runtime packaging and output manifest
complete. A failed run removes that stage and keeps any previous output.
The version-1 capability report lists each exported stateful circuit in
source order with its Compact source position and whether the generated crate
has a replayable `recorded` method and a typed observed-state `*_call` method.
Pure circuits have no ledger call and are omitted. The report is included in
the hashed compiler output manifest. A native-only method can be useful, but
does not by itself supply a proof-ready ledger call. To require both APIs for
every exported stateful circuit, use:

```sh
compactc --target rust --rust-require-recording \
  examples/rust_backend/counter.compact /tmp/compact-rust-output
```

The strict option reports an unsupported circuit at its Compact source
position and publishes no partial output. The capability report describes
generated API availability; proof, transcript parity, and wallet admission
still require the corresponding acceptance tests. Internal circuits are
omitted, and the report schema is separate from the private IR schema and
runtime ABI.
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

Use `--rust-runtime-registry` to generate a crate that depends on the
compiler-distributed runtime version exactly, without copying runtime or macro
sources:

```sh
compactc --target rust --rust-runtime-registry \
  examples/rust_backend/counter.compact /tmp/compact-counter-registry
```

The generated dependency is
`midnight-compact-runtime = { version = "=0.1.0", package = "midnight-compact-runtime" }`
for this compiler. The option cannot be combined with `--rust-runtime-root`.
The package is not yet published, so use this mode with the local archive-only
release rehearsal below until macro and runtime crates are available from a
registry. The default bundled mode remains the way to build one generated
crate without a registry release.

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
List, and plain/historic Merkle descriptors. Nested Maps have a typed structural slot with
`MapNode<K, V>` values. Its `is_empty`, `size`, and `member` shape reads are
available for native and recorded calls; scalar lookup and value mutations
remain unavailable until nested value semantics are proven.
The runtime's recording frame is opt-in. When a circuit has a complete recorded
trace, the generated `contract.recording()` handle exposes it as a typed method.
For witnessed root Cell circuits, this handle borrows the contract's
witness implementation and records private outputs in execution order. A
supported Unit-returning stateful call uses one private, frame-taking Rust
helper for each referenced callee. Calls evaluate typed arguments in source
order and share the frame across nested witness and ledger operations. The
recorded emitter also shares a private frame-taking helper for a supported
Field-returning callee with no actions used in a Field action binding. It
evaluates typed arguments in source order and preserves the witness result for
the subsequent write. Value calls nested inside larger expressions still use
inline lowering. The recorded emitter evaluates supported Field reads, witness
calls, addition, and internal Field-returning calls in source order. Supported
Cell reads, writes, assertions and Field read expressions can record at
compiler-assigned chunked paths through typed slots. Action-free Boolean and
Field returns with an explicit Cell read also use the same recorded frame;
witness-only returned expressions still need separate proof acceptance. Set and scalar Map
mutation, membership, lookup, size, and emptiness calls have
replayable recorded methods, including declaration-typed `FixedVector` keys in
supported circuits and compiler-assigned chunked paths. Typed List push, pop,
reset, length, emptiness, and head
calls also have recorded methods. Plain and historic Merkle append circuits
have recorded methods when their complete trace is supported; other Merkle
operations remain native. Other methods continue to
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
For a complete recorded circuit, the generated
`recording` handle also has a `*_call(&ObservedContractState, private_state, ...args)`
method. It carries the Compact entry point and FAB input into
`RecordedCall::prepare(verifier, commitment_randomness)`, which checks the
observed address, state, and installed verifier. The observation metadata is
supplied by the caller; it does not authenticate finality. Zero-to-two
parameters use the existing FAB conversion; larger signatures concatenate
each typed parameter's upstream FAB value and alignment in declared order.
Boolean/Field and Boolean/Boolean/Field inputs have direct TypeScript and
pre-proof parity evidence in ADR-0044 / issue #143; nested value shapes still
need independent parity evidence. See also ADR-0043 / issue #142.
`contract_crate::runtime::transaction::decode_verifier_key(&bytes)` reads the
compiler-emitted `.verifier` artifact without an extra direct dependency;
it rejects trailing bytes.
Generated modules suppress Rust's naming lint so public names retain their
Compact spelling without warning in consumer builds.

## Version compatibility

| Boundary | Current contract | Failure behavior |
|---|---|---|
| Compact compiler | Toolchain 0.31.133, language 0.23.105 | Versions are recorded in `compiler/contract-manifest.json`. |
| Rust IR | Schema 8, private to this backend | The renderer rejects any other schema before writing `lib.rs`. Ledger, circuit, witness, constructor, and exported alias declarations carry optional Compact source locations for diagnostics. |
| Generated code and Rust runtime | ABI 28 | Generated modules assert the ABI at Rust compile time. ABI 25–28 add physical List paths, chunked Map and Cell calls, and Cell-read scalar returns; ABI 21–24 add typed multi-argument observed calls and composite/chunked Set calls. The [runtime guide](../../runtime-rs/README.md) records earlier ABI changes. |
| Rust runtime source | Bundled runtime crates or an explicit shared source root | Cargo resolves the matching runtime and its pinned Midnight crates. |

`--runtime-version` reports the TypeScript runtime version; the Rust runtime
compatibility contract is the ABI assertion and matching source packages. The
generated `Cargo.toml` has `publish = false` because it is a contract-specific
artifact. Change the runtime source only alongside an ABI and consumer test review.

### Fallible witnesses

Generated crates derive the public `TryWitnesses<Private>` companion from their
visible `Witnesses<Private>` signatures. Implement its methods with
`Result<(Private, T), CompactError>` so a
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
through the derived adapter. Choose one trait for each witness type; the
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

After packaging, rehearse separately generated Counter and Boolean Cell
contracts together against the two exact archives in that manifest:

```sh
nix develop .#compiler --command python3 \
  tools/compact-rust-backend/check_archive_consumer.py \
  --manifest target/rust-runtime-release.json
```

This isolated gate vendors the pinned upstream crates and installs the macro
and runtime from their `.crate` bytes. It compiles both contracts independently
with `compactc --target rust --rust-runtime-registry` and runs their untouched
generated crates from one external consumer. It checks
that Cargo resolves exactly one shared runtime and macro as registry-style
packages from the archive source, including direct dependencies from both
generated crates. No local path patch is used for the final consumer. Source
replacement is a local release rehearsal; crates.io publication, a public
registry consumer, and remote CI remain #106 exit gates.

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

### Ledger-v8 wallet byte handoff

The proof smoke can export its sealed counter deployment and proven call as
two sequential ledger transactions. The matching JavaScript ledger package
must accept the exact bytes that Rust writes and confirm that the call targets
the deployment address:

```sh
nix develop .#compiler --command env COMPACTC=compactc \
  COMPACT_RUST_WALLET_HANDOFF=/tmp/compact-counter-wallet-handoff.bin \
  COMPACT_RUST_DEPLOY_HANDOFF=/tmp/compact-counter-deploy-handoff.bin \
  python3 tools/compact-rust-backend/check_compactc_target.py --proof
npm ci --prefix tools/compact-rust-backend/wallet-handoff --ignore-scripts
node tools/compact-rust-backend/check_wallet_handoff.mjs \
  /tmp/compact-counter-deploy-handoff.bin \
  /tmp/compact-counter-wallet-handoff.bin \
  tools/compact-rust-backend/wallet-handoff/node_modules/@midnight-ntwrk/ledger-v8
```

The pinned `@midnight-ntwrk/ledger-v8@8.0.3` decoder checks both transactions'
signature, proof and binding markers, one deploy and one call, matching
addresses, and byte-for-byte reserialization. The proof gate applies the
deployment with ledger semantics before checking and applying the call. These
exported transactions are **offline fixtures**: their network ID is
`local-test`, their intent TTL is timestamp zero, and fee balancing is disabled.
For an isolated ledger-8 devnet, opt in to that network ID and a future Unix
expiry no more than one hour ahead. The proof smoke applies each transaction
with a matching block timestamp and still checks all 58 offline calls:

```sh
COMPACT_RUST_HANDOFF_TTL_SECS=$(python3 -c 'import time; print(int(time.time()) + 1800)')
nix develop .#compiler --command env COMPACTC=compactc \
  COMPACT_RUST_HANDOFF_NETWORK_ID=undeployed \
  COMPACT_RUST_HANDOFF_TTL_SECS="$COMPACT_RUST_HANDOFF_TTL_SECS" \
  COMPACT_RUST_DEPLOY_HANDOFF=target/compact-rust-live-deploy.bin \
  COMPACT_RUST_WALLET_HANDOFF=target/compact-rust-live-call.bin \
  python3 tools/compact-rust-backend/check_compactc_target.py --proof
```

The ledger-8.0.3 wallet facade owns balancing, finalization, and submission
with application keys and funds. Its pinned 3.0.0 API has no
`validateTransaction` method. A version-pinned local integration driver is in
`wallet-live`; it uses node 0.22.3, indexer 4.0.1, proof server 8.0.3,
wallet facade 3.0.0, and `undeployed` network. The tested stack matches
[`midnight-local-dev` at `c16aa4f`](https://github.com/midnightntwrk/midnight-local-dev/tree/c16aa4ff57374a3b8519e90d6f003227f8041996).
Run it against a fresh, funded local devnet, supplying its 32-byte wallet seed
through the environment:

```sh
npm ci --prefix tools/compact-rust-backend/wallet-live --ignore-scripts
COMPACT_RUST_WALLET_SEED_HEX=<local-devnet-seed> \
COMPACT_RUST_HANDOFF_NETWORK_ID=undeployed \
COMPACT_RUST_INDEXER_URL=http://127.0.0.1:8088/api/v3/graphql \
COMPACT_RUST_NODE_URL=http://127.0.0.1:9944 \
COMPACT_RUST_PROOF_SERVER_URL=http://127.0.0.1:6300 \
node tools/compact-rust-backend/wallet-live/check.mjs \
  target/compact-rust-live-deploy.bin target/compact-rust-live-call.bin
```

The driver syncs the wallet, registers NIGHT for DUST when necessary, rejects
an already used contract address, then balances, finalizes and submits the
deployment. It waits for the indexer to report `ContractDeploy` at the exact
address and final merged transaction hash before balancing and submitting the
call. It requires the same exact transaction identity for each indexed
`ContractCall`, then decodes ledger-v8 state to check `round` is 1 and, in the
opt-in second call, 2. For every action, the driver checks that the indexer's
block hash matches the node's canonical hash at a height no greater than its
finalized head. The wallet's returned intent identifier can differ from the
merged transaction hash. This check trusts the connected node and indexer; it
does not verify a consensus finality proof. ADR-0042 and
[issue #141](https://github.com/MediaNoxLabs/compact/issues/141) track the
identity and finalized-block gate.

To prove a subsequent call from the indexed `round = 1` state, compile the
Counter artifacts and Rust builder, export both opt-in environment variables,
then rerun the wallet command above against a fresh devnet:

```sh
nix develop .#compiler --command compactc --target rust \
  examples/rust_backend/counter.compact target/compact-rust-counter-live
cargo build -p compact-rust-proof-smoke --example record_from_confirmed
export COMPACT_RUST_CONFIRMED_CALL_BUILDER=target/debug/examples/record_from_confirmed
export COMPACT_RUST_COUNTER_ARTIFACTS=target/compact-rust-counter-live
```

The driver writes the indexed `ContractState` to a temporary file, passes the
action's transaction hash, block hash and height, then invokes the Rust
builder introduced at ABI 20. Its generated `increment_call` method records from
`ObservedContractState`, and `RecordedCall::prepare` checks the installed
verifier against the emitted artifact. The builder proves and submits a new
call and requires a second indexed `ContractCall` with `round` equal to 2.
It validates the call on a local projection and rejects replay against the
projected `round = 2` state before wallet balancing. The caller remains
responsible for the indexer/address association and chain finality. ADR-0041 and
[issue #140](https://github.com/MediaNoxLabs/compact/issues/140) record this
Counter-specific decision and its broader production limits.

At signed/DCO ABI-28 commit `e75f13ba`, this pinned local stack admitted the
Rust-exported deployment, first call, and generated confirmed-state second
call through wallet facade 3.0.0. The indexed Counter state advanced from
`round = 1` to `round = 2`; the last call's block matched the node's canonical
hash under finalized head. The [same-head delivery record](https://github.com/MediaNoxLabs/compact/issues/105#issuecomment-5972867741)
gives the exact source, versions, proof and transaction evidence. This result
uses a trusted local node and indexer; remote CI and release gates remain open.

The wallet operations are equivalent to:

```ts
const tx = ledger.Transaction.deserialize('signature', 'proof', 'binding', bytes);
const recipe = await facade.balanceFinalizedTransaction(tx, keys, { ttl });
const finalTx = await facade.finalizeRecipe(recipe);
await facade.submitTransaction(finalTx);
```

The default byte checks alone do not establish network admission. ADR-0040 and
[issue #139](https://github.com/MediaNoxLabs/compact/issues/139) track this
local admission gate and its remaining production limits. ADR-0033 and
[issue #132](https://github.com/MediaNoxLabs/compact/issues/132) track the
call boundary; ADR-0035 and [issue #134](https://github.com/MediaNoxLabs/compact/issues/134)
track the paired deployment. Parent #105 retains broader wallet/node release readiness.

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
native Merkle queries do not imply recorded proof coverage for every operation;
supported plain and historic append circuits have separate recorded traces.
The `--proof` target check derives the Counter increment statement from the
generated recorded trace, proves it against emitted ZKIR and keys, and rejects
a changed binding input. It also validates offline ledger-8 deployments and
proves, verifies, validates, and applies the supported Counter, Cell, Set,
Map, List, enum Cell, `tiny`, plain/historic Merkle append, and vector-key Set
insert call fixtures (56 offline calls). `tiny` is proved in both
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
