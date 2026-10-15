# Terminal lexical return oracle

`build.rs` generates the fixture from the exact retained
`terminal-lexical-return-schema20-ir.json` using the current backend. The source
is `examples/rust_backend/terminal_lexical_return_oracle.compact`. This avoids
qualifying only a stale checked-in generated implementation after admission
changes.

The existing runtime test retains all 13 TypeScript oracle rows, including
lexical returns, nested calls, witness ordering, assertion failure and zero gas.
The proof smoke adapter `--terminal-lexical-return` checks the exact retained IR
before loading keys. It proves `two`, `three`, `nested` and `observed`, balances
with separate Dust, and applies under default strict ledger validation. Earlier
local state is seeded natively and is not represented as a proved transaction.
