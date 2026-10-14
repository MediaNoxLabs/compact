# Runtime and macro domain model

The Rust backend is not one generated file. It is a set of ownership
boundaries. The most useful mental model is a transition from a source-owned
contract to a ledger-owned call, with the runtime carrying typed evidence
between them.

## Domain map

```text
generated contract
  ├─ types ─────────────── derives ────── FAB / field / binary representation
  ├─ ledger_slots ──────── typed path ─── canonical ledger-8 VM operation
  ├─ Witnesses ─────────── private step ─ private state + private FAB output
  ├─ native circuit ────── CircuitFrame / CircuitResult
  └─ recorded circuit ──── RecordingFrame / RecordedCircuitResult
                                      │
                                      ▼
observation ── RecordedCall ── replay + partition ── ContractCallPrototype
                                                       │
                                  verifier/prover/ledger application
```

## Core execution types

| Type | Owns | Must not be mistaken for |
| --- | --- | --- |
| `ConstructorContext<Private>` | Initial private state, wallet-local Zswap state and source-ordered circuit intents | A deployed ledger contract |
| `ConstructorResult<Private>` | Constructed charged ledger state plus private/Zswap successors | Proof or deployment evidence |
| `CircuitContext<Private>` | Private state, upstream `QueryContext`, cost model, per-query gas limit, Zswap plan and optional execution key | A replayable trace |
| `WitnessContext` | Borrowed private state, typed ledger view and contract address | Permission to disclose witness values |
| `WitnessReadMeter` | Canonical VM reads made through witness projections and their observed cost | A public verifying operation |
| `CircuitResult<Private, Output>` | Successor context, source result, summed query gas and private FAB outputs | A proof-ready transaction |
| `CircuitFrame` | Ordered native composition of nested calls, witness steps and their costs/outputs | A procedural macro or compiler IR |

`CircuitContext.gas_limit` is a ledger-8 limit applied to each individual VM
query. `CircuitResult.gas_cost` is the accumulated diagnostic cost of accepted
queries. A whole-circuit budget is not implied.

## Ledger slots

The generated declaration fixes a value type and a physical path. Runtime slot
types expose only operations appropriate to that declaration:

| Slot | Representative operations |
| --- | --- |
| `CellSlot<T>` | typed read/write, local inspection, witness read, recorded read/write |
| `CounterSlot` | increment/decrement, less-than query, witness and recorded forms |
| `SetSlot<T>` | insert/remove/member/size/is-empty with structural and metered views |
| `MapSlot<K,V>` | typed key/value operations; nested `MapNode` distinguishes non-scalar values |
| `ListSlot<T>` | push/pop/head/length with declared physical path |
| `MerkleSlot<T, DEPTH, HISTORIC>` | typed leaves, declared depth and compile-time historic capability |

Slots prevent the generator from rebuilding VM opcodes for every contract.
They do not make arbitrary paths valid: runtime builders still check operand
width, root/reset semantics, alignment and upstream query rejection.

## Recording domain

`RecordingFrame` executes admitted operations while retaining two products:

1. `CircuitResult` — what native execution did, including private outputs and
   observed query gas;
2. `PublicTrace` — initial public context plus ordered ledger-8 Verify
   operations and sealed execution/Zswap identities.

`RecordedCircuitResult` pairs those products without claiming that replay,
proof or transaction preparation has succeeded.

## Transaction domain

| Type | Boundary it authenticates |
| --- | --- |
| `Observation` | Caller-supplied transaction/block identity; explicitly not finality proof |
| `ObservedContractState` | Address + public contract + observation + allocation/execution identity policy |
| `RecordedCall` | Recorded trace + source entry point + typed FAB input + originating observed state |
| `CallSpec` | Exact verifier, key location, entry point, public input and commitment randomness |
| `ContractCallPrototype` | Replayed and partitioned ledger call ready for later proving/transaction construction |

Preparation checks address, initial state, execution key, installed operation
and verifier identity before replay/partition. Errors such as
`StateMismatch`, `VerifierMismatch`, `ReplayEffectsMismatch` and
`PartitionEffectsMismatch` are trust-boundary failures rather than Compact
source type errors.

## Shielded intent domain

`CircuitZswapPlan` records ordered circuit input/output intents and allocation
state. Its output indices can be provisional during native execution or exact
when bound to an authoritative offer. It is deliberately separate from wallet
state and from a validated offer.

## What the macros own

The proc-macro crate is intentionally narrow:

| Macro | Mechanical responsibility |
| --- | --- |
| `CompactCellValue` | FAB alignment/conversion and checked Cell decoding |
| `CompactEnum` | Default first variant, checked ordinal, field and binary representations |
| `CompactStructRepr` | Fully qualified delegation of field/binary representations for named concrete structs |
| `CompactMerkleTreeDigest`, `CompactMerklePathEntry`, `CompactMerklePath` | Mechanical Merkle representation glue |
| `compact_witness_bridge` | Generate fallible `TryWitnesses` and the infallible adapter from one visible trait |

Macros do **not** decide Compact control flow, recording admission, ledger
paths, disclosure, gas policy or proof support. Those stay in the frontend,
typed backend and runtime respectively. This boundary follows
[RUST-ADR-0004](../adr/0004-derive-only-mechanical-rust-representation-rules.md),
[RUST-ADR-0005](../adr/0005-use-a-typed-runtime-dsl-instead-of-a-body-wide-macro.md)
and [RUST-ADR-0038](../adr/0038-declare-generated-witness-signatures-once.md).

## Compatibility boundaries

At the documented head, generated code asserts runtime ABI 50 at compile time.
The Chez-to-Rust private IR is schema 20 and the capability report is schema 3.
Those numbers describe different internal contracts and must not be compared
as one version line. The runtime reuses ledger-8.0.3 primitives; a matching ABI
does not by itself establish proof keys, network compatibility or application
adoption.
