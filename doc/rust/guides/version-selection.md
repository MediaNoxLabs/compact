# Select a compatible compiler, runtime and target

Current policy, 2026-10-07. Version values match the accepted5efa91c2 compatibility checkpoint. Retained migration and target receipts identify their own sources, features and locks; installation guidance alone is not an executed check.

## Keep these version domains separate

| Boundary | Current value | Meaning |
|---|---|---|
| Initiative milestone/artifact label | 0.3.0 / `rust-backend-v0.3.0` | Engineering milestone; no tag or published package is implied here |
| Compact compiler | 0.31.133 | Frontend/toolchain version |
| Compact language | 0.23.105 | Source language version |
| TypeScript runtime | 0.16.101 | `compactc --runtime-version` reports this version |
| Backend / Rust runtime / macros | 0.2.0 | Exact source package pairing; registry availability is not asserted |
| ContractLab / default generated application | 0.1.0 | Separate unpublished package identities |
| Runtime ABI | 50 | Generated-source/runtime compatibility marker, not stable Rust binary ABI |
| Private Rust IR | 20 | Internal frontend/backend data format; historical “schema6” was our earlier private format |
| Capability / compatibility schema | 3 / 1 | Report formats |
| Native ledger | 8.0.3 | Pinned ledger graph |
| Declared native MSRV | Rust1.88 | Qualified only with the documented lock/features/host |

Generated Rust asserts `runtime::RUST_RUNTIME_ABI == 50`. Matching this number helps detect an accidental mismatch; it does not authenticate arbitrary source or establish semantic correctness by itself.

## Obtain a matching source distribution

Use the Rust-capable compiler artifact built from this branch and its matching Scheme component, runtime/macros sources and compatibility record. This guide does not assert a public download URL, registry package or release tag. If working from source, use the branch's retained build/distribution workflow and record the actual compiler identity before accepting example results.

An installed compiler bundle supplies its Scheme component through the bundle layout. A development wrapper needs `COMPACTC_SCHEME` set to its matching executable. The examples require `compactc` on PATH. `--skip-zk` skips key generation, not contract code generation.

Three source-selection modes have different responsibilities:

| Mode | Behavior |
|---|---|
| Default Rust generation | Validates and bundles matching runtime/macros sources in the generated output |
| `--rust-runtime-root /absolute/source/root` | Uses a validated shared source pair; the root contains `runtime-rs` and `runtime-rs-macros` |
| `--rust-runtime-registry` | Emits the exact runtime dependency; does not check that a registry package has been published |

`COMPACT_RUST_RUNTIME_DIR` is also an explicit root selection. Missing or inconsistent compatibility records are rejected rather than silently falling back. Obtain the matching source pair; never copy newer metadata onto old sources to bypass validation.

For ContractLab, use one runtime identity in Cargo: generated contract and testkit must resolve the same source package. The validated tutorials stage runtime/macros/testkit together in `examples/sdk/` and generate with that absolute root. `prepare-sdk.py` was executed for the external witnessed/DID tutorials; it preserves source manifests and copies the testkit's five declared fixture path dependencies. No registry publication is required.

## Target support and evidence

| Target | Retained evidence | Limits |
|---|---|---|
| Native macOS ARM64 | Focused Rust1.99 execution; actual Rust1.88 checks/selected migration consumers against pinned graphs | Warm caches; no arbitrary future-lock or every-feature guarantee |
| Native Linux / other hosts | Earlier milestone distribution/CI history exists | This documentation slice has not performed fresh0.3.0 consumer qualification there; use current release receipts before claiming support |
| Node24.14 and Chromium151 via `wasm32-unknown-unknown` | ADR0256 ran four bounded generated examples with default runtime, Rust1.99/wasm-bindgen0.2.129/wasm-pack0.13.1 | Prototype only; no `ledger-transaction`, browser prover, live wallet or all-contract support claim |
| DID ledger8.1 public transaction interchange | Separate finite JS roundtrip/malformed-refusal receipts | Does not change native ledger8.0.3 or establish a continuous8.1 chain |

The tutorial commands target native Rust. They are not Node/browser build instructions. Retained WASM receipts include exact wrapper/build tooling and host Clang requirements.

## Migrate consumers

1. Preserve old generated code with its original matching runtime, or regenerate source with the ABI50 compiler/runtime/macros pair. Do not edit the ABI assertion.
2. Prefer the generated crate's `runtime` re-export for value types, avoiding duplicate crate identities.
3. Backend0.2 exposes non-exhaustive `RenderError`. External matches need a fallback; resource diagnostics expose `{resource, limit, observed}`. Display text/resource labels are human diagnostics, not a stable machine protocol.
4. Keep the qualified Cargo.lock. A successful build of one workspace lock does not qualify a different standalone or target-specific resolution.
5. Positional generated signatures remain current; named Args/facade probes are research dispositions, not a required API migration.

The ADR0293/0304 migration receipts already contain old ABI49 refusal, working ABI50 controls, non-exhaustive compile failures, migrated callers and standalone Rust1.88 lock qualification. Those are historical qualified executions, not reruns by this documentation reconciliation. See [Static constraints and migration evidence](static-constraints.md) for exact controls and limitations.

## Accepted compatibility checkpoint

The authoritative local policy is the [feature/source policy](feature-source-policy.json), accepted at `5efa91c2`. Its current MSRV receipt covers actual Rust1.88 workspace backend/Jubjub all-feature and isolated standalone compilation at49898aed; the only later source changes before5efa91c2 are two test files. The accepted feature profiles are default native, exact-evidence `ledger-transaction`, source-only ContractLab, experimental bounded WASM and registry **manifest emission only**. Default native includes crypto/proof dependencies.

DID's original compiler profile (ledger8.0.2/compiler0.31.1/runtime0.16.0), its npm ledger-v8@8.1.0 wire profile and the isolated coherent Rust8.1 receiver are separate from native8.0.3. No native promotion follows from those receipts. See [candidate qualification](candidate-qualification.md) for current source, validation and documentation dispositions. The resumed ADR0285/#409 consumer-safety controls passed at4c8aebc3; independent audit fixes are delivered through231559f1. The owner accepted encoded-byte admission for 0.3.0; ADR0359 delivered the opt-in 64 MiB preset with overrides, while aggregate decoded heap/object/CPU containment is deferred in CoPS-001/#495. Historical policy metadata retains its original checkpoint status and is not a current blocker register.

## Historical labels

ADR0261 originally described the0.1.0 source-package line. ADR0293 introduced backend/runtime/macros0.2.0 with exact pairing. Earlier ABI49 and capability-schema1 documents describe their original releases; the current generated source uses ABI50 and finalized capability schema3. Preserve those records rather than rewriting their measurements. `--runtime-version` continues to report the TypeScript runtime version.

## Source build instructions

Use the existing [portable compiler archive workflow](../../../tools/compact-rust-backend/README.md#portable-compiler-archives). This links the branch workflow; no published release URL is implied.
