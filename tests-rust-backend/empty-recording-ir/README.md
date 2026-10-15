# Empty recording: synthetic IR consumer

This is the exact complete 8,385-byte `helper-no-public` typed-IR control from the guarded-pure retirement research. It is **not a frontend-produced source fixture**. Starting from complete corpus074, the derivation replaces the Counter continuation with a declared zero-argument Unit helper containing `assert(true)`. The retained JSON is canonical complete input, not generated Rust.

`build.rs` renders that input with the current backend without rewriting emitted code. Signed597 baseline actual owner was `guard.guarded-struct-pure`; native, recorded and observed APIs were accepted. The checked candidate was already supported.

Two tests cover fresh/unchecked-age success, future/expired guard failure, exact unchanged state, zero public operations/gas/private outputs, and direct/observed typed `EmptyTranscript` preparation refusal. An installed serialized empty verifier carrier is only for observation identity; it is not a proving key.

Run locally: `cargo test -p compact-rust-empty-recording-ir-fixture --features ledger-transaction`.

The separate `empty-recording-witness-source` fixture supplies genuine frontend evidence. Neither fixture claims an empty ledger transaction can be proved or applied.

Input SHA256: `6d0e78dee3ec3564d6ed8c7dbcaedeee3f0367eb07a732c203f7c35971709018`.
