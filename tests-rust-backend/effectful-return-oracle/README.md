# Effectful-return migration oracle

This fixture checks the existing `choose` circuit while its recorded implementation
moves from the legacy shape planner to the compositional effect plan.

- `build.rs` generates the package's Rust API from the retained, compiler-produced
  `tools/compact-rust-backend/tests/effectful-return-schema20-ir.json` on each backend
  rebuild. The backend build dependency requests no experimental feature.
- `fixture.rs`, the package library root, includes that fresh output at the crate
  root so generated `crate::ledger_slots` references keep their ordinary meaning.
- `lib.rs` remains the current checked-in generated snapshot used by the existing
  fixture regeneration gates; those gates may refresh it normally.
- `legacy_reference.rs` is the byte-preserved generated output from before the
  migration. It is an intentionally frozen reference, not a regeneration target.
- `tests/effectful_return_oracle.rs` retains the two TypeScript oracle tests and
  now runs them against fresh default output.
- `tests/migration.rs` includes the frozen output at its own integration-test crate
  root and compares it with the fresh fixture: results, public and private state,
  ordered witness observations, effects, private outputs, public transcript, all
  execution gas dimensions, replay state/effects/gas, and witness error prefixes.
  Its 120 success cases include field wraparound and both selected branches; four
  callback-refusal cases check selected and skipped callbacks.

The existing `compact-rust-proof-smoke --effectful-return <artifacts>` route uses
this package and therefore proves the fresh generated implementation. Its two
cases prove, verify and ledger-apply both selected branches with the shared smoke
policy (balancing disabled). Key artifacts must come from the matching Compact
source, `examples/rust_backend/effectful_return_oracle.compact`.

Run the package by itself to check the default emitter without Cargo feature
unification from the separate experimental fixture:

```sh
cargo test -p compact-rust-effectful-return-oracle-fixture
```

The retained legacy source is test evidence. Do not refresh it when regenerating
ordinary fixture snapshots. No extra source-language coverage is claimed by the
parameter matrix.
