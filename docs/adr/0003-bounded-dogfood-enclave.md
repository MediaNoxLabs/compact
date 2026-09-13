<!--
This file is part of Compact.
Copyright (C) 2026 Midnight Foundation
SPDX-License-Identifier: Apache-2.0
-->

# ADR 0003 — A bounded third-party dogfood enclave

- Status: Accepted
- Date: 2026-09-13
- Scope: `examples/dogfood/**`, `header_config.json`, `docs/adr/`, AGENT.md §1
- Supersedes / relates to: supersedes commit `08decb1`'s corpus de-branding
  decision **for the `examples/dogfood/` enclave only**; relates to
  [ADR 0001](0001-rust-struct-name-disambiguation.md), whose follow-up note
  closed the unregistered-`digital-passport` gap by removal.

## Context

Commit `08decb1` de-branded the fixture corpus: it deleted the vendored
`examples/did-05/**` third-party contract, the hand-imported (sourceless)
`digital-passport` crate, and the identity-specific fixtures built on them,
so the corpus held no third-party material needing a carve-out from
license-header validation. [ADR 0001](0001-rust-struct-name-disambiguation.md)'s
follow-up note recorded that clean end-state: every crate under
`tests-e2e-rust/contracts/` has a committed `.compact` source and is
byte-parity gated, none rely on `fmt`/`clippy` alone.

De-branding bought header hygiene at the cost of dogfooding: the toolchain no
longer compiled any **real, named, third-party production contract**. A
hand-written fixture proves one emitter path in isolation; it does not prove
the toolchain builds the contract this fork exists to serve. The
digital-passport contract (`midnight-verifiable-credential-digital-passport`,
Apache-2.0) is exactly that contract, and `vendor-digital-passport-harness`
re-vendored its compact-only subset at a pinned revision so it could be
compiled again. That left an unresolved policy question: the corpus is meant
to be third-party-free (`08decb1`), yet a third-party tree now sits under
`examples/`.

## Decision

Admit the vendored material as a **bounded enclave**, not as corpus.

1. **Bounds are directory-level and mechanical.** Third-party material lives
   only under `examples/dogfood/`. `header_config.json`'s
   `excluded_directories` carries the `dogfood` entry (the directory *name*,
   matched at any depth), so `add_headers.py --validate` exempts everything
   beneath it while every file elsewhere still has to carry an Apache-2.0
   header. There is no per-file exclusion list to drift.

2. **Supersession is explicit and scoped.** This ADR supersedes `08decb1`'s
   de-branding stance **only for `examples/dogfood/`** — it does not reverse
   the corpus decision. Outside `examples/dogfood/` the corpus remains
   third-party-free: the de-branded fixtures stay de-branded, and no new
   third-party source may land outside the enclave. The bound is the point —
   a named, reviewable exception, not a precedent.

3. **Vendoring is verbatim and compact-only.** The enclave holds the
   `.compact` subset of upstream's package `src/` tree (entry + five modules)
   plus `core-compact-staging/` (the 15 files staged from
   `@midnight-ntwrk/credential-compact`), each copied byte-for-byte with
   upstream headers, names, and branding intact — recognizability and
   byte-comparability against upstream are the whole point. No TypeScript is
   vendored (see Follow-ups). `PROVENANCE.md` is the manifest of what is
   vendored and why.

4. **Pinning is explicit; refresh is a human act.** The enclave is pinned to
   upstream revision `cdeb860b` (recorded in `PROVENANCE.md`), with the core
   package pinned to `@midnight-ntwrk/credential-compact@0.1.0-rc3`. Nothing
   in this repo tracks upstream: a refresh updates the vendored bytes **and**
   the recorded revision in the same commit, following `PROVENANCE.md`'s
   procedure and verification (per-file byte identity + subset-completeness
   `diff`). No scheduled drift detection; a moved upstream changes nothing
   here until a human re-syncs.

5. **The enclave is a first-class fixture, not a probe.** It is registered
   like any other crate — workspace member, `tests-e2e-rust`
   dev-dependency, and a `codegen_regression` FIXTURES row — so regen
   byte-parity, fmt, clippy, compile, and the Rust↔TS behaviour test all gate
   it in CI. `add-digital-passport-dogfood-fixture` owns that registration;
   the enclave's compile gate proved its worth immediately by surfacing a
   real zero-field `FromFieldRepr` codegen defect.

## Consequences

### Positive

- The toolchain continuously proves it compiles a real, named, production
  third-party contract — the dogfood claim is mechanical, not narrative.
- Header hygiene is preserved without a per-file allow-list: one directory
  exclusion covers the whole enclave, and everything outside it still has to
  pass `add_headers.py --validate`.
- The exception is discoverable and bounded: AGENT.md §1 explains the
  category, `PROVENANCE.md` explains the contents and refresh, and this ADR
  explains the policy and its limits.

### Neutral / guarded

- **Corpus neutrality is narrowed, not reversed.** The de-branded corpus is
  unchanged; only the new `examples/dogfood/` tree is third-party. A future
  contributor who finds branded content under `examples/` reads §1 and this
  ADR before "cleaning it up".
- **No CI cost beyond one crate.** The enclave adds fixtures to the existing
  gates; the ~3.5k–6k-line crate compiles only as a dev-dependency.

### Verification

- `python add_headers.py --validate` → green: files under `examples/dogfood/`
  are exempt, and every other new file still requires a header.
- `git ls-files examples/dogfood | wc -l` → 22 (6 `src/` + 15 staged +
  `PROVENANCE.md`), and no `.ts` anywhere beneath the enclave.

## Alternatives considered

- **Keep the corpus strictly third-party-free (no dogfood).** Rejected: it
  drops the one fixture that proves the toolchain builds the real contract it
  exists to serve; a synthetic fixture cannot carry that claim.
- **De-brand the vendored copy (rename modules/identifiers).** Rejected: it
  destroys byte-comparability against upstream and the very recognizability
  that makes the dogfood meaningful; `08decb1` was about *removing*
  third-party material, not laundering it in place.
- **Vendor to a non-`examples/` location to dodge the header carve-out.**
  Rejected: the exclusion would have to be path-specific and brittle; a
  single directory-level bound is simpler and auditable.
- **Track upstream automatically / scheduled drift detection.** Rejected:
  refresh stays an explicit human act (Decision 4); hermetic CI must not
  depend on the network.

## Follow-ups

- **TypeScript is deliberately not vendored.** The enclave is a compact-only
  subset of upstream's package tree: the TypeScript half of `src/` (runtime
  codecs, contract wrapper, testing utils, vitest suites) is excluded. This
  is a recorded divergence from whole-tree verbatim — no gate compiles,
  executes, or reads it (compiler input is `.compact` only, and the parity
  capture is a *port* of upstream's testing TS that imports the
  compiler-generated output, not the vendored tree), so the dogfood claim is
  fully carried by the `.compact` subset. A future neutralization pass that
  renames/rebadges the enclave would have to be recorded here and in
  `PROVENANCE.md` as a deliberate divergence from verbatim.
- **Refresh remains manual.** See
  `examples/dogfood/digital-passport-credential/PROVENANCE.md` for the exact
  re-sync and verification procedure.
