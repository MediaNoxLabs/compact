# Current-generated original Coracle

`build.rs` generates this fixture from the complete retained
`coracle-source-schema20-ir.json` with the current default backend. That IR is
fresh compiler output from unchanged `test-center/test-contracts/coracle.compact`.
All existing types, pure functions, four public recorded APIs and tests remain
available. Historical per-export IR files remain separate dependency projections;
they are not substituted for the complete source artifact.

For the unit-action family retirement, the focused runtime gate is
`--test guess`: its 26 existing TypeScript/native/recorded scenarios include both
colors, selected witness order, identity/board/turn/position failures, gas limits
and empty-Maybe behavior. No additional whole-game qualification is implied.

The `--coracle-guess` proof adapter compares the exact complete retained IR before
reading keys. Its fresh keys are generated only for the unchanged source's
`guess.zkir`; this slice does not generate or claim proofs of every export.
Both colors execute from explicit prior states, prove, verify and ledger-apply,
then reject a repeated same-color move against applied state. The policy remains
the existing unbalanced smoke policy. Seeded state is not a claim of a funded
or historically proved game setup.
