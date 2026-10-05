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
position, with a reason code and structural IR path, and publishes no partial
output. Capability report schema 3 retains the `recorded` and `observed_call`
booleans, adds compiler-derived `proof_required` and `recording_status`, and retains `recording_unavailable` or `observed_call_unavailable` when
an API is missing. Each reason includes a stable code, IR node name, path, and
detail. These paths identify the nearest definite failure; some nested
expressions still report the enclosing action or return. The report describes
generated API availability and compiler proof applicability; proof execution, transcript parity, and wallet admission
still require the corresponding acceptance tests. Internal circuits are
omitted, and the report schema is separate from the private IR schema and
runtime ABI.
The library's `render_with_capabilities` returns an unclassified schema-2
lowering draft for diagnostics. `render_with_proof_capabilities` requires the
frontend `contract-info.json` value, validates one proof flag per exported
circuit, permits extra nonexported helper entries, and produces the published
schema-3 report. The CLI uses this validated path; unknown applicability never
defaults to `false`.

The checked parity inventory also includes the 18 TypeScript-positive
PM-19252 Compact sources listed in `parity_positive_sources.json`. Its one
expected-rejection source, `example_fourteen.compact`, is pinned in that
manifest but excluded from the positive inventory. Run
`check_positive_source_scope.py --compiler /path/to/immutable/compactc --output
/tmp/pm19252-scope.json` with `COMPACTC_SCHEME` set for source-built compilers
to verify TypeScript/Rust compile outcomes and authoritative contract-info
proof flags. Compiler-backed inventory joins capabilities from all 18
TypeScript-positive PM-19252 sources. ADR-0085/#186 makes
`example_ten.compact` compile for Rust and tests its native
`ownPublicKey()` private output in a generated consumer. Its exported circuit
is proof-false and adds no recorded proof API.
The checked ADT Set cohort in `parity_positive_adt_set_sources.json` pins all
five `examples/adt/tests/set_*.compact` sources from the TypeScript ADT
acceptance suite. Run `check_positive_source_scope.py --manifest
tools/compact-rust-backend/parity_positive_adt_set_sources.json --compiler
/path/to/immutable/compactc --output /tmp/adt-set-scope.json` for its separate
compiler receipt. The manifest locks glob membership and authoritative proof
flags. All five sources compile for Rust. The original
`set_qualified_coin_info` now lowers its typed `insertCoin` through ledger-8's
transaction commitment index and executes natively; its two proof-required
exports remain native-only because recording does not yet cover their full
root `Let` bodies. `set_struct` has no contract circuit. The proof-required
`set_field.test`, `set_enum.test`, and `set_vector.test` have recorded and
observed-call APIs. `set_enum` and
`set_vector` have TypeScript/native/recorded state, gas, VM, and proof
application coverage through generated fixtures. The cohort is tracked in
#188, #190, #196, and #201; source acceptance alone does not establish
executing parity for the remaining circuits.
`qualified_coin_set_oracle.compact` separately checks a contract and a user
recipient at nonzero allocated indices against an independent TypeScript
capture, including Set state, effects, gas, missing commitment rejection and
wrong Set element alignment. Run `check_compactc_target.py --adt-set-qualified`
for the unchanged original source's schema-14 and native-only capability gate.
Use `check_fixture_outputs.py --only qualified_coin_set_oracle.compact` for a
focused generated fixture check. All checked fixtures use the combined ABI-39
runtime, including audited local helpers and qualified coin insertion.
The bounded ADT List manifest `parity_positive_adt_list_sources.json` admits
`examples/adt/tests/list_field.compact::test` after the nested List query
lowering in ADR-0101/#204. Its generated fixture checks the TypeScript,
native, and recorded state, four gas dimensions, ordered VM trace, and replay;
the proof smoke verifies and applies the observed call through ledger-8.
The other eleven `list_*.compact` sources are not claimed by this receipt.
The top-level source cohort in `parity_positive_top_level_sources.json` checks
the original `examples/counter.compact` and `examples/tiny.compact` paths with
the same TypeScript/Rust compiler and compares their formatted generated Rust
against the existing Counter and Tiny fixtures. The corresponding Compact
sources have identical bodies after comments are removed and the fixtures
carry separate local runtime tests;
the check preserves original source identity instead of attributing a fixture
path to an example. Counter contributes two known proof-required recorded APIs.
Tiny contributes three proof-required recorded APIs and one compiler-pure,
proof-false helper. The full local parity gate runs this cohort check. Original
`parity_positive_original_election_zerocash_sources.json` now checks the exact
original `examples/election.compact` and `examples/zerocash.compact` paths.
Both compile for TypeScript and Rust; the seven exported circuits are
compiler-proof-required but explicitly recording-unavailable at their first
typed Assert or Let action. The generated original Zerocash library is
byte-identical after formatting to the existing checked oracle fixture. The
Election oracle fixture has a different constructor because it adds an
authority parameter; its native TypeScript state test is analogous evidence,
not a test of the original constructor. The separate original generated crates
have passed `cargo check`, but source acceptance and crate construction do not
claim recorded calls or executable parity for the original sources. The full
local parity gate runs this cohort check.
The original test-center Counter in
`parity_positive_test_center_counter_sources.json` has one proof-required
`increment` call. Its public Counter increment followed by a standalone Unit
witness now records through the existing metered witness frame. The checked
TypeScript capture, native and recorded Rust fixture, ordered public VM trace,
private output, and replay agree; a pinned emitted ZKIR proof verifies and
applies through ledger-8. The exact original `welcome.compact` source now
compiles with a typed constructor Vector parameter loop and conditional Maybe
insertion. Its generated crate and empty/one-participant constructor state
match the checked TypeScript capture on a default Rust test thread. Its three
proof-required exports remain recording-unavailable at their first assertions;
constructor acceptance adds no proven call. The complete original
`bboard.compact` source now compiles to native Rust after retaining a typed
tail-`Let` binding across its ordered writes. Its empty-board rejection and
Unicode post/take-down cycle match TypeScript in result, serialized state,
ledger values, witness calls, and summed query gas. `post` and `take_down`
remain proof-required but recording-unavailable; pure `public_key` requires no
proof. `coracle` still rejects a nested ledger query, and `micro-dao` rejects
a standard-library expression.
The separate `let_return_oracle.compact` source now covers a root circuit
`let` whose bound Cell read must retain its pre-write value across ordered
actions and the final return. Its generated Rust crate matches two sequential
TypeScript calls in result, state, ordered VM transcript, private outputs,
and four query-meter gas dimensions, and its recorded call proves and applies
with the pinned ledger-8 tools. This assesses one new proof-required export.
The `bboard` native acceptance is a separate source cohort with explicit
recording gaps; it adds no proven call.
The lexical scanner includes `pure circuit` and `export pure circuit`
declarations, with compiler `contract-info.json` supplying proof applicability
even when the Rust capability report has no recorded method for a pure circuit.
This fixes the earlier pure-declaration omission (#184); the scanner remains a
source inventory, not a full Compact parser or executing parity gate.
The inventory distinguishes a module member's `module_export` from a contract
`export`: only a top-level export or top-level `export { name }` makes the
declaration a contract API candidate. For example, `Schnorr.schnorrVerify`
and `Alpha.makeAlpha` are module exports but not contract circuits; the
re-exported `M.bump_inner` is a contract circuit. The checked baseline includes
both scopes, while `--require-full` gates only contract exports. Compiler
`contract-info.json` remains authoritative for those exported APIs. Any
contract export absent from that metadata remains explicitly unknown in
`missing_compiler_proof_rows`; the full gate does not assume that it is
nonprovable.
The generated `ledger_slots` module exposes named typed descriptors for Cell,
Counter, Set, Map, List, and Merkle declarations. For example,
`ledger_slots::tree.insert(context, value)` accepts the declared Merkle leaf
type and uses its declared path and depth. Plain and historic Merkle slots
share the native API; only historic slots expose `reset_history`. Plain slots
expose `record_reset_to_default`, sharing the native ledger VM reset program.
The original `merkle_tree_oracle.compact` now has recorded and observed-call
APIs for all eight proof-required exports and passes strict recording mode.
Its reset coverage checks empty and populated trees against TypeScript state,
gas and ordered VM execution, then proves and applies a populated-tree reset.
These slot
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
calls, addition, subtraction, multiplication, and internal Field-returning
calls in source order. Field arithmetic reuses the upstream ledger-8 `Field`
operators; only its ledger and witness operands contribute recording effects. Supported
Cell reads, writes, assertions and Field read expressions can record at
compiler-assigned chunked paths through typed slots. Action-free Boolean and
Field returns with an explicit Cell read also use the same recorded frame;
witness-only returned expressions still need separate proof acceptance. Set and scalar Map
mutation, membership, lookup, size, and emptiness calls have
replayable recorded methods, including declaration-typed `FixedVector` keys in
supported circuits and compiler-assigned chunked paths. Typed List push, pop,
reset, length, emptiness, and head
calls also have recorded methods. Plain and historic Merkle append circuits,
and direct `isFull()` Boolean returns, have recorded methods when their complete trace is supported; other Merkle
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
| Rust IR | Schema 14, private to this backend | The renderer rejects any other schema before writing `lib.rs`. Schema 14 adds typed qualified-coin Set insertion. Schema 13 adds an explicit typed Field-to-Bytes32 expression and nested Counter read; the cast uses midnight-zk's canonical 32-byte little-endian Field representation. Ledger, circuit, witness, constructor, and exported alias declarations carry optional Compact source locations for diagnostics. |
| Generated code and Rust runtime | ABI 39 | Generated modules assert the ABI at Rust compile time. ABI 39 adds qualified-coin Set insertion using ledger coin and recipient types and the allocated commitment index. ABI 38 adds audited local-helper adoption for private witnesses, gas, and transcript while checking the public and Zswap context. ABI 37 adds a caller coin key for native `ownPublicKey()` and its private output; ABI 36 adds recorded plain Merkle root checks through typed slots; ABI 35 adds recorded Counter reset through typed slots; ABI 34 adds recorded direct plain/historic Merkle fullness reads; ABI 33 adds typed local plain/historic Merkle views with checked depth; ABI 32 adds List views; ABI 31 adds cell-valued Map views; ABI 30 adds Set views; ABI 29 adds Cell/Counter views; ABI 25–28 add physical List paths, chunked Map and Cell calls, and Cell-read scalar returns; ABI 21–24 add typed multi-argument observed calls and composite/chunked Set calls. The [runtime guide](../../runtime-rs/README.md) records earlier ABI changes. |
| Rust runtime source | Bundled runtime crates or an explicit shared source root | Cargo resolves the matching runtime and its pinned Midnight crates. |

`--runtime-version` reports the TypeScript runtime version; the Rust runtime
compatibility contract is the ABI assertion and matching source packages. The
generated `Cargo.toml` has `publish = false` because it is a contract-specific
artifact. Change the runtime source only alongside an ABI and consumer test review.

`ownPublicKey()` can be used as a returned value or inside an expression such
as `ownPublicKey().bytes`. The generated circuit reads the key from its
`CircuitContext`, returns the named `ZswapCoinPublicKey` type where applicable,
and emits one aligned private FAB output per call. Callers pass the key with
`context.with_coin_public_key_bytes(key)`; they do not implement a user witness
for this native. These circuits are proof false in the pinned TypeScript
compiler, so recording and observed calls are not applicable.

### Inspecting public state

For a contract with declared Cell, Counter, Set or cell-valued Map fields,
`ledger_contract::PublicStateView`
borrows existing ledger state and exposes fallible getters with the declaration's
Rust type. Counter getters return Compact `BoundedUint<2^64-1>`; Cell getters
return the declared type. A Set getter returns the runtime's borrowed `SetView`,
whose `member` and `is_empty` inspect local state and whose `size` is fallible.
Map getters return the runtime's borrowed `MapView`; `lookup` fails when a key
is absent and retains the declared value decoder. Nested Maps have no scalar
getter. For example, a generated Counter named `round` can
be read after a call without copying its physical ledger index:

```rust
let view = ledger_contract::PublicStateView::from(&result);
let round = view.round()?.value();
```

The view also accepts borrowed constructor results, query contexts and replay
query results,
`ContractState`, recorded results, and, with `ledger-transaction`,
`ObservedContractState`. It only decodes local state;
the caller remains responsible for the origin and freshness of an observation.
If a Compact field is named `from`, use `let view: PublicStateView<'_> = state.into();`
so its generated getter does not shadow trait method syntax. List and
Merkle inspection are still available through the lower-level ledger APIs.

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

### Local parity delivery gate

Run [`local_parity_gate.py`](local_parity_gate.py) from the repository root for
the smallest affected Rust fixture slice. It copies `compactc` and its Scheme
frontend into a unique run directory before compiling, so a concurrent Cargo
build cannot replace the compiler in use. Each run writes `receipt.json` with
the exact Git HEAD, dirty paths, SHA-256 of both compiler binaries, Rust
ABI/IR and upstream lock metadata, source and fixture hashes, capability
reports, a name-checked join of exported APIs against the compiler's `proof`
flags, commands, and log paths. Its summary separates proof-eligible recording gaps from
nonproof native circuits. The receipt is kept in the printed run
directory. It records what ran; a dirty worktree remains visible in the
receipt.

The focused gate requires the published schema-3 capability report. It checks
`proof_required` and `recording_status` against the compiler's own
`contract-info.json` for every exported circuit; extra nonexported helper
circuits in that metadata are allowed.

```sh
python3 tools/compact-rust-backend/local_parity_gate.py \
  --source bounded_uint_oracle --source wide_uint_oracle \
  --compiler target/debug/compactc \
  --scheme /path/to/compactc-scheme \
  --expect-head "$(git rev-parse HEAD)"
```

Focused mode compiles and compares only the selected generated fixtures,
validates their capability reports and pinned declaration identities, and
runs only their Cargo fixture packages. Omit `--source` for the small
`counter_parameter` smoke. `--skip-cargo` is available for a compile and
fixture check while another Cargo gate owns the shared target directory.
Use `--run-dir /path/to/new-directory` to choose where the receipt and logs
are stored. If Scheme is beside `compactc`, `--scheme` can be omitted; the
`COMPACTC_SCHEME` environment variable is also accepted. Cargo uses the
reusable, shared `target/compact-rust-parity-gate` directory to keep it separate
from `target/debug/compactc`. Concurrent agents or worktrees should pass
distinct `--cargo-target-dir` paths.

`--full` is an explicit broader local gate: all checked Rust fixtures and
curated declaration identities, Rust formatting, compiler rejection checks,
the pinned TypeScript oracle source inventory, backend workspace tests and
Clippy, Compact CLI unit tests, then the consumer and proof checks. The legacy
compactup integration tests depend on the host-installed compiler and live
GitHub release data; run those separately when validating compactup. The full
backend gate can take substantially longer. Neither mode
pushes commits, starts remote CI, or interprets fixture coverage as full
TypeScript parity across the repository's Compact corpus.

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
supported plain and historic append and direct fullness circuits have separate recorded traces.
The `--proof` target check derives the Counter increment statement from the
generated recorded trace, proves it against emitted ZKIR and keys, and rejects
a changed binding input. It also validates offline ledger-8 deployments and
proves, verifies, validates, and applies the supported Counter, Cell, Set,
Map, List, enum Cell, `tiny`, plain/historic Merkle append and fullness, and vector-key Set
insert call fixtures. `tiny` is proved in both
present and absent `get` branches. Other circuit operations still need
recording coverage before wallet submission.
The scalar pure Field argument check also proves, verifies, validates, and
applies `internal_pure_call.save`, `ternary_cond_oracle.streamCallPure`, and
`streamCallWitness` from their emitted artifacts. Its proof smoke runs on a
dedicated 64 MiB thread because ledger proof composition overflows macOS's
default main-thread stack; the shell's stack limit need not be raised.
The same scoped proof stack covers `stateful_pure_call.save`, whose recorded
Field result is evaluated after its Cell write.

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

### Local parity toolchain

`local_parity_gate.py` pins all child processes to Rust 1.99.0, including the
packaged consumer and proof checks that invoke plain `cargo`. The receipt
records `rust_toolchain`. This avoids rebuilding the shared target with the
machine default after workspace checks use 1.99.0; machine settings are not
changed. Run separate explicit toolchain checks for MSRV coverage.

### Checked literal Bytes-to-Field casts (ADR0168)

The Rust frontend normalizes byte-string literals and closed literal byte vectors
cast to `Field` into the existing `field_literal` IR. Decoding is little-endian;
values must be canonical (`0..=MAX_FIELD`), with no modular reduction. This adds
no IR schema or runtime ABI. Dynamic bytes/vectors and non-Field targets remain
explicit source gaps. `Bytes<0>` retains the language type-checking rejection.
Out-of-range closed constants fail Rust compilation; TypeScript accepts those
sources and rejects the cast when evaluated. This difference in failure timing
is deliberate and covered by the source gate.

`check_compactc_target.py --literal-bytes-field` checks canonical boundaries and
six rejection cases; add `--proof` for the recorded snapshot proof and ledger
application. The TS runtime must be built under `runtime`, or supplied via
`COMPACT_TS_RUNTIME_DIR`. The TS oracle stores its reported gas and individual
query costs; Rust total gas is compared against the query sum. A native direct
literal stateful return still has the existing recorded-return capability gap.

The original unchanged `test-center/test-contracts/micro-dao.compact` now passes
its standard-library nonce-domain casts. Its next diagnosed source gap is nested
`Counter.read` at line 192 (`no.lessThan(yes)`); shielded coin operations and
`pot.writeCoin` remain separate admission work. This is not full DAO parity.

### Full-gate Cargo target selection

The local parity gate reads Cargo workspace metadata after comparing every
fixture with fresh compiler output. Core packages and unknown fixture shapes
retain `--all-targets`. Verified generated libraries with only the generator's
known attributes/macros and no test hooks run every integration target through
`--test '*'`, avoiding empty library unit-test executables. A new inline test,
unknown macro, external module, extra target kind, or unverified source restores
all-target execution for that package. Packages without integration tests also
keep their full selection. No manifest settings are changed.

The receipt records `workspace_test_plan`, including package membership, all
integration target names and omitted empty-library source hashes. Python harness
tests, compact CLI unit tests, all-target Clippy and consumer/proof gates remain
part of the broad run. ADR-0172/#276 records the Cargo selection probe; target
coverage is verified, while wall-clock savings require a later measured run.
