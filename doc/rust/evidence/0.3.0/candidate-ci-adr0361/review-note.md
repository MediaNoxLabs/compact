# ADR0361 — Strict Clippy and bounded candidate CI

Candidate: `2c798d2dc5f94ad13e89610068035ed70d94f469`, signed GPG Good/DCO. Issue #498, parent #364. Only two test function lint expectations changed; root removed those exact eight lines in memory and compared the complete file to17433b40. All test cases/assertions and all production code are identical.

The original17433b40 Linux failure and local reproduction both report25 result_large_err warnings in test closures retaining the official ledger error. Local fix passes four path tests, strict four-owner all-target/all-feature Clippy, formatting and diffcheck.

[Remote run37742764841](https://github.com/MediaNoxLabs/compact/actions/runs/37742764841) passed both jobs. Retained artifact sources match candidateSHA and current locks. Root verified every artifact hash in both result.json files. Core Rust1.99:742 passing test cases across63 result blocks, formatting and strict Clippy. This includes doc-test cases and is not a coverage percentage. Rust1.88: selected backend/generated Jubjub workspace consumer and standalone backend checks passed; this is not a whole-workspace MSRV claim. A separate local all-feature testkit1.88 check is running because ProofLab is new.

The first20b2b1b8commit had a bad GPG signature and was replaced under an exact remote lease by the same-tree signed2c798d2d;20b2b1b8is not an accepted source candidate. Its CI was cancelled by the branch concurrency policy. Full local/proof/host/package qualification remains open.

Archive: [[ADR0361 — Strict Clippy and bounded candidate CI.zip]]
SHA256: `82624dd30644c613acba0306babc1b28a451b49b08e1a6a28bc98d9176662828`
