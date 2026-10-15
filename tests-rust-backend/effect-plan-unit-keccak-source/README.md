# Keccak through local Unit calls and a lazy product return

This genuine Compact source covers a local helper that checks a Keccak digest,
then authorizes a Boolean ledger write. A third export returns a named `Digest`
product and selects its witness payload lazily. All witnesses read the initial
`active` cell, making callback order, private-state threading, and metered reads
observable.

## Compatibility boundary

The source frontend requires `--feature-zkir-v3`. Default ZKIRv2 rejects Keccak;
this fixture makes **no ledger8 ZKIRv2 or Keccak proof claim**. It tests native,
recorded, observed and manually assembled execution; verifier replay; prepared
call FAB; and fresh same-source official TypeScript execution. The separate Unit
composition fixture covers the selected ledger8 proof scenario.

At signed baseline `a52b324e`, `close` and `lazy_close` were already recorded via
`selector.unit-composition`, while the checked candidate refused `Keccak`.
`digest` was not recorded (`legacy.action-dispatch`). The original small reviewer
control now selects `algebra.kernel` after #581; the Unit selector still exists.
Current measured owners are `algebra.kernel` for `close` and `lazy_close`, and
`algebra.composite-values` for `digest`. This prerequisite does not claim the
broad selector has been retired.

The first derivative used `default<Bytes<32>>` on the unselected branch; the
checked candidate refused that unrelated default expression. The retained source
uses an explicit `(0 as Bytes<32>)` literal with the same intended zero bytes.
The failed source revision and baseline/capture receipts are preserved separately.

## Independent answers and exact cases

The three test functions map all 30 named TypeScript cases uniquely: 17 successful
executions and 13 assertion/provider refusals. Configuration checks include the
circuit, original arguments, input vector, selection, mismatch answer,
authorization answer, and configured failing witness. Equal-output lazy cases
remain distinct cases.

The manual reference uses frozen official TypeScript digest answers, not the
Rust hash implementation under test. Compact hashes canonical FAB atom bytes:
`Bytes<32>` filled with zeros becomes an empty atom, so its answer is
`Keccak(empty)`. All-255 and increasing-byte vectors also have pinned answers.
The TypeScript capture independently checks all three answers using its official
runtime before running the source contract.

Assertions cover results, state/effects, original observation immutability,
private state and transcript, witness order and exact refusal prefix, public
verification instructions, replay, original input/output alignment, and execution
gas including witness ledger views. Replay gas is compared against the manual
replay, since replay and execution need not have the same query cost.

Unselected `lazy_close` calls succeed locally without invoking providers or
writing ledger state. Their observed preparation correctly returns
`PrepareCallError::EmptyTranscript`; the capture's `rustPreparationExpectation`
is an explicit Rust expectation, not a measurement performed by TypeScript.
Other successful cases prepare a nonempty ledger call. Random verifier material
is used only for preparation and is not represented as a real circuit proof.

## Reproduce

Generate this source into a fresh directory using the matching compiler:

```sh
compactc --target rust --target ts --feature-zkir-v3 --skip-zk \
  examples/rust_backend/effect_plan_unit_keccak.compact /path/to/output
node tests-rust-backend/effect-plan-unit-keccak-source/oracle/capture.mjs \
  examples/rust_backend/effect_plan_unit_keccak.compact \
  /path/to/output/contract /path/to/official/compact-runtime \
  tests-rust-backend/effect-plan-unit-keccak-source/oracle/typescript.json
cargo test --locked -p compact-rust-effect-plan-unit-keccak-source-fixture \
  --features ledger-transaction
```

`oracle/typescript.json` binds the exact source, full IR, generated JavaScript,
capture script, runtime build, package lock and WASM package through SHA-256
receipts. Build-time generation uses the exact retained full IR, and requires
recorded and observed support for all three exports.
