# Delta Spec: dogfooding/digital-passport-fixture

## Purpose

The repository compiles the pinned upstream `midnight-verifiable-credential-digital-passport` contract with both codegen targets, regenerates its Rust crate byte-identically from the vendored Compact source, gates it in CI, and pins Rust↔TS behaviour parity.

## ADDED Requirements

### Requirement: Both codegen targets build the contract

The current toolchain MUST compile `examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact` successfully with `compactc --target ts --skip-zk` and `compactc --target rust --skip-zk`, with zero `unimplemented!`/`todo!` in the emitted Rust. CI MUST fail if either target stops compiling (smoke step) or if the committed crate stops building/formatting/linting.

#### Scenario: rust codegen regression is caught
- **WHEN** a compiler change makes the dogfood contract fail under `--target rust`
- **THEN** the CI smoke step and the byte-parity fixture regeneration both fail

#### Scenario: crate gates apply
- **WHEN** the generated crate drifts from fmt/clippy/compile expectations
- **THEN** `rust-runtime-test.yml` gates fail, including the crate-specific clippy step

### Requirement: Byte-parity from vendored source

The committed crate at `tests-e2e-rust/contracts/digital-passport-credential/` MUST be regenerable byte-identically from the vendored source by `codegen_regression` (FIXTURES row present), enforcing that the committed artifact is exactly what the current compiler emits. The crate MUST also be a `tests-e2e-rust` dev-dependency so the CI build gate actually type-checks it.

#### Scenario: emitter drift against the dogfood is detected
- **WHEN** `cargo test -p tests-e2e-rust rust_codegen_byte_parity` runs
- **THEN** regeneration over the dogfood entry either matches the committed `lib.rs` byte-for-byte or the gate fails

#### Scenario: the crate is actually compiled
- **WHEN** `cargo build -p tests-e2e-rust --tests --locked` runs in CI
- **THEN** the dogfood crate is compiled because it is a dev-dependency, not merely a workspace member

### Requirement: Rust-to-TypeScript behaviour parity

The fixture MUST pin observable behaviour parity with the TypeScript backend using the committed TS reference captures: the civil-date helpers (including every conditional-expression site and their assert-fail paths) and one issuance/presentation/verification round-trip. The executing Rust test MUST assert byte-equal outcomes at each captured step, using serialized state bytes where the capture records state.

#### Scenario: helper behaviour parity
- **WHEN** the executing test runs the civil-date helper circuits against the captured TS reference values
- **THEN** Rust outcomes (including assertion-failure cases) match the reference at each step

#### Scenario: state-byte parity
- **WHEN** a captured step records ledger state
- **THEN** the Rust serialized state bytes equal the TS reference bytes, so an alignment/width divergence cannot pass on decoded value alone

### Requirement: Bounded third-party enclave

Third-party material MUST live only under `examples/dogfood/`, excluded from the repo's license-header validation via the enclave's directory exclusion, with upstream license headers intact; the enclave's existence, its compact-only scope, and its relationship to the corpus de-branding decision MUST be recorded in an ADR and AGENT.md.

#### Scenario: header validation tolerates upstream headers
- **WHEN** `python add_headers.py --validate` runs in CI
- **THEN** files under `examples/dogfood/` are exempt while every other new file still requires an Apache-2.0 header

#### Scenario: the policy is discoverable
- **WHEN** a future contributor finds branded content under `examples/`
- **THEN** AGENT.md §1 and the ADR explain the dogfood enclave, its bounds, and why it does not reverse the neutrality of the rest of the corpus
