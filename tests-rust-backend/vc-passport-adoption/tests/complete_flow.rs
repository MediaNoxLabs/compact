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
fn b64(seed: u8) -> rt::FixedBytes<64> {
    rt::FixedBytes::new(std::array::from_fn(|index| seed.wrapping_add(index as u8)))
}
fn padded(text: &str) -> rt::FixedBytes<32> {
    let mut bytes = [0; 32];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    rt::FixedBytes::new(bytes)
}
fn u16(value: u128) -> rt::BoundedUint<65535> {
    rt::BoundedUint::new(value).unwrap()
}
fn u64(value: u128) -> rt::BoundedUint<18446744073709551615> {
    rt::BoundedUint::new(value).unwrap()
}
fn scalar_from_hex(text: &str) -> rt::Field {
    let mut bytes = hex::decode(format!("{text:0>64}")).unwrap();
    bytes.reverse();
    rt::Field::from_le_bytes(&bytes).unwrap()
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
fn private_parts() -> types::DigitalPassportCredentialPrivateParts {
    types::DigitalPassportCredentialPrivateParts {
        claimValues: types::DigitalPassportClaimValues {
            firstNameValuePadded: b64(10),
            lastNameValuePadded: b64(20),
            dateOfBirthDays: rt::BoundedUint::<4294967295>::new(12345).unwrap(),
            documentNumberValue: b32(30),
            issuingStateValue: b32(40),
        },
        openings: types::DigitalPassportOpenings {
            firstNameOpening: b32(100),
            lastNameOpening: b32(110),
            dateOfBirthOpening: b32(120),
            documentNumberOpening: b32(130),
            issuingStateOpening: b32(140),
        },
    }
}
fn credential(parts: &types::DigitalPassportCredentialPrivateParts) -> types::Credential {
    let claims = &parts.claimValues;
    let openings = &parts.openings;
    let commitments = types::DigitalPassportClaimCommitments {
        firstNameCommitment: pure::firstNameCommitment(
            claims.firstNameValuePadded,
            openings.firstNameOpening,
        )
        .unwrap(),
        lastNameCommitment: pure::lastNameCommitment(
            claims.lastNameValuePadded,
            openings.lastNameOpening,
        )
        .unwrap(),
        dateOfBirthCommitment: pure::dateOfBirthCommitment(
            claims.dateOfBirthDays,
            openings.dateOfBirthOpening,
        )
        .unwrap(),
        documentNumberCommitment: pure::documentNumberCommitment(
            claims.documentNumberValue,
            openings.documentNumberOpening,
        )
        .unwrap(),
        issuingStateCommitment: pure::issuingStateCommitment(
            claims.issuingStateValue,
            openings.issuingStateOpening,
        )
        .unwrap(),
    };
    types::Credential {
        version: u16(1),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        holderBinding: types::ExplicitHolderBinding {
            holderVerificationMethodRef: method(),
        },
        issuedAt: u64(100),
        claimRoot: pure::digitalPassportClaimRoot(commitments.clone()).unwrap(),
        claimCommitments: commitments,
        ..Default::default()
    }
}
fn presentation(
    credential: &types::Credential,
    parts: &types::DigitalPassportCredentialPrivateParts,
) -> types::Presentation {
    types::Presentation {
        version: u16(1),
        schema: schema(),
        credentialClaimRoot: credential.claimRoot,
        issuerVerificationMethodRef: method(),
        holderBinding: credential.holderBinding.clone(),
        disclosed: types::DigitalPassportDisclosures {
            revealFirstName: true,
            firstNameValuePadded: parts.claimValues.firstNameValuePadded,
            firstNameOpening: parts.openings.firstNameOpening,
            lastNameValuePadded: parts.claimValues.lastNameValuePadded,
            lastNameOpening: parts.openings.lastNameOpening,
            documentNumberValue: parts.claimValues.documentNumberValue,
            documentNumberOpening: parts.openings.documentNumberOpening,
            issuingStateValue: parts.claimValues.issuingStateValue,
            issuingStateOpening: parts.openings.issuingStateOpening,
            ..Default::default()
        },
    }
}
fn proof(nonce: u64, scalar: &str) -> types::Proof {
    types::Proof {
        signerVerificationMethodRef: method(),
        createdAt: u64(101),
        challengeHash: b32(55),
        publicKey: rt::ec_mul_generator(rt::Field::from(3_u64)).unwrap(),
        signature: types::Signature {
            r: rt::ec_mul_generator(rt::Field::from(nonce)).unwrap(),
            s: scalar_from_hex(scalar),
        },
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
        ..Default::default()
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
fn final_response() -> types::ProtocolMessageEnvelope {
    types::ProtocolMessageEnvelope {
        messageId: b32(16),
        respondsToMessageId: b32(12),
        createdAt: u64(102),
        ..response()
    }
}
fn features() -> types::CredentialProtocolFeatures {
    types::CredentialProtocolFeatures {
        supportsSelectiveDisclosure: true,
        supportsPredicateProofs: true,
        ..Default::default()
    }
}
fn issuance_request() -> types::DigitalPassportIssuance_RequestMessage {
    types::DigitalPassportIssuance_RequestMessage {
        envelope: response(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        holderBindingProfile: types::HolderBindingProfile::explicitDid,
        body: types::DigitalPassportIssuanceRequestBody {
            holderBinding: types::ExplicitHolderBinding {
                holderVerificationMethodRef: method(),
            },
            holderPublicKey: rt::ec_mul_generator(rt::Field::from(5_u64)).unwrap(),
            holderChallengeHash: b32(55),
            ..Default::default()
        },
    }
}
fn issuance_result(
    credential: &types::Credential,
    credential_proof: &types::Proof,
    parts: &types::DigitalPassportCredentialPrivateParts,
) -> types::DigitalPassportIssuance_ResultMessage {
    types::DigitalPassportIssuance_ResultMessage {
        envelope: final_response(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        holderBindingProfile: types::HolderBindingProfile::explicitDid,
        body: types::DigitalPassportIssuanceResultBody {
            credential: credential.clone(),
            credentialProof: credential_proof.clone(),
            holderPublicKey: issuance_request().body.holderPublicKey,
            issuanceChallengeHash: b32(55),
            privateParts: parts.clone(),
        },
    }
}
fn verification_request() -> types::DigitalPassportVerification_RequestMessage {
    types::DigitalPassportVerification_RequestMessage {
        envelope: initial(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        holderBindingProfile: types::HolderBindingProfile::explicitDid,
        features: features(),
        verifierChallengeHash: b32(55),
        body: types::DigitalPassportVerificationRequestBody {
            requireFirstNameDisclosure: true,
            ..Default::default()
        },
    }
}
fn submission(
    credential: &types::Credential,
    credential_proof: &types::Proof,
    presentation: &types::Presentation,
    presentation_proof: &types::Proof,
) -> types::DigitalPassportVerification_SubmissionMessage {
    types::DigitalPassportVerification_SubmissionMessage {
        envelope: response(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        holderBindingProfile: types::HolderBindingProfile::explicitDid,
        challengeHash: b32(55),
        body: types::DigitalPassportVerificationSubmissionBody {
            credential: credential.clone(),
            credentialProof: credential_proof.clone(),
            presentation: presentation.clone(),
            presentationProof: presentation_proof.clone(),
        },
    }
}
fn verification_result(
    credential_root: rt::FixedBytes<32>,
) -> types::DigitalPassportVerification_ResultMessage {
    types::DigitalPassportVerification_ResultMessage {
        envelope: final_response(),
        approved: true,
        body: types::DigitalPassportVerificationResultBody {
            credentialRoot: credential_root,
            ..Default::default()
        },
    }
}
fn assert_case(name: &str, actual: Result<(), rt::CompactError>, rows: &[serde_json::Value]) {
    let expected = rows.iter().find(|row| row["name"] == name).unwrap();
    if expected["outcome"] == "ok" {
        assert_eq!(expected["value"], serde_json::json!([]), "{name}");
        assert!(actual.is_ok(), "{name}: {actual:?}");
    } else {
        assert_eq!(
            actual.unwrap_err().to_string(),
            expected["message"].as_str().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn complete_signed_issuance_and_verification_match_dual_typescript() {
    let capture: serde_json::Value = serde_json::from_str(include_str!(
        "../oracle/upstream-complete-flow-capture.json"
    ))
    .unwrap();
    let rows = capture["rows"].as_array().unwrap();
    let vectors = &capture["vectors"];
    assert_eq!(rows.len(), 10);
    let parts = private_parts();
    let credential = credential(&parts);
    let presentation = presentation(&credential, &parts);
    let credential_root = pure::digitalPassportCredentialBodyRoot(credential.clone()).unwrap();
    let presentation_root =
        pure::digitalPassportPresentationBodyRoot(presentation.clone()).unwrap();
    assert_eq!(
        hex::encode(credential_root.into_array()),
        vectors["credentialBodyRoot"]
    );
    assert_eq!(
        hex::encode(presentation_root.into_array()),
        vectors["presentationBodyRoot"]
    );
    let credential_proof = proof(7, vectors["issuanceScalar"].as_str().unwrap());
    let presentation_proof = proof(11, vectors["presentationScalar"].as_str().unwrap());
    let issuance_result = issuance_result(&credential, &credential_proof, &parts);
    let submission = submission(
        &credential,
        &credential_proof,
        &presentation,
        &presentation_proof,
    );
    let verification_result = verification_result(credential_root);
    assert_case(
        "full_issuance_result_valid",
        pure::assertValidDigitalPassportIssuanceResult(issuance_result.clone()),
        rows,
    );
    let mut changed_parts = issuance_result.clone();
    changed_parts.body.privateParts.openings.firstNameOpening = b32(101);
    assert_case(
        "full_issuance_result_private_tamper",
        pure::assertValidDigitalPassportIssuanceResult(changed_parts),
        rows,
    );
    assert_case(
        "full_issuance_result_matches_request",
        pure::assertDigitalPassportIssuanceResultMatchesRequest(
            issuance_request(),
            issuance_result.clone(),
        ),
        rows,
    );
    let mut changed_holder = issuance_result;
    changed_holder.body.holderPublicKey = rt::ec_mul_generator(rt::Field::from(6_u64)).unwrap();
    assert_case(
        "full_issuance_result_other_holder",
        pure::assertDigitalPassportIssuanceResultMatchesRequest(issuance_request(), changed_holder),
        rows,
    );
    assert_case(
        "full_verification_submission_valid",
        pure::assertValidDigitalPassportVerificationSubmissionMessage(submission.clone()),
        rows,
    );
    let mut changed_challenge = submission.clone();
    changed_challenge.challengeHash = b32(56);
    assert_case(
        "full_verification_submission_other_challenge",
        pure::assertValidDigitalPassportVerificationSubmissionMessage(changed_challenge),
        rows,
    );
    assert_case(
        "full_submission_matches_request",
        pure::assertDigitalPassportVerificationSubmissionMatchesRequest(
            verification_request(),
            submission.clone(),
        ),
        rows,
    );
    let mut stricter_request = verification_request();
    stricter_request.body.requireLastNameDisclosure = true;
    assert_case(
        "full_submission_missing_disclosure",
        pure::assertDigitalPassportVerificationSubmissionMatchesRequest(
            stricter_request,
            submission.clone(),
        ),
        rows,
    );
    assert_case(
        "full_result_matches_submission",
        pure::assertDigitalPassportVerificationResultMatchesSubmission(
            submission.clone(),
            verification_result.clone(),
        ),
        rows,
    );
    let mut other_result = verification_result;
    other_result.body.credentialRoot = b32(99);
    assert_case(
        "full_result_other_root",
        pure::assertDigitalPassportVerificationResultMatchesSubmission(submission, other_result),
        rows,
    );
}
