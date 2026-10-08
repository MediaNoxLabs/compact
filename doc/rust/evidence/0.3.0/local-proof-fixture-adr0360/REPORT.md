# ADR0360 — Official provider test fixture receipt

2026-10-08. Local Rust 1.99.0; ledger8 pinned dependency graph. Issue #497, parent #496. Scope: testkit only; no generated/runtime behavior changes.

Commands use CARGO_INCREMENTAL=0 and CARGO_TARGET_DIR=target/compact-rust-parity-gate:

- cargo +1.99.0 test -p midnight-compact-testkit --features proof --offline: 25 unit/integration + one then-existing doctest passed.
- cargo +1.99.0 test -p midnight-compact-testkit --features proof --doc --locked --offline: 2 doctests passed after adding the official-provider example.
- cargo +1.99.0 test -p midnight-compact-testkit --no-default-features --locked --offline: 21 unit/integration + 1 doctest passed.
- cargo +1.99.0 clippy -p midnight-compact-testkit --all-targets --features proof --no-deps --locked --offline -- -D warnings: passed.
- Scoped rustfmt and git diff --check: passed. Existing unrelated documentation changes excluded and preserved.

Four new tests exercise official provider composition, real assertion IR checks and refusal paths. Check is not proof generation; the successful prove example is compile-only. No network or successful proof run performed for this narrow fixture.

## Convenient usage

Enable testkit feature `proof`. Use ContractLab for native/recorded generated calls. Use ProofLab::new(resolver, params), then lab.provider(rng) to obtain the upstream LocalProvingProvider. Import official ProvingProvider to call check/prove/split directly. Use lab.verifier(key, limit) when a decoded key is needed for call preparation. Errors distinguish unavailable material, resolver I/O and verifier decode failure. Runtime and canonical ledger/zk objects remain unchanged.

No remote CI run was awaited; final milestone qualification is still a separate task. Source-level independent review found no outstanding issue. Research in external product repositories remains outside Compact's notes/code.
