# Ordinary terminal composition expansion

The exact source is `examples/rust_backend/effect_plan_terminal_composition.compact`.
Build-time generation uses its retained schema-20 IR and the current default
backend. `update` composes two declared Field Cells, a native pure hash helper,
and a terminal lexical return. This crosses the former one-Cell admission rule.
`optional_write` has no read and can select an empty public program; it crosses
the former static read/write-count rule.

The runtime tests compare native and recorded execution with an independent
manual recording program, including exact operation order, execution gas, replay
gas, state, effects and return. Both optional-write branches are exercised.
An empty recording must fail preparation with `EmptyTranscript`, without a dummy
query or write. The test key carrier is only for proving that early refusal and
is not a valid proof key.

These are explicit capability expansions, separate from old-domain inclusion.
This fixture has no proof claim. Source-matched strict ledger proofs use the
existing `terminal-lexical-return-oracle` contract; the root-let adapter retains
its existing unbalanced smoke policy.
