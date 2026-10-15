# Direct context query and persistent commitment

Current backend generation from the exact source
`examples/rust_backend/effect_plan_context_query.compact`; retained schema20 IR
contains a direct outer `persistent_commit` in `direct_commit`, with one
`kernel_self` inside its value tuple and another in its opening. A native pure
helper cannot mask the new recording operation.

Runtime tests compare native and recorded results, state, effects, execution gas,
private data, and the exact public program against an independent composition of
official recording-frame address queries and the ledger-backed persistent
commitment. Four addresses exercise the returned address-dependent value. Replay
gas is compared against a separate manual-program replay, not execution gas.
Both public reads return the same immutable address: the test checks their count
and complete program, while source/IR and compiler tests establish operand order.

`no_context` uses a witness-supplied opening, preserving one private output and
private-state transition without inventing public queries. Tests cover witness
refusal and the explicit `EmptyTranscript` preparation boundary. The refusal test's
verifier-key carrier is not a cryptographic proof key.

```sh
cargo test -p compact-rust-effect-plan-context-query-source-fixture --all-features
```

These are finite source/runtime qualifications and an interface expansion beyond
the historical context-query grammar. No proof is claimed for this small source;
the exact original microDAO token source supplies this slice's real proof receipt.
