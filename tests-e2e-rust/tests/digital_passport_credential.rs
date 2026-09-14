// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//
// Rust ⇄ TS behaviour parity for the vendored dogfood contract
// `examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact`
// (task 2.2 of `add-digital-passport-dogfood-fixture`).
//
// `codegen_regression` proves the committed crate is regenerable byte-for-byte;
// it cannot see behaviour. This test replays the reference captures authored by
// `vendor-digital-passport-harness`
// (`fixtures/digital-passport-credential-ts-state.json`, produced by
// `capture-digital-passport-credential.mjs` under `compactc --target ts`) and
// asserts the generated Rust crate produces the SAME observable outcome at
// every captured step:
//
//   * civil-date helpers — `assertValidDigitalPassportAgePredicate` (which
//     drives the non-exported `assertCivilDateMatchesEpochDays` twice), so
//     every conditional-expression site and every `assert`-fail path is
//     exercised. Each `{ok:true}` / `{ok:false,error}` must match exactly,
//     including the failure message.
//   * the issuance/presentation/verification round-trip — the derived body
//     roots are compared **byte-for-byte** (hex `result` values), so an
//     alignment or width divergence cannot pass on a decoded value alone; the
//     ten validator circuits must accept (`ok:true`) as the reference does.
//
// Because the contract is a pure helper library (no ledger, no impure
// circuits), there is no serialised `ContractState` to compare — the captured
// outcome *is* the observable behaviour. The reference is the oracle: it is
// reused verbatim from change 2 and must not be re-authored here.

use compact_contract_digital_passport_credential::pure_circuits;
use compact_contract_digital_passport_credential::{
    ContractAddress, Credential, CredentialProtocolFeatures, DigitalPassportCivilDate,
    DigitalPassportClaimCommitments, DigitalPassportClaimValues,
    DigitalPassportCredentialPrivateParts, DigitalPassportDisclosures,
    DigitalPassportIssuanceOfferBody, DigitalPassportIssuanceRequestBody,
    DigitalPassportIssuanceResultBody, DigitalPassportOpenings, DigitalPassportPresentationRequest,
    DigitalPassportVerificationRequestBody, DigitalPassportVerificationResultBody,
    DigitalPassportVerificationSubmissionBody, ExplicitHolderBinding, HolderBindingProfile,
    NoPublicClaims, NoStatusBinding, OfferMessage, Presentation, Proof, ProtocolMessageEnvelope,
    RequestMessage, RequestMessage_1, ResultMessage, ResultMessage_1, SchemaRef, Signature,
    SubmissionMessage, VerificationMethodRef,
};
use midnight_compact_runtime::transient_crypto::curve::{embedded, EmbeddedFr};
use midnight_compact_runtime::{ec_mul_generator, CompactError, Fr, JubjubPoint};
use tests_e2e_rust::{CivilDateFields, DigitalPassportCredentialTsReference, StepOutcome};

// The scenario set captured by change 2. Pinned so a renamed or emptied
// reference fails here instead of passing vacuously.
const CIVIL_DATE_SCENARIOS: &[&str] = &[
    "accept_birthday_passed",
    "accept_birthday_pending",
    "accept_february_leap_dob",
    "accept_february_nonleap",
    "accept_30day_month",
    "accept_birthday_today_at_threshold",
    "reject_prove_age_disabled",
    "reject_dob_commitment_mismatch",
    "reject_current_before_dob",
    "reject_year_before_1970",
    "reject_month_zero",
    "reject_month_thirteen",
    "reject_day_zero",
    "reject_quotient4_invalid",
    "reject_quotient100_invalid",
    "reject_quotient400_invalid",
    "reject_month_day_offset_invalid",
    "reject_31day_month_overflow",
    "reject_february_nonleap_29",
    "reject_february_leap_30",
    "reject_30day_month_overflow",
    "reject_epoch_days_mismatch",
    "reject_age_below_threshold",
    "reject_before_birthday_disjunct",
    "reject_before_birthday_first_disjunct",
];

// The round-trip step set captured by change 2, in capture order. The three
// roots are byte-compared; the ten validators must accept.
const ROUND_TRIP_STEPS: &[&str] = &[
    "credentialBodyRoot",
    "presentationBodyRoot",
    "presentationRequestBodyRoot",
    "assertValidDigitalPassportIssuanceOffer",
    "assertValidDigitalPassportIssuanceRequest",
    "assertDigitalPassportIssuanceRequestMatchesOffer",
    "assertValidDigitalPassportIssuanceResult",
    "assertDigitalPassportIssuanceResultMatchesRequest",
    "assertValidDigitalPassportVerificationRequestMessage",
    "assertValidDigitalPassportVerificationSubmissionMessage",
    "assertDigitalPassportVerificationSubmissionMatchesRequest",
    "assertValidDigitalPassportVerificationResultMessage",
    "assertDigitalPassportVerificationResultMatchesSubmission",
];

fn fixture() -> DigitalPassportCredentialTsReference {
    DigitalPassportCredentialTsReference::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/digital-passport-credential-ts-state.json"
    ))
}

// ---------------------------------------------------------------------------
// Byte / value helpers
// ---------------------------------------------------------------------------

fn bytes32(fill: u8) -> [u8; 32] {
    [fill; 32]
}

fn hex32(s: &str) -> [u8; 32] {
    hex::decode(s)
        .expect("decode hex")
        .try_into()
        .expect("expected 32 bytes")
}

/// Left-pad `s`'s UTF-8 bytes into a 32-byte array, matching the capture's
/// `pad32`.
fn pad32(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    let bytes = s.as_bytes();
    assert!(bytes.len() <= 32, "`{s}` does not fit in 32 bytes");
    out[..bytes.len()].copy_from_slice(bytes);
    out
}

fn u32_of(s: &str) -> u32 {
    s.parse().expect("parse u32")
}

fn u8_of(s: &str) -> u8 {
    s.parse().expect("parse u8")
}

// ---------------------------------------------------------------------------
// Contract value constructors (mirroring the capture's builders)
// ---------------------------------------------------------------------------

fn issuer_method() -> VerificationMethodRef {
    VerificationMethodRef {
        didContractAddress: ContractAddress {
            bytes: bytes32(0x11),
        },
        methodId: bytes32(0x22),
    }
}

fn holder_method() -> VerificationMethodRef {
    VerificationMethodRef {
        didContractAddress: ContractAddress {
            bytes: bytes32(0x33),
        },
        methodId: bytes32(0x44),
    }
}

fn explicit_holder_binding() -> ExplicitHolderBinding {
    ExplicitHolderBinding {
        holderVerificationMethodRef: holder_method(),
    }
}

fn domain() -> SchemaRef {
    SchemaRef {
        packageId: pad32("midnight:vc:digital-passport"),
        schemaId: pad32("digital-passport:v1"),
        majorVersion: 1,
        minorVersion: 0,
    }
}

/// Minimal credential for the age predicate: only `claimCommitments
/// .dateOfBirthCommitment` is read, so every other field is the capture's
/// constant filler.
fn helper_credential(date_of_birth_commitment: [u8; 32]) -> Credential {
    Credential {
        version: 1,
        schema: domain(),
        issuerVerificationMethodRef: issuer_method(),
        holderBinding: explicit_holder_binding(),
        statusBinding: NoStatusBinding {},
        issuedAt: 0,
        hasExpiration: false,
        expiresAt: 0,
        claims: NoPublicClaims {},
        claimCommitments: DigitalPassportClaimCommitments {
            firstNameCommitment: bytes32(0x01),
            lastNameCommitment: bytes32(0x02),
            dateOfBirthCommitment: date_of_birth_commitment,
            documentNumberCommitment: pure_circuits::document_number_null_commitment()
                .expect("document-number null commitment"),
            issuingStateCommitment: bytes32(0x05),
        },
        claimRoot: bytes32(0x09),
    }
}

fn helper_presentation(prove_age_over_threshold: bool, age_threshold_years: u8) -> Presentation {
    Presentation {
        version: 1,
        schema: domain(),
        credentialClaimRoot: bytes32(0x09),
        issuerVerificationMethodRef: issuer_method(),
        holderBinding: explicit_holder_binding(),
        disclosed: DigitalPassportDisclosures {
            revealFirstName: false,
            firstNameValuePadded: [0u8; 64],
            firstNameOpening: bytes32(0),
            revealLastName: false,
            lastNameValuePadded: [0u8; 64],
            lastNameOpening: bytes32(0),
            proveAgeOverThreshold: prove_age_over_threshold,
            ageThresholdYears: age_threshold_years,
            revealDocumentNumber: false,
            documentNumberValue: bytes32(0),
            documentNumberOpening: bytes32(0),
            revealIssuingState: false,
            issuingStateValue: bytes32(0),
            issuingStateOpening: bytes32(0),
        },
    }
}

fn civil_date(c: &CivilDateFields) -> DigitalPassportCivilDate {
    DigitalPassportCivilDate {
        year: u32_of(&c.year),
        month: u32_of(&c.month),
        day: u32_of(&c.day),
        yearAdjustedQuotient4: u32_of(&c.year_adjusted_quotient4),
        yearAdjustedQuotient100: u32_of(&c.year_adjusted_quotient100),
        yearAdjustedQuotient400: u32_of(&c.year_adjusted_quotient400),
        marchBasedMonthDayOffset: u32_of(&c.march_based_month_day_offset),
    }
}

// ---------------------------------------------------------------------------
// Schnorr proof construction (mirrors the capture's `signProof`)
// ---------------------------------------------------------------------------

const ISSUER_SK: u64 = 0x1234;
const NONCE: u64 = 0x5678;

fn issuer_pk() -> JubjubPoint {
    ec_mul_generator(Fr::from(ISSUER_SK))
}

fn nonce_point() -> JubjubPoint {
    ec_mul_generator(Fr::from(NONCE))
}

/// Reduce an outer-curve `Fr` into the embedded (Jubjub) scalar field, exactly
/// as the capture and the off-circuit verifier do.
fn fr_to_embedded(fr: Fr) -> EmbeddedFr {
    let mut wide = [0u8; 64];
    wide[..32].copy_from_slice(&fr.as_le_bytes());
    EmbeddedFr(embedded::Scalar::from_bytes_wide(&wide))
}

fn embedded_to_fr(s: EmbeddedFr) -> Fr {
    Fr::from_le_bytes(&s.0.to_bytes()).expect("embedded scalar fits in Fr")
}

/// `challenge < EMBEDDED_ORDER`: reducing into the embedded field and reading
/// it back is lossless iff the outer value was already in range.
fn challenge_in_embedded_range(challenge: Fr) -> bool {
    embedded_to_fr(fr_to_embedded(challenge)) == challenge
}

/// Produce an honest Schnorr proof over `body_root`. The challenge comes from
/// the contract's own pure circuit (the same one the verifier uses); `createdAt`
/// is stepped from zero until the challenge falls in the embedded scalar range,
/// then `s = nonce + challenge * sk (mod order)` satisfies
/// `s*G == r + challenge*pk`.
fn sign_proof<F>(
    body_root: [u8; 32],
    challenge_fn: F,
    signer: VerificationMethodRef,
    challenge_bytes: [u8; 32],
) -> Proof
where
    F: Fn([u8; 32], Proof) -> Result<Fr, CompactError>,
{
    let sk = EmbeddedFr(embedded::Scalar::from(ISSUER_SK));
    let nonce = EmbeddedFr(embedded::Scalar::from(NONCE));

    let mut created_at = 0u64;
    loop {
        let mut proof = Proof {
            signerVerificationMethodRef: signer.clone(),
            createdAt: created_at,
            challengeHash: challenge_bytes,
            publicKey: issuer_pk(),
            signature: Signature {
                r: nonce_point(),
                s: Fr::from(0u64),
            },
        };
        let challenge = challenge_fn(body_root, proof.clone()).expect("proof challenge");
        if challenge_in_embedded_range(challenge) {
            let c = fr_to_embedded(challenge);
            proof.signature.s = embedded_to_fr(nonce + c * sk);
            return proof;
        }
        created_at += 1;
    }
}

// ---------------------------------------------------------------------------
// Round-trip value set
// ---------------------------------------------------------------------------

struct RoundTrip {
    credential: Credential,
    presentation: Presentation,
    issuance_offer: OfferMessage,
    issuance_request: RequestMessage_1,
    issuance_result: ResultMessage_1,
    verification_request: RequestMessage,
    verification_submission: SubmissionMessage,
    verification_result: ResultMessage,
    presentation_request: DigitalPassportPresentationRequest,
}

fn envelope(
    message_id: [u8; 32],
    thread_id: [u8; 32],
    initial_message: bool,
    responds_to: [u8; 32],
    created_at: u64,
) -> ProtocolMessageEnvelope {
    ProtocolMessageEnvelope {
        version: 1,
        messageId: message_id,
        threadId: thread_id,
        initialMessage: initial_message,
        respondsToMessageId: responds_to,
        createdAt: created_at,
        hasExpiresAt: false,
        expiresAt: 0,
    }
}

fn no_response() -> [u8; 32] {
    pad32("midnight:vc:protocol:none")
}

fn features() -> CredentialProtocolFeatures {
    CredentialProtocolFeatures {
        supportsSelectiveDisclosure: true,
        supportsPredicateProofs: true,
        supportsVerifierScopedPseudonym: false,
        supportsSameHolderProof: false,
    }
}

/// Rebuild the exact round-trip value set the capture produced, then run every
/// step. The body roots are re-derived from the same values, so a root mismatch
/// is a real byte divergence.
fn build_round_trip() -> RoundTrip {
    let first_name_value_padded = [0x01u8; 64];
    let last_name_value_padded = [0x02u8; 64];
    let date_of_birth_days: u32 = 7400;
    let date_of_birth_opening = bytes32(0x0a);
    let issuing_state_value = bytes32(0x05);
    let issuing_state_opening = bytes32(0x0b);

    let claim_commitments = DigitalPassportClaimCommitments {
        firstNameCommitment: pure_circuits::first_name_commitment(
            first_name_value_padded,
            bytes32(0x03),
        )
        .expect("first-name commitment"),
        lastNameCommitment: pure_circuits::last_name_commitment(
            last_name_value_padded,
            bytes32(0x04),
        )
        .expect("last-name commitment"),
        dateOfBirthCommitment: pure_circuits::date_of_birth_commitment(
            date_of_birth_days,
            date_of_birth_opening,
        )
        .expect("date-of-birth commitment"),
        documentNumberCommitment: pure_circuits::document_number_null_commitment()
            .expect("document-number null commitment"),
        issuingStateCommitment: pure_circuits::issuing_state_commitment(
            issuing_state_value,
            issuing_state_opening,
        )
        .expect("issuing-state commitment"),
    };
    let claim_root =
        pure_circuits::digital_passport_claim_root(claim_commitments.clone()).expect("claim root");

    let credential = Credential {
        version: 1,
        schema: domain(),
        issuerVerificationMethodRef: issuer_method(),
        holderBinding: explicit_holder_binding(),
        statusBinding: NoStatusBinding {},
        issuedAt: 1000,
        hasExpiration: false,
        expiresAt: 0,
        claims: NoPublicClaims {},
        claimCommitments: claim_commitments,
        claimRoot: claim_root,
    };

    let proof_challenge_bytes = bytes32(0x77);
    let credential_body_root =
        pure_circuits::digital_passport_credential_body_root(credential.clone())
            .expect("credential body root");
    let credential_proof = sign_proof(
        credential_body_root,
        pure_circuits::issuance_proof_challenge,
        issuer_method(),
        proof_challenge_bytes,
    );

    let presentation = Presentation {
        version: 1,
        schema: domain(),
        credentialClaimRoot: claim_root,
        issuerVerificationMethodRef: issuer_method(),
        holderBinding: explicit_holder_binding(),
        disclosed: DigitalPassportDisclosures {
            revealFirstName: false,
            firstNameValuePadded: first_name_value_padded,
            firstNameOpening: bytes32(0x03),
            revealLastName: false,
            lastNameValuePadded: last_name_value_padded,
            lastNameOpening: bytes32(0x04),
            proveAgeOverThreshold: false,
            ageThresholdYears: 0,
            revealDocumentNumber: false,
            documentNumberValue: bytes32(0),
            documentNumberOpening: bytes32(0),
            revealIssuingState: false,
            issuingStateValue: issuing_state_value,
            issuingStateOpening: issuing_state_opening,
        },
    };
    let presentation_challenge_bytes = bytes32(0x88);
    let presentation_body_root =
        pure_circuits::digital_passport_presentation_body_root(presentation.clone())
            .expect("presentation body root");
    let presentation_proof = sign_proof(
        presentation_body_root,
        pure_circuits::presentation_proof_challenge,
        holder_method(),
        presentation_challenge_bytes,
    );

    let thread_id = bytes32(0x20);
    let offer_message_id = bytes32(0x10);
    let request_message_id = bytes32(0x11);
    let result_message_id = bytes32(0x12);
    let verification_request_message_id = bytes32(0x30);
    let submission_message_id = bytes32(0x31);
    let verification_result_message_id = bytes32(0x32);

    let issuance_offer = OfferMessage {
        envelope: envelope(offer_message_id, thread_id, true, no_response(), 1),
        schema: domain(),
        issuerVerificationMethodRef: issuer_method(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        features: features(),
        body: DigitalPassportIssuanceOfferBody {
            supportsExpiration: false,
            defaultExpirationDays: 0,
            requiresHolderPublicKey: false,
        },
    };

    let issuance_request = RequestMessage_1 {
        envelope: envelope(request_message_id, thread_id, false, offer_message_id, 2),
        schema: domain(),
        issuerVerificationMethodRef: issuer_method(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        body: DigitalPassportIssuanceRequestBody {
            holderBinding: explicit_holder_binding(),
            holderPublicKey: issuer_pk(),
            holderChallengeHash: proof_challenge_bytes,
            requestExpiration: false,
            requestedExpirationDays: 0,
        },
    };

    let issuance_result = ResultMessage_1 {
        envelope: envelope(result_message_id, thread_id, false, request_message_id, 3),
        schema: domain(),
        issuerVerificationMethodRef: issuer_method(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        body: DigitalPassportIssuanceResultBody {
            credential: credential.clone(),
            credentialProof: credential_proof.clone(),
            holderPublicKey: issuer_pk(),
            issuanceChallengeHash: proof_challenge_bytes,
            privateParts: DigitalPassportCredentialPrivateParts {
                claimValues: DigitalPassportClaimValues {
                    firstNameValuePadded: first_name_value_padded,
                    lastNameValuePadded: last_name_value_padded,
                    dateOfBirthDays: date_of_birth_days,
                    documentNumberValue: bytes32(0),
                    issuingStateValue: issuing_state_value,
                },
                openings: DigitalPassportOpenings {
                    firstNameOpening: bytes32(0x03),
                    lastNameOpening: bytes32(0x04),
                    dateOfBirthOpening: date_of_birth_opening,
                    documentNumberOpening: bytes32(0),
                    issuingStateOpening: issuing_state_opening,
                },
            },
        },
    };

    let verification_request = RequestMessage {
        envelope: envelope(
            verification_request_message_id,
            thread_id,
            true,
            no_response(),
            4,
        ),
        schema: domain(),
        issuerVerificationMethodRef: issuer_method(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        features: features(),
        verifierChallengeHash: presentation_challenge_bytes,
        body: DigitalPassportVerificationRequestBody {
            requireFirstNameDisclosure: false,
            requireLastNameDisclosure: false,
            requireAgeOverThreshold: false,
            requestedAgeThresholdYears: 0,
            requireDocumentNumberDisclosure: false,
            requireIssuingStateDisclosure: false,
        },
    };

    let verification_submission = SubmissionMessage {
        envelope: envelope(
            submission_message_id,
            thread_id,
            false,
            verification_request_message_id,
            5,
        ),
        schema: domain(),
        issuerVerificationMethodRef: issuer_method(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        challengeHash: presentation_challenge_bytes,
        body: DigitalPassportVerificationSubmissionBody {
            credential: credential.clone(),
            credentialProof: credential_proof.clone(),
            presentation: presentation.clone(),
            presentationProof: presentation_proof.clone(),
        },
    };

    let verification_result = ResultMessage {
        envelope: envelope(
            verification_result_message_id,
            thread_id,
            false,
            submission_message_id,
            6,
        ),
        approved: true,
        body: DigitalPassportVerificationResultBody {
            credentialRoot: credential_body_root,
            verifiedThresholdYears: 0,
        },
    };

    let presentation_request = pure_circuits::digital_passport_presentation_request_from_protocol(
        verification_request.clone(),
    )
    .expect("presentation request from protocol");

    RoundTrip {
        credential,
        presentation,
        issuance_offer,
        issuance_request,
        issuance_result,
        verification_request,
        verification_submission,
        verification_result,
        presentation_request,
    }
}

// ---------------------------------------------------------------------------
// Outcome assertions
// ---------------------------------------------------------------------------

fn assert_void_outcome(label: &str, result: Result<(), CompactError>, expected: &StepOutcome) {
    match result {
        Ok(()) => assert!(
            expected.ok,
            "{label}: Rust returned Ok, but the TS reference recorded a failure ({:?})",
            expected.error
        ),
        Err(CompactError::AssertionFailed(msg)) => {
            assert!(
                !expected.ok,
                "{label}: Rust asserted `{msg}`, but the TS reference recorded success"
            );
            let expected_error = expected.error.as_deref().unwrap_or_else(|| {
                panic!("{label}: TS reference recorded a failure with no message")
            });
            assert_eq!(
                format!("failed assert: {msg}"),
                expected_error,
                "{label}: assertion message differs from the TS reference"
            );
        }
        Err(other) => panic!("{label}: Rust returned a non-assert error {other:?}"),
    }
}

fn assert_root_bytes(label: &str, actual: [u8; 32], expected: &StepOutcome) {
    let expected_hex = expected
        .result
        .as_deref()
        .unwrap_or_else(|| panic!("{label}: TS reference recorded no result bytes for this step"));
    assert_eq!(
        hex::encode(actual),
        expected_hex,
        "{label}: Rust 32-byte body root differs from the TS reference"
    );
}

fn round_trip_step<'a>(
    reference: &'a DigitalPassportCredentialTsReference,
    name: &str,
) -> &'a StepOutcome {
    reference
        .round_trip
        .get(name)
        .unwrap_or_else(|| panic!("TS reference is missing round-trip step `{name}`"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Every civil-date scenario must match the TS outcome (assert-fail paths
/// included, message and all).
#[test]
fn digital_passport_civil_date_helper_parity() {
    let reference = fixture();

    assert_eq!(
        reference.civil_date_helpers.len(),
        CIVIL_DATE_SCENARIOS.len(),
        "the TS reference civilDateHelpers set no longer matches the pinned scenarios"
    );
    for name in CIVIL_DATE_SCENARIOS {
        assert!(
            reference.civil_date_helpers.contains_key(*name),
            "TS reference is missing civil-date scenario `{name}`"
        );
    }

    for (name, scenario) in &reference.civil_date_helpers {
        let inputs = &scenario.inputs;
        let credential = helper_credential(hex32(&inputs.date_of_birth_commitment));
        let presentation = helper_presentation(
            inputs.prove_age_over_threshold,
            u8_of(&inputs.age_threshold_years),
        );

        let result = pure_circuits::assert_valid_digital_passport_age_predicate(
            credential,
            presentation,
            u32_of(&inputs.current_day),
            u32_of(&inputs.date_of_birth_days),
            hex32(&inputs.date_of_birth_opening),
            civil_date(&inputs.current_date),
            civil_date(&inputs.date_of_birth_date),
        );

        assert_void_outcome(
            &format!("civilDateHelpers/{name} [{}]", scenario.site),
            result,
            &scenario.outcome,
        );
    }
}

/// The round-trip body roots must be byte-identical to the TS reference and
/// every validator circuit must accept.
#[test]
fn digital_passport_round_trip_parity() {
    let reference = fixture();

    assert_eq!(
        reference.round_trip.len(),
        ROUND_TRIP_STEPS.len(),
        "the TS reference roundTrip set no longer matches the pinned steps"
    );
    for name in ROUND_TRIP_STEPS {
        let _ = round_trip_step(&reference, name);
    }

    let rt = build_round_trip();

    // Derived body roots: byte-for-byte.
    assert_root_bytes(
        "credentialBodyRoot",
        pure_circuits::digital_passport_credential_body_root(rt.credential.clone())
            .expect("credential body root"),
        round_trip_step(&reference, "credentialBodyRoot"),
    );
    assert_root_bytes(
        "presentationBodyRoot",
        pure_circuits::digital_passport_presentation_body_root(rt.presentation.clone())
            .expect("presentation body root"),
        round_trip_step(&reference, "presentationBodyRoot"),
    );
    assert_root_bytes(
        "presentationRequestBodyRoot",
        pure_circuits::digital_passport_presentation_request_body_root(
            rt.presentation_request.clone(),
        )
        .expect("presentation request body root"),
        round_trip_step(&reference, "presentationRequestBodyRoot"),
    );

    // Issuance sequence.
    assert_void_outcome(
        "assertValidDigitalPassportIssuanceOffer",
        pure_circuits::assert_valid_digital_passport_issuance_offer(rt.issuance_offer.clone()),
        round_trip_step(&reference, "assertValidDigitalPassportIssuanceOffer"),
    );
    assert_void_outcome(
        "assertValidDigitalPassportIssuanceRequest",
        pure_circuits::assert_valid_digital_passport_issuance_request(rt.issuance_request.clone()),
        round_trip_step(&reference, "assertValidDigitalPassportIssuanceRequest"),
    );
    assert_void_outcome(
        "assertDigitalPassportIssuanceRequestMatchesOffer",
        pure_circuits::assert_digital_passport_issuance_request_matches_offer(
            rt.issuance_offer.clone(),
            rt.issuance_request.clone(),
        ),
        round_trip_step(
            &reference,
            "assertDigitalPassportIssuanceRequestMatchesOffer",
        ),
    );
    assert_void_outcome(
        "assertValidDigitalPassportIssuanceResult",
        pure_circuits::assert_valid_digital_passport_issuance_result(rt.issuance_result.clone()),
        round_trip_step(&reference, "assertValidDigitalPassportIssuanceResult"),
    );
    assert_void_outcome(
        "assertDigitalPassportIssuanceResultMatchesRequest",
        pure_circuits::assert_digital_passport_issuance_result_matches_request(
            rt.issuance_request.clone(),
            rt.issuance_result.clone(),
        ),
        round_trip_step(
            &reference,
            "assertDigitalPassportIssuanceResultMatchesRequest",
        ),
    );

    // Verification sequence.
    assert_void_outcome(
        "assertValidDigitalPassportVerificationRequestMessage",
        pure_circuits::assert_valid_digital_passport_verification_request_message(
            rt.verification_request.clone(),
        ),
        round_trip_step(
            &reference,
            "assertValidDigitalPassportVerificationRequestMessage",
        ),
    );
    assert_void_outcome(
        "assertValidDigitalPassportVerificationSubmissionMessage",
        pure_circuits::assert_valid_digital_passport_verification_submission_message(
            rt.verification_submission.clone(),
        ),
        round_trip_step(
            &reference,
            "assertValidDigitalPassportVerificationSubmissionMessage",
        ),
    );
    assert_void_outcome(
        "assertDigitalPassportVerificationSubmissionMatchesRequest",
        pure_circuits::assert_digital_passport_verification_submission_matches_request(
            rt.verification_request.clone(),
            rt.verification_submission.clone(),
        ),
        round_trip_step(
            &reference,
            "assertDigitalPassportVerificationSubmissionMatchesRequest",
        ),
    );
    assert_void_outcome(
        "assertValidDigitalPassportVerificationResultMessage",
        pure_circuits::assert_valid_digital_passport_verification_result_message(
            rt.verification_result.clone(),
        ),
        round_trip_step(
            &reference,
            "assertValidDigitalPassportVerificationResultMessage",
        ),
    );
    assert_void_outcome(
        "assertDigitalPassportVerificationResultMatchesSubmission",
        pure_circuits::assert_digital_passport_verification_result_matches_submission(
            rt.verification_submission.clone(),
            rt.verification_result.clone(),
        ),
        round_trip_step(
            &reference,
            "assertDigitalPassportVerificationResultMatchesSubmission",
        ),
    );
}
