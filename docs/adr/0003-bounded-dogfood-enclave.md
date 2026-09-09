<!--
This file is part of Compact.
Copyright (C) 2026 Midnight Foundation
SPDX-License-Identifier: Apache-2.0
-->

# ADR 0003 — A bounded third-party dogfood enclave under `examples/dogfood/`

- Status: Accepted
- Date: 2026-09-10
- Scope: `examples/dogfood/**`, `header_config.json`,
  `tests-e2e-rust/contracts/digital-passport-credential/`,
  `.github/workflows/rust-runtime-test.yml`, `.github/workflows/build-compiler.yml`,
  AGENT.md §1
- Supersedes / relates to: the corpus de-branding of commit `08decb1` (**for
  this enclave only** — its stance remains in force everywhere else), ADR 0001
  and its follow-up note, the did-05 source-of-truth pattern (commit `09263a9`)

## Context

Commit `08decb1` removed the last third-party material from the fixture
corpus: the vendored `did-05` contract and the hand-imported
`digital-passport` crate. That was correct at the time and remains correct
for what it removed:

- the `digital-passport` crate had **no `.compact` source in the tree**, was
  never in the `codegen_regression` FIXTURES table, and had no test consuming
  it — its only gate was "it compiles and is rustfmt-clean" (ADR 0001's
  follow-up note closed the resulting registration gap *by removal*); and
- the corpus needed to be neutral — free of upstream branding — to be
  upstreamable, with neutral fixtures (asset-registry, schnorr-attest)
  covering every codegen path the identity contracts had been the sole
  source of.

But the removal also ended **dogfooding**: nothing in the repo proved
anymore that `compactc --target rust` compiles the real, named, production
contracts this toolchain exists to serve. Synthetic fixtures cover *shapes*;
they cannot cover *contracts we do not author*, whose idioms drift from our
own. That loss was not hypothetical: the day the dogfood enclave's first
tenant landed, it exposed two genuine rust-backend gaps that every neutral
fixture had passed through —

1. conditional (ternary) expressions in sub-expression position failed with
   `no walker shape matched` at 5 sites (`fix-ternary-expression-codegen`,
   0.31.117); and
2. comparison operands of mixed minimal widths emitted non-compiling Rust —
   13 `E0308`s across `assertCivilDateMatchesEpochDays` and the age
   predicate (`fix-mixed-width-operand-casts`, 0.31.118).

## Decision

Reintroduce third-party material, but as a **bounded dogfood enclave**:
`examples/dogfood/` — a new category under `examples/`, partitioned from the
neutral corpus — whose first tenant is
`examples/dogfood/digital-passport-credential/`, the upstream
`midnight-verifiable-credential-digital-passport` package `src/` tree
vendored **verbatim** (upstream Apache-2.0 headers intact) at pinned
revision `cdeb860b`, plus its hermetic `core-compact-staging/` (the 15 npm
`dist/` files the compiler's relative include needs).

The enclave differs from what `08decb1` removed in every dimension that
mattered:

| | removed `digital-passport` crate | removed did-05 vendoring | dogfood enclave |
| --- | --- | --- | --- |
| source in tree | none | verbatim `.compact` | verbatim `.compact` + staged core |
| registered in FIXTURES | no | yes | yes (nested path row) |
| provenance documented | none | none | `PROVENANCE.md` (URL, rev, license, refresh) |
| purpose | incidental import | codegen-coverage | dogfooding real upstream code |
| location | inside the neutral corpus | inside the neutral corpus | partitioned `examples/dogfood/` |

So this **supersedes `08decb1`'s stance for this enclave only**: it is a
deliberate, documented exception with a different purpose (standing proof
against real production source, not codegen-path coverage — the neutral
corpus remains the coverage corpus), not a reversal of the de-branding.
Everything outside `examples/dogfood/` stays third-party-free; the boundary
is enforced socially by AGENT.md §1 and this ADR, and mechanically by the
`header_config.json` `excluded_directories` entry (`dogfood` prunes the
enclave from license-header validation at any depth, so upstream headers
survive verbatim while every other file still requires the Apache-2.0
header).

### Pinning policy

- The pin is **exact** (revision `cdeb860b`, core package
  `@midnight-ntwrk/credential-compact@0.1.0-rc3`) and recorded in
  `examples/dogfood/digital-passport-credential/PROVENANCE.md`, together
  with a byte-verification and refresh procedure a reader can execute
  without this ADR.
- Refresh is a **manual, explicit human act** — nothing in CI or tooling
  tracks upstream or auto-syncs. A refresh updates the vendored bytes and
  the recorded revision **in the same commit**, and re-runs the byte-parity
  gate (`cargo test -p tests-e2e-rust rust_codegen_byte_parity`) so the
  committed crate is exactly what the current compiler emits for the new
  bytes.
- The vendored tree is **never locally modified**. If the branding cost ever
  outweighs the dogfood value, a neutralization pass remains possible and
  must be recorded in PROVENANCE.md and here as a deliberate divergence.

### Gating (how the dogfood is actually dogfooded)

The crate is registered like any first-class fixture — root workspace
member, `tests-e2e-rust` dev-dependency, FIXTURES row — so
`codegen_regression` byte-parity, `cargo fmt --all`, and the build gates all
cover it locally and in CI, plus: an explicit
`cargo clippy -p compact-contract-digital-passport-credential` step in
`rust-runtime-test.yml` (dev-deps build `--cap-lints allow`, so nothing else
would lint it) and a both-target codegen-only smoke pair in
`build-compiler.yml`. Rust↔TS behavior parity for a representative subset
(civil-date helpers incl. every ternary site and assert-fail path, one
issuance/presentation/verification round-trip) is pinned by committed
captures (`tests-e2e-rust/fixtures/capture-digital-passport-credential.mjs`
→ JSON) and an executing test
(`tests-e2e-rust/tests/digital_passport_credential.rs`).

## Consequences

### Positive

- Compiler regressions against real upstream source surface **before**
  upstream hits them, not after — as demonstrated twice within one release
  cycle (0.31.117, 0.31.118).
- The de-branding rationale (neutral, upstreamable corpus) stays intact:
  branded material lives only under `examples/dogfood/`, and the neutral
  fixtures keep their codegen-coverage monopoly.
- The enclave is hermetic: no CI network dependency, no pnpm, byte-stable
  crate source.

### Costs / accepted risks

- **Verbatim branding** (upstream names, Midnight Foundation headers) is
  visible in the tree. Accepted deliberately: verbatim is what keeps the
  fixture byte-comparable and recognizably real; a neutralized variant
  would be neither.
- **Pin staleness**: upstream moves; the pin does not. Accepted: the value
  is regression-gating *today's* contract, not tracking HEAD; PROVENANCE
  makes refresh cheap when wanted.
- **~6k lines of generated crate** in the workspace. Bounded: one more
  member among an already multi-crate workspace; dev-dep compile only.

### Verification (at landing)

- `git diff --no-index` of `src/` against a clone of the pinned rev: zero
  diffs; `core-compact-staging/` byte-compares clean against the npm
  tarball; `git ls-files examples/dogfood | wc -l` = 38.
- `python add_headers.py --validate` green with the enclave excluded.
- Both `compactc` targets compile the entry; `cargo build/test -p
  tests-e2e-rust --locked` green incl. `rust_codegen_byte_parity`;
  `cargo clippy -p compact-contract-digital-passport-credential
  --all-targets --all-features -- -D warnings` green; parity test
  `digital_passport_credential` green.

## Alternatives considered

- **Stay fully third-party-free (do nothing).** Rejected: two real backend
  gaps shipped past the entire neutral corpus; the risk dogfooding
  addresses is demonstrated, not speculative.
- **Re-vendor into the neutral corpus (did-05 style, flat `examples/`).**
  Rejected: re-opens the `08decb1` tension pointlessly — mixing branded
  material back into the neutral corpus re-couples upstreaming to
  third-party branding.
- **A scheduled CI job fetching upstream HEAD.** Rejected in planning: a
  network-dependent, time-variant gate breaks the byte-stability that makes
  committed-crate byte-parity meaningful. Refresh stays a human act.
