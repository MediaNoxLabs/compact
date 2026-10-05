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

## Effectful return recording

The bounded Field Cell profile records an ordered return body containing lexical
bindings, reads/writes, and conditional Field returns. Conditions execute after
preceding effects; only the selected branch consumes witnesses or emits VM
operations. Field witnesses, Field addition/equality, and literal unsigned locals
explicitly cast to Field preserve their declared types. Each branch returns the
updated recording frame with its typed result. Both branches are audited before
recording is advertised; unsupported effects and types remain explicit gaps.

`effectful_return_oracle.compact` has independent TypeScript/native/recorded
transcript, state, private-output, query-gas and replay-gas parity, plus both-branch
proof verification and ledger application under the existing unbalanced smoke
policy. This does not establish funded transaction admission or recording parity
for arbitrary effectful returns. The earlier single-slot root-Let profile keeps
its separate admission rules. IR schema 20 and runtime ABI 46 are unchanged.

## Stateful assertion recording

A separate read-only profile records Boolean short-circuit assertions over
Boolean Cells, Counter reads/comparisons, and typed Boolean witnesses. It keeps
lexical bindings and returns Unit or Uint64. Every sequence step must be Unit;
a failed assertion stops before later witnesses or result queries. Both branches
are audited, and additional writes or unsupported calls remain recording gaps.

The stateful assertion oracle preserves twelve original TypeScript cases and
adds a per-query budget rejection after two successful reads. Native and recorded
execution match success transcripts, outputs, state, query-summed gas and replay
cost; failures match rejection class and observable witness prefixes. Failed
Rust calls do not expose consumed context/gas, so no failed aggregate gas parity
is claimed. Both APIs pass proof verification and ledger application under the
shared unbalanced smoke policy. No runtime, IR schema or ABI change is required.

## Recording eligibility and unrelated witnesses

Field root-Let and Counter comparison recording count witness calls used by the
complete lowered circuit, including both audited branch bodies. An unrelated
export that retains its own witness declaration no longer removes these APIs.
The original no-witness domain remains enforced: adding a witness to the
Field update, Counter threshold or an untaken branch is still rejected by these
profiles. Source composition checks join compiler proof applicability and compare
generated native/recorded calls against the unchanged original fixtures.

## Typed composite return recording

A bounded struct-result profile records ordered typed members, small unsigned
casts, declared unsigned witnesses, defaults and canonical `Kernel.self` queries.
Matching composite conditional branches return their frame and value together.
Expression-only local helpers are audited transitively and lowered into the same
frame: arguments evaluate once in caller order, callee scopes are isolated, and
cycles, signature mismatches and unsupported effects are rejected. No native
helper bridge is used to hide public VM reads.

The unchanged struct oracle records `snapshot`, `reverse` and `nested`, with
TypeScript/native/recorded output, private sequence, program and gas parity.
Named source fields use the compiler's normalized declared-member order. All
three nonempty paths prove and ledger-apply under the existing unbalanced smoke
policy. `snapshot(false)` preserves zero operations and rejects preparation with
`EmptyTranscript`, matching the pinned ledger boundary. ADR0191 below adds the
`planned` Zswap output member using the same typed planner. ADR0187 itself made
no runtime ABI or schema change.

## Portable compiler archives

`nix build .#compactc-binary` includes the matching runtime and derive sources
under `share/compactc`. Package this output with:

```sh
python3 tools/compact-rust-backend/build_compiler_archive.py \
  --package result --output compactc.zip
```

The archive retains the installer's existing top-level command names while
preserving the full runtime directory tree, including crate licenses. Nix epoch
timestamps are clamped to the ZIP format minimum. Optional `--release-notes FILE`
adds notes without overwriting a compiler file. Extract the whole archive;
copying only the executables is insufficient for Rust generation. The compiler
finds runtime sources relative to its executable, and the launcher resolves
installer symlinks and paths containing spaces before locating ZKIR helpers.
Explicit `COMPACT_RUST_RUNTIME_DIR` overrides retain priority.

Local release checks must exercise the **extracted portable archive** with
runtime and Scheme overrides unset, from outside the repository, including
default TypeScript generation, strict Rust generation and an offline generated
Cargo consumer. A check of the normal Nix package alone does not cover this
release path. Verify each supported platform separately.

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
are recorded and observed; pure `public_key` requires no
proof. Complete original `coracle` and `micro-dao` now compile to native Rust.
Their recording gaps remain explicit in their positive source manifests.
The separate `let_return_oracle.compact` source now covers a root circuit
`let` whose bound Cell read must retain its pre-write value across ordered
actions and the final return. Its generated Rust crate matches two sequential
TypeScript calls in result, state, ordered VM transcript, private outputs,
and four query-meter gas dimensions, and its recorded call proves and applies
with the pinned ledger-8 tools. This assesses one new proof-required export.
The `bboard` source cohort tracks its recorded exports explicitly.
The `root_let_action_return_oracle.compact` source covers the complementary
root-Let shape: local bindings feed a Cell read/write action, while the final
return is independent of those bindings. Scheme now extracts the ordered
actions from this lexical frame using the existing schema-14 `Let` action.
Two sequential TypeScript and native calls agree on result, serialized state,
ledger effects, one read and one write per call, and the summed four-dimension
query gas. Its export now supports recorded/observed calls. Combined root-Let,
effectful-return and stateful-assertion lowering admits complete original Coracle.
`parity_positive_test_center_coracle_sources.json` checks all nine exports:
five pure and four proof-required (`start`, `guess`, `concede`, `withdraw`).
The four recording gaps remain exact: `StateReturn::Effectful` for `start`,
`StateReturn::Expression` for the others, all at `return_value`. Both
`--effectful-return` and `--coracle-root-let` check native acceptance and strict
recording refusal. The full local gate runs the Coracle source manifest,
including one offline Rust 1.99 native Cargo check of the emitted crate.
This establishes source and native crate acceptance; full original-contract
behavior, recorded proofs and funded transactions remain separate work.
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
| Rust IR | Schema 20, private to this backend | The renderer rejects any other schema before writing `lib.rs`. Schema 20 adds typed Kernel mint/claim effects. Schema 19 adds single-owner typed effectful return plans. Schema 18 adds typed native circuit Zswap intents. Schema 16 adds native qualified-coin Cell writes. Schema 15 adds typed Counter less-than queries. Schema 14 adds typed qualified-coin Set insertion. Schema 13 adds an explicit typed Field-to-Bytes32 expression and nested Counter read; the cast uses midnight-zk's canonical 32-byte little-endian Field representation. Ledger, circuit, witness, constructor, and exported alias declarations carry optional Compact source locations for diagnostics. |
| Generated code and Rust runtime | ABI 49 | Generated modules assert the ABI at Rust compile time. ABI 49 adds recorded native own-key outputs and sealed execution identity. ABI 48 adds exact offer-bound native intent recording and sealed intent reconciliation. ABI 47 adds bounded recorded Kernel effects using canonical upstream programs. ABI 46 adds checked native wide unsigned addition. ABI 45 adds native Kernel shielded effects through upstream VM queries. ABI 44 adds recorded qualified-coin Cell writes through the shared native VM builder. ABI 43 adds typed circuit Zswap intents and locked observed allocation. ABI 42 adds native qualified-coin Cell writes; ABI 41 adds typed Counter less-than queries; ABI 40 adds recorded qualified-coin Set insertion, metered `kernel.self()`, and offer-backed observed calls. ABI 39 adds qualified-coin Set insertion using ledger coin and recipient types and the allocated commitment index. ABI 38 adds audited local-helper adoption for private witnesses, gas, and transcript while checking the public and Zswap context. ABI 37 adds a caller coin key for native `ownPublicKey()` and its private output; ABI 36 adds recorded plain Merkle root checks through typed slots; ABI 35 adds recorded Counter reset through typed slots; ABI 34 adds recorded direct plain/historic Merkle fullness reads; ABI 33 adds typed local plain/historic Merkle views with checked depth; ABI 32 adds List views; ABI 31 adds cell-valued Map views; ABI 30 adds Set views; ABI 29 adds Cell/Counter views; ABI 25–28 add physical List paths, chunked Map and Cell calls, and Cell-read scalar returns; ABI 21–24 add typed multi-argument observed calls and composite/chunked Set calls. The [runtime guide](../../runtime-rs/README.md) records earlier ABI changes. |
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
ADR219 additionally pins exact defining-file locations for nested expressions,
constructors, imported helpers, witness results, exported struct fields and
ledger-map values. The cast cases compile with TypeScript; unknown opaque type
cases are explicitly shared target refusals. Each context checks fresh output
and byte-for-byte preservation of an existing complete Rust output, alongside
the existing strict capability diagnostics and post-render publication checks.
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

### Typed Counter comparisons (ADR0171)

`Counter.lessThan(threshold)` uses a typed `CounterLessThan` expression with an
exact `Uint<64>` threshold and a validated Counter slot. It executes Compact's
actual `dup / idx / push / lt / popeq` ledger program. Counter reads nested in
lexical bindings reuse `CounterRead`. Runtime ABI41 adds `CounterSlot::less_than`
and `record_less_than`; recording preserves the gathered Boolean's FAB alignment.
The bounded shared typed planner supports read-only Boolean/Unit comparison
circuits with Boolean/Uint64 arguments, scoped bindings and short-circuit frames.

`check_compactc_target.py --counter-less-than [--proof]` compiles the fixture and
checks all four recorded/observed exports; proof mode verifies and ledger-applies
both comparison outcomes. TS captures all actual queries, their gas and program
shape; native/recorded totals use the query-cost sum. Assertion rejection is
checked separately because Rust error results do not expose partial query gas.
Original micro-dao now reaches unsupported `pot.writeCoin` at line171; this is
source progress, not complete native or recorded DAO admission.

### Native qualified-coin Cell writes (ADR0173)

Schema16 `CellWriteCoin` validates a Cell of the exact Compact qualified-coin
shape and typed coin/recipient operands, sharing native operand lowering with
qualified Set insertion. ABI42 adds `CellSlot::write_coin` and the context method.
Both collection operations share the upstream coin commitment calculation and
allocation-presence validation. The actual index lookup and qualification happen
inside the ledger VM; the runtime never fabricates an index. ABI44 shares the
qualified Cell program with recorded execution and exposes a typed observed call.

The Cell program preserves Compact's root and parent-path stack offsets, concat91
and insertion flags. Root and `[1,14]` parent paths have independent generated TS
program/state/effect/gas evidence; empty paths and depths exceeding the VM's
four-bit dup operand reject. Caller-supplied allocation tables in native fixtures
are test context inputs; production allocation remains ledger-owned. Missing or
wrong-recipient allocations reject before querying, matching TS preflight checks.

`check_compactc_target.py --qualified-coin-cell` verifies native and recorded
source admission, the chunked path probe, and required-recording acceptance.
With `--proof`, it runs the qualified Cell proof smoke described below. Its
recipient is a parameter, so it does not mask the separate Kernel.self query/gas
fix from ADR0170.

The unchanged original micro-dao advances past `pot.writeCoin` to an unsupported
standard-library native-witness expression. Its createZswapOutput/createZswapInput
family and further shielded operations remain separate source-admission work.


### Funded qualified-coin proof smokes (ADR0180)

Both `compact-rust-proof-smoke --qualified-coin-set <proof-output>` and
`--qualified-coin-cell <proof-output>` retain the output-only default-strict
`BalanceCheckOverspend(-42)` negative and its explicitly balancing-disabled
proof/application check. They also exercise a funded call under unchanged
`WellFormedStrictness::default()` and require successful ledger application.

The funded fixture encrypts a genesis coin to upstream Zswap keys, recovers it
through the upstream local wallet, seals its ledger Merkle root, and spends it
into a same-token/value output using `Offer::new`. It compares native and recorded
state, effects and gas, then checks applied contract state, actual allocated
index/commitment, the spent nullifier, and exact `NullifierAlreadyPresent` replay
rejection. Stateful replay rejection is checked by `zswap.try_apply`.

Strict fee validation also needs Night-backed Dust. Upstream
`TestState::give_fee_token` registers Dust generation, rewards Night, and advances
fixture time to the Dust cap; `balance_tx` proves the fee spend. The upstream
reward resolver requires `MIDNIGHT_LEDGER_TEST_STATIC_DIR` to name the ledger
checkout's `ledger/static` directory. Set it before either smoke (including the
Python `--proof` wrappers):

```sh
export MIDNIGHT_LEDGER_TEST_STATIC_DIR=/path/to/midnight-ledger/ledger/static
cargo run -p compact-rust-proof-smoke -- --qualified-coin-set /path/to/set-proof-output
cargo run -p compact-rust-proof-smoke -- --qualified-coin-cell /path/to/cell-proof-output
```

Use pinned ZKIR 2.1.0 contract artifacts and the upstream Zswap/Dust proving
material. Genesis insertion, reward registration and time advancement are
privileged local fixture setup. This proves funded local transaction acceptance;
it does not establish a wallet funding API, network submission or finality.


### Native circuit Zswap intents (ADR0175)

Schema18 adds typed `CreateZswapInput` / `CreateZswapOutput` expressions. ABI43
exposes an ordered `CircuitZswapPlan` built from ledger8 coin and recipient
carriers. Generated code evaluates operands left to right and retains one empty
aligned private output per native call, including Unit-return and nested calls.
The plan is separate from `midnight_zswap::local::State`, which remains wallet
state. Native output indices are provisional; they do not establish a valid
ledger offer, funding, or authoritative Merkle allocation.

Native contexts default to cursor0 and can set a start before their first output.
The cursor is bounded by u64; overflow rejects before any map or intent change.
This deliberately restricts the TypeScript bigint cursor domain. Ordinary observed
contexts lock allocation; ADR0188 permits outputs only against exact authoritative
offer indices. Both reject external cursor changes. Authoritative maps remain
unchanged. Input intents and cursor/output changes stay outside the
`RecordingFrame::call_local` audit's allowed effects.
Constructor/result transitions retain the intent log and lock; provisional output
indices travel with the log and reconstruct its native map after a transition.
ConstructorResult does not retain arbitrary call-context or offer allocation maps;
this guarantee concerns only the provisional indices in the intent log.

The independent TS fixture covers zero/nonzero cursors, duplicate commitments,
left/right recipients, branch selection, witness order/private state, exact private
outputs, gas/state/effects, and qualified Cell writes after native outputs. ADR0188
adds recorded APIs for the bounded Unit intent exports. Check
`check_compactc_target.py --native-zswap-intents` for source admission and strict
recording; `--proof` also runs the distinct-output funded transfer below.


### Native Kernel shielded effects (ADR0177)

Schema20 separates Kernel claim/mint Unit expressions from ordinary ledger slot
expressions. One Scheme operation handler validates the special empty Kernel path,
its exact ADT signature, and a bounded operation allowlist. Actions, direct returns
and nested expressions share it; pure Rust expression use still rejects effects.
ABI45 public methods accept upstream Commitment, Nullifier and HashOutput carriers
and u64 mint amounts. Shared canonical programs use the actual ledger8 effects
frame (six ops per claim, sixteen per mint), with no invented public ledger field.

Native queries preserve duplicate-claim set behavior, same-domain mint accumulation
and upstream arithmetic-overflow rejection. The independent TypeScript oracle
serializes its JavaScript Maps explicitly, retains reported aggregate gas separately,
and compares Rust gas to the sum of observed queries. TS batch reported aggregate
gas currently contains only the final query cost. Native Kernel operations emit no
synthetic private witness outputs; actual witness outputs keep source order.
ADR0184 adds bounded recording and a matched mint transaction below. General
matching-offer claim composition remains separate native Zswap integration work.

### Ordered stateful struct construction (ADR0179)

Stateful `StructLiteral` members reuse typed expression lowering, including direct
returns, nested structs, witness calls, metered Kernel.self and native Zswap
intents. Each member binds exactly once before the next member. The order is the
compiler's normalized declared-member order: independently generated TypeScript
also evaluates reversed named-field spelling in that order. Actual member types
must match the declaration; explicit widening nodes perform conversions.

The six-case TypeScript oracle compares values, private state and aligned outputs,
public state/effects, query gas and provisional intent allocation. Pure structs
retain their existing path. Schema20/ABI45 remain unchanged. The four stateful
probe exports remain native-only, without recording or proof claims. The unchanged
micro-dao now reaches `standard-library.compact` line207 and rejects the exact
Uint maximum `680564733841876926926749214863536422911` (2^129−1); support for that
arithmetic intermediate is a separate primitive-domain change. Run
`check_compactc_target.py --stateful-struct` for both boundaries.


### Checked native wide addition (ADR0181)

ABI46 adds `add_wide_unsigned` over a sealed operand view of existing BoundedUint
and WideUint carriers. Two checked limbs preserve integer carry without modular
Field arithmetic. The result is validated against its exact declared bound and
31-byte domain; existing wide casts and checked narrowing remain in force. Both
pure and stateful expression paths share arithmetic selection and validate all
operand/result maxima. Wide subtraction, multiplication and ordered comparison
remain explicitly unsupported. Schema20 is unchanged.

The independent TypeScript fixture covers fourteen cases: zero, bit128 carry,
2×u128MAX, mixed widths, checked Uint128/Uint129 narrowing rejections and selected
witness ordering/private outputs with zero VM gas and unchanged public state.
Runtime tests exercise arbitrary bounds, the248bit ceiling, and internal high-limb
overflow. The unchanged original micro-dao advances from standard-library207 to
its own line187 (`stateful expression requires stateful evaluation`); full native
source and recording support remain separate work. Older generated libraries need
the integration-owned ABI46 refresh before a workspace-wide Cargo gate.

### Typed stateful assertions and original micro-dao admission (ADR0183)

Nested assertion expressions lower their conditions with the stateful evaluator,
require Boolean, and yield Unit. Scheme preserves witness classification inside
the condition; Rust preserves short-circuit query/witness order and rejects false
with the existing assertion error. Schema20/ABI46 remain unchanged, and recorded
assertion admission is unchanged.

Twelve independent TypeScript cases cover successful value/Unit returns, each
short-circuit stop, failed Counter comparison, witness order/private state, and
zero-budget distinction between an assertion before any query and the first read
rejecting before the next witness. Successful Rust gas matches the captured query
sum; TypeScript's reported aggregate retains only its final query cost. Both are
retained in the fixture. Failed calls compare rejection and external witness traces;
Rust's Result API does not return their partial context or gas.

The complete unchanged `test-center/test-contracts/micro-dao.compact` now emits a
Rust crate that passes Cargo checking. The authoritative contract-info join covers
11 exports: four pure and seven proof-required. All seven have explicit recording
gaps in `parity_positive_test_center_micro_dao_sources.json`. This is native source
admission, not full contract behavioral, funded-transaction or proof coverage.
`check_compactc_target.py --stateful-assert` checks the generated original crate;
the full local gate also checks its exact source cohort and recording gaps.

### Recorded Kernel shielded effects (ADR0184)

ABI47 records the seven Kernel proof APIs through typed `RecordingFrame` methods
that reuse the canonical native programs and common query/gas recording path.
Schema20 is unchanged. The separate planner admits Unit Kernel effects in ordered
Sequence/Let/If forms with exact Bytes32, Uint64 and Boolean operands and typed
zero-argument witnesses. Public-slot composition, native Zswap intents, helper
composition, escaped bindings and malformed operand types remain rejected.

All twelve independent TS cases compare native and recorded effects, ordered
public programs, private witness order and gas. Replay preserves the original
query boundaries for gas comparison: combining queries changes upstream cache
costs even when final effects agree. Duplicate claims and overflow preserve the
upstream behavior. Seven nonempty API shapes prove and independently verify with
ZKIR2.1.0, including binding-tamper rejection. `selected(false)` has no ledger
queries and explicitly refuses observed preparation with `EmptyTranscript`; it is
covered by execution parity, not a call proof. Pinned ledger-v8 8.0.3 JavaScript
partitioning returns two absent sections, its builder retains the call, and ledger
validation rejects `CallHasEmptyTranscripts`. No empty-call acceptance is claimed.

The proof smoke also applies mint42 with the actual custom-token output offer and
Night-backed Dust fee funding using unchanged default strictness. This is offline
ledger application, with the actual output commitment/index checked. Arbitrary
fixture nullifier/spend/receive claims prove cryptographically but fail exact
upstream effects checks with unmatched offers; balancing is disabled only for
those negative cases to isolate claim validation. Matching claim offer composition,
network submission and finality are outside this evidence. Run the source gate
with `--kernel-shielded-effects --proof`; funded mint needs
`MIDNIGHT_LEDGER_TEST_STATIC_DIR` pointing to the pinned ledger's `ledger/static`.

### Exact offer-bound Zswap intent recording (ADR0188)

ABI48 adds typed frame input/output methods and an isolated Unit recording planner
for ordered Sequence/Let/If, exact coin/recipient operands, typed zero-argument
witnesses, qualified Cell writes, Kernel effects and bounded inlined Unit helpers.
Schema20 is unchanged. Every native intent retains its existing empty aligned
private transcript entry; no public VM operation or gas is invented. PublicTrace
seals initial/final intent plans so replacing mutable execution context cannot
bypass preparation checks.

Private allocation state distinguishes provisional native execution, locked
ordinary observations and authoritative offer-backed execution. Nonempty intent
plans require exactly the offer's contract-owned inputs and normalized ordered
outputs/actual indices. Reconciliation checks input index, stored commitment and
owner through the upstream tree, derives contract nullifiers upstream, and checks
final cursor/map against the initial binding. Extra wallet inputs, trailing change
outputs and transients are rejected. Empty intent plans preserve existing funded
Set/Cell and Kernel mint behavior. Generic unbound preparation rejects nonempty
plans, after the existing `EmptyTranscript` boundary.

The original `flow(true)` deliberately creates two identical output intents, then
writes the last provisional index. At start7, native/TS recording retains cursor9
and qualified index8. Pinned ledger-v8 8.0.3 rejects duplicate offer merge; a
normalized singleton actually allocates index7/cursor8. Neither indices nor
intents are rewritten. The original retains nine-case native execution parity; its three `flow` cases
also have recorded parity and exact bound/unbound/duplicate rejection evidence.
Compiler contract-info marks `produce`, `consume` and `witness_order` nonproof: they
remain native-only. Recording requires a structurally present public query, such
as the Cell write in `flow`; `read_coin` remains proof-required and recorded. Its false branch retains
`EmptyTranscript` and has no call-proof claim.

The separate `zswap_transfer_oracle` uses one distinct output with explicit Kernel
nullifier/spend claims and a qualified Cell write. Two TS cases match native and
recorded state/effects, ordered private/public transcripts, intents and query-sum
gas. A 4480-byte call proof verifies independently and rejects changed binding.
An explicit offline contract-owned genesis input funds output42 of the same token;
Night-backed Dust pays fees separately. Unchanged default strictness and ledger
application pass, including actual output index/commitment, stored coin and exact
spent-nullifier replay rejection. A wrong Kernel claim rejects the exact upstream
effects check; balancing is disabled only for that negative. No wallet/network or
finality claim is made. ADR0191 below covers composite-return `planned`; broader
offer composition remains separate work. Funded smoke requires
`MIDNIGHT_LEDGER_TEST_STATIC_DIR`.

### Context-derived token query recording (ADR0190)

The unchanged `test-center/test-contracts/micro-dao.compact` now records
`dao_voting_token`: a metered `Kernel.self` query followed by typed pure token
helper evaluation using the upstream ledger persistent commitment. A separate
bounded profile admits action-free, parameter-free Bytes32 results, exact
Bytes32/ContractAddress helper signatures, checked lexical bindings/projections,
and a pair-of-Bytes32 commitment with a Bytes32 opening. Helper calls are
acyclic, argument evaluation happens once in caller order, and callee scope is
isolated. Witnesses, public-slot reads/writes, Zswap intents and arbitrary helper
forms remain outside this profile. ABI47/schema20 and the runtime are unchanged.

The original source fixture retains all eleven exports and all six unrelated
witness declarations. Independent TypeScript captures at three addresses compare
the token, full VM transcript, gas, state/effects, private outputs and replay;
upstream `ContractAddress::custom_shielded_token_type` supplies an additional
canonical result check. `check_compactc_target.py --micro-dao-token --proof`
generates only the token circuit's pinned ZKIR keys from the complete original
source and proves/verifies/ledger-applies its nonempty query under the shared
unbalanced smoke policy. This is not a funding claim. At this delivery, the other six microDAO proof-required exports and all four
Coracle exports remained recording gaps; ADR0192 below closes vote_reveal.


### Counter-dependent membership helpers (ADR0192)

The unchanged original microDAO now records `vote_reveal`, preserving seven
ordered queries, five selected witnesses, both ballot outcomes and the exact
round-dependent reveal nullifier and commitment. Shared declaration-directed
call resolution separates pure helpers from stateful helpers before profile
admission; ambiguous declarations, recursion, leaked locals and mismatched types
are rejected. A bounded membership profile admits action-free Bytes32 helpers
whose three-Bytes32 persistent hash has exactly one declared Counter read cast
through Field to Bytes32. Counter provenance remains helper-local. Existing
Counter/witness profiles are unchanged. ABI48/schema20 and the runtime are unchanged.

Seventeen independent TypeScript scenarios compare native/recorded execution,
private outputs, selected callbacks, public transcript, state/effects and replay,
including absent/malformed paths, wrong phase/root/leaf/round, repeated and duplicate
reveals, and gas failures. `check_compactc_target.py --micro-dao-reveal --proof`
selectively generates the original circuit's keys. Both ballot outcomes prove,
verify and apply to the ledger; an applied-state duplicate is rejected. The prior
state is explicitly seeded with valid commitments, rather than obtained through a
funded commit lifecycle. Proofs use the shared unbalanced smoke policy. That delivery retained five microDAO proof-required recording gaps and four
Coracle gaps; ADR0193 below closes guess.

Gas has three distinct measurements. The TypeScript wrapper at
`runtime/src/circuit-context.ts:206` assigns the last query's cost. For the round-0
positive ballot this reports read time 170,000,000; the sum of its seven queries
and both Rust execution modes is 1,360,000,000. The fixture retains `reportedGas`,
`queryCostSum` and `replayGas` separately. Whole-program replay has its own compute
cost because query setup/cache behavior differs. This delivery preserves these
behaviors and does not claim equality between wrapper gas and aggregate gas, or
invent post-error contexts/costs.

### Unit-valued shielded composite results (ADR0191)

The original `stateful_struct_oracle.planned` and the separate
`composite_zswap_transfer_oracle.transfer` have recorded and observed-call APIs.
The composite profile reuses the existing typed values, scopes, member ordering
and declaration-directed helper dispatch. A small shared effect-leaf emitter
accepts already evaluated operands and calls existing frame methods; it owns no
second evaluator or call graph. The bounded profile admits exact input/output
intents and Kernel claims, requires a structurally present public query, and
rejects mint composition. Runtime ABI48 and IR schema20 are unchanged.

Before this slice, `planned` could execute only through the native API:

```rust
let execution = ledger_contract::planned(context, &witnesses, coin, recipient)?;
```

It now also returns its typed composite through a recorded, offer-bound call:

```rust
let contract = ledger_contract::Contract::from(witnesses);
let call = contract.recording().planned_call(
    offer_bound.observed(), private_state, coin, recipient,
)?;
let result = &call.recorded().execution.result;
// result.first, result.emitted: (), result.address, result.after
let prepared = offer_bound.prepare(call, verifier, communication_randomness)?;
```

Native execution may allocate provisional intents. Offer-backed preparation
retains ADR0188's exact input/output order, cursor and index-map validation.
Extra wallet inputs, trailing change and transients remain unsupported; generic
unbound preparation rejects the nonempty plan. The original output-only circuit
cannot fund value42: its exact offer fails the shielded-token -42 balance check,
and adding a wallet input fails reconciliation. A contract recipient fails the
exact missing-receive-claim check under default strictness, including at value0.

Four original and two transfer TypeScript cases compare native/recorded aligned
results, witnesses, private/public transcripts, effects, replay, gas and exact
intents/indices. The original `planned` user output with value0 is proved and
ledger-applied with separate Night-backed Dust. The separate composite transfer
uses an explicitly seeded contract-owned input and a distinct output42 of the
same token, with actual nullifier/spend claims and same-frame `Kernel.self`.
Both retain ordered Unit members and independently verified call proofs with
binding-tamper rejection. No synthetic public query, missing claim or funding
input is inserted. Run `check_compactc_target.py --composite-zswap --proof` for
these strict cases; its nonproof mode checks compiler admission. The actual local
parity gate joins proof applicability to contract-info before testing fixtures.

### Same-frame Unit Cell helpers (ADR0193)

The complete original `test-center/test-contracts/coracle.compact` records
`guess(Field): Unit`. Declaration-directed calls reuse the shared typed plan;
a separate bounded admission module permits actionful Field→Unit helpers with
Cell observations/writes, composite witnesses, assertions and audited pure
commitment predicates. Arguments evaluate once in caller order, helpers have
isolated scopes, calls are acyclic, and the selected branch retains its frame.
Both transient-commit operands are audited for hidden effects. The profile
excludes coins/intents, Kernel, Counter/Merkle operations and arbitrary helper
returns. ABI48/schema20 and runtime primitives remain unchanged.

Twenty-six fresh original TypeScript scenarios compare native/recorded witness
prefixes, full public programs, private outputs, state/effects, gas and replay.
Both root player-key queries execute; only the selected player helper's two
witnesses and Cell writes execute. Empty `last_guess` follows the original
source's direct `.value` access. The first-query-sized budget succeeds because
limits apply per query; wrapper last-query, aggregate query and replay costs
remain separate. Errors do not fabricate post-error contexts or costs.

`check_compactc_target.py --coracle-guess --proof` selectively generates the
original circuit's keys. Both colors are checked against explicit canonical
prior-state fixtures, under the shared unbalanced proof-smoke policy; funded
game setup/payout remains separate. Source applicability preserves all nine
exports, records one of four proof-required APIs, and retains `start`, `concede`
and `withdraw` recording gaps. Native final nontrivial Unit expressions use
`discard_expression` and `result: ()`, preserving all effects while removing an
unnecessary binding; existing literal Unit output remains unchanged.


### Original microDAO advancement and reset recording (ADR194)

The original `test-center/test-contracts/micro-dao.compact` now records `advance`.
The generated native API remains `contract.advance(context)`. Previously that
export had no recorded preparation path; it now also provides:

```rust
let recorded = contract.recording().advance(context)?;
let prepared = contract.recording()
    .advance_call(&observed, private_state)?
    .prepare(verifier, communication_randomness)?;
```

The shared typed planner keeps the secret witness, organizer/phase assertions,
Counter threshold, and selected reset helper in one recording frame. The bounded
phase-reset profile audits the complete internal `reset_state(Boolean)` body,
including both conditional branches, but admits only the original literal-false
call. Enum/Maybe/qualified-coin/Boolean Cell writes and Counter/Merkle/Set resets
reuse existing typed slots. No runtime ABI or schema change is needed.

The exact widened `no + 1` and checked Uint64 conversion are preserved:
`no == u64::MAX` returns `UnsignedOutOfRange` before `yes.lessThan`; a final reset
at `round == u64::MAX` returns upstream `ArithmeticOverflow`. The pinned TypeScript
prefixes contain four and thirteen successful queries respectively. Failed Rust
calls consume their context, so tests assert the exact failure and witness order
without claiming a returned partial state. Successful native/recorded/replay cases
match the full state and transcript; final resets leave pot value, index and flag
unchanged. TypeScript's reported gas is still its last query cost; Rust cumulative
gas matches the captured sum.

Twenty original-source scenarios cover authorization, all phases, populated and
empty resets, threshold limits, overflow, witness errors and gas refusal. Rust's
`FixedBytes<32>` excludes the malformed dynamic TypeScript witness value; the
corresponding Rust boundary uses an explicit witness error. Three original
branch proofs (commit, reveal, final reset) verify independently, reject changed
bindings and apply under default ledger strictness with separate Night-backed
Dust fees. Prior DAO states are explicit fixtures; this does not establish a
funded deposit/vote lifecycle or proof coverage for `reset_state(true)`/`cash_out`.

Run `check_compactc_target.py --micro-dao-advance` for the source capability guard,
and optionally `--proof` for selective key generation and proof smoke. The proof
smoke also accepts `--micro-dao-advance <proof-output>` to reuse pinned keys.
`MIDNIGHT_LEDGER_TEST_STATIC_DIR` must point to the upstream static fixtures for
Dust fee funding. At the ADR194 delivery, the source cross-tab retained four original proof gaps:
`vote_commit`, `set_topic`, `buy_in`, and `cash_out`. Later sections describe subsequent admission.

### Read-only Field snapshots (ADR0201)

A Field observation can be returned inside a typed struct, including selected
branches and scalar or composite helpers, using the same recording frame:

```rust
let call = contract.recording.snapshot_call(&observed, private_state)?;
let snapshot: &types::Snapshot = &call.recorded().execution.result;
```

Use the generated call's result accessors for application integration; preparation
uses the normal typed observed-call API. The bounded read-only observation policy
requires a struct result, no actions, declared root `Cell<Field>` reads and an
acyclic helper graph. Every binding, argument, branch and helper is audited;
witnesses, writes, Kernel queries, Zswap intents and other collection operations
remain outside this policy. Argument evaluation and lexical scopes stay in the
shared typed planner. Runtime ABI48 and IR schema20 are unchanged.

`CompositeDomain` explicitly distinguishes value composites, intent composites,
shielded receive (intent capability without composite helper routing), and Field
observations. Phase-reset admission remains independent. The seven-API fixture
has 23 independent TypeScript/native/recorded/replay cases. Eight nonempty paths
prove, verify and apply under default ledger strictness with separate Night-backed
Dust fee funding from the pinned upstream test fixture. Prior contract states are
explicit fixtures; deployment lifecycle and wallet submission are separate gates. `optional(false)` retains zero operations and exact
`EmptyTranscript` preparation refusal. Query sums, TypeScript wrapper last-query
cost and whole-program replay gas remain distinct measurements.

Run `check_compactc_target.py --field-observation` for strict source recording
admission; add `--proof` for selective proof/application checks. Existing keys can
be reused with `compact-rust-proof-smoke --field-observation <proof-output>`.
Set `MIDNIGHT_LEDGER_TEST_STATIC_DIR` to the pinned upstream static fixtures for
the Night-backed Dust funding helper.

### Qualified shielded send (ADR0203)

The unchanged `sendShielded` helper now records typed `ShieldedSendResult`
returns for self, user and contract recipients. Its single frame preserves the
qualified input and nullifier claim before checked u128 subtraction, then the
sent output and optional branch-local change output. The transient nonce chain
uses the pinned ledger cryptography through the existing runtime primitives.
Pure helpers are audited by declaration, and every stateful binding, argument
and branch is checked before admission; hidden collection or witness queries
refuse recording. IR schema20 and runtime ABI48 remain unchanged.

`check_compactc_target.py --shielded-send` checks all four exported recording
APIs; `--proof` runs the pinned call and ledger proof. The checked-in corrected
TypeScript oracle covers full, partial, underflow and wide u128 values, with
native/recorded/replay and independent nonce/commitment comparisons. A
contract-owned full send and a two-output partial send in source/normalized
order pass default strictness and ledger application with separate Night-backed
Dust. A reversed two-output order is rejected by exact offer binding. Other
partial orders require an explicit indexed offer policy; this profile does not
admit immediate sends, wallet input funding or the remaining original-source
operations.


### Terminal lexical return recording (ADR202)

The existing terminal-return fixture now records all five exports. Previously
`two`, `three`, `nested`, and `observed` had native methods but no recorded call
preparation; `echo` was the recorded control. The witnessed handle now supports:

```rust
let recorded = contract.recording().two(context)?;
let after = recorded.execution.result;
let call = contract.recording().observed_call(&observed, private_state, false)?;
let prepared = call.prepare(verifier, communication_randomness)?;
```

A structural adapter attaches extracted return expressions only to the terminal
Sequence/Let continuation. The shared typed planner evaluates that ReturnPlan;
it retains earlier sibling, branch, and caller/helper scope isolation. Existing
explicit ReturnPlans retain their own scopes. The bounded new domain audits one
Field Cell, Field/Boolean values, Field witnesses, assertions, and acyclic typed
Field-returning actionful helpers. It requires read and write effects, audits
unused bindings and both branches, and excludes other ADTs, Kernel and Zswap
operations. No source identity, exact operation count, runtime, ABI48 or schema20
change is needed. Equivalent terminal nesting also works in already admitted
profiles without changing their value/effect domains.

Recording the witnessed export exposed a generated borrowed facade collision:
`echo(context, echo)` resolved to its argument. Calls shadowed by an emitted
parameter identifier or the fixed `context` local now use
`crate::ledger_contract::recorded::<name>(...)`; normalization and raw Rust
identifiers are covered too. Other facade output remains unchanged.

All 13 original TypeScript/native scenarios also check recorded results, complete
state/effects, gas, witness order, private output and successful replay. Rejection
after the witness and zero gas before it remain failures; failed Rust calls
consume their context and do not expose partial state. Four original-key proofs
(`two`, `three`, `nested`, `observed`) verify the returned Field, reject changed
bindings, and apply under default ledger strictness using separate Night-backed
Dust. Prior Cell state is explicitly seeded by native calls, not a proved prior
transaction history. The original `echo` behavior and `let_return_oracle` remain
controls; the latter's generated read now uses the shared planner.

Run `check_compactc_target.py --terminal-lexical-return` for the compiler
contract-info cross-tab and generated Cargo check; add `--proof` for four strict
proof cases. Reuse keys with `compact-rust-proof-smoke --terminal-lexical-return
<proof-output>` and set `MIDNIGHT_LEDGER_TEST_STATIC_DIR` to the upstream static
fixtures for Dust funding.

### Opt-in canonical persistent output allocation (ADR205)

Upstream `Offer::new` sorts outputs. A circuit can emit sent/change in the opposite
order, so the default exact-order policy still rejects that execution. Applications
can explicitly select canonical indices before recording:

```rust
use midnight_compact_runtime::transaction::{
    OfferBackedObservedState, OfferBindingOptions, PersistentOutputAllocation,
};
let options = OfferBindingOptions::default()
    .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices);
let bound = OfferBackedObservedState::with_options(observed, &ledger, offer, options)?;
let call = contract.recording.distribute_call(bound.observed(), private_state, /* inputs */)?;
let prepared = bound.prepare(call, verifier, randomness)?;
```

`with_wallet_funding` on the options composes the existing explicit upstream wallet
input selection; `new` and `OfferBackedObservedState::with_wallet_funding` retain
exact-order behavior. Output ownership is not inferred from the funding selection.
Without an explicit transient selection, the canonical policy accepts normalized
complete offers with persistent outputs only. It never normalizes after execution,
deduplicates outputs, or admits wallet change. ADR204 adds a separate typed
transient selection below.

The immutable upstream commitment map is installed before execution. Each source
intent looks up its actual allocated index and checks typed recipient/owner
metadata. Source intent, private Unit output, witness and query order remain
unchanged. Final reconciliation requires an exact output bijection plus retained
input/owner/nullifier checks, unchanged allocation/context maps and sealed intent
snapshots. Additional owner checks reject malformed metadata early; upstream
proof and default-strict ledger validation remain authoritative.

`CircuitZswapOutput.provisional_index` holds the actual index in bound mode.
`next_index()` is logical progress, start plus the emitted count: source indices
can be `[8, 7]` while progress advances `7 → 8 → 9`. Qualified Cell/Set queries use
the actual upstream map; no result, state or transcript is patched afterward.
Raw TypeScript/provisional execution still allocates source-order indices, so its
qualified indices can differ from canonical execution for the same coin sequence.
The independent capture checks that provisional behavior; canonical replay and
strict ledger application check the different bound context.

Canonical empty plans always return `CanonicalEmptyPlan`, including empty offers
and no-query calls. Default no-query preparation retains `EmptyTranscript`; the
existing funded policy retains `WalletFundingEmptyPlan`. Legacy default empty-plan
offer calls remain supported. The policy is additive and opt-in: ABI48/schema20
and generated method signatures remain unchanged.

The two-output fixture spends an explicitly seeded contract coin42 into user17
and contract change25, then stores the qualified change. Two original-key proof
cases exercise both normalized sort orders, verify independently, reject changed
bindings, pay separate Night-backed Dust, pass unchanged default strictness and
ledger application, check both leaf owners/indices and the stored qualified index,
and reject spent-nullifier replay. The seed is an offline genesis prerequisite,
not proof of an earlier transaction or network finality. Run
`check_compactc_target.py --canonical-output-order` (optionally `--proof`), or reuse
keys with `compact-rust-proof-smoke --canonical-output-order <proof-output>`.


### Receive then full immediate shielded send (ADR204)

The original `receiveShielded(coin)` followed by
`sendImmediateShielded(coin, recipient, coin.value)` now has a recorded API through
the shared typed planner. Its structural policy requires the same coin, forwarded
recipient, full value, and the standard singleton-index-zero bridge. Every helper,
unused binding and branch is audited. Partial immediate sends, merges, extra
prefix effects, alternative coin/value projections and helper cycles remain
outside this policy. Existing qualified historical sends retain their own policy.

Before this change the generated crate exposed native execution for this wrapper;
now it also exposes `ledger_contract::recorded::receive_then_send` and the typed
`contract.recording.receive_then_send_call` facade.
The runtime operations emitted by the planner are unchanged. Applications opt into
transient reconciliation with actual upstream proof-bearing values:

```rust
let options = OfferBindingOptions::default()
    .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
    .with_wallet_funding(wallet_inputs)
    .with_transient_coins(ContractTransientCoins::from_transients(vec![transient])?);
let bound = OfferBackedObservedState::with_options(observed, &ledger, offer, options)?;
```

The private plan retains the complete ordered input/output event projection.
A selected transient must be output to the executing contract before it is spent,
match the exact coin and nullifier, and use the singleton witness index zero.
It is distinct from an already-qualified historical input even if that input's
real ledger index is zero. Full upstream input/output proofs and metadata must
match the selected offer carrier. Guaranteed segments, exact disjoint ordinary,
wallet and transient input coverage, output ownership and the sealed allocation
map are checked. The upstream prover and ledger remain responsible for proof
validity. Default and wallet-only policies still reject transients.

Seven independently captured TypeScript cases compare native and recorded values,
all public operations, private outputs, effects, gas and replay. Gas evidence keeps
the summed query cost, TypeScript wrapper's last-query cost and whole-program
replay cost separate. Provisional source execution assigns received/final indices
`[2, 3]`; the normalized offer assigns `[3, 2]`. Bound execution installs that
immutable map before evaluation; independent replay uses that map too. No
transcript or state is patched after execution.

The original-source strict proof case starts above ledger frontier one, spends a
real wallet input, carries a real contract transient and creates an ordinary user
output. Its 4,480-byte contract proof verifies; all Zswap proofs, separate
Night-backed Dust, default-strict validation and ledger application pass. Both
nullifiers reject replay. A malformed singleton-index-one transient fails proving
with a public-transcript input mismatch, under the same keys and funding setup.
The seed is an explicit offline prerequisite, not a proved prior deposit history.

Run `check_compactc_target.py --transient-receive-send` for the source/report/Cargo
guard; add `--proof` for key generation and strict execution. Retained keys can be
used by `compact-rust-proof-smoke --transient-receive-send <proof-output>` with
`MIDNIGHT_LEDGER_TEST_STATIC_DIR` set to the upstream static fixture directory.
ABI48/schema20 remain unchanged: policy options are additive, ordered events are
private, and generated methods use existing runtime entry points. This closes
this full immediate-send wrapper, not merge support or all original application
recording gaps.


### Original Coracle withdraw (ADR206)

The original `withdraw()` now emits recorded and observed APIs using the shared
typed planner and the bounded `ShieldedPayout` domain. Before this change its
native API was available but complete recording was refused. The generated
recorded API calls `frame.own_coin_public_key()?`, preserves the original
declared/native witness order, reads only the audited enum/Bytes32/qualified
coin Cells, and reuses the existing full-value shielded send leaves. It does
not admit writes, reset helpers, Counters, Sets, or Merkle mutations. ABI49 is
required by the new frame call; schema20 is unchanged.

```rust,ignore
let options = OfferBindingOptions::default()
    .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
    .with_offer_placement(OfferPlacement::Fallible(NonZeroU16::new(1).unwrap()));
let bound = OfferBackedObservedState::with_options(
    observed.with_coin_public_key(recipient), &ledger, segment_one_offer, options)?;
let recorded = ledger_contract::recorded::withdraw(
    bound.observed().circuit_context(private), &witnesses)?;
let call = RecordedCall::new(bound.observed(), recorded, "withdraw", ());
let prepared = bound.prepare(call, verifier, Fr::from(0))?;
```

All 27 original TypeScript/native scenarios cover selected red/blue payout,
dual authorization, phase/authentication rejection, missing execution key and
witness/gas failure. Successful recorded results compare the complete state,
effects, query replay, gas, raw intent order and eight private outputs.
Default small Rust test threads overflow for the complete debug recording; the
behavior harness uses an explicit 8 MiB worker and the proof runner its existing
64 MiB worker. This is a measured host-stack requirement, not a circuit limit.

The upstream partitioner places original withdraw wholly in its fallible
transcript. Guaranteed offer placement therefore rejects its nullifier claims
with `NullifiersNEClaimedNullifiers`, even when the call proof verifies. The
explicit whole-fallible policy retains source placement and upstream proof tags
instead of moving claims or changing VM operations. Dust fee funding is separate.
Proof fixtures seed prior game/coin state offline; they do not prove a complete
funded game lifecycle. `start` and `concede` retain their explicit recording gaps.

The independently constructed segment-0 offer fails effect matching; moving an
already-proven segment-1 offer into the guaranteed slot instead fails earlier
with `Zswap(InvalidProof)`. These are distinct retained negative cases. Physical
intent keys are nonzero: fee balancing adds a separate Dust-only intent at its
own key, while logical application phase0 denotes guaranteed execution.

Both original red and blue withdrawals pass default strict proof verification
and ledger application with two real inputs/outputs and opposite normalized
output orders. All original Cells remain unchanged and the third deposit stays
unspent. Reapplication rejects an already-present nullifier. A changed public
phase fails with `Transcript(Execution(ReadMismatch))`: fallible Zswap/contract
state rolls back while guaranteed Dust and replay bookkeeping persist.

### Qualified and immediate shielded merges (ADR207)

`shielded_merge_oracle.compact` retains unchanged `mergeCoin` and
`receiveShielded` + `mergeCoinImmediate` standard-library calls. A distinct
`ShieldedMerge` policy describes historical-pair versus received-right input
provenance; shared typed planning owns all evaluation, helper scope and
upstream arithmetic syntax. The widened operand bound is `2 * u128::MAX`,
the addition result bound is `2^129 - 1`, and checked narrowing retains
overflow rejection after the color assertion. The singleton qualification
audit is shared with immediate send. No runtime/ABI change is introduced.

Twenty independent TS/native/recorded/replay cases and two separately
Dust-funded, default-strict proofs cover this slice. Initial coins are
explicitly seeded offline. Whole fallible transient funding and original
application composition remain separate delivery work.

### Original Coracle concede (ADR209)

The unchanged original `concede` export now has a recorded/observed Rust API.
The actionful payout profile is separate from ADR206's readonly payout profile:
every selected path must execute exactly one audited helper that writes the
winner enum before reading and sending the selected deposit. A second helper,
an unwritten branch, unsupported action, hidden effect or malformed witness
refuses recording. The existing typed Plan keeps the two branches and their
local scopes in source order; no new IR schema or runtime API is required.

Twenty-seven independent corrected-TypeScript/native cases cover four successful
and 23 rejecting calls. Successful recorded/replay rows compare exact state,
effects, return value, VM program and query order, private outputs, gas and
offer intents. Both original red and blue calls independently prove and apply
with a single contract-owned input and user output in the guaranteed segment,
default strict offer binding and separate NIGHT-backed Dust. Binding mutation
and spent-nullifier replay reject. Fixtures start from an explicitly seeded
prior game; `start` remains the original Coracle recording gap.

### Original microDAO set_topic and explicit fallible funding (ADR211)

The unchanged original `set_topic` export now has a recorded/observed Rust API.
A separate `GuardedShieldedDeposit` policy audits the received seed, selected
historical merge, the shared singleton bridge, typed costs and optional Cells.
The existing Plan owns all values, lexical scopes, helper calls and branch-local
frame evaluation. Authority/phase/cost checks and the witness prefix retain
source order. Unsupported hidden effects, malformed slots/types, changed seed
provenance, helper cycles and receive-after-merge remain refused.

`ContractTransientCoins::from_transients_for_placement` explicitly retains
upstream pairs constructed for the selected placement. Whole-fallible binding
now composes exact selected wallet Inputs and complete Transients with historical
contract inputs. Both input owner classes require the exact `[1,s]` vector;
each output requires `[s]`, including both transient proof halves. Full proof
identity, disjoint input/output coverage, causal source events, observed state
and canonical allocation stay sealed. Defaults remain guaranteed; no proof is
retargeted, and mixed guaranteed/fallible calls remain outside this policy.
The consumer API is additive; generated calls retain ABI49/schema20.

Sixteen corrected-TS/native/recorded cases preserve exact transcripts, private
outputs, effects, query gas and failure witness prefixes. Raw TS replay uses
its provisional commitment table and reports wrapper-last-query, query-sum and
whole-program replay gas separately. Canonical offer-backed recording is tested
separately by two original-key, independently Dust-funded proofs with default
ledger strictness. Empty pot is wholly guaranteed (44 VM operations); occupied
pot is wholly fallible (71 operations). Both apply, store the authoritative
qualified pot index and reject spent-nullifier replay. The occupied proof also
checks precise concurrent-state `ReadMismatch`: fallible coin and contract
updates roll back while guaranteed Dust/replay effects persist. Coins and prior
contract state are explicitly seeded offline, not a proved DAO lifecycle.



### Original microDAO buy_in recording (ADR0215)

The unchanged source now generates `ledger_contract::recorded::buy_in(context,
coin, amount)` and `recorded::Contract.buy_in_call(observed, private, coin,
amount)`. The returned coin remains typed `ShieldedCoinInfo`; no witness provider
or new runtime primitive is needed. Schema20/ABI49 are unchanged.

A separate funded-mint policy composes the existing received-coin provenance
checks, optional historical/transient merge, canonical qualified store, sealed
execution key and upstream shielded mint/output/claim operations. Price checks
retain the declared widened unsigned operands and checked multiplication; the
maximum product is `(2^64 - 1)^2`, not wrapping or Field arithmetic. Root coin
and amount shadowing, missing per-path stores, malformed bounds/types/slots,
hidden effects and invalid helper scope are refused. Small specialized emitter
methods keep unrelated recursive Plan evaluation within its existing debug
stack budget. The guarded-deposit policy retains its previous expression domain.

Eighteen captured TypeScript cases compare native and recorded results, state,
mint effects, private outputs, exact public programs, execution/replay gas and
failure ordering. Both empty and occupied original paths are wholly guaranteed
(54/81 VM operations). The strict proof selector `--micro-dao-buy-in` checks
actual wallet funds, an optional historical pot at a nonzero Merkle index,
complete received Transient, canonical pot and user mint outputs, original call
proofs and changed-binding rejection, separate Dust, default-strict ledger
application, wallet recovery of the minted instance token and nullifier replay
refusal. Prior coins and contract state are seeded offline; this does not claim a
funded full DAO lifecycle or live wallet submission. Zero-price and maximum
arithmetic captures are execution controls, not corresponding monetary proofs.

All seven original microDAO proof-required exports now have recorded APIs.

### Original microDAO cash-out and closed reset (ADR212)

The unchanged original `cash_out` now exposes `ledger_contract::recorded::cash_out(context)`
and its observed-call facade. A distinct structural profile reuses the typed Plan,
same-frame shielded payout, and the complete Boolean-to-Unit `reset_state` helper;
only a literal-true call is admitted here. ADR194's literal-false advance profile
and the readonly/actionful payout profiles remain separate. No runtime API, ABI49,
or schema20 change is required.

The 21 pinned cases compare TypeScript, native Rust and recorded Rust; successful
transcripts replay through the upstream VM. They preserve short-circuit guards,
private output order, optional beneficiary checks, saved sent result, and reset
order. Empty participant sets and `pot_has_coin=false` still succeed when the
actual source guards pass; no membership or flag guard is invented.

`check_compactc_target.py --micro-dao-cash-out --proof` runs the focused proof
harness (cash_out k=15, 18,864 rows). It spends an explicitly seeded historical
contract pot of 99 at index1, creates the full-value user output at index2, uses
canonical allocation and explicit persistent fallible offer segment1, and funds
fees with separate Dust. The cryptographic proof and default-strict ledger
application pass. Reapplying against a changed round at u64MAX produces exact
`Transcript(Execution(ArithmeticOverflow))` after the earlier reset operations;
the contract and Zswap state roll back, while guaranteed Dust/replay effects remain.
The original prestate applies successfully and rejects the spent nullifier on replay.
This validates a selected seeded final DAO state, not a proved deposit/vote lifecycle.


### Original microDAO vote_commit recording (ADR214)

A separate voting-commit policy records the ordered phase/token/value guards,
Counter-derived membership key, received voting coin and full-value immediate
send, private ballot witness, Merkle commitment insertion, participant Set write
and private-state advance. The literal-one send is admitted only after the exact
coin-value assertion. Shared singleton qualification and typed context/hash
helpers preserve lexical scope; unrelated profiles retain their own guards.
Eighteen corrected-TS/native/recorded/replay cases cover the actual original
source. `--micro-dao-vote-commit --proof` proves yes at round0 and no at Uint64
maximum, with actual wallet/transient/output proofs, separate Dust, default-strict
apply, exact fallible ReadMismatch rollback, wrong-placement InvalidProof and
spent-nullifier replay refusal. Voting coins and DAO state are explicitly seeded;
the zero-key output follows source behavior without a separate unspendability
claim. Schema20/ABI49 and runtime are unchanged.

### Direct pinned-oracle behavior (ADR216)

`oracle_direct_behavior_review.json` records a reviewed subset of the 37-source
oracle cohort: 43 literal-coercion pure exports with 91 independently captured
TypeScript cases, plus `assert_parity.ping` and both Boolean branches of
`ternary_cond.walkerVectorElement`. The literal test directly invokes every
export, checks the exact export/case sets, and compares each typed result to
the captured value. It includes widened Uint inputs, modular Field arithmetic,
huge/Field-only equality branches, ordered aggregate hash inputs and curve values.

The two stateful APIs check native/recorded result and state, the complete ordered
public VM program, summed query gas, empty private outputs and upstream replay.
They previously appeared only in serialized operations metadata. The constructor
is not treated as execution of arbitrary pure exports. Cryptographic proofs and
ledger application are explicitly outside this test-only delivery; no generated
code, ABI49, schema20, runtime or Compact source changed.

The larger behavior audit remains separate and incomplete. Function-pointer
dispatch (for example the six mixed-width comparisons) counts when actual
assertions consume its output; invocation regexes alone do not establish coverage.

### Direct pure ternary behavior (ADR218)

`oracle_ternary_behavior_review.json` maps all 25 compiler-pure exports of the
unchanged ternary oracle to 83 independent TypeScript cases and explicit typed
Rust calls. The cases include both conditional arms, all four nested choices,
distinct aggregate inputs, wide integer boundaries and actual curve/hash values.
The eight failures assert source assertion errors or typed unsigned underflow;
TypeScript's subtraction-assertion message and Rust's arithmetic error remain
explicitly distinct. Lazy unselected subtraction paths must succeed.

The matrix pins source, capture, script and Rust assertion hashes; its checker
validates case identity and provenance, not semantic coverage inferred from text.
This test-only tranche adds no proof or ledger claim and changes no compiler,
runtime, generated crate, ABI49 or schema20. The broader 37-source review remains
partial. Six call-argument pure exports and three AssetRegistry pure assertion
exports still have transitive evidence awaiting separate direct-boundary review.
