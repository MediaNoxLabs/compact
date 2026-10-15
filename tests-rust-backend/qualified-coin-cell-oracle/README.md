# Qualified coin Cell compatibility fixture

The build script renders the complete retained schema-20 IR with the current Rust backend. `RETAINED_IR` identifies the input used by the proof adapter's whole-IR pairing guard. The older schema-16 receipt remains unchanged.

## Runtime coverage

The three existing tests retain the independent TypeScript fixtures and their allocation/replacement, nested runtime-slot path, missing/wrong-recipient, alignment, and VM rejection checks. Two additional tests compare current generated native and recorded execution with an independently assembled runtime `CellSlot`/`RecordingFrame`:

- Eight write/read combinations cover user and contract recipients, coin values zero and `u128::MAX`, allocated indices zero and `u64::MAX`, and empty/replaced storage. Unselected recipient carriers are deliberately nonzero. Full transcript, state, effects, gas, private outputs, unchanged intents, and replay are compared.
- Three allocation refusal cases cover absent commitment, a different selected user recipient, and a different selected contract address. Native, recorded, and manual errors agree, and the held query snapshot is preserved.

These five tests are current-backend compatibility and runtime evidence. The `write_coin` export has a Unit result with coin/recipient parameters and can still use the legacy recorded emitter. It does not independently qualify the new effect-algebra operation. The separate effect-plan qualified-coin source fixture asserts algebra ownership. The nested TypeScript test here uses a runtime slot directly; it does not establish frontend nested-path generation.

## Proof boundary

The proof adapter first compares the entire supplied IR to `RETAINED_IR`. This is input pairing, not authenticated key provenance. The adapter retains an unfunded diagnostic that first requires default balancing rejection, then deliberately disables balancing. Its later funded lane uses default strict validation, real funding and fee balancing, ledger application, allocated output commitment/index, spent nullifier, and replay refusal. Only the funded lane counts as strict successful proof/application evidence. No new proof run is implied by this fixture refresh.

Run locally from the repository:

```sh
cargo +1.99.0 test -p compact-rust-qualified-coin-cell-oracle-fixture --all-features --offline
```
