// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_vc_passport_adoption_fixture::{pure_circuits as pure, runtime as rt, types};

fn b32(seed: u8) -> rt::FixedBytes<32> {
    rt::FixedBytes::new(std::array::from_fn(|index| seed.wrapping_add(index as u8)))
}
fn padded(text: &str) -> rt::FixedBytes<32> {
    let mut bytes = [0; 32];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    rt::FixedBytes::new(bytes)
}
fn hex(bytes: rt::FixedBytes<32>) -> String {
    hex::encode(bytes.into_array())
}
fn u16(value: u128) -> rt::BoundedUint<65535> {
    rt::BoundedUint::new(value).unwrap()
}
fn u64(value: u128) -> rt::BoundedUint<18446744073709551615> {
    rt::BoundedUint::new(value).unwrap()
}
fn schema() -> types::SchemaRef {
    types::SchemaRef {
        packageId: padded("midnight:vc:digital-passport"),
        schemaId: padded("digital-passport:v1"),
        majorVersion: u16(1),
        minorVersion: u16(0),
    }
}
fn method() -> types::VerificationMethodRef {
    types::VerificationMethodRef {
        controllerAddress: types::ContractAddress { bytes: b32(1) },
        methodId: b32(2),
    }
}
fn initial() -> types::ProtocolMessageEnvelope {
    types::ProtocolMessageEnvelope {
        version: u16(1),
        messageId: b32(10),
        threadId: b32(11),
        initialMessage: true,
        respondsToMessageId: pure::noProtocolResponseReference().unwrap(),
        createdAt: u64(100),
        hasExpiresAt: false,
        expiresAt: u64(0),
    }
}
fn response() -> types::ProtocolMessageEnvelope {
    types::ProtocolMessageEnvelope {
        messageId: b32(12),
        initialMessage: false,
        respondsToMessageId: b32(10),
        createdAt: u64(101),
        ..initial()
    }
}
fn features() -> types::CredentialProtocolFeatures {
    types::CredentialProtocolFeatures {
        supportsSelectiveDisclosure: true,
        supportsPredicateProofs: true,
        ..Default::default()
    }
}
fn offer() -> types::DigitalPassportIssuance_OfferMessage {
    types::DigitalPassportIssuance_OfferMessage {
        envelope: initial(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        features: features(),
        ..Default::default()
    }
}
fn issuance_request() -> types::DigitalPassportIssuance_RequestMessage {
    types::DigitalPassportIssuance_RequestMessage {
        envelope: response(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        body: types::DigitalPassportIssuanceRequestBody {
            holderBinding: types::ExplicitHolderBinding {
                holderVerificationMethodRef: method(),
            },
            holderChallengeHash: b32(13),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn verification_request() -> types::DigitalPassportVerification_RequestMessage {
    types::DigitalPassportVerification_RequestMessage {
        envelope: initial(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        features: features(),
        verifierChallengeHash: b32(14),
        body: types::DigitalPassportVerificationRequestBody {
            requireFirstNameDisclosure: true,
            requireAgeOverThreshold: true,
            requestedAgeThresholdYears: rt::BoundedUint::<255>::new(18).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn verification_result() -> types::DigitalPassportVerification_ResultMessage {
    types::DigitalPassportVerification_ResultMessage {
        envelope: response(),
        approved: true,
        body: types::DigitalPassportVerificationResultBody {
            credentialRoot: b32(15),
            verifiedThresholdYears: rt::BoundedUint::<255>::new(18).unwrap(),
        },
    }
}

fn final_response() -> types::ProtocolMessageEnvelope {
    types::ProtocolMessageEnvelope {
        messageId: b32(16),
        respondsToMessageId: b32(12),
        createdAt: u64(102),
        ..response()
    }
}

fn protocol_credential() -> types::Credential {
    types::Credential {
        version: u16(1),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        holderBinding: types::ExplicitHolderBinding {
            holderVerificationMethodRef: method(),
        },
        issuedAt: u64(100),
        claimCommitments: types::DigitalPassportClaimCommitments {
            firstNameCommitment: b32(20),
            lastNameCommitment: b32(21),
            dateOfBirthCommitment: b32(22),
            documentNumberCommitment: b32(23),
            issuingStateCommitment: b32(24),
        },
        claimRoot: b32(25),
        ..Default::default()
    }
}

fn protocol_proof() -> types::Proof {
    types::Proof {
        signerVerificationMethodRef: method(),
        createdAt: u64(101),
        challengeHash: b32(13),
        ..Default::default()
    }
}

fn protocol_private_parts() -> types::DigitalPassportCredentialPrivateParts {
    types::DigitalPassportCredentialPrivateParts {
        claimValues: types::DigitalPassportClaimValues {
            firstNameValuePadded: rt::FixedBytes::new(std::array::from_fn(|i| 30 + i as u8)),
            lastNameValuePadded: rt::FixedBytes::new(std::array::from_fn(|i| 31 + i as u8)),
            dateOfBirthDays: rt::BoundedUint::<4294967295>::new(12345).unwrap(),
            documentNumberValue: b32(32),
            issuingStateValue: b32(33),
        },
        openings: types::DigitalPassportOpenings {
            firstNameOpening: b32(40),
            lastNameOpening: b32(41),
            dateOfBirthOpening: b32(42),
            documentNumberOpening: b32(43),
            issuingStateOpening: b32(44),
        },
    }
}

fn issuance_result() -> types::DigitalPassportIssuance_ResultMessage {
    types::DigitalPassportIssuance_ResultMessage {
        envelope: final_response(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        holderBindingProfile: types::HolderBindingProfile::explicitDid,
        body: types::DigitalPassportIssuanceResultBody {
            credential: protocol_credential(),
            credentialProof: protocol_proof(),
            holderPublicKey: rt::JubjubPoint::identity(),
            issuanceChallengeHash: b32(13),
            privateParts: protocol_private_parts(),
        },
    }
}

fn verification_submission() -> types::DigitalPassportVerification_SubmissionMessage {
    types::DigitalPassportVerification_SubmissionMessage {
        envelope: response(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        holderBindingProfile: types::HolderBindingProfile::explicitDid,
        challengeHash: b32(14),
        body: types::DigitalPassportVerificationSubmissionBody {
            credential: protocol_credential(),
            credentialProof: protocol_proof(),
            presentation: types::Presentation {
                version: u16(1),
                schema: schema(),
                credentialClaimRoot: b32(25),
                issuerVerificationMethodRef: method(),
                holderBinding: types::ExplicitHolderBinding {
                    holderVerificationMethodRef: method(),
                },
                disclosed: types::DigitalPassportDisclosures {
                    firstNameValuePadded: rt::FixedBytes::new(std::array::from_fn(|i| {
                        50 + i as u8
                    })),
                    firstNameOpening: b32(51),
                    lastNameValuePadded: rt::FixedBytes::new(std::array::from_fn(|i| 52 + i as u8)),
                    lastNameOpening: b32(53),
                    documentNumberValue: b32(54),
                    documentNumberOpening: b32(55),
                    issuingStateValue: b32(56),
                    issuingStateOpening: b32(57),
                    ..Default::default()
                },
            },
            presentationProof: protocol_proof(),
        },
    }
}
fn assert_case(name: &str, result: Result<(), rt::CompactError>, rows: &[serde_json::Value]) {
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    if row["outcome"] == "ok" {
        assert_eq!(row["value"], serde_json::json!([]), "{name}");
        assert!(result.is_ok(), "{name}: {result:?}");
    } else {
        assert_eq!(
            result.unwrap_err().to_string(),
            row["message"].as_str().unwrap(),
            "{name}"
        );
    }
}
fn row<'a>(name: &str, rows: &'a [serde_json::Value]) -> &'a serde_json::Value {
    rows.iter().find(|row| row["name"] == name).unwrap()
}
fn request_json(request: types::DigitalPassportPresentationRequest) -> serde_json::Value {
    serde_json::json!({
        "version": request.version.value().to_string(),
        "schema": {
            "packageId": hex(request.schema.packageId),
            "schemaId": hex(request.schema.schemaId),
            "majorVersion": request.schema.majorVersion.value().to_string(),
            "minorVersion": request.schema.minorVersion.value().to_string()
        },
        "issuerVerificationMethodRef": {
            "controllerAddress": {"bytes": hex(request.issuerVerificationMethodRef.controllerAddress.bytes)},
            "methodId": hex(request.issuerVerificationMethodRef.methodId)
        },
        "requireFirstNameDisclosure": request.requireFirstNameDisclosure,
        "requireLastNameDisclosure": request.requireLastNameDisclosure,
        "requireAgeOverThreshold": request.requireAgeOverThreshold,
        "requestedAgeThresholdYears": request.requestedAgeThresholdYears.value().to_string(),
        "requireDocumentNumberDisclosure": request.requireDocumentNumberDisclosure,
        "requireIssuingStateDisclosure": request.requireIssuingStateDisclosure,
        "verifierChallengeHash": hex(request.verifierChallengeHash)
    })
}

#[test]
fn protocol_envelopes_and_wrappers_match_dual_typescript_capture() {
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-protocol-capture.json")).unwrap();
    let rows = capture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 20);
    assert_case(
        "response_envelope_valid",
        pure::assertProtocolResponseEnvelope(initial(), response()),
        rows,
    );
    assert_case(
        "response_envelope_wrong_thread",
        pure::assertProtocolResponseEnvelope(
            initial(),
            types::ProtocolMessageEnvelope {
                threadId: b32(99),
                ..response()
            },
        ),
        rows,
    );
    assert_case(
        "response_envelope_wrong_previous",
        pure::assertProtocolResponseEnvelope(
            initial(),
            types::ProtocolMessageEnvelope {
                respondsToMessageId: b32(98),
                ..response()
            },
        ),
        rows,
    );
    assert_case(
        "response_envelope_early",
        pure::assertProtocolResponseEnvelope(
            initial(),
            types::ProtocolMessageEnvelope {
                createdAt: u64(99),
                ..response()
            },
        ),
        rows,
    );
    assert_case(
        "response_envelope_initial",
        pure::assertProtocolResponseEnvelope(
            initial(),
            types::ProtocolMessageEnvelope {
                messageId: b32(12),
                ..initial()
            },
        ),
        rows,
    );
    assert_case(
        "issuance_offer_valid",
        pure::DigitalPassportIssuance_assertValidOfferMessage(offer()),
        rows,
    );
    assert_case(
        "issuance_offer_response_envelope",
        pure::DigitalPassportIssuance_assertValidOfferMessage(
            types::DigitalPassportIssuance_OfferMessage {
                envelope: response(),
                ..offer()
            },
        ),
        rows,
    );
    assert_case(
        "issuance_request_valid",
        pure::DigitalPassportIssuance_assertValidRequestMessage(issuance_request()),
        rows,
    );
    assert_case(
        "issuance_request_initial_envelope",
        pure::DigitalPassportIssuance_assertValidRequestMessage(
            types::DigitalPassportIssuance_RequestMessage {
                envelope: initial(),
                ..issuance_request()
            },
        ),
        rows,
    );
    assert_case(
        "issuance_alignment_valid",
        pure::DigitalPassportIssuance_assertOfferRequestAlignment(offer(), issuance_request()),
        rows,
    );
    assert_case(
        "issuance_alignment_wrong_method",
        pure::DigitalPassportIssuance_assertOfferRequestAlignment(
            offer(),
            types::DigitalPassportIssuance_RequestMessage {
                issuerVerificationMethodRef: types::VerificationMethodRef {
                    methodId: b32(55),
                    ..method()
                },
                ..issuance_request()
            },
        ),
        rows,
    );
    assert_case(
        "issuance_alignment_wrong_schema",
        pure::DigitalPassportIssuance_assertOfferRequestAlignment(
            offer(),
            types::DigitalPassportIssuance_RequestMessage {
                schema: types::SchemaRef {
                    minorVersion: u16(1),
                    ..schema()
                },
                ..issuance_request()
            },
        ),
        rows,
    );
    assert_case(
        "issuance_alignment_wrong_thread",
        pure::DigitalPassportIssuance_assertOfferRequestAlignment(
            offer(),
            types::DigitalPassportIssuance_RequestMessage {
                envelope: types::ProtocolMessageEnvelope {
                    threadId: b32(88),
                    ..response()
                },
                ..issuance_request()
            },
        ),
        rows,
    );
    assert_case(
        "verification_request_valid",
        pure::DigitalPassportVerification_assertValidRequestMessage(verification_request()),
        rows,
    );
    assert_case(
        "verification_request_response_envelope",
        pure::DigitalPassportVerification_assertValidRequestMessage(
            types::DigitalPassportVerification_RequestMessage {
                envelope: response(),
                ..verification_request()
            },
        ),
        rows,
    );
    assert_case(
        "verification_result_valid",
        pure::DigitalPassportVerification_assertValidResultMessage(verification_result()),
        rows,
    );
    assert_case(
        "verification_result_initial_envelope",
        pure::DigitalPassportVerification_assertValidResultMessage(
            types::DigitalPassportVerification_ResultMessage {
                envelope: initial(),
                ..verification_result()
            },
        ),
        rows,
    );
    let mapped =
        pure::digitalPassportPresentationRequestFromProtocol(verification_request()).unwrap();
    assert_eq!(
        request_json(mapped.clone()),
        row("presentation_from_protocol", rows)["value"]
    );
    let future = pure::digitalPassportPresentationRequestFromProtocol(
        types::DigitalPassportVerification_RequestMessage {
            envelope: types::ProtocolMessageEnvelope {
                version: u16(2),
                ..initial()
            },
            ..verification_request()
        },
    )
    .unwrap();
    assert_eq!(
        request_json(future),
        row("presentation_from_future_transport", rows)["value"]
    );
    assert_eq!(
        hex(pure::digitalPassportPresentationRequestBodyRoot(mapped).unwrap()),
        row("presentation_request_body_root", rows)["value"]
    );
}

#[test]
fn protocol_roundtrip_guards_match_dual_typescript_capture() {
    let capture: serde_json::Value = serde_json::from_str(include_str!(
        "../oracle/upstream-protocol-roundtrip-capture.json"
    ))
    .unwrap();
    let rows = capture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 20);
    assert_case(
        "generic_issuance_result_valid",
        pure::DigitalPassportIssuance_assertValidResultMessage(issuance_result()),
        rows,
    );
    assert_case(
        "generic_issuance_result_initial",
        pure::DigitalPassportIssuance_assertValidResultMessage(
            types::DigitalPassportIssuance_ResultMessage {
                envelope: initial(),
                ..issuance_result()
            },
        ),
        rows,
    );
    assert_case(
        "generic_issuance_result_alignment",
        pure::DigitalPassportIssuance_assertRequestResultAlignment(
            issuance_request(),
            issuance_result(),
        ),
        rows,
    );
    assert_case(
        "generic_issuance_result_wrong_thread",
        pure::DigitalPassportIssuance_assertRequestResultAlignment(
            issuance_request(),
            types::DigitalPassportIssuance_ResultMessage {
                envelope: types::ProtocolMessageEnvelope {
                    threadId: b32(88),
                    ..final_response()
                },
                ..issuance_result()
            },
        ),
        rows,
    );
    assert_case(
        "generic_submission_valid",
        pure::DigitalPassportVerification_assertValidSubmissionMessage(verification_submission()),
        rows,
    );
    assert_case(
        "generic_submission_initial",
        pure::DigitalPassportVerification_assertValidSubmissionMessage(
            types::DigitalPassportVerification_SubmissionMessage {
                envelope: initial(),
                ..verification_submission()
            },
        ),
        rows,
    );
    assert_case(
        "generic_submission_alignment",
        pure::DigitalPassportVerification_assertRequestSubmissionAlignment(
            verification_request(),
            verification_submission(),
        ),
        rows,
    );
    assert_case(
        "generic_submission_wrong_challenge",
        pure::DigitalPassportVerification_assertRequestSubmissionAlignment(
            verification_request(),
            types::DigitalPassportVerification_SubmissionMessage {
                challengeHash: b32(90),
                ..verification_submission()
            },
        ),
        rows,
    );
    assert_case(
        "generic_result_alignment",
        pure::DigitalPassportVerification_assertSubmissionResultAlignment(
            verification_submission(),
            types::DigitalPassportVerification_ResultMessage {
                envelope: final_response(),
                ..verification_result()
            },
        ),
        rows,
    );
    assert_case(
        "generic_result_wrong_previous",
        pure::DigitalPassportVerification_assertSubmissionResultAlignment(
            verification_submission(),
            types::DigitalPassportVerification_ResultMessage {
                envelope: types::ProtocolMessageEnvelope {
                    respondsToMessageId: b32(89),
                    ..final_response()
                },
                ..verification_result()
            },
        ),
        rows,
    );
    assert_case(
        "passport_offer_valid",
        pure::assertValidDigitalPassportIssuanceOffer(offer()),
        rows,
    );
    assert_case(
        "passport_offer_bad_expiration",
        pure::assertValidDigitalPassportIssuanceOffer(
            types::DigitalPassportIssuance_OfferMessage {
                body: types::DigitalPassportIssuanceOfferBody {
                    defaultExpirationDays: u16(1),
                    ..offer().body
                },
                ..offer()
            },
        ),
        rows,
    );
    assert_case(
        "passport_request_valid",
        pure::assertValidDigitalPassportIssuanceRequest(issuance_request()),
        rows,
    );
    assert_case(
        "passport_request_missing_challenge",
        pure::assertValidDigitalPassportIssuanceRequest(
            types::DigitalPassportIssuance_RequestMessage {
                body: types::DigitalPassportIssuanceRequestBody {
                    holderChallengeHash: pure::noProtocolResponseReference().unwrap(),
                    ..issuance_request().body
                },
                ..issuance_request()
            },
        ),
        rows,
    );
    assert_case(
        "passport_offer_request_match",
        pure::assertDigitalPassportIssuanceRequestMatchesOffer(offer(), issuance_request()),
        rows,
    );
    assert_case(
        "passport_offer_request_unsupported_expiration",
        pure::assertDigitalPassportIssuanceRequestMatchesOffer(
            offer(),
            types::DigitalPassportIssuance_RequestMessage {
                body: types::DigitalPassportIssuanceRequestBody {
                    requestExpiration: true,
                    requestedExpirationDays: u16(1),
                    ..issuance_request().body
                },
                ..issuance_request()
            },
        ),
        rows,
    );
    assert_case(
        "passport_verification_request_valid",
        pure::assertValidDigitalPassportVerificationRequestMessage(verification_request()),
        rows,
    );
    assert_case(
        "passport_verification_request_missing_method",
        pure::assertValidDigitalPassportVerificationRequestMessage(
            types::DigitalPassportVerification_RequestMessage {
                issuerVerificationMethodRef: types::VerificationMethodRef {
                    methodId: rt::FixedBytes::new([0; 32]),
                    ..method()
                },
                ..verification_request()
            },
        ),
        rows,
    );
    assert_case(
        "passport_verification_result_valid",
        pure::assertValidDigitalPassportVerificationResultMessage(
            types::DigitalPassportVerification_ResultMessage {
                envelope: final_response(),
                ..verification_result()
            },
        ),
        rows,
    );
    assert_case(
        "passport_verification_result_missing_root",
        pure::assertValidDigitalPassportVerificationResultMessage(
            types::DigitalPassportVerification_ResultMessage {
                envelope: final_response(),
                body: types::DigitalPassportVerificationResultBody {
                    credentialRoot: pure::noProtocolResponseReference().unwrap(),
                    ..verification_result().body
                },
                ..verification_result()
            },
        ),
        rows,
    );
}
