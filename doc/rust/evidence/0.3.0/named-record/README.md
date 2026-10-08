# Application-owned named inputs for passport age predicates

`src/lib.rs` is ordinary application Rust. The maintained compiler still emits the positional seven-argument function. This wrapper makes the call site easier to inspect without changing generated code, the runtime, or source semantics.

The isolated `Cargo.toml` aliases the byte-identical maintained passport library as `passport`. Its manifest relocates one runtime dependency into `sdk/runtime-rs`; the generated `lib.rs` is untouched. The runtime and macros are byte-identical copies of the SDK staged by ADR0324. Metadata verifies exactly one runtime identity and no path dependencies outside this directory.

## Reproduce the checked example

From this directory, with the qualified included `Cargo.lock` and Rust 1.99.0:

```sh
CARGO_INCREMENTAL=0 cargo +1.99.0 test --offline --locked -j4 --test age_predicate
CARGO_INCREMENTAL=0 cargo +1.99.0 clippy --offline --locked -j4 --all-targets -- -D warnings
rustfmt +1.99.0 --edition 2024 --check src/lib.rs tests/age_predicate.rs
```

The original local run reused `CARGO_TARGET_DIR=/tmp/compact-m2-bench-target`; setting a different target changes only build-cache reuse. Offline commands require the pinned dependencies already present in Cargo's cache. On a fresh machine, fetch the included graph with `cargo +1.99.0 fetch --locked` first.

`tests/age_predicate.rs` contains the complete typed construction example. It preserves the maintained uint32/bytes32/civil-date parsing and the seven field values/order, then calls `AgePredicateInputs::evaluate()`. All 25 independent stored TS age scenarios pass: 6 successes and 19 exact captured error strings. The test also requires upstream and branch capture rows to match. This is one table-driven integration test, not 25 test functions.

The root lock was copied, pruned by offline metadata, and qualified before locked tests. All 324 registry identities/checksums match the repository lock. This recipe does not regenerate a lock with unconstrained newer versions.

## Limits

Field names improve readability and completeness. They do not prevent assigning a birth date to a same-typed current-date field. Construct values before the record when input evaluation order matters. Application role wrappers or validation remain explicit application policy.

This check ran locally on macOS ARM64 using existing captures. It does not refresh TS oracle generation, generate new compiler output, qualify proof/network execution, establish runtime performance, or promote named Args into compactc. See `evidence/receipt.json`, `evidence/source.json`, and `evidence/lock-qualification.json` for exact identities and commands.
