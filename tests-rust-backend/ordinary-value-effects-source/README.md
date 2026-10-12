# Ordinary value effects

This fixture compiles the complete, unchanged `frontend-return-plans/value_effects.compact` schema-20 IR at crate root. `provenance.json` binds the original source and full retained IR; no action or signature is rewritten.

The new `subtracted` recording route is checked against native execution, explicit integer expectations and a manual official Counter-slot recording. Five successes cover selected subtraction and both short-circuit paths; two zero-underflow controls preserve the generated error across native/recorded/observed calls. Every successful call checks state, effects, execution gas, replay gas against manual replay, and prepared input/output FAB.

Counter state and a verifier carrier are supplied for local preparation. No constructor proof, fresh TypeScript capture or cryptographic proof is claimed here.
