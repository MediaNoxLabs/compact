# Jubjub values: native calls, recording and proof boundaries

Checkpoint: signed `dce3c9e9`, 2026-10-07. This guide explains the maintained `jubjub-scalar-cell` example. Code excerpts come from the generated fixture and exercised tests; this page is not a separately built external-consumer package.

## The contract

```compact
export ledger result: JubjubPoint;
export circuit apply(point: JubjubPoint, scalar: Field): JubjubPoint {
  const reduced = jubjubScalarFromNative(scalar);
  const value = ecAdd(ecMulGenerator(reduced), ecMul(point, reduced));
  result = disclose(value);
  return value;
}
```

The contract reduces a native Field to the Jubjub subgroup scalar range, calculates two scalar multiplications and adds the points. It writes one typed point Cell and returns that same point. Reduction is an explicit operation with observable boundary semantics: raw `ecMul(point, scalar)` rejects noncanonical scalars; `jubjubScalarFromNative` reduces them. Do not remove the reduction when porting the original Compact0.35 scalar casts.

## Three useful surfaces

| Surface | Result | What it establishes |
|---|---|---|
| `ledger_contract::apply(context, point, scalar)` | `CircuitResult<Private, JubjubPoint>` | Native execution, typed output, state/effects/gas |
| `ledger_contract::recorded::Contract.apply(context, point, scalar)` | `RecordedCircuitResult<Private, JubjubPoint>` | Native result plus ordered ledger program for VM replay |
| `ledger_contract::recorded::Contract.apply_call(observed, private_state, point, scalar)` | `RecordedCall<'observed, Private, JubjubPoint>` | Recorded call tied to the observed contract; available with `ledger-transaction` |

The generated developer-facing method parameters retain `point` and `scalar`. Internal temporary names only describe emitted evaluation steps. Consumers use the typed methods and existing runtime point/field types.

The facade uses the existing preparation API. The following excerpt is exercised in the strict proof runner, with the verifier loaded from the observed contract's `apply` entry and `supplied`/`scalar` prepared earlier:

```rust
let typed = acc::recorded::Contract
    .apply_call(observed, 17_u64, supplied, scalar)?
    .prepare(
        verifier.clone(),
        midnight_transient_crypto::curve::Fr::from(0_u64),
    )?;
```

Preparation is one stage. A prover produces the proof and ledger validation/application accepts or rejects the transaction. The test runner exercises those later stages; a successful native call or replay alone does not establish proof acceptance.

## Domain and ownership

`JubjubPoint` and `Field` remain runtime types backed by the qualified Midnight primitives. The compiler's shared operation helper accepts already lowered, typed operands. Pure lowering owns pure evaluation; the recording plan owns lexical bindings, evaluation order and ledger effects. Both use the same checked operation syntax. There is no new cryptographic implementation in this slice.

The recording profile accepts four operations: native-to-Jubjub scalar reduction, generator multiplication, point multiplication and point addition. It supports typed inputs/locals, nested terminal lets/sequences, one declared point Cell write and a point result. Scope/type/path/declaration checks happen before exposing recording capability. Branches, helper/witness calls, additional writes and other curve operations are outside this bounded profile.

## Evidence and local commands

The fixture compares seven native scalar boundaries against retained independently captured original0.35/runtime0.20 values and complete fresh ledger8TS observations. ContractLab checks raw canonical success and noncanonical rollback. These are three maintained Rust test methods with nine behavior rows and exact source-reference hashes.

From the repository root:

```sh
cargo +1.99.0 test --locked --offline -p compact-rust-jubjub-scalar-cell-fixture --all-features
python3 -m unittest discover -s tools/compact-rust-backend -p 'test_acc_jubjub_gate.py'
```

The maintained strict gate is `tools/compact-rust-backend/acc_jubjub_gate.py`. It requires explicit compiler, Scheme, Cargo target and new output-directory paths, cached Midnight proof parameters, and verified ledger static fixtures. It regenerates source, checks TS/reference parity, runs Rust behavior tests, makes fresh k11/1190-row keys, and requires seven real proof applications plus changed-binding and replay refusals. Each delivered proof is3296 bytes. The gate is now a required child of the local parity orchestration.

## Limits

Constructor data is deployed without a constructor-execution proof. `raw` has behavior/refusal coverage; this delivery does not claim a real raw-call proof. The profile is a generic scalar/point building block. ACC adoption was removed under ADR0316/#441 after construct review; no future ACC work is scheduled. P256/WebAuthn and secp256k1 backports are outside this initiative. The separate ledger8 block-time lowering gap is #442. Existing DID curve-name metadata remains application data, not an implementation of those signature algorithms. Historical coverage measurements are not coverage percentages for this new slice.

“ACC PR177 — Typed Jubjub recording delivery” (historical vault reference; not bundled here) · “ADR-0308 — Record typed Jubjub value operations and a point Cell write” (historical vault reference; not bundled here) · [Testing generated contracts with ContractLab](contract-lab.md)

## Historical provenance

The generic fixture originated during ACC research, so some retained filenames, Rust module aliases and receipt titles still contain `acc`. Those names identify historical evidence; they do not schedule ACC adoption or widen this profile. ADR0314's earlier deferral was superseded by ADR0316 removal.
