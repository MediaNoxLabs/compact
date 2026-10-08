# Using the digital-passport pure API from Rust

Current guide for 0.3.0 closeout. **Historical executed example:** verified locally at1705fc1a with Rust1.99 on macOSarm64, using a fresh standalone consumer and bundled runtime sources. Keep the matching compiler/runtime bundle together; see [Version and runtime selection](version-selection.md).

## What this contract provides

The adopted digital-passport source exports75 pure functions for credentials, commitments, selective disclosure, age predicates, requests and signed protocol envelopes. Its authoritative IR has no ledger fields, witnesses or stateful circuits. Call `pure_circuits` directly. ContractLab is useful for contracts that have ledger state; no simulator setup or ledger transaction is needed for this pure source family.

The pinned integration root is midnight-vc-passport `2e13b029f7f9e37a7ba1e4d987fdcd8620c4eab3`; its declared credential-compact0.2.0 source is `1a65558b9fcc1c3334932464ca23141f82c8e974`. Preserve all17 Compact files and their import layout from the accepted source manifest. Later upstream changes require a new compatibility check.

The [complete two-test consumer and pinned Compact source closure](examples/passport/README.md) are included. The [retained P1 receipt](../evidence/0.3.0/passport-p1/receipt.json) keeps its historical execution identity.

## Generate and consume

With the matching Rust-capable compactc distribution installed, place the complete pinned source tree in `passport-source/` and run:

```sh
compactc --target rust --skip-zk \
  passport-source/src/digital-passport-credential.compact generated
```

The output includes `contract/lib.rs`, its Cargo manifest, capability/IR files, compiler compatibility metadata and bundled `runtime-rs`/`runtime-rs-macros` sources. `--skip-zk` skips proving-key generation; it does not skip Rust contract generation. This closure has zero proof-applicable exports independently of that flag.

Create `consumer/Cargo.toml` alongside `generated/`:

```toml
[package]
name = "passport-guide-consumer"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
passport = { package = "compact-contract-digital-passport-credential", path = "../generated/contract" }
```

Use the generated crate's `runtime` re-export so values and errors come from the exact runtime pair. The example below belongs in `consumer/src/lib.rs`:

```rust
use passport::{pure_circuits as pure, runtime as rt, types};

// Deterministic test data only; an application supplies real commitments.
fn bytes32(seed: u8) -> rt::FixedBytes<32> {
    rt::FixedBytes::new(std::array::from_fn(|i| seed.wrapping_add(i as u8)))
}

pub fn claim_root() -> Result<rt::FixedBytes<32>, rt::CompactError> {
    pure::digitalPassportClaimRoot(types::DigitalPassportClaimCommitments {
        firstNameCommitment: bytes32(1),
        lastNameCommitment: bytes32(2),
        dateOfBirthCommitment: bytes32(3),
        documentNumberCommitment: bytes32(4),
        issuingStateCommitment: bytes32(5),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_matches_the_pinned_typescript_vector() {
        let hex: String = claim_root().unwrap().into_array().iter()
            .map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(hex,
            "1719d1840150ddc72ed9bd11126c673158594527eb03c0e3935868201a64b28d");
    }

    #[test]
    fn schema_checks_still_run_for_well_typed_values() {
        let mut schema = types::SchemaRef {
            packageId: bytes32(1),
            schemaId: bytes32(2),
            majorVersion: rt::BoundedUint::<65535>::new(1).unwrap(),
            minorVersion: rt::BoundedUint::<65535>::new(0).unwrap(),
        };
        pure::assertValidSchemaRef(schema.clone()).unwrap();
        schema.majorVersion = rt::BoundedUint::<65535>::new(0).unwrap();
        assert!(matches!(pure::assertValidSchemaRef(schema),
            Err(rt::CompactError::AssertionFailed(message))
                if message == "Schema major version must be positive"));
    }
}
```

The byte sequences are deterministic test fixtures, not real passport claims or randomized openings. `claim_root` propagates errors with `?`. The test checks a root captured independently from the original TypeScript profile. The second test shows the distinction between a well-typed integer and a value satisfying contract rules: zero fits Uint<16>, but the schema requires a positive major version.

For a new consumer, seed the qualified source lock, let Cargo adapt local package entries, then verify that every registry name/version/source/checksum matches the qualified lock before testing:

```sh
cp /absolute/path/to/reviewed/compact/Cargo.lock consumer/Cargo.lock
cargo metadata --offline --format-version 1 --manifest-path consumer/Cargo.toml > consumer/metadata.json
cargo test --locked --manifest-path consumer/Cargo.toml
```

These revised lock-preparation commands are guidance; this passport example was not rerun by the current documentation review. The retained acceptance run used `+1.99.0`, `--offline`, `-j4` and a warm dependency target. Its complete lockfile pins the exact registry versions/checksums already reviewed in the Compact workspace. Every local Cargo package resolves under the external example directory; there is no runtime path back into the source checkout. Keep the generated sources unedited.

## Translate TypeScript calls

| TypeScript concept | Generated Rust API |
|---|---|
| `pureCircuits.digitalPassportClaimRoot(value)` | `pure_circuits::digitalPassportClaimRoot(value)?` |
| bigint for Compact Uint<16> | `runtime::BoundedUint<65535>` constructed with `new` |
| 32-byte Uint8Array | `runtime::FixedBytes<32>` |
| object representing a Compact struct | Named type in `types` with declared field names |
| thrown assertion error | `Result::Err(CompactError::AssertionFailed(message))` |
| spread/copy while retaining old object | Explicit `clone()` or struct-update syntax according to ownership |

Rust and typed TypeScript can both catch ordinary argument-shape mistakes. Rust additionally expresses byte lengths and integer bounds in these carrier types; contract-dependent rules still run dynamically. Default-generated values are construction helpers and do not establish valid credentials, signatures or schema references.

## Give a long call named inputs in your application

The generated age-predicate function keeps the Compact source's seven positional arguments. If the call is repeated in your application, a small Rust record can make each input's role visible at the construction site. This wrapper is application code: it adds no codegen option, derive, runtime type or macro.

**Validated application recipe.** The complete [wrapper](age_predicate_inputs.rs) and [consumer test/captured inputs](examples/named-record/README.md) is byte-identical to the compiled external wrapper. Its one table-driven integration test passed all 25 retained TypeScript scenarios: 6 successes and 19 exact captured errors; upstream and branch capture rows also matched. Strict Clippy and formatting passed. The `passport` dependency alias is the one shown above.

```rust
// Application-owned wrapper; not generated by compactc.
use passport::{pure_circuits, runtime, types};

pub struct AgePredicateInputs {
    pub credential: types::Credential,
    pub presentation: types::Presentation,
    pub current_day: runtime::BoundedUint<4294967295>,
    pub date_of_birth_days: runtime::BoundedUint<4294967295>,
    pub date_of_birth_opening: runtime::FixedBytes<32>,
    pub current_date: types::DigitalPassportCivilDate,
    pub date_of_birth_date: types::DigitalPassportCivilDate,
}

impl AgePredicateInputs {
    pub fn evaluate(self) -> Result<(), runtime::CompactError> {
        pure_circuits::assertValidDigitalPassportAgePredicate(
            self.credential,
            self.presentation,
            self.current_day,
            self.date_of_birth_days,
            self.date_of_birth_opening,
            self.current_date,
            self.date_of_birth_date,
        )
    }
}
```

Build the actual typed credential, presentation, openings and dates before constructing `AgePredicateInputs` when their evaluation order matters, then call `inputs.evaluate()?`. The method consumes the record and propagates the original contract error. It does not clone values or replace any source validation.

The executed wrapper [receipt](../evidence/0.3.0/named-record/evidence/receipt.json), [source](../evidence/0.3.0/named-record/evidence/source.json) and [lock qualification](../evidence/0.3.0/named-record/evidence/lock-qualification.json) are preserved unchanged. Source snapshot: `fba6845f`. Wrapper SHA256: `482b5b8c7b8858a9e7a516c3ad3af57ecee4089d9927e059599743b16777a738`. The external lock contains 324 registry entries matching the reviewed root lock, one runtime source identity and no path dependency outside the consumer. The maintained generated `lib.rs` is unchanged; only its manifest runtime path was relocated.

This validation used Rust 1.99.0 on macOS ARM64 with existing captures. It did not regenerate the compiler output or TypeScript oracles, measure performance, run proofs, submit a network transaction, or promote named Args into the compiler. One table test covers 25 cases; it is not 25 independent test functions.

A missing field is visible to the Rust compiler. A label still cannot stop an application from putting a birth-day value in `current_day`: both have the same carrier type. Use explicit application role wrappers and validation where those roles need stronger guarantees. The record itself does not establish age, authorization or credential validity.

ADR0276 tested generated named Args and deferred promotion after comparing its wider API and output costs. This ordinary Rust alternative can be added where a consumer needs it, without changing every generated contract. Keep the original positional API when a call is already clear.

## Navigate the complete workflow

1. Build the declared claim values/openings, then call the field commitment functions and `digitalPassportClaimRoot`.
2. Build the credential with its schema, holder/issuer references and claim root. Validate private parts through `assertValidDigitalPassportCredentialPrivateParts`.
3. Construct the presentation/disclosures and request. Run the source's request and age-predicate validators; age inputs include calendar decomposition fields required by the Compact source, not just a birth-year integer.
4. For signed issuance/verification flows, use the generated payload-root/challenge functions and verification functions with real caller-supplied values. The maintained examples are in `tests-rust-backend/vc-passport-adoption/tests/{signed_flow,complete_flow}.rs`.
5. Treat DID method resolution, live ledger provenance, secret storage and network services as application integration responsibilities. Pure credential validation by itself does not attest current DID state or chain finality.

This outline is navigation for the accepted scenario suite; the fully runnable small example above is the two-test consumer. The full adoption evidence covers all75exports with202sampled cases,67maintained upstream tests and four exact cross-language codec vectors. It is not exhaustive input coverage. No Rust transport decoder, ZK circuit proof or ledger transaction is supplied by this pure source closure.

## Common integration errors

- **Runtime mismatch:** regenerate with the matching compiler/runtime pair; do not copy compatibility metadata onto an older runtime.
- **Integer conversion fails:** validate incoming numbers before constructing BoundedUint. Avoid JS Number roundtrips for large values; preserve integer/byte representations at transport boundaries.
- **An assertion fails for typed input:** inspect the typed CompactError and the corresponding source rule. Display text includes presentation prefixes; match the error variant when application logic needs classification.
- **Looking for a recorded call:** inspect the capability report. This passport closure contains pure exports; DID has separate stateful recording/proof paths.
- **Trying to publish the example:** the generated package is publish=false and uses bundled sources. The current backend/runtime/macros line is0.2.0; the default generated application and unpublished testkit remain0.1.0. These labels do not assert registry publication or interchangeability of arbitrary source snapshots.

## Evidence and decisions

“Passport adoption acceptance — 2026-10-07” (historical vault reference; not bundled here) · “ADR0250 — Passport codec and maintained suite receipt” (historical vault reference; not bundled here) · [Passport developer guide external consumer — 2026-10-07](../evidence/0.3.0/passport-p1/receipt.json) · [Runtime and compiler ownership map](ownership.md). See [candidate qualification](candidate-qualification.md) for the current validation and documentation status.
