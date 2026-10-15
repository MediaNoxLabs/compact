# Receive composition source fixture

Generated at build time from the complete retained IR of
`examples/rust_backend/effect_plan_receive_composition.compact`.

The source combines a Boolean Cell write, a declared witness and a conditional
standard-library receive. The prior whole-shape receive recorder rejects it.
Tests compare native execution, current recording and independently composed
runtime operations, including witness input/state order, success/skip branches,
nonzero and wide coin amounts, failure, transcript, intents and public replay.
The fixed witness error produces no recording result; no transactional rollback
of external witness side effects is promised.

This is a runtime-qualified compositional expansion. It does not qualify new
proofs, network submission or balance validity for the wide coin amount.
Original receive proofs use their unchanged source and separate fixture.
