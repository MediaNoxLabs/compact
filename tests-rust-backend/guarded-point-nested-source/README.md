# Nested guard source fixture

Generated at build time from the exact retained IR of
`tools/compact-rust-backend/tests/proof-regressions/guarded-point-read/nested.compact`.
Its point read remains inside nested source branches. No synthetic exports or
source rewrites are used by its proof adapter.

`compact-rust-proof-smoke --guarded-point-nested <matching compiler output>`
checks retained IR before key loading, then qualifies all four Boolean guard
combinations with native/recorded agreement, independent expected result/state,
direct/observed preparation agreement, the official preimage checker and
proof/verification/ledger application under the shared unbalanced smoke policy.
The outer false cases leave the inner expression inactive, including when the
inner input is true. Finite cases are not a general proof of guard correctness.
