# ADR0331 F4 — reachable witnessed recording handle

The emitter now conditionally implements `core::convert::From<&ledger_contract::Contract<W>>` for `recorded::BorrowedContract<'_, W>` when an exported circuit occupies `recording`. The implementation borrows the existing private witnesses. No new inherent helper name, runtime operation or ABI is introduced. Noncolliding generation retains the ordinary accessor and no extra From implementation.

The existing witnessed render test now asserts both conditional behavior and private-IR `from` shadowing alongside `recording`/`r#recording`. It passed. Owned Rust formatting and whitespace checks pass.

## External evidence

- Ordinary valid Compact `recording.compact`: generated without edits; separate consumer constructs the borrowed handle using fully qualified trait syntax, and typechecks replay plus observed-call use.
- Artificial private schema20 IR: duplicate source-derived `recording` declaration named `from`, emitted by the actual backend stdin renderer into a new crate. Separate consumer constructs via fully qualified trait syntax even with the inherent `from` circuit, and typechecks both recorded and observed methods.
- Both consumers: one actual handle-construction test passes in default and ledger-transaction modes; strict all-target Clippy passes in both modes. Recorded/observed generic methods are compile controls, not transaction execution claims.
- Each lock qualifies all324 registry packages against root versions/source/checksums. Metadata shows exactly one runtime source identity in each consumer and no checkout dependency or testkit dependency. Generated runtime/macros Rust source equals current checkout byte-for-byte; unused staged testkit is explicitly excluded.

## Corrected experiment

The first attempt used `from` as an ordinary Compact circuit name. The frontend correctly rejected it as a keyword before generation. That source, consumer, command logs and receipt remain under `initial-from-source-refusal/`. It was replaced by the valid-source reachability control and explicitly private-IR shadowing control above. No generated Rust was edited.

## Limits

No proof, network, transaction execution, ownership/trust-boundary or performance claim. This fixes compiler facade reachability only. Runtime and generated recorded algorithms are unchanged. The stopped ADR0285 lane was not touched.

## Files

`receipt.json`: ordinary source consumer, commands, compiler/source hashes, lock qualification.
`private-ir/receipt.json`: artificial input, renderer/generated hashes and compiled controls.
`generated-runtime-source-comparison.json`: current runtime/macros Rust source match.
`consumer/src/lib.rs` and `private-ir/consumer/src/lib.rs`: exact external usage.
