# Troubleshoot a Rust consumer

These diagnosis steps follow the inspected API and compatibility policy. Each linked receipt states what was actually executed; the instructions themselves are not additional regression evidence.

| Symptom | Check and fix |
|---|---|
| Two apparently identical `Field`/context types do not match | Inspect Cargo package identities. Use the generated crate's `runtime` re-export; make generated contract and ContractLab resolve the same runtime source directory. Do not patch generated type declarations. |
| Missing compatibility record or source/version mismatch | Select the matching runtime/macros pair from the compiler source distribution. Old roots without the record are intentionally rejected. Copying new metadata onto old source is not migration. |
| Generated ABI assertion fails | Keep the old contract with its exact historical runtime, or regenerate with the current matching pair. Do not change the numeric assertion. |
| Backend `RenderError` match stops compiling | Backend0.2 is non-exhaustive. Keep specific arms and add a wildcard/fallback. Handle `ResourceLimit` explicitly when useful; do not treat Display text as a machine protocol. |
| Fixed byte or bounded integer conversion fails | Validate transport length/range and construct the exact generated carrier. Avoid JS Number roundtrips for large integers. Bounds errors are runtime `Result` failures; Rust types do not make arbitrary external input valid. |
| Typed argument still produces an assertion error | Check the corresponding Compact rule: authorization, active state, version, membership or schema constraints remain dynamic. |
| A source circuit owns the name `recording` | The convenience accessor is omitted. For a witnessed contract, use the conditional standard `From<&Contract<W>>` conversion described below. This does not make a separately unavailable `*_call` method available. |
| No recorded or `*_call` method | Inspect the actual per-export schema3 capability report and `proof_required`. Pure exports can be not-applicable; unsupported stateful recording needs a source/capability diagnostic. Enable `ledger-transaction` only when the call API exists. |
| Witness method signature differs | Implement the generated `TryWitnesses<Private>` trait and its exact argument/result types. The generated `LedgerView` provides the typed witness query surface. |
| `LabError` only says execution failed | Default diagnostics redact potentially private payloads. Inspect `execution_error()` only in a context allowed to see the underlying data. Text does not authenticate script-vs-application origin. |
| Script mismatch/exhaustion | Check answer order and exact arguments. Mismatch/exhaustion leave the queue/journal unchanged; a matched invocation consumes an answer, including a scripted error. Owned lab rollback and external witness activity are separate. |
| Snapshot restore refuses | Check source/generated identity, ABI, ledger label, state mode and environment. Use the correct snapshot/runtime; do not overwrite metadata to bypass checks. |
| Execution and replay gas differ | Execution sums generated queries including witness reads; replay runs the sealed public program as one VM query. Query grouping also changes cost. Do not subtract them as universal witness cost or add them as a fee estimate. |
| Keys or proof materials are missing | `--skip-zk` does not generate keys. Use the matching ZKIR/proof-provider workflow and qualified public material; replay alone requires no real proof. |
| Compiler resource refusal | Keep the resource, observed value, limit and source diagnostic in a minimal reproducer. Supported limits are bounded safeguards, not a universal process-memory/stack guarantee. |
| A different source/lock or platform fails | Compare exact toolchain, feature graph, lock and source identities with the retained receipt. Registry emission does not assert package availability; WASM is a bounded prototype. |

For a bug report retain: source/import closure, compiler identity, compatibility and capability reports, selected runtime source identity, Cargo.lock, target/features, exact failure and a minimal reproducer. Keep private witness/application values out of shared logs. Distinguish generation, compilation, native execution, replay, proof verification and network failures.

## When a circuit is named `recording`

Normally, `contract.recording()` borrows a witnessed contract's existing witnesses for its recorded methods. If an exported Compact circuit itself is named `recording`, that circuit owns the inherent method name and the convenience accessor is omitted. ADR0331 supplies a conditional standard `From<&Contract<W>>` implementation for the borrowed recording handle instead.

The following function is the same conversion used by the retained external collision consumer. Here `ledger` is an alias for that generated crate's `ledger_contract` module; it requires a witnessed generated contract with this specific collision:

```rust
pub fn handle<W>(contract: &ledger::Contract<W>) -> ledger::recorded::BorrowedContract<'_, W> {
    <ledger::recorded::BorrowedContract<'_, W> as ::core::convert::From<&ledger::Contract<W>>>::from(
        contract,
    )
}
```

The handle borrows the witnesses already owned by `contract`. The fully qualified trait syntax avoids interpreting `from` as an inherent generated circuit method. This fallback is emitted only for the collision case; use `contract.recording()` for ordinary witnessed contracts. A contract with no recorded witnesses keeps the public `recording` field as its access path when the accessor collides.

Use the returned handle's actual generated methods and consult its capability report. A different exported circuit can separately occupy a generated `*_call` name; constructing the handle does not override that suppression. Observed-call methods still require `ledger-transaction`.

The ADR0331 external receipt contains actual handle-construction tests in default and transaction-feature builds, plus recorded/replay/observed-method compile controls. Those controls do not execute a transaction or establish a proof or trust guarantee. This excerpt is navigation to that tested API, not a new standalone tutorial or a new test run.
