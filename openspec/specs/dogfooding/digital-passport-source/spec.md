# dogfooding/digital-passport-source Specification

## Purpose

The repository carries a pinned, byte-faithful copy of the upstream digital-passport contract's Compact sources — and no unrelated upstream material — and records the contract's current Rust-codegen gap as an executable oracle that fix changes are expected to flip.

## Requirements

### Requirement: Vendored upstream Compact source at a pinned revision

The repo MUST contain the upstream contract's Compact sources byte-identical to upstream rev `cdeb860b` under `examples/dogfood/digital-passport-credential/`: the `*.compact` subset of the upstream package `src/` tree (6 files — the entry `digital-passport-credential.compact` and its five modules) plus `core-compact-staging/` (15 `.compact` files staged from `@midnight-ntwrk/credential-compact@0.1.0-rc3` `dist/`). A `PROVENANCE.md` MUST record the upstream URL, revision, license, npm package+version, the staging procedure, and the refresh policy. The vendored sources MUST NOT be locally modified — upstream divergence is picked up only by an explicit, recorded re-sync.

#### Scenario: provenance is answerable
- **WHEN** a reader asks "what code is this and where did it come from"
- **THEN** `PROVENANCE.md` states the upstream repo, exact rev, npm core package+version, and how to refresh both

#### Scenario: refresh is an explicit act
- **WHEN** upstream moves after the pin
- **THEN** nothing in this repo changes until a human re-syncs and the PROVENANCE revision is updated in the same commit

#### Scenario: vendored set is complete and compact-only
- **WHEN** the vendored tree is compared against the pin
- **THEN** the `src/**/*.compact` list matches upstream's, the staged core has 15 files, and no `.ts` file exists anywhere under the enclave

### Requirement: TypeScript reference behaviour is captured

The fixture MUST carry a committed TypeScript reference capture (`tests-e2e-rust/fixtures/capture-digital-passport-credential.mjs` → JSON) covering the civil-date helpers — including every conditional-expression site and their assert-fail paths — and one issuance/presentation/verification round-trip, produced under the TS codegen target.

#### Scenario: captures cover the conditional sites
- **WHEN** the capture script is run against the TS target
- **THEN** it emits a committed JSON reference covering every helper conditional-expression site and assert-fail path, plus one protocol round-trip

### Requirement: The current Rust-codegen gap is an executable oracle

The repo MUST encode the contract's known Rust-codegen gaps as executable expectations: `rejection_corpus` entries asserting the exact current refusal diagnostics for the ternary sites and a mixed-width operand site, plus a whole-entry expected-failure gate. Each probe MUST name the change that flips it from refusal to acceptance. These probes MUST run in CI.

#### Scenario: the oracle is green pre-fix
- **WHEN** the current (pre-fix) compiler is used
- **THEN** every oracle probe passes by asserting the current refusal, and none is skipped

#### Scenario: the oracle gates the fixes
- **WHEN** a fix change lands
- **THEN** the probes it owns flip from REJECTION to ACCEPTION in that change, and the whole-entry gate remains red until all owned fixes have landed

### Requirement: Bounded vendoring policy is mechanically visible

Third-party material MUST live only under `examples/dogfood/`, excluded from license-header validation via the `dogfood` `excluded_directories` entry, so upstream headers stay verbatim while every other new file still requires an Apache-2.0 header. (The full enclave ADR and AGENT.md policy are recorded by `add-digital-passport-dogfood-fixture`.)

#### Scenario: header validation tolerates upstream headers
- **WHEN** `python add_headers.py --validate` runs
- **THEN** files under `examples/dogfood/` are exempt while every other new file still requires a header

#### Scenario: no TypeScript is vendored
- **WHEN** the enclave is inspected
- **THEN** it contains only `.compact` files and `PROVENANCE.md`, and a completeness check fails if any `.ts` file is added
