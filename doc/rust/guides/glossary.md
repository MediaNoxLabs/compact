---
type: developer-glossary
status: reference
created: 2026-10-07
aliases:
  - ContractLab glossary
  - Rust backend terminology
tags:
  - compact
  - rust-backend
  - developer-documentation
---

# Rust backend and ContractLab glossary

A plain-language reference for engineers using our generated Rust contracts. Created from the ABI/ContractLab discussion on 2026-10-07. Definitions describe the inspected implementation; they do not add capabilities or change milestone acceptance. Repository HEAD at documentation time: `f1140869a1518017e09af4600cd78fecce47d0fc` on `codex/rust-backend-ast`.

Start with [Start here — Rust backend](index.md). For executable setup and complete examples, use [Testing generated contracts with ContractLab](contract-lab.md). For current package selection, use [Version and runtime selection](version-selection.md).

## ABI — the generated-code/runtime compatibility agreement

**ABI** normally means **Application Binary Interface**: the rules compiled components use to call each other and exchange data.

In this implementation, **runtime ABI** is our project-specific compatibility marker for the agreement between **generated Rust source and its runtime**: available types, traits, operations and their expected semantics. It is not a promise of a stable native Rust binary interface or a universal Compact ABI standard.

At this checkpoint, generated code contains:

```rust
const _: () = assert!(runtime::RUST_RUNTIME_ABI == 50);
```

The generator expects ABI 50 and the selected runtime must advertise ABI 50. A mismatch fails compilation. For example, when generated code requires a new incompatible representation or runtime interface, changing this marker prevents accidental use with an older runtime. Matching numbers are a compatibility guard; they do not authenticate sources or independently prove correctness.

### Versions with different jobs

| Label | Meaning | Value at this checkpoint |
|---|---|---|
| Milestone | Engineering delivery and acceptance scope | 0.3.0 |
| Backend/runtime/macros package version | Cargo package release identity | 0.2.0 |
| Runtime ABI | Generated-source/runtime compatibility agreement | 50 |
| Private IR schema | Format between the compiler frontend and Rust backend | 20 |
| Capability schema | Format describing generated API/proof availability | 3 |
| Ledger version | Underlying native ledger implementation | 8.0.3 |

These values evolve separately. The historical phrase **schema-6 IR** referred to version 6 of our private intermediate representation, not a generally recognized “Compact schema-6” standard. Current version selection and historical migration evidence belong in [Version and runtime selection](version-selection.md).

## ContractLab — a reusable local contract test bench

**ContractLab** is the test harness in `midnight-compact-testkit`. It owns a scenario's public state, private state and execution environment. You initialize it from a generated constructor, invoke generated circuits, and inspect results.

A useful mental model: **our Rust contract simulator, with checkpoints, alternative scenarios, execution reports and explicit ledger-VM replay**. Current lab scope is plain-ledger scenarios; funded wallet/Zswap flows use separate integration machinery.

### Snapshot — a saved checkpoint

`snapshot()` captures the current public state, private state, environment and metadata in memory. `restore(&snapshot)` returns the lab to that checkpoint after checking artifact identity, ABI, ledger label, snapshot format, state mode and environment.

Example: save the counter at 3, run several calls, then restore it to 3. A snapshot is not a disk backup, private-state database or blockchain-finality certificate.

### Fork — another scenario starting here

`fork()` creates another lab starting from the same checkpoint. If the counter is 3, one lab can add 2 and another add 7, producing independent scenarios with values 5 and 10.

Private state must clone independently or use immutable persistence. A Rust `Clone` that shares mutable data does not automatically provide isolation. This is a local testing fork, not a blockchain fork or Git branch.

### Native execution — run the generated Rust circuit

`lab.native(...)` calls generated Rust, validates the returned boundary state/environment, and commits a successful result to the lab. Stateful runtime operations can already use upstream ledger machinery. “Native” does not imply that all behavior bypasses the VM.

### Recording — capture the public ledger operations

A recorded generated call performs execution and collects a sealed public operation program. For a counter this can be understood as read counter → add amount → write counter. The recording is executable ledger data, not just a textual debug log, and does not contain every private computation.

Generated recording support is per circuit; consult [Reading Rust capability reports](capabilities.md).

### Replay — execute that recording in the ledger VM

`lab.recorded(...)` executes the recorded generated call, replays the public program through the real ledger VM from its original starting state, and compares public state and effects before committing.

```text
Starting state
  ├─ generated Rust execution ──> public state + effects
  └─ recorded public VM program > public state + effects
                                  must agree
```

Replay failure does not fall back to native success. Successful replay is local consistency evidence; it does not generate a zero-knowledge proof or establish network acceptance.

### Commit and rollback — what state becomes the next starting point

A successful accepted call replaces the lab's owned checkpoint. A failed call preserves its previous committed checkpoint. Here, “commit” means a local lab state update, not a Git commit or an on-chain transaction.

Rollback does not undo external witness activity, network/filesystem changes, or shared mutable data outside the lab's ownership. For example, a witness can write to an external journal before the contract rejects the call; that journal entry remains.

### Gas — metered ledger work

Gas is work measured using the configured ledger cost model. It supports regression comparisons and configured query limits. It is not elapsed time, prover time or a complete transaction fee quote.

| Accessor | What it measures |
|---|---|
| `report.execution_gas()` | Sum of separately executed query costs, including metered witness reads |
| `report.replay().unwrap().gas()` | Cost of the public Verify program replayed as one VM query; available on successful recorded reports |

The values can differ because witness-only reads are excluded from replay and query grouping/write accounting differ. No equality or universal ordering is promised. Do not add them as transaction cost or subtract them to infer general witness cost. `query_gas_limit` is a per-query generated-execution limit, not an aggregate call/replay budget.

### Environment — the explicit test surroundings

`Environment` supplies the contract address, block context, optional coin public key, cost model and optional query gas limit. Explicit inputs help make scenarios reproducible. `fixture_seed` is a provenance label for caller-created fixtures; ContractLab does not turn it into an RNG or read the wall clock.

### Witness and private state

A **witness** supplies private/off-chain information to a circuit. **Private state** is the application's private data passed between calls. Neither is synonymous with public ledger state. Witness execution can have external side effects, so tests should distinguish private-state rollback from external activity.

### CallReport, effects and private outputs

A **CallReport** exposes the returned output, before/after public state, ledger effects, private transcript outputs, execution gas and optional replay evidence. **Effects** are the ledger-recorded consequences checked alongside the resulting public state. **Private outputs** require deliberate access and careful logging.

`CallReport::replay()` returns an `Option<&ReplayReport>`: native reports have none; successful recorded lab reports have evidence. Different runtime types have different APIs: for a runtime `RecordedCall`, use `call.recorded().replay()` rather than assuming the lab report API applies everywhere.

### Artifact identity

`ArtifactIdentity` carries caller-supplied source/generated hashes, helping detect an accidental snapshot/artifact mismatch. It does not authenticate who produced those files or verify their hashes against the filesystem by itself.

## Relationship to the TypeScript CounterSimulator

The inspected upstream `CounterSimulator` creates a generated contract and constructor context, keeps its circuit context, calls `impureCircuits.increment(...)`, replaces the context with the returned one, and exposes ledger/private state. That is the same basic local-testing purpose served by ContractLab.

ContractLab makes the pattern reusable across Rust contracts and adds explicit checkpoint/fork/report/replay APIs. This comparison concerns this particular CounterSimulator wrapper, not every TypeScript testing framework.

| CounterSimulator task | ContractLab equivalent |
|---|---|
| Construct initial contract/context | Generated constructor → `ContractLab::from_constructor(...)` |
| Keep current circuit context | Lab owns scenario state and environment |
| Invoke `increment()` | Adapter calling generated code via `native(...)` or `recorded(...)` |
| Read the ledger/private state | Generated typed ledger accessors and lab private-state access |
| Save or reset a scenario | `snapshot()` / `restore()` |
| Explore two alternatives | `fork()` |
| Inspect costs and effects | `CallReport` |
| Require public VM replay agreement | `recorded(...)` |

Pinned reference: [example-counter CounterSimulator](https://github.com/midnightntwrk/example-counter/blob/e8e44ef1a81ea87d72c09b343d80614b6cd21b48/contract/src/test/counter-simulator.ts). This is a conceptual comparison, not a claim of package/version compatibility with our ledger-8 branch.

## One counter scenario tying the terms together

Excerpt from the validated consumer in [Testing generated contracts with ContractLab](contract-lab.md); assumes its imports and initialized `lab`:

```rust
let start = lab.snapshot();
let mut alternative = lab.fork();

let native = alternative.native(|ctx| {
    counter::increment_by(ctx, BoundedUint::new(3)?)
})?;

let recorded = lab.recorded(|ctx| {
    counter::recorded::increment_by(ctx, BoundedUint::new(3)?)
})?;

assert_eq!(native.public_state(), recorded.public_state());
assert_eq!(native.effects(), recorded.effects());
assert!(native.replay().is_none());
assert!(recorded.replay().is_some());

lab.restore(&start)?;
```

Both branches begin at the same checkpoint. One executes natively; the other requires recording and VM replay agreement. Restore resets the original lab to its saved starting point.

## Related compiler and evidence terms

| Term | Plain meaning |
|---|---|
| AST | Abstract syntax tree: structured code nodes instead of concatenated source text. Rust emission uses `syn`, `quote`, `proc-macro2` and `prettyplease`. |
| IR | Intermediate representation: compiler data describing the contract between processing stages. Our Rust IR has its own schema version. |
| Emitter | The component that translates the checked compiler model into generated Rust source. |
| Runtime | Shared support used by generated code, reusing upstream ledger/zk primitives. |
| Ledger slot | A generated typed descriptor identifying a declared ledger location and its value type. |
| Capability report | Machine-readable description of generated native/recorded/observed APIs and compiler proof applicability. Availability is not proof that a particular call will succeed. |
| Checked plan | A compiler representation admitted after a profile checks the shape and invariants it owns. It is not a zero-knowledge proof. |
| Admission/profile | A supported lowering strategy examining whether it can handle a circuit. `NotApplicable`, `Rejected` and `Admitted` distinguish outcomes. |
| Observed call | A generated call preparation route using an explicit ledger observation; observation trust and later transaction validation remain separate concerns. |
| Pure call | A generated computation without the stateful lab lifecycle; useful for the adopted digital-passport API. |
| Proof verification | Cryptographic verification of a specific proof. It is a separate test level from recorded VM replay. |
| Strict ledger application | Applying a transaction under the specified ledger validation rules; broader evidence than local replay alone. |
| Oracle | An independently obtained reference behavior/output used for comparison, such as pinned TypeScript results. It is not automatically an on-chain data oracle. |
| Parity | Agreement between implementations for explicitly tested behaviors and inputs; always read the stated scope. |
| Reducer/minimal reproducer | A small Compact contract isolating a failed primitive or behavior from a larger contract. |
| Receipt | A retained evidence record with source identity, checks, results and limits. A local test receipt is not necessarily a network transaction receipt. |

## Maintenance and implementation anchors

Prefer linking to this glossary over copying its definitions into each guide. Add new project-specific terms with a plain definition, example, API owner and scope limits. Revalidate version examples at release closeout; historical receipts retain their original values. This note explains existing decisions and does not create a new ADR or implementation promise.

Repository paths at the checkpoint above:

- `runtime-rs/src/lib.rs`: runtime ABI and ledger version.
- `tools/compact-rust-backend/src/lib.rs`: emitted ABI assertions.
- `testkit-rs/src/lab.rs`: native/recorded execution, commit, fork and restore.
- `testkit-rs/src/snapshot.rs`: checkpoint contents, metadata and identity validation.
- `testkit-rs/src/environment.rs`: execution environment and query-limit semantics.
- `testkit-rs/src/report.rs`: execution/replay gas and report accessors.

See [Runtime and compiler ownership map](ownership.md), [Reading Rust capability reports](capabilities.md) and “Delivery dashboard” (historical vault reference; not bundled here) for deeper explanations and current acceptance.
