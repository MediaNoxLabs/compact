# Compact contracts in Rust

**Rust backend developer guide — milestone 0.3.0.** Based on the accepted source APIs and scoped receipts of milestone0.3.0. The witnessed-contract tutorial and both DID lifecycle tests now pass in isolated external consumers at source `5efa91c2`, with qualified locks and unedited generated contracts. See [the retained ADR0324 evidence](../evidence/0.3.0/index.md#tutorials-adr0324) for exact receipts. [The snippet matrix](snippet-matrix.md) distinguishes measured examples and their limits. See [candidate qualification](candidate-qualification.md) for the overall validation and documentation status.

The Rust backend uses the ledger8 Compact frontend, a private typed IR and structured Rust AST emission with `syn`, `quote`, `proc-macro2` and `prettyplease`. Generated contracts reuse the pinned Midnight ledger/zk runtime primitives.

## Pick a task

| Task | Guide |
|---|---|
| Select the compiler/runtime and understand version numbers | [Versions, installation and targets](versions-and-targets.md) |
| Implement a witness and test a stateful contract | [Witnessed ContractLab tutorial](testing-witnesses.md) |
| Run a DID constructor and recorded lifecycle | [DID walkthrough](did.md) |
| Diagnose generation, types, recording and test failures | [Troubleshooting](troubleshooting.md) |
| Reach a witnessed recording handle when a source circuit is named `recording` | [Recording accessor collisions](troubleshooting.md#when-a-circuit-is-named-recording) |
| Understand proof preparation and trust boundaries | [Proof and ledger integration](proof-integration.md) |
| See which examples were actually executed | [Snippet matrix](snippet-matrix.md) |

Companion guides cover the [digital passport](passport.md), [capability reports](capabilities.md), [component ownership](ownership.md), and [static constraints and migrations](static-constraints.md). Their current text is reconciled with the accepted API; dated receipts retain their original source identities. The application-owned passport named-input recipe passes 25 independent stored cases.

## Read a generated crate

| Module | What it owns |
|---|---|
| `types` | Named Compact records/enums and their Rust representations |
| `pure_circuits` | Typed pure functions returning values or `CompactError` |
| `ledger_slots` | Typed descriptors for declared ledger locations |
| `ledger_contract` | Constructor, stateful functions, generated witness traits and typed public-state views |
| `ledger_contract::recorded` | Supported per-circuit recording APIs |
| `runtime` | Re-export of the runtime used by this generated crate |

Keep handwritten application/test code in a separate consumer. Regenerate the contract after source/compiler changes. Editing generated Rust or capability JSON is not a supported way to add missing behavior.

## Know the level of evidence

Compilation establishes type compatibility for the selected graph. Native execution checks values/errors/state for supplied inputs. Recorded replay checks a sealed program against the actual VM. Real proof verification, strict ledger application and network submission each require separate evidence.

`ContractLab` supports local plain-ledger execution and replay. Its typed API does not turn caller-supplied observations into trusted chain state or undo external witness side effects.

## Adopted applications

- **DID:** pinned unchanged v0.7.0 source closure; twelve original stateful recorded APIs locally accepted, with separate finite lifecycle/proof and ledger8.1 wire-profile receipts. Native runtime remains8.0.3.
- **Digital passport:** the selected `midnight-vc-passport` reference resolves to the pinned credential source closure. Its75 exports are pure. Existing acceptance includes202 finite sampled cases; this does not supply ledger transactions or ZK proofs for the pure closure.
- **Passport ACC:** removed from the initiative after construct review. No ACC migration or future adoption is scheduled. Generic Jubjub support and its scoped tests remain; ledger8 time lowering is separately tracked in #442.

Milestone0.3.0 is not a blanket “production ready” assertion for untested platforms, arbitrary later source versions, or every possible input.

The optional [ProofLab fixture](local-proof-tests.md) reuses official Midnight providers for local proof checks and proving setup alongside ContractLab.

## Current candidate status

See [candidate qualification](candidate-qualification.md) for current bounded CI, full-gate and documentation status.
