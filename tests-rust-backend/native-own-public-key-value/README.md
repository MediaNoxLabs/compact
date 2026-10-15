# native-own-public-key-value

The maintained native tests preserve the ledger-8 TypeScript oracle comparison. The recording tests cover newly generated recorded and observed APIs with zero and nonzero configured keys, exact missing-key errors, one private key output, unchanged state and effects, and an empty public transcript.

Run this package with `--all-features` to exercise the observed-call API. Its preparation must return `EmptyTranscript`: a private native witness alone does not produce a supported public proof program. The empty serialized verifier carrier used in that negative test is not a proving artifact. These tests do not claim proof generation or wallet ownership authentication.
