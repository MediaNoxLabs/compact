# ContractLab — Compact Rust testkit

`midnight-compact-testkit` provides typed, in-memory scenarios for generated Compact contracts using the real ledger VM. It supports native calls and local recorded replay as distinct operations.

## Start here

- [Implement a witness and run a stateful contract test](../doc/rust/guides/testing-witnesses.md)
- [Exercise a DID constructor and recorded lifecycle](../doc/rust/guides/did.md)
- [Choose matching compiler, runtime and target versions](../doc/rust/guides/versions-and-targets.md)
- [Read the evidence behind the examples](../doc/rust/guides/snippet-matrix.md)

Use the complete tutorial consumer sources and their SDK preparation instructions. Keep handwritten witnesses and tests outside the generated crate. The testkit is a source package in this repository; its Cargo manifest has `publish = false`.

## Optional local proof setup

Enable the testkit's optional `proof` feature for [ProofLab](../doc/rust/guides/local-proof-tests.md). `ProofLab<S, P>` owns the official `Resolver` and `ParamsProverProvider`; `provider(rng)` returns the official `LocalProvingProvider`. Import `ProvingProvider` to use `check`, consuming `prove`, and `split` directly. `verifier(key, limit)` distinguishes unavailable material, resolver I/O and decoding errors; it applies encoded admission after acquisition. There is no new provider protocol, wallet, transport or custody model.

At `17433b40a14da83c5c500d92420149c28f755b88`, retained proof-feature qualification contains 25 unit/integration tests and two doc tests; default features contain 21 tests and one doc test. These are separate commands. Four new tests cover official-provider checks/refusals and resolver/decoder behavior. The rustdoc proof call is compile-checked only; no new successful proof, verification, ledger application or network receipt is claimed. See [the exact retained evidence](../doc/rust/evidence/0.3.0/index.md#local-proof-fixture-adr0360).

## Components

| Type | Purpose |
|---|---|
| `ContractLab` | Run generated calls against a local contract state and manage scenario checkpoints |
| `Environment` | Supply the local execution environment |
| `WitnessScript` | Provide a finite sequence of expected witness inputs and responses |
| `CallReport` / `ReplayReport` | Keep native execution and verified local replay results distinct |
| `Snapshot` / `SnapshotMetadata` / `ArtifactIdentity` | Retain scenario data with explicit format and artifact identity |
| `LabError` | Report testkit boundary and execution failures |
| `ProofLab` / `ProofError` (`proof` feature) | Reuse official local proving providers and explicit verifier resolution/decode errors |

The crate re-exports its matching runtime as `runtime`. Current source package versions are testkit **0.1.0** and runtime **0.2.0**; the runtime ABI is **50**. These numbers describe different compatibility boundaries. See [the version guide](../doc/rust/guides/versions-and-targets.md).

## What a local test establishes

A passing native call checks the result, state and errors asserted by that test. Recorded replay checks a sealed program against the local VM. Transaction preparation, cryptographic proofs and network acceptance require separate evidence; see [proof and ledger integration](../doc/rust/guides/proof-integration.md).

Application adapters and witnesses are trusted Rust code. A private checkpoint requires owned data or immutable persistence: cloning does not undo external witness side effects or shared interior mutation. Replay gas is measured separately and does not enforce an aggregate execution budget. The [crate documentation](src/lib.rs) states the remaining execution boundaries.

## Maintained examples

The [test directory](tests) contains the maintained testkit scenarios. The [source modules](src) own the API; the task guides provide complete consumer recipes with explicit historical execution scope. Do not infer support for every generated circuit from one successful example—consult each generated capability report.
