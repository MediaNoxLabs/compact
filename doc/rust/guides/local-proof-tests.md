# Local proof tests with official providers

Use `ContractLab` for generated contract execution and recorded VM replay. Enable the testkit's optional `proof` feature when the test also needs reusable proof-provider setup. `ProofLab` composes the existing Midnight interfaces; it does not introduce another provider protocol or domain model.

## Choose the operation

| Test task | API | Result establishes |
|---|---|---|
| Run a generated native call | `ContractLab::native` | The assertions made about outputs, state and errors for those inputs |
| Record and replay a generated call | `ContractLab::recorded` | Local VM replay of the sealed program |
| Resolve a verifier | `ProofLab::verifier(key, limit)` | Material was found and verifier bytes decoded under the selected input policy |
| Check a proof preimage | Official `ProvingProvider::check` | The supplied preimage satisfies the resolved circuit check |
| Generate a proof | Official `ProvingProvider::prove` | Proof generation succeeded; verification and ledger application are separate operations |

The new fixture is for local tests. Generated circuits, ledger objects, `ProofPreimage`, keys, fields and proofs keep their existing types. There is no networking, wallet, retry or custody layer to configure in the testkit.

## Reusable setup

`ProofLab<S, P>` owns `S: Resolver` and `P: ParamsProverProvider`. Its `provider(rng)` method returns the official `midnight_zkir::LocalProvingProvider` borrowing those providers and owning the RNG. Import the official `ProvingProvider` trait to call `check`, `prove` and `split`.

The following is an API excerpt, not a standalone runnable test. The complete generic version in `testkit-rs/src/proof.rs` is checked by rustdoc; its successful prove path is not executed by that documentation test.

```rust
let proofs = ProofLab::new(resolver, params);
let provider = proofs.provider(rng);
let skip_sequence = provider.check(&preimage).await?;
let proof = provider.prove(&preimage, None).await?;
```

The resolver supplies matching circuit IR and key material. The parameter provider supplies the appropriate proving parameters. `prove` consumes the provider; obtain another provider with a fresh test RNG when needed, or use upstream `split` deliberately. These async traits do not imply trait-object compatibility or a mandatory async executor. Seeded RNGs in the maintained examples are for repeatable local tests.

For typed observed-call preparation, `proofs.verifier(key, limit).await?` uses the same official resolver and returns the runtime's canonical `VerifierKey`. It distinguishes `ProofError::Unavailable`, `ProofError::Resolve` and `ProofError::Decode`. Display/Debug use stable messages; `Error::source` deliberately exposes the underlying diagnostic. Upstream check/prove errors remain upstream errors.

## Run the maintained tests

From the Compact checkout, using the qualified dependency cache:

```sh
cargo +1.99.0 test -p midnight-compact-testkit --features proof --locked --offline
```

This is a repository test command, not an external-consumer tutorial. The default-feature ContractLab package remains usable without selecting proof support.

At commit `17433b40a14da83c5c500d92420149c28f755b88`, the proof-enabled testkit passed 25 unit/integration tests, plus two doc tests after the usage example was added. Default features passed 21 tests and one doc test. The four new tests exercise resolver outcomes, verifier decoding and actual official-provider assertion checks and proof refusal paths. Their assertion IR is authored directly in the test. They do not establish a newly generated Compact proof, successful proving, proof verification, transaction application or network acceptance. Those operations retain the separate [proof integration](proof-integration.md) receipts and maintained harnesses.

## Dependency and ownership notes

The fixture uses the same official ledger8 graph as Compact: `midnight-transient-crypto` 2.0.1 for the three traits and proof types, `midnight-zkir` 2.1.0 for `LocalProvingProvider`, and `midnight-base-crypto` 1.0.0 for RNG capabilities. The existing runtime transaction feature uses `midnight-ledger` 8.0.3. No new versions or product repository dependencies were introduced.

The verifier helper applies its encoded-byte policy after resolver acquisition. It is a test convenience, not an acquisition/aggregate memory guarantee or authentication of artifacts. For the decoder's exact scope and configurable 64 MiB preset, see [input admission](proof-integration.md#admit-encoded-state-and-verifier-inputs).

Decision: ADR0360, issue [#497](https://github.com/MediaNoxLabs/compact/issues/497). Broader application-client integration remains [CoPS-002 / #496](https://github.com/MediaNoxLabs/compact/issues/496); it is not required to use this local runner.

## Retained qualification

See [original ADR0360 logs](../evidence/0.3.0/index.md#local-proof-fixture-adr0360) and the separate [candidate testkit coverage receipt](../evidence/0.3.0/candidate-consumers-adr0362/coverage-delta/coverage-reconciliation.json). Distinct checkpoints retain their own commands; no successful proof is inferred.
