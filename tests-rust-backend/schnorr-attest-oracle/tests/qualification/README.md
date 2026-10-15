# Checked Schnorr helper qualification

The original three tests and frozen TypeScript capture remain unchanged. These two grouped tests exercise the generated native, recorded and observed APIs plus a manually composed path using declared ledger slots and official runtime arithmetic/curve primitives.

Success checks cover both `verifyAttestation` and `acceptAttestation`: exact public query order, execution gas, independently replayed query gas, private reduction FAB, state, and prepared input/output FAB. The expected challenge is reconstructed from the original TS reduction output. A separate provider reads the accepted counter through its witness meter, proving the helper observes zero before the accepting root increments it. Its additional read cost is compared across Rust/manual lanes, not attributed to the unchanged TS callback.

Failure checks cover the closed guard (no callback), provider refusal, quotient 116, incorrect quotient/remainder reconstruction, identity announcement, identity ledger key, and an incorrect signature response. The identity points are valid curve carriers; these cases do not claim testing arbitrary malformed/off-curve points. Once-only callback challenge/private/counter observations must agree in all four lanes. The accepting counter remains downstream of successful verification.

An empty serialized verifier carrier exercises preparation only. Actual proof verification, changed-binding refusal and strict funded ledger8 application belong to the separate source-bound proof adapter. Constructor state is supplied; these tests do not prove deployment.
