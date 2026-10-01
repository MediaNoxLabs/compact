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

use compact_rust_passport_dogfood_fixture::pure_circuits;
use compact_rust_passport_dogfood_fixture::types::*;
use midnight_compact_runtime::{
    BoundedUint, CompactError, Field, FixedBytes, JubjubPoint, ec_mul_generator,
};
use midnight_transient_crypto::curve::{EmbeddedFr, embedded};

fn uint<const MAX: u128>(value: u128) -> BoundedUint<MAX> {
    BoundedUint::new(value).unwrap()
}

fn bytes32(fill: u8) -> FixedBytes<32> {
    FixedBytes::new([fill; 32])
}

fn bytes64(fill: u8) -> FixedBytes<64> {
    FixedBytes::new([fill; 64])
}

fn pad32(value: &str) -> FixedBytes<32> {
    let mut bytes = [0; 32];
    bytes[..value.len()].copy_from_slice(value.as_bytes());
    FixedBytes::new(bytes)
}

fn method(address: u8, id: u8) -> VerificationMethodRef {
    VerificationMethodRef {
        didContractAddress: ContractAddress {
            bytes: bytes32(address),
        },
        methodId: bytes32(id),
    }
}

fn issuer_method() -> VerificationMethodRef {
    method(0x11, 0x22)
}

fn holder_method() -> VerificationMethodRef {
    method(0x33, 0x44)
}

fn holder_binding() -> ExplicitHolderBinding {
    ExplicitHolderBinding {
        holderVerificationMethodRef: holder_method(),
    }
}

fn schema() -> SchemaRef {
    SchemaRef {
        packageId: pad32("midnight:vc:digital-passport"),
        schemaId: pad32("digital-passport:v1"),
        majorVersion: uint(1),
        minorVersion: uint(0),
    }
}

fn envelope(
    message_id: FixedBytes<32>,
    thread_id: FixedBytes<32>,
    initial_message: bool,
    responds_to: FixedBytes<32>,
    created_at: u64,
) -> ProtocolMessageEnvelope {
    ProtocolMessageEnvelope {
        version: uint(1),
        messageId: message_id,
        threadId: thread_id,
        initialMessage: initial_message,
        respondsToMessageId: responds_to,
        createdAt: uint(created_at as u128),
        hasExpiresAt: false,
        expiresAt: uint(0),
    }
}

fn no_response() -> FixedBytes<32> {
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

fn capabilities() -> SchemaCapabilities {
    SchemaCapabilities {
        supportsSelectiveDisclosure: true,
        supportsPredicateProofs: true,
        supportsVerifierScopedPseudonym: false,
        supportsSameHolderProof: false,
    }
}

fn issuer_pk() -> JubjubPoint {
    ec_mul_generator(Field::from(0x1234u64)).unwrap()
}

fn nonce_point() -> JubjubPoint {
    ec_mul_generator(Field::from(0x5678u64)).unwrap()
}

fn fr_to_embedded(value: Field) -> EmbeddedFr {
    let mut wide = [0; 64];
    wide[..32].copy_from_slice(&value.as_le_bytes());
    EmbeddedFr(embedded::Scalar::from_bytes_wide(&wide))
}

fn embedded_to_fr(value: EmbeddedFr) -> Field {
    Field::from_le_bytes(&value.0.to_bytes()).unwrap()
}

fn sign_proof<F>(
    root: FixedBytes<32>,
    challenge_fn: F,
    signer: VerificationMethodRef,
    challenge_hash: FixedBytes<32>,
) -> Proof
where
    F: Fn(FixedBytes<32>, Proof) -> Result<Field, CompactError>,
{
    let secret = EmbeddedFr(embedded::Scalar::from(0x1234u64));
    let nonce = EmbeddedFr(embedded::Scalar::from(0x5678u64));
    let mut created_at = 0u64;
    loop {
        let mut proof = Proof {
            signerVerificationMethodRef: signer.clone(),
            createdAt: uint(created_at as u128),
            challengeHash: challenge_hash.clone(),
            publicKey: issuer_pk(),
            signature: Signature {
                r: nonce_point(),
                s: Field::from(0u64),
            },
        };
        let challenge = challenge_fn(root.clone(), proof.clone()).unwrap();
        let reduced = fr_to_embedded(challenge);
        if embedded_to_fr(reduced) == challenge {
            proof.signature.s = embedded_to_fr(nonce + reduced * secret);
            return proof;
        }
        created_at += 1;
    }
}

struct RoundTrip {
    credential: Credential,
    presentation: Presentation,
    issuance_offer: OfferMessage,
    issuance_request: RequestMessageCompact1,
    issuance_result: ResultMessageCompact1,
    verification_request: RequestMessage,
    verification_submission: SubmissionMessage,
    verification_result: ResultMessage,
    presentation_request: DigitalPassportPresentationRequest,
}

fn build_round_trip() -> RoundTrip {
    let first_name = bytes64(0x01);
    let last_name = bytes64(0x02);
    let date_of_birth_opening = bytes32(0x0a);
    let issuing_state = bytes32(0x05);
    let issuing_state_opening = bytes32(0x0b);
    let commitments = DigitalPassportClaimCommitments {
        firstNameCommitment: pure_circuits::firstNameCommitment(first_name.clone(), bytes32(0x03))
            .unwrap(),
        lastNameCommitment: pure_circuits::lastNameCommitment(last_name.clone(), bytes32(0x04))
            .unwrap(),
        dateOfBirthCommitment: pure_circuits::dateOfBirthCommitment(
            uint(7400),
            date_of_birth_opening.clone(),
        )
        .unwrap(),
        documentNumberCommitment: pure_circuits::documentNumberNullCommitment().unwrap(),
        issuingStateCommitment: pure_circuits::issuingStateCommitment(
            issuing_state.clone(),
            issuing_state_opening.clone(),
        )
        .unwrap(),
    };
    let claim_root = pure_circuits::digitalPassportClaimRoot(commitments.clone()).unwrap();
    let credential = Credential {
        version: uint(1),
        schema: schema(),
        issuerVerificationMethodRef: issuer_method(),
        holderBinding: holder_binding(),
        statusBinding: NoStatusBinding {},
        issuedAt: uint(1000),
        hasExpiration: false,
        expiresAt: uint(0),
        claims: NoPublicClaims {},
        claimCommitments: commitments,
        claimRoot: claim_root.clone(),
    };
    let issuance_challenge = bytes32(0x77);
    let credential_root =
        pure_circuits::digitalPassportCredentialBodyRoot(credential.clone()).unwrap();
    let credential_proof = sign_proof(
        credential_root.clone(),
        pure_circuits::issuanceProofChallenge,
        issuer_method(),
        issuance_challenge.clone(),
    );
    let presentation = Presentation {
        version: uint(1),
        schema: schema(),
        credentialClaimRoot: claim_root,
        issuerVerificationMethodRef: issuer_method(),
        holderBinding: holder_binding(),
        disclosed: DigitalPassportDisclosures {
            revealFirstName: false,
            firstNameValuePadded: first_name.clone(),
            firstNameOpening: bytes32(0x03),
            revealLastName: false,
            lastNameValuePadded: last_name.clone(),
            lastNameOpening: bytes32(0x04),
            proveAgeOverThreshold: false,
            ageThresholdYears: uint(0),
            revealDocumentNumber: false,
            documentNumberValue: bytes32(0),
            documentNumberOpening: bytes32(0),
            revealIssuingState: false,
            issuingStateValue: issuing_state.clone(),
            issuingStateOpening: issuing_state_opening.clone(),
        },
    };
    let presentation_challenge = bytes32(0x88);
    let presentation_root =
        pure_circuits::digitalPassportPresentationBodyRoot(presentation.clone()).unwrap();
    let presentation_proof = sign_proof(
        presentation_root,
        pure_circuits::presentationProofChallenge,
        holder_method(),
        presentation_challenge.clone(),
    );
    let thread = bytes32(0x20);
    let offer_id = bytes32(0x10);
    let issuance_request_id = bytes32(0x11);
    let issuance_result_id = bytes32(0x12);
    let verification_request_id = bytes32(0x30);
    let submission_id = bytes32(0x31);
    let verification_result_id = bytes32(0x32);
    let issuance_offer = OfferMessage {
        envelope: envelope(offer_id.clone(), thread.clone(), true, no_response(), 1),
        schema: schema(),
        issuerVerificationMethodRef: issuer_method(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        features: features(),
        body: DigitalPassportIssuanceOfferBody {
            supportsExpiration: false,
            defaultExpirationDays: uint(0),
            requiresHolderPublicKey: false,
        },
    };
    let issuance_request = RequestMessageCompact1 {
        envelope: envelope(
            issuance_request_id.clone(),
            thread.clone(),
            false,
            offer_id,
            2,
        ),
        schema: schema(),
        issuerVerificationMethodRef: issuer_method(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        body: DigitalPassportIssuanceRequestBody {
            holderBinding: holder_binding(),
            holderPublicKey: issuer_pk(),
            holderChallengeHash: issuance_challenge.clone(),
            requestExpiration: false,
            requestedExpirationDays: uint(0),
        },
    };
    let issuance_result = ResultMessageCompact1 {
        envelope: envelope(
            issuance_result_id,
            thread.clone(),
            false,
            issuance_request_id,
            3,
        ),
        schema: schema(),
        issuerVerificationMethodRef: issuer_method(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        body: DigitalPassportIssuanceResultBody {
            credential: credential.clone(),
            credentialProof: credential_proof.clone(),
            holderPublicKey: issuer_pk(),
            issuanceChallengeHash: issuance_challenge,
            privateParts: DigitalPassportCredentialPrivateParts {
                claimValues: DigitalPassportClaimValues {
                    firstNameValuePadded: first_name,
                    lastNameValuePadded: last_name,
                    dateOfBirthDays: uint(7400),
                    documentNumberValue: bytes32(0),
                    issuingStateValue: issuing_state,
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
            verification_request_id.clone(),
            thread.clone(),
            true,
            no_response(),
            4,
        ),
        schema: schema(),
        issuerVerificationMethodRef: issuer_method(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        features: features(),
        verifierChallengeHash: presentation_challenge.clone(),
        body: DigitalPassportVerificationRequestBody {
            requireFirstNameDisclosure: false,
            requireLastNameDisclosure: false,
            requireAgeOverThreshold: false,
            requestedAgeThresholdYears: uint(0),
            requireDocumentNumberDisclosure: false,
            requireIssuingStateDisclosure: false,
        },
    };
    let verification_submission = SubmissionMessage {
        envelope: envelope(
            submission_id.clone(),
            thread.clone(),
            false,
            verification_request_id,
            5,
        ),
        schema: schema(),
        issuerVerificationMethodRef: issuer_method(),
        holderBindingProfile: HolderBindingProfile::explicitDid,
        challengeHash: presentation_challenge,
        body: DigitalPassportVerificationSubmissionBody {
            credential: credential.clone(),
            credentialProof: credential_proof,
            presentation: presentation.clone(),
            presentationProof: presentation_proof,
        },
    };
    let verification_result = ResultMessage {
        envelope: envelope(verification_result_id, thread, false, submission_id, 6),
        approved: true,
        body: DigitalPassportVerificationResultBody {
            credentialRoot: credential_root,
            verifiedThresholdYears: uint(0),
        },
    };
    let presentation_request =
        pure_circuits::digitalPassportPresentationRequestFromProtocol(verification_request.clone())
            .unwrap();
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

fn assert_success(name: &str, result: Result<(), CompactError>, oracle: &serde_json::Value) {
    assert_eq!(oracle[name]["ok"], true, "{name}: oracle did not succeed");
    assert!(result.is_ok(), "{name}: {result:?}");
}

#[test]
fn passport_round_trip_matches_typescript_roots_and_validators() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/passport-dogfood-oracle.json"
    ))
    .unwrap();
    let steps = &oracle["roundTrip"];
    assert_eq!(steps.as_object().unwrap().len(), 13);
    let values = build_round_trip();
    let credential_root =
        pure_circuits::digitalPassportCredentialBodyRoot(values.credential.clone()).unwrap();
    assert_eq!(
        hex::encode(credential_root.into_array()),
        steps["credentialBodyRoot"]["result"]
    );
    let presentation_root =
        pure_circuits::digitalPassportPresentationBodyRoot(values.presentation.clone()).unwrap();
    assert_eq!(
        hex::encode(presentation_root.into_array()),
        steps["presentationBodyRoot"]["result"]
    );
    let request_root = pure_circuits::digitalPassportPresentationRequestBodyRoot(
        values.presentation_request.clone(),
    )
    .unwrap();
    assert_eq!(
        hex::encode(request_root.into_array()),
        steps["presentationRequestBodyRoot"]["result"]
    );
    assert_success(
        "assertValidDigitalPassportIssuanceOffer",
        pure_circuits::assertValidDigitalPassportIssuanceOffer(values.issuance_offer.clone()),
        steps,
    );
    assert_success(
        "assertValidDigitalPassportIssuanceRequest",
        pure_circuits::assertValidDigitalPassportIssuanceRequest(values.issuance_request.clone()),
        steps,
    );
    assert_success(
        "assertDigitalPassportIssuanceRequestMatchesOffer",
        pure_circuits::assertDigitalPassportIssuanceRequestMatchesOffer(
            values.issuance_offer.clone(),
            values.issuance_request.clone(),
        ),
        steps,
    );
    assert_success(
        "assertValidDigitalPassportIssuanceResult",
        pure_circuits::assertValidDigitalPassportIssuanceResult(values.issuance_result.clone()),
        steps,
    );
    assert_success(
        "assertDigitalPassportIssuanceResultMatchesRequest",
        pure_circuits::assertDigitalPassportIssuanceResultMatchesRequest(
            values.issuance_request.clone(),
            values.issuance_result.clone(),
        ),
        steps,
    );
    assert_success(
        "assertValidDigitalPassportVerificationRequestMessage",
        pure_circuits::assertValidDigitalPassportVerificationRequestMessage(
            values.verification_request.clone(),
        ),
        steps,
    );
    assert_success(
        "assertValidDigitalPassportVerificationSubmissionMessage",
        pure_circuits::assertValidDigitalPassportVerificationSubmissionMessage(
            values.verification_submission.clone(),
        ),
        steps,
    );
    assert_success(
        "assertDigitalPassportVerificationSubmissionMatchesRequest",
        pure_circuits::assertDigitalPassportVerificationSubmissionMatchesRequest(
            values.verification_request.clone(),
            values.verification_submission.clone(),
        ),
        steps,
    );
    assert_success(
        "assertValidDigitalPassportVerificationResultMessage",
        pure_circuits::assertValidDigitalPassportVerificationResultMessage(
            values.verification_result.clone(),
        ),
        steps,
    );
    assert_success(
        "assertDigitalPassportVerificationResultMatchesSubmission",
        pure_circuits::assertDigitalPassportVerificationResultMatchesSubmission(
            values.verification_submission,
            values.verification_result,
        ),
        steps,
    );
}

#[test]
fn proof_signatures_are_separated_by_issuance_and_presentation_context() {
    let values = build_round_trip();
    let credential_root =
        pure_circuits::digitalPassportCredentialBodyRoot(values.credential.clone()).unwrap();
    let presentation_root =
        pure_circuits::digitalPassportPresentationBodyRoot(values.presentation.clone()).unwrap();
    let issuance_proof = values.issuance_result.body.credentialProof;
    let presentation_proof = values.verification_submission.body.presentationProof;

    assert!(
        pure_circuits::assertValidIssuanceContextProof(
            credential_root.clone(),
            issuance_proof.clone(),
        )
        .is_ok()
    );
    assert!(matches!(
        pure_circuits::assertValidPresentationContextProof(credential_root, issuance_proof),
        Err(CompactError::AssertionFailed(_))
    ));
    assert!(
        pure_circuits::assertValidPresentationContextProof(
            presentation_root.clone(),
            presentation_proof.clone(),
        )
        .is_ok()
    );
    assert!(matches!(
        pure_circuits::assertValidIssuanceContextProof(presentation_root, presentation_proof),
        Err(CompactError::AssertionFailed(_))
    ));
}

#[test]
fn protocol_features_must_match_schema_capabilities() {
    let expected = capabilities();
    assert!(
        pure_circuits::assertMatchingSchemaCapabilities(expected.clone(), expected.clone(),)
            .is_ok()
    );
    assert!(
        pure_circuits::assertProtocolFeaturesMatchSchemaCapabilities(features(), expected.clone(),)
            .is_ok()
    );

    let mismatches = [
        SchemaCapabilities {
            supportsSelectiveDisclosure: false,
            ..expected.clone()
        },
        SchemaCapabilities {
            supportsPredicateProofs: false,
            ..expected.clone()
        },
        SchemaCapabilities {
            supportsVerifierScopedPseudonym: true,
            ..expected.clone()
        },
        SchemaCapabilities {
            supportsSameHolderProof: true,
            ..expected
        },
    ];
    for mismatched in mismatches {
        assert!(matches!(
            pure_circuits::assertMatchingSchemaCapabilities(capabilities(), mismatched.clone()),
            Err(CompactError::AssertionFailed(_))
        ));
        assert!(matches!(
            pure_circuits::assertProtocolFeaturesMatchSchemaCapabilities(features(), mismatched),
            Err(CompactError::AssertionFailed(_))
        ));
    }
}

#[test]
fn schema_descriptor_requires_the_no_hint_sentinel_when_hint_is_absent() {
    let mut descriptor = SchemaDescriptor {
        schema: schema(),
        capabilities: capabilities(),
        familyResolutionHint: SchemaFamilyResolutionHint {
            hasResolverHint: false,
            resolverHint: pure_circuits::noSchemaFamilyResolverHint().unwrap(),
        },
    };
    assert!(pure_circuits::assertValidSchemaDescriptor(descriptor.clone()).is_ok());

    descriptor.familyResolutionHint.resolverHint = bytes32(0);
    assert!(matches!(
        pure_circuits::assertValidSchemaDescriptor(descriptor.clone()),
        Err(CompactError::AssertionFailed(_))
    ));
    descriptor.familyResolutionHint.hasResolverHint = true;
    descriptor.familyResolutionHint.resolverHint =
        pure_circuits::noSchemaFamilyResolverHint().unwrap();
    assert!(matches!(
        pure_circuits::assertValidSchemaDescriptor(descriptor),
        Err(CompactError::AssertionFailed(_))
    ));
}
