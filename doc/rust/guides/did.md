# Run the original DID constructor and a recorded lifecycle

**Locally validated external tutorial.** This walkthrough uses unchanged `midnight-did v0.7.0` Compact source and adapts the existing generated-fixture ContractLab tests into a standalone package. Both lifecycle tests passed after fresh generation in this isolated layout, using Rust1.99.0 on aarch64-apple-darwin, default features and the qualified source lock at `5efa91c2`. The receipt records strict Clippy, formatting, path isolation and one shared runtime identity.

## What is included

`examples/did-lifecycle/source/` contains the pinned source manifest and two-file Compact closure, copied byte-identically. Its upstream commit is `4e7f6b0f69bf4e2c8506a9693f8d0c3dfe68e550`. `data/unit-composition.json` is the byte-identical maintained independent TS capture.

The complete tests and witness implementations are present:

- `tests/lifecycle.rs`: adapted from `tests-rust-backend/did-adoption/tests/contract_lab.rs`.
- `support/lifecycle_witness.rs`: constructor controller/recovery/timestamp witnesses.
- `support/witness.rs`: call-time Schnorr reduction and timestamp witnesses.
- `support/codec.rs`: deterministic fixture value conversions.
- `Cargo.toml`: new external manifest using generated `compact-contract-did`, staged ContractLab and exact serialization dependency.

Only imports and local include paths in the copied Rust support/tests are adapted. The original generated crate is not patched. These files are fixtures: their deterministic keys, captured signatures and timestamps are not production identity/key-management code.

## Prepare and generate

Follow the SDK preparation in [the witnessed tutorial](testing-witnesses.md). Both examples share `examples/sdk/` so generated code and ContractLab use one runtime source identity. The validation receipt records this staged SDK layout.

```sh
cd /absolute/path/to/draft/examples/did-lifecycle
compactc --target rust --skip-zk --rust-runtime-root /absolute/path/to/draft/examples/sdk source/packages/contract/src/did.compact generated
cp /absolute/path/to/reviewed/compact/Cargo.lock Cargo.lock
cargo +1.99.0 metadata --offline --format-version 1 > metadata.json
cargo +1.99.0 test --locked --test lifecycle
```

Review and retain the dependency lock against qualified identities before accepting results. Check path isolation and unique runtime identity through Cargo metadata. This local tutorial uses default features; it does not require `ledger-transaction` because it performs native execution and VM replay, not transaction preparation.

## Follow the two scenarios

The successful scenario constructs the DID from actual witnesses. The retained oracle starts with private state7; the test initializes private4 and expects controller, recovery and timestamp witnesses to increment it once each. It checks the constructed ledger data matches the TS prestate before executing a call.

It then runs `deactivate` natively on one fork and `recorded::deactivate` on the other, using the real captured Schnorr signature/reduction inputs. Assertions compare returned Unit, state/effects, private state, witness order, private outputs, public VM program, query-summed gas and replay. The result must match the independent TS capture. No always-true authorization witness replaces source checks.

The second scenario deliberately fails the timestamp witness late in the call. It checks the lab checkpoint remains unchanged while the external witness journal records work already performed. That distinction is part of the API contract; rollback does not erase external activity.

The complete tests supply every context, witness, signature, version and capture input. Read `tests/lifecycle.rs` rather than treating the following call shape as a standalone program:

```rust
let report = lab.recorded(|ctx| {
    did::recorded::deactivate(ctx, &recorded_witness, signature(&row), version(&row))
})?;
```

This fence is a conceptual excerpt from the adapted test; the file uses assertions/unwraps appropriate to a finite regression scenario.

## Broader API and evidence

The adopted source has twelve stateful exports with native/recorded/observed-call surfaces where advertised. The [Service guide](did-service.md) covers typed Service, SchnorrJubjub method, nested JWK method, digest and relation APIs. This two-scenario tutorial does not rerun every adopted export.

Original application acceptance is tied to native ledger8.0.3. Separate finite ledger8.1 transaction interchange receipts do not establish general runtime interchangeability, a continuous8.1 chain or network finality. Constructor data is deployed in proof gates; constructor execution itself remains unproved.

For a new source revision, inspect generated capability metadata again. Typed arguments do not imply source authorization/version/membership rules will succeed. Use the matching source closure and application semantics when diagnosing a refusal.

For real transaction integration, continue with [proof and ledger integration](proof-integration.md). This tutorial supplies no live wallet, key storage, proof provider or network submission.
