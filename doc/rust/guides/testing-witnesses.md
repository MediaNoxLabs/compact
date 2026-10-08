# Test a witnessed contract with ContractLab

**Locally validated external tutorial.** This complete source example derives from the maintained `witnessed_cell_matches_frozen_typescript_and_commits_owned_script` scenario in `testkit-rs/tests/generated_scenarios.rs`. The external package was freshly generated and its single test passed with Rust1.99.0 on aarch64-apple-darwin, default features, and a qualified lock based on source revision `5efa91c2`. Strict Clippy and formatting results are recorded in the validation receipt. This example compares native and recorded execution; its underlying maintained scenario owns separate TS evidence.

## Files

[examples/witnessed-cell/](examples/witnessed-cell/) contains:

- `witnesses_oracle.compact`: byte-identical maintained source, including its unused witness declarations.
- `Cargo.toml`: new external manifest using generated `compact-contract-witnesses-oracle` and shared staged testkit.
- `tests/witnessed.rs`: complete adapted Rust test, with no undefined context or witness variables.

The exercised Compact export is:

```compact
witness fetch_field(): Field;
export ledger v: Field;
export circuit pull(): [] {
  const x = fetch_field();
  v.write(disclose(x));
}
```

This fence is an explanatory excerpt. Compile the complete checked-in example file. The generated circuit requires `fetch_field`; unused source witness declarations are not extra methods that this call needs.

## Prepare and run

Prerequisites: matching Rust-capable compiler/Scheme pair, Rust toolchain, Python3, source tree at the reviewed revision, sufficient disk, qualified dependency cache/lock. Work in a copy of the example outside the original checkout.

```sh
python3 prepare-sdk.py --source /absolute/path/to/reviewed/compact --examples /absolute/path/to/draft/examples
cd /absolute/path/to/draft/examples/witnessed-cell
compactc --target rust --skip-zk --rust-runtime-root /absolute/path/to/draft/examples/sdk witnesses_oracle.compact generated
cp /absolute/path/to/reviewed/compact/Cargo.lock Cargo.lock
cargo +1.99.0 metadata --offline --format-version 1 > metadata.json
cargo +1.99.0 test --locked --test witnessed
```

Seeding the reviewed source lock keeps dependency versions consistent; Cargo updates the local application entries and prunes unused packages. Before testing, compare every external registry package name/version/source/checksum against that source lock. A fresh `generate-lockfile` selected newer cached dependencies in validation and was rejected before compilation. Use `--offline` only when the reviewed graph is already cached. A copied, clean application directory may reuse an owned warm Cargo target; it does not require a cold dependency rebuild.

After generation, verify Cargo metadata resolves **one** `midnight-compact-runtime` and that all local package manifests are under the external tutorial directory. Record the final source, generated output, compiler/runtime identity, Cargo.lock, toolchain, features and results. The validation receipt records execution of preparation, generation, lock qualification, metadata checks and focused tests. Paths in these shell examples must be replaced for your checkout.

## Understand the test

The witness implementation has the generated signature:

```rust
fn fetch_field(
    &self,
    context: WitnessContext<'_, Private, cell::LedgerView<'_>>,
) -> Result<(Private, Field), CompactError>
```

This is a signature excerpt; the complete implementation is in `tests/witnessed.rs`. It reads `context.ledger.v()?`, clones owned private state, consumes one scripted answer and returns the updated state/value. `WitnessScript<(), Field>` makes zero arguments and a Field reply explicit.

The test:

1. Constructs the actual generated initial state with one scripted answer,42.
2. Forks one checkpoint for native execution and runs the other through recorded VM replay.
3. Compares state/effects/query-summed execution gas and checks the declared Field cell is42.
4. Checks one witness answer was consumed and the owned call count/journal advanced.
5. Calls again with an exhausted script, inspects the known redacted execution error deliberately, and checks the committed state/private script stayed unchanged.
6. Restores the initial checkpoint and checks the owned script is available again.

This adapted example compares native and recorded behavior. It does not embed an independent TS oracle. The maintained source scenario separately checks the pinned TS transcript/gas capture. Do not label the new adaptation “TS parity passed” without an actual independent comparison.

## Boundaries that matter

ContractLab accepts trusted Rust adapters around generated calls. Its checks are not a sandbox. Rollback covers the lab's owned ledger/private checkpoint, not external witness journals, files, network effects or shared interior mutation. Choose independently cloned owned data or immutable persistence for private state.

`execution_gas()` sums generated queries, including metered witness reads. Replay runs a sealed public program as one VM query; query grouping and witness-only work can change costs. Neither equality nor a universal ordering between execution and replay gas is promised. The example compares native and recorded **execution** gas for the same generated operation.

`LabError` redacts payloads in Display/Debug. `execution_error()` deliberately reveals a payload; a message does not authenticate whether it came from a script or a contract assertion. Keep private values out of shared logs.

Snapshots are in-memory checkpoints. Artifact hashes detect caller-declared mismatches; they do not authenticate source. Successful replay is not a proof, transaction funding or network acceptance.

## Add proof-provider setup when needed

Keep native and recorded scenarios in ContractLab. For tests that already have circuit artifacts and parameters, the optional [ProofLab fixture](local-proof-tests.md) constructs the official local provider. No production adapter stack is required.
