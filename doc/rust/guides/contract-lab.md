# Test generated contracts with ContractLab

Evidence checkpoint: 2026-10-07. The example below is a historical executed standalone consumer retained in [ContractLab standalone consumer —2026-10-07](../evidence/0.3.0/counter-c1/retained-receipt.md). Host qualification: macOS ARM64, Rust1.99 execution and Rust1.88 compilation, ABI50/ledger8.0.3. See [candidate qualification](candidate-qualification.md) for later candidate checks and their source continuity.

## Choose the test level

| Level | What you run | What success establishes |
|---|---|---|
| Pure | Generated typed pure function | Its returned value or expected error for the supplied inputs |
| Native | `lab.native(...)` | Generated execution and resulting local public/private state |
| Recorded and replayed | `lab.recorded(...)` | Native execution plus sealed trace replay by the real ledger VM; state/effects must agree before commit |
| Proof and ledger application | Separate transaction/proof harness | A specific proof verifies and the transaction is accepted under the tested ledger rules |
| Network | Separate wallet/node integration | A specific submitted transaction is accepted by the selected network |

ContractLab supplies the native and recorded/replay levels. A successful lab test does not supply a proof or a network receipt. The passport source family contains only pure circuits and uses direct typed tests. DID has stateful APIs and uses the lab; recording availability must be read for each exported circuit from the compiler capability report.

## Generate with one runtime source

For the current source distribution, obtain the matching compiler, runtime, macros and testkit checkout. Use an absolute path for `--rust-runtime-root` and use the testkit from that same checkout in Cargo. This lets the generated crate and testkit use one Cargo runtime identity.

```sh
compactc --target rust --skip-zk \
  --rust-runtime-root /absolute/path/to/compact \
  counter.compact generated
```

When using a development compiler wrapper, set `COMPACTC_SCHEME` to its matching Scheme executable. An installed bundle supplies that component through its package layout. The default generated runtime bundle remains useful for standalone generated consumers; pairing it with a separate source testkit requires a common runtime dependency identity. Do not edit generated Rust to work around a duplicate-runtime type error.

The Counter example increments/decrements a declared ledger Counter using a `Uint<16>` parameter. Its original source and all files appear in the linked execution receipt. Create a separate Cargo workspace with these dependencies, substituting your source checkout path:

```toml
[workspace]
[package]
name = "contractlab-external-consumer"
version = "0.0.0"
edition = "2024"
rust-version = "1.88"

[dependencies]
contract = { package = "compact-contract-counter-parameter", path = "generated/contract" }
midnight-compact-testkit = { path = "/absolute/path/to/compact/testkit-rs" }
sha2 = "0.10"
```

Resolve and retain Cargo.lock, then run `cargo +1.99.0 run --locked`. Use `--offline` when that lock's dependencies are already cached. Testkit is currently `publish = false`; this guide does not depend on a registry release.

## Execute, compare and restore

The following is the actual validated external consumer source. Its source/generated hashes detect a caller-declared artifact mismatch when restoring snapshots. They do not authenticate who supplied those files.

```rust
use contract::ledger_contract as counter;
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use midnight_compact_testkit::runtime::{BoundedUint, context::ConstructorContext, ledger::ContractAddress};
use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let identity = ArtifactIdentity {
        source_sha256: Sha256::digest(include_bytes!("../counter.compact")).into(),
        generated_sha256: Sha256::digest(include_bytes!("../generated/contract/lib.rs")).into(),
    };
    let initial = counter::initial_state(ConstructorContext::new(()))?;
    let environment = Environment::new(ContractAddress::default(), Default::default(), [7; 32]);
    let mut lab = ContractLab::from_constructor(identity, environment, initial)?;
    let start = lab.snapshot();
    let mut native = lab.fork();
    let expected = native.native(|context| counter::increment_by(context, BoundedUint::new(3)?))?;
    let actual = lab.recorded(|context| counter::recorded::increment_by(context, BoundedUint::new(3)?))?;
    assert_eq!(expected.public_state(), actual.public_state());
    assert_eq!(expected.effects(), actual.effects());
    assert_eq!(expected.execution_gas(), actual.execution_gas());
    assert!(expected.replay().is_none());
    assert!(actual.replay().is_some());
    assert_eq!(contract::ledger_slots::round.inspect(actual.public_state().get_ref())?, 3);
    let before = lab.snapshot();
    let error = lab.recorded(|context| counter::recorded::decrement_by(context, BoundedUint::new(4)?)).unwrap_err();
    assert!(matches!(error, LabError::Execution(_)));
    assert_eq!(lab.snapshot().public_state(), before.public_state());
    lab.restore(&start)?;
    assert_eq!(contract::ledger_slots::round.inspect(lab.snapshot().public_state().get_ref())?, 0);
    println!("PASS: native/recorded parity, VM replay, rejected underflow, rollback, restore");
    Ok(())
}

```

`Environment` supplies the contract address, block information, coin key, cost model and optional per-query gas limit. Set values explicitly in each fixture. Its `fixture_seed` is a provenance label: ContractLab does not construct a random generator from it or read the wall clock.

The test calls the original generated constructor, forks an owned checkpoint, executes native and recorded increments, compares state/effects/gas, requires replay evidence, rejects underflow without committing state, and restores the initial snapshot. `ledger_slots::round.inspect` uses the generated typed descriptor for the declared Counter.

## Make witnesses explicit

Implement the generated `TryWitnesses` trait using ordinary Rust methods. When deterministic scripted answers are useful, store `WitnessScript<Arguments, Reply>` in the lab's owned private state. It validates arguments and consumes answers in order. Inspect `journal()` only where private arguments can be handled safely. A mismatched argument leaves the pending answer intact; an accepted invocation consumes its answer, including an intentional witness error.

Choose owned `Clone` data for private state, or immutable persistent data. A snapshot cannot undo mutations hidden behind shared interior mutability, filesystem/network effects, or an external witness journal. A failed lab call preserves the committed lab checkpoint; any external witness activity that already happened remains observable. Existing DID tests explicitly distinguish these two histories.

### Diagnose a known script failure without exposing it by default

The generated witness adapter returns `CompactError`. Script exhaustion, argument mismatch and application assertions can therefore share the same error representation and even the same message. The lab reports all of them as `LabError::Execution`; it does not infer an origin from text.

In a test that already knows which failure it expects, use the existing accessor deliberately:

```rust
// `error` is the failed lab call; `expected_error` is chosen by this test.
// Inspection does not authenticate whether a script or application produced it.
assert_eq!(error.to_string(), "circuit execution failed (details redacted)");
assert_eq!(error.execution_error(), Some(&expected_error));
```

`Debug`, `Display` and generic `Error::source` traversal do not reveal the payload. The accessor can expose private application or witness data: do not print it, the script journal, or custom assertion values in shared logs without reviewing them. Exhaustion and argument mismatch leave the queue and journal intact. A matching invocation consumes one answer and journals its arguments even if that answer is an error. If the script is owned by the lab's private state, a failed lab invocation rolls back that working copy together with its public state; external activity and shared interior mutations remain outside this guarantee.

## Inspect reports deliberately

| Accessor | Meaning |
|---|---|
| `output()` | Typed generated return value |
| `before()` / `public_state()` | Local charged ledger state before/after the call |
| `effects()` | Resulting ledger effects |
| `private_outputs()` | Ordered aligned private transcript values, exposed only on deliberate access |
| `execution_gas()` | Sum of generated query costs, including metered witness reads |
| `replay().program()` | Ordered sealed Verify operations used by the VM replay |
| `replay().gas()` | Cost of one VM query executing the sealed public Verify program |

Execution gas sums separately executed queries, including metered witness reads. Replay executes the concatenated public Verify program as one query, so it excludes witness-only reads. Query grouping and write accounting can also change the costs without any witness. Neither equality nor a universal ordering is promised, and subtracting the two costs is not a general witness-cost measurement.

The ADR0273 local regressions independently query the component operations and full program. In the pinned ledger 8.0.3 witnessed Cell fixture, one read plus one write matches execution, while replay matches the write alone. That equality is specific to this fixture. A separate runtime-frame control writes 42 then 43 without witnesses: execution accounts for 76 bytes written and 74 deleted, while the full replay accounts for 38 written and 36 deleted. Both reach the same state and effects. These are fixture observations, not transaction fee estimates or performance claims. The runtime-frame control is handwritten test orchestration using the generated Cell constructor; it is not a newly emitted contract.

Do not add execution and replay gas and call it transaction cost. The per-query limit is not an aggregate call/replay budget. Report `Debug` intentionally omits private payloads and error contents. Explicit accessors and custom assertion messages can still expose data; the library does not redact text assembled by callers.

## Trust and failure boundaries

Adapters are trusted Rust closures around generated calls. The lab checks starting/returned environment and replay consistency, but it does not sandbox a closure or detect every temporary mutation that a closure later restores. Current snapshots describe plain-ledger state; funded Zswap/wallet ownership requires the separate transaction integration surface. Snapshots are in-memory checkpoints, not a durable private-state database or a secret serialization format.

Snapshot restore checks format, source/generated identity, runtime ABI, ledger label, state mode and environment. Fix mismatches by choosing the correct fixture/runtime/environment; do not overwrite metadata to force a restore across incompatible state.

## More examples and evidence

- Counter and witnessed Cell: `testkit-rs/tests/` and “ADR0248 — ContractLab local receipt” (historical vault reference; not bundled here).
- Set/Map and Merkle: `testkit-rs/tests/merkle_scenarios.rs`, “ADR0258 — ContractLab Merkle local receipt” (historical vault reference; not bundled here).
- Original DID native constructor/lifecycle: “ADR0255 — Original DID native lifecycle local receipt” (historical vault reference; not bundled here). Recorded lab coverage is delivered under ADR0262/#386. The current external DID tutorial separately passes both original-constructor/deactivate and late timestamp-witness failure scenarios at the staged5efa91c2 checkpoint.
- Pure passport adoption: “Passport adoption acceptance — 2026-10-07” (historical vault reference; not bundled here).
- Decisions: ADR0248 (testkit boundary), ADR0258 (Merkle demonstration), ADR0262 (adopted DID recording).


## Terminology and the TypeScript simulator connection

See [Rust backend and ContractLab glossary](glossary.md) for ABI, snapshot, fork, native execution, recording, replay, commit/rollback, gas, environment, witnesses and reports. It includes a counter scenario and a mapping from the upstream TypeScript CounterSimulator to ContractLab. Recorded replay establishes local VM state/effect agreement; proof verification and network acceptance remain separate test levels.

## Complete current external tutorials

The current edition contains a [witnessed Cell tutorial](testing-witnesses.md) and [original DID lifecycle tutorial](did.md). Their three tests ran against freshly generated, unedited contracts and an isolated staged SDK. Each resolved graph contains one runtime source identity. Formatting and strict Clippy also passed. Read their receipt for exact compiler, source, lock, features and host. The witnessed adaptation compares native and recorded execution; the DID adaptation also checks its independent retained TypeScript capture.
