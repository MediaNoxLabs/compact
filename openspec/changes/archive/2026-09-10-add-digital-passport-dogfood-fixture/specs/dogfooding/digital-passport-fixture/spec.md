# Delta Spec: dogfooding/digital-passport-fixture

## Purpose

The repository carries a pinned, verbatim copy of the upstream `midnight-verifiable-credential-digital-passport` contract and continuously proves the toolchain compiles it — keeping dogfood regressions visible in CI and local byte-parity gates.

## ADDED Requirements

### Requirement: Vendored upstream source at a pinned revision

The repo MUST contain the upstream contract sources byte-identical to upstream rev `cdeb860b` under `examples/dogfood/digital-passport-credential/` (package `src/` tree plus `core-compact-staging/` from `@midnight-ntwrk/credential-compact@0.1.0-rc3` `dist/`), with a `PROVENANCE.md` recording upstream URL, revision, npm package/version, staging and refresh procedure. The vendored sources MUST NOT be locally modified — any upstream divergence is picked up only by an explicit, recorded re-sync.

#### Scenario: provenance is answerable
- **WHEN** a reader asks "what code is this and where did it come from"
- **THEN** `PROVENANCE.md` states the upstream repo, exact rev, npm core package+version, and how to refresh both

#### Scenario: refresh is an explicit act
- **WHEN** upstream moves after the pin
- **THEN** nothing in this repo changes until a human re-syncs and the PROVENANCE revision is updated in the same commit

### Requirement: Both codegen targets build the contract

The current toolchain MUST compile `examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact` successfully with `compactc --target ts --skip-zk` and `compactc --target rust --skip-zk` (after `fix-ternary-expression-codegen`), and CI MUST fail if either target stops compiling (smoke step) or if the committed crate stops building/formatting/linting (rust-runtime-test gates).

#### Scenario: rust codegen regression is caught
- **WHEN** a compiler change makes the dogfood contract fail under `--target rust`
- **THEN** the CI smoke step and the byte-parity fixture regeneration both fail

#### Scenario: crate gates apply
- **WHEN** the generated crate drifts from fmt/clippy/compile expectations
- **THEN** `rust-runtime-test.yml` gates fail, including the crate-specific clippy step

### Requirement: Byte-parity from vendored source

The committed crate at `tests-e2e-rust/contracts/digital-passport-credential/` MUST be regenerable byte-identically from the vendored source by `codegen_regression` (FIXTURES row present), enforcing that the committed artifact is exactly what the current compiler emits.

#### Scenario: emitter drift against the dogfood is detected
- **WHEN** `cargo test -p tests-e2e-rust rust_codegen_byte_parity` runs locally per AGENT.md §2.1
- **THEN** regeneration over the dogfood entry either matches the committed `lib.rs` byte-for-byte or the gate fails

### Requirement: Representative Rust↔TS parity captures

The fixture MUST pin observable behavior parity for a representative subset of the contract — the civil-date helpers (including every conditional-expression site and their assert-fail paths) and one issuance/presentation/verification round-trip — via a committed TS reference capture and an executing Rust test asserting equivalence.

#### Scenario: helper behavior parity
- **WHEN** the executing test runs the civil-date helper circuits against the captured TS reference values
- **THEN** Rust outcomes (including assertion-failure cases) match the reference byte-for-byte at each step

### Requirement: Bounded third-party enclave

Third-party material MUST live only under `examples/dogfood/`, excluded from the repo's license-header validation via the enclave's directory exclusion, with upstream license headers intact; the enclave's existence and its relationship to the corpus de-branding decision MUST be recorded in an ADR and AGENT.md.

#### Scenario: header validation tolerates upstream headers
- **WHEN** `python add_headers.py --validate` runs in CI
- **THEN** files under `examples/dogfood/` are exempt while every other new file still requires an Apache-2.0 header

#### Scenario: the policy is discoverable
- **WHEN** a future contributor finds branded content under `examples/`
- **THEN** AGENT.md §1 and the ADR explain the dogfood enclave, its bounds, and why it does not reverse the neutrality of the rest of the corpus
