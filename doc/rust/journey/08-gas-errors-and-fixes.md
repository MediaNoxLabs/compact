# Gas, errors and corrected boundaries

The backend became more trustworthy by moving failures to the earliest layer
that has enough information to explain them—without pretending every failure
can happen at compile time.

## The gas-accounting correction

The original comparison looked alarming: Rust reported much more gas than the
TypeScript wrapper for the same `tiny` circuit. The implementations were not
using the same reporting denominator.

TypeScript replaced `context.gasCost` after each query and exposed the last
query. Generated Rust accumulated every query. The accepted decision in
[RUST-ADR-0010](../adr/0010-report-total-rust-circuit-gas-across-ledger-queries.md)
kept the Rust total and changed the comparison to the sum of captured
TypeScript query costs. For `clear`, `set` and `get`, all four dimensions then
matched exactly at commit
[`3799746a`](https://github.com/MediaNoxLabs/compact/commit/3799746acec6e5898e5e7b121621045d8dac925a).

```text
reported TS wrapper cost       = last query only
captured TS comparison cost    = Σ every executed query
Rust CircuitResult.gas_cost    = Σ every executed query
transaction gas                = replay/partition result, separately owned
```

The four dimensions are read time, compute time, bytes written and bytes
deleted. They should not be collapsed into one scalar.

## Witness reads became visible

A witness could read a ledger Cell structurally while TypeScript executed a VM
read. Rust therefore underreported the call. The explicit `WitnessReadMeter`
introduced at
[`98d33ca7`](https://github.com/MediaNoxLabs/compact/commit/98d33ca7d8ee52ab129d861f0604dc0826d929ca)
charges canonical witness projection queries while keeping their private value
out of the public trace. The Set extension followed at
[`2ca64f6f`](https://github.com/MediaNoxLabs/compact/commit/2ca64f6f26390b17f018ba5319f5330dd4ec3922).

One witnessed Cell write moved from an incomplete Rust read-time total of
85,000,000 to the exact captured total of 255,000,000: 170,000,000 for the
witness read plus 85,000,000 for the write. That is corrected accounting, not
a cheaper VM operation.

## Per-query guard, whole-circuit observation

At ledger-8, `CircuitContext.gas_limit` guards each query independently. A
circuit containing two accepted reads may therefore report a total above the
configured one-query limit. Commit
[`7c489892`](https://github.com/MediaNoxLabs/compact/commit/7c4898929a7c9ab8bc357d844a1843c5531129c8)
made that contract explicit and tested. Silently turning it into a cumulative
budget would diverge from the pinned TypeScript/ledger behavior and introduce
new rollback questions.

## Compile-time versus runtime failures

| Layer | Example | Why it belongs there |
| --- | --- | --- |
| Compact frontend | Type mismatch, invalid disclosure or source recursion | The source language has complete typing/scope information |
| Private IR admission | Unknown schema node, recursive direct IR call, invalid declaration reference | Prevent malformed or forged compiler input from reaching emission |
| Rust backend | Emitted-name collision, unsupported wide operation, incomplete recording capability | Target naming and backend capability are known before publication |
| Proc macro / rustc | Data-carrying `CompactEnum`, hostile introduced name, wrong witness impl signature | The error is a Rust representation/consumer integration defect |
| Native runtime | Failed Compact assertion, unsigned overflow, invalid ledger Cell, rejected VM query | Depends on call values or ledger state |
| Recording | Unsupported complete effect plan, replay-visible invariant failure | Native execution may exist while complete recording does not |
| Preparation | Address/state/verifier mismatch, empty transcript, unbound Zswap intent | Requires observed state and exact artifacts |
| Ledger/prover | Invalid proof, unsatisfied transaction policy, application refusal | Owned by authoritative proof and ledger validation |

## Representative issues that were fixed

### Invalid Rust became a source-located compiler error

Distinct Compact circuit names such as `a$b` and `a_b` normalize to the same
Rust identifier. Before, rustc rejected a duplicated function after emission.
Commit
[`c70dd63c`](https://github.com/MediaNoxLabs/compact/commit/c70dd63cf91ce59e2bda82a0f27f32073639c669)
added target-namespace preflight and reports both source names at the later
declaration. See [RUST-ADR-0246](../adr/0246-reject-circuit-collisions-in-emitted-rust-namespaces.md).

### Macro expansion stopped depending on caller names

Derives introduced unqualified `Result`, `Option`, `Vec` and `From`, so valid
consumer names could break compilation. Commit
[`77c9ee14`](https://github.com/MediaNoxLabs/compact/commit/77c9ee144f3c326f29cb44a69b7174c5d81b8945)
fully qualified macro-owned names. Source structs named `Option`, `Fr`,
`MemWrite`, `runtime` or `FieldRepr` exposed a second failure in upstream
derive/import expansion; the owned `CompactStructRepr` boundary fixed the
bounded generated shapes at
[`ee768a60`](https://github.com/MediaNoxLabs/compact/commit/ee768a60706607580d4ff8b3c949f22c882501b0).

### Forged recursive IR stopped becoming recursive Rust

The normal frontend rejected recursive Compact, but direct schema-20 IR could
still emit recursive pure Rust. Commit
[`dbfd7dc2`](https://github.com/MediaNoxLabs/compact/commit/dbfd7dc248b6bbe3b188a5a362142facf06621de)
moved the refusal to private-IR semantic admission while preserving acyclic
shared helpers. See [RUST-ADR-0332](../adr/0332-reject-recursive-pure-calls-at-the-private-ir-admission-boundary.md).

### Malformed ledger paths stopped panicking or truncating

Merkle/list reset builders used `split_last().expect`; other builders narrowed
path lengths before constructing compact VM operands. Commit
[`e6807884`](https://github.com/MediaNoxLabs/compact/commit/e68078846260e4c6b2d18f40ec9ec1a2571cae33)
checks the actual 4-bit `Ins`/`Idx` encoding limits and returns the existing
fallible ledger error rather than panicking, truncating or emitting an
ambiguous program. See [RUST-ADR-0351](../adr/0351-check-merkle-vm-path-operands-before-program-construction.md).

### A truthful refusal replaced a misleading bounds error

Valid wide unsigned types could be represented, but subtraction,
multiplication and ordered comparison were unsupported. They were incorrectly
reported as malformed maxima. Commit
[`6aa345fd`](https://github.com/MediaNoxLabs/compact/commit/6aa345fd76cf99ba491a761c42a39608d7e207eb)
added `UnsupportedWideUnsignedOperation` without widening accepted behavior.
See [RUST-ADR-0352](../adr/0352-distinguish-unsupported-wide-operations-from-malformed-uint-bounds.md).

## The architectural rule

Move a failure earlier only when the earlier layer has authoritative facts.
Source recursion and name collisions belong before Rust publication. An
assertion depending on current ledger state remains a runtime `Result`.
Verifier identity cannot be authenticated until preparation. Proof and ledger
rejection must not be simulated by compiler heuristics.
