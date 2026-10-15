# Opaque Cell Unit interfaces

This fixture compiles the unchanged `examples/rust_backend/opaque_cell_inputs.compact` whole IR at crate root. No generated Rust text is rewritten. It exposes the actual generated library to both integration tests and the selected `--opaque-cell-input` proof adapter.

## Coverage

With `ledger-transaction`, three tests qualify:

- Three canonical OpaqueBytes values: empty, one byte, and interior-zero bytes.
- Three nested Envelope values: empty payload, interior-NUL Unicode text, and Unicode/interior-zero bytes.
- Four invalid trailing-zero leaves: two direct bytes and two nested text/bytes cases, each through native, recorded and observed APIs.

Successes compare native/recorded/observed/manual Cell writes, full state/effects/execution gas, unchanged private state, empty private FAB, exact public verifier program, replay state/effects and independently replayed gas. Preparation checks original argument atoms/alignment and Unit output with an official synthetic test verifier. This is preparation testing, not a proof run. Invalid observations retain the existing canonical-input refusal policy.

The old owned-cells fixture retains its native constructor/input controls and setText recording tests. Existing public-products tests qualify Unit/tuple11/vector9/wide argument codecs on **Struct-returning** roots; they are not mislabeled as Unit-return coverage.

No new TS oracle capture is claimed here. Source/native references are preserved. The separate proof adapter binds the whole IR before reading keys and proves `setBytes([1,0,2])`, with changed-binding rejection and default-strict funded application. Its constructor state is supplied; deployment is not proved. Execution receipts are held in the milestone evidence directory.
