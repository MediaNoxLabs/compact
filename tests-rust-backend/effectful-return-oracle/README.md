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

## Wide-value inclusion probes — #510 / ADR0370

`build_support/wide_values.rs` generates a separate `wide_contract.rs` for
`tests/wide_values.rs`. It retains the actual `choose` body from
`examples/rust_backend/effect_plan_wide.compact` and its saved schema20 IR, then
adds three explicitly synthetic IR exports. The original package API, snapshot
and frozen legacy reference remain unchanged.

The probes exercise values above u128, the maximum current 248-bit value,
equality, Field coercion into writes/returns/witness arguments, witness failure
and skipped callbacks. Tests compare native and recorded execution exactly,
including all execution gas dimensions. An independently constructed public
program checks transcript and replay state/effects/gas; per-query execution gas
is not equated with batched public replay gas.

The combined probe contract has no matching proof artifact set. Its runtime
checks are not a prove/verify/apply claim. The original proof command above
continues to use only the original source and freshly generated original API.

```sh
cargo test -p compact-rust-effectful-return-oracle-fixture --test wide_values
```
