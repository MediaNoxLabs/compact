# Nested qualified Set source fixture

This crate builds the current Rust backend output from the complete retained IR
of `runtime-rs/tests/fixtures/qualified-coin-set-chunked.compact`. The original
source places `pot` at physical path `[1,14]` and keeps the `insert_coin` and
`contains` APIs. Its public recording capabilities must both remain available.

The proof-smoke `--qualified-coin-set-chunked` command binds this crate to the
shared qualified Set scenario. It checks the complete supplied IR before state
creation or key loading. The same scenario also serves the separate original
flat fixture; no generated Rust modules are renamed or patched.

The scenario includes an intentionally unfunded output: default balancing must
reject it, and a separate diagnostic deliberately disables balancing. The later
funded scenario uses default strict validation and checks ledger application,
input nullifier, output commitment/index, qualified Set membership and replay
rejection. Only that later scenario is evidence of a funded strict transaction.
Source identity checks do not authenticate proving-key provenance; qualification
must pair fresh or independently authenticated matching artifacts.
