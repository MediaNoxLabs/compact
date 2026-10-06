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
fn field_hex(value: rt::Field) -> String {
    let bytes = value.as_le_bytes();
    let hex = hex::encode(bytes.iter().rev().copied().collect::<Vec<_>>());
    hex.trim_start_matches('0').to_owned()
}
fn scalar_from_hex(text: &str) -> rt::Field {
    let mut bytes = hex::decode(format!("{text:0>64}")).unwrap();
    bytes.reverse();
    rt::Field::from_le_bytes(&bytes).unwrap()
}
fn method() -> types::VerificationMethodRef {
    types::VerificationMethodRef {
        controllerAddress: types::ContractAddress { bytes: b32(1) },
        methodId: b32(2),
    }
}
fn schema() -> types::SchemaRef {
    types::SchemaRef {
        packageId: padded("midnight:vc:digital-passport"),
        schemaId: padded("digital-passport:v1"),
        majorVersion: rt::BoundedUint::<65535>::new(1).unwrap(),
        minorVersion: rt::BoundedUint::<65535>::new(0).unwrap(),
    }
}
fn credential() -> types::Credential {
    let commitments = types::DigitalPassportClaimCommitments {
        firstNameCommitment: pure::firstNameCommitment(b64(20), b32(21)).unwrap(),
        lastNameCommitment: b32(11),
        dateOfBirthCommitment: b32(12),
        documentNumberCommitment: b32(13),
        issuingStateCommitment: b32(14),
    };
    types::Credential {
        version: rt::BoundedUint::<65535>::new(1).unwrap(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        holderBinding: types::ExplicitHolderBinding {
            holderVerificationMethodRef: method(),
        },
        issuedAt: rt::BoundedUint::<18446744073709551615>::new(100).unwrap(),
        claimRoot: pure::digitalPassportClaimRoot(commitments.clone()).unwrap(),
        claimCommitments: commitments,
        ..Default::default()
    }
}
fn presentation(credential: &types::Credential) -> types::Presentation {
    types::Presentation {
        version: rt::BoundedUint::<65535>::new(1).unwrap(),
        schema: schema(),
        credentialClaimRoot: credential.claimRoot,
        issuerVerificationMethodRef: method(),
        holderBinding: credential.holderBinding.clone(),
        disclosed: types::DigitalPassportDisclosures {
            revealFirstName: true,
            firstNameValuePadded: b64(20),
            firstNameOpening: b32(21),
            lastNameValuePadded: b64(22),
            lastNameOpening: b32(23),
            documentNumberValue: b32(24),
            documentNumberOpening: b32(25),
            issuingStateValue: b32(26),
            issuingStateOpening: b32(27),
            ..Default::default()
        },
    }
}
fn signed_proof(nonce: u64, scalar: &str) -> types::Proof {
    types::Proof {
        signerVerificationMethodRef: method(),
        createdAt: rt::BoundedUint::<18446744073709551615>::new(101).unwrap(),
        challengeHash: b32(55),
        publicKey: rt::ec_mul_generator(rt::Field::from(3_u64)).unwrap(),
        signature: types::Signature {
            r: rt::ec_mul_generator(rt::Field::from(nonce)).unwrap(),
            s: scalar_from_hex(scalar),
        },
    }
}
fn request() -> types::DigitalPassportPresentationRequest {
    types::DigitalPassportPresentationRequest {
        version: rt::BoundedUint::<65535>::new(1).unwrap(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        requireFirstNameDisclosure: true,
        verifierChallengeHash: b32(55),
        ..Default::default()
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
fn assert_point(point: rt::JubjubPoint, vector: &serde_json::Value) {
    assert_eq!(field_hex(point.x().unwrap()), vector["x"].as_str().unwrap());
    assert_eq!(field_hex(point.y().unwrap()), vector["y"].as_str().unwrap());
}

fn assert_root_case(
    name: &str,
    actual: Result<rt::FixedBytes<32>, rt::CompactError>,
    rows: &[serde_json::Value],
) {
    let expected = rows.iter().find(|row| row["name"] == name).unwrap();
    if expected["outcome"] == "ok" {
        assert_eq!(
            hex::encode(actual.unwrap().into_array()),
            expected["value"],
            "{name}"
        );
    } else {
        assert_eq!(
            actual.unwrap_err().to_string(),
            expected["message"].as_str().unwrap(),
            "{name}"
        );
    }
}

fn authorization_descriptor() -> types::AuthorizedSignerDescriptor {
    types::AuthorizedSignerDescriptor {
        version: rt::BoundedUint::<65535>::new(1).unwrap(),
        authorizationId: b32(40),
        decisionSequence: rt::BoundedUint::<18446744073709551615>::new(1).unwrap(),
        state: types::AuthorizationState::active,
        role: types::SignerRole::issuer,
        signerVerificationMethodRef: method(),
        signerPublicKey: rt::ec_mul_generator(rt::Field::from(3_u64)).unwrap(),
        didStateVersion: rt::BoundedUint::<18446744073709551615>::new(1).unwrap(),
        verificationRelationship: types::VerificationRelationship::assertionMethod,
        scopeCommitment: pure::issuerScopeCommitment(schema()).unwrap(),
        policyCommitment: b32(41),
    }
}

#[test]
fn authorization_descriptor_and_issuer_binding_match_pinned_typescript() {
    let capture: serde_json::Value = serde_json::from_str(include_str!(
        "../oracle/upstream-authorization-capture.json"
    ))
    .unwrap();
    let rows = capture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 25);
    let vectors = &capture["vectors"];
    let descriptor = authorization_descriptor();
    let proof = types::Proof {
        signerVerificationMethodRef: method(),
        createdAt: rt::BoundedUint::<18446744073709551615>::new(101).unwrap(),
        challengeHash: b32(55),
        publicKey: rt::ec_mul_generator(rt::Field::from(3_u64)).unwrap(),
        signature: types::Signature {
            r: rt::ec_mul_generator(rt::Field::from(7_u64)).unwrap(),
            s: rt::Field::from(0_u64),
        },
    };
    assert_case(
        "auth_descriptor_valid",
        pure::assertValidAuthorizedSignerDescriptor(descriptor.clone()),
        rows,
    );
    assert_root_case(
        "auth_descriptor_root",
        pure::authorizedSignerDescriptorRoot(descriptor.clone()),
        rows,
    );
    let mut wrong_version = descriptor.clone();
    wrong_version.version = rt::BoundedUint::<65535>::new(2).unwrap();
    assert_case(
        "auth_descriptor_bad_version",
        pure::assertValidAuthorizedSignerDescriptor(wrong_version),
        rows,
    );
    let mut identity = descriptor.clone();
    identity.signerPublicKey = rt::JubjubPoint::identity();
    assert_case(
        "auth_descriptor_identity_key",
        pure::assertValidAuthorizedSignerDescriptor(identity),
        rows,
    );
    let mut wrong_relation = descriptor.clone();
    wrong_relation.verificationRelationship = types::VerificationRelationship::authentication;
    assert_case(
        "auth_descriptor_wrong_relation",
        pure::assertValidAuthorizedSignerDescriptor(wrong_relation),
        rows,
    );
    assert_root_case(
        "auth_scope_commitment",
        pure::issuerScopeCommitment(schema()),
        rows,
    );
    assert_case(
        "auth_proof_matches",
        pure::assertProofSignerMatchesAuthorization(proof.clone(), descriptor.clone()),
        rows,
    );
    let mut other_key_proof = proof.clone();
    other_key_proof.publicKey = rt::ec_mul_generator(rt::Field::from(4_u64)).unwrap();
    assert_case(
        "auth_proof_wrong_key",
        pure::assertProofSignerMatchesAuthorization(other_key_proof, descriptor.clone()),
        rows,
    );
    let mut suspended = descriptor.clone();
    suspended.state = types::AuthorizationState::suspended;
    assert_case(
        "auth_proof_suspended",
        pure::assertProofSignerMatchesAuthorization(proof.clone(), suspended),
        rows,
    );
    assert_case(
        "auth_issuer_valid",
        pure::assertAuthorizedIssuerDescriptor(schema(), proof.clone(), descriptor.clone()),
        rows,
    );
    let mut other_schema = schema();
    other_schema.schemaId = b32(33);
    assert_case(
        "auth_issuer_wrong_scope",
        pure::assertAuthorizedIssuerDescriptor(other_schema, proof.clone(), descriptor.clone()),
        rows,
    );
    let mut verifier = descriptor.clone();
    verifier.role = types::SignerRole::verifier;
    verifier.verificationRelationship = types::VerificationRelationship::authentication;
    assert_case(
        "auth_issuer_wrong_role",
        pure::assertAuthorizedIssuerDescriptor(schema(), proof, verifier),
        rows,
    );
    let authority = types::SignerAuthorizationAuthority {
        domainCommitment: b32(42),
        verificationMethodRef: method(),
        publicKey: rt::ec_mul_generator(rt::Field::from(5_u64)).unwrap(),
    };
    assert_case(
        "auth_authority_valid",
        pure::assertValidSignerAuthorizationAuthority(authority.clone()),
        rows,
    );
    let mut no_domain_authority = authority;
    no_domain_authority.domainCommitment = rt::FixedBytes::new([0; 32]);
    assert_case(
        "auth_authority_zero_domain",
        pure::assertValidSignerAuthorizationAuthority(no_domain_authority),
        rows,
    );
    assert_root_case(
        "auth_decision_root",
        pure::signerAuthorizationDecisionRoot(descriptor.clone(), b32(42)),
        rows,
    );
    assert_root_case(
        "auth_decision_zero_domain",
        pure::signerAuthorizationDecisionRoot(descriptor.clone(), rt::FixedBytes::new([0; 32])),
        rows,
    );
    let mut verifier = descriptor.clone();
    verifier.role = types::SignerRole::verifier;
    verifier.verificationRelationship = types::VerificationRelationship::authentication;
    verifier.scopeCommitment = b32(60);
    let verifier_proof = signed_proof(17, vectors["verifierScalar"].as_str().unwrap());
    assert_case(
        "auth_verifier_valid",
        pure::assertAuthorizedVerifierProof(b32(60), verifier_proof.clone(), verifier.clone()),
        rows,
    );
    assert_case(
        "auth_verifier_wrong_scope",
        pure::assertAuthorizedVerifierProof(b32(61), verifier_proof.clone(), verifier),
        rows,
    );
    assert_case(
        "auth_verifier_wrong_role",
        pure::assertAuthorizedVerifierProof(b32(60), verifier_proof, descriptor.clone()),
        rows,
    );
    let auth_proof = types::Proof {
        signerVerificationMethodRef: method(),
        createdAt: rt::BoundedUint::<18446744073709551615>::new(1).unwrap(),
        challengeHash: b32(55),
        publicKey: rt::ec_mul_generator(rt::Field::from(5_u64)).unwrap(),
        signature: types::Signature {
            r: rt::ec_mul_generator(rt::Field::from(13_u64)).unwrap(),
            s: scalar_from_hex(vectors["authorizationScalar"].as_str().unwrap()),
        },
    };
    let authority = types::SignerAuthorizationAuthority {
        domainCommitment: b32(42),
        verificationMethodRef: method(),
        publicKey: rt::ec_mul_generator(rt::Field::from(5_u64)).unwrap(),
    };
    assert_eq!(
        field_hex(auth_proof.signature.s),
        vectors["authorizationScalar"]
    );
    assert_case(
        "auth_proof_valid",
        pure::assertValidSignerAuthorizationProof(
            descriptor.clone(),
            auth_proof.clone(),
            authority.clone(),
        ),
        rows,
    );
    let mut wrong_sequence = auth_proof.clone();
    wrong_sequence.createdAt = rt::BoundedUint::<18446744073709551615>::new(2).unwrap();
    assert_case(
        "auth_proof_wrong_sequence",
        pure::assertValidSignerAuthorizationProof(
            descriptor.clone(),
            wrong_sequence,
            authority.clone(),
        ),
        rows,
    );
    let mut wrong_authority = authority;
    wrong_authority.verificationMethodRef.methodId = b32(62);
    assert_case(
        "auth_proof_wrong_authority",
        pure::assertValidSignerAuthorizationProof(descriptor.clone(), auth_proof, wrong_authority),
        rows,
    );
    let mut next = descriptor.clone();
    next.decisionSequence = rt::BoundedUint::<18446744073709551615>::new(2).unwrap();
    assert_case(
        "auth_update_valid",
        pure::assertValidSignerAuthorizationUpdate(descriptor.clone(), next.clone()),
        rows,
    );
    assert_case(
        "auth_update_same_sequence",
        pure::assertValidSignerAuthorizationUpdate(descriptor.clone(), descriptor.clone()),
        rows,
    );
    let mut revoked = descriptor;
    revoked.state = types::AuthorizationState::revoked;
    assert_case(
        "auth_update_revoked_reactivated",
        pure::assertValidSignerAuthorizationUpdate(revoked, next),
        rows,
    );
}

#[test]
fn valid_signatures_and_tamper_paths_match_pinned_typescript() {
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-signed-flow-capture.json")).unwrap();
    let rows = capture["rows"].as_array().unwrap();
    let vectors = &capture["vectors"];
    assert_eq!(rows.len(), 9);
    let credential = credential();
    let presentation = presentation(&credential);
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

    let credential_proof = signed_proof(7, vectors["issuanceScalar"].as_str().unwrap());
    let presentation_proof = signed_proof(11, vectors["presentationScalar"].as_str().unwrap());
    assert_point(credential_proof.publicKey, &vectors["publicKey"]);
    assert_point(credential_proof.signature.r, &vectors["issuanceNoncePoint"]);
    assert_point(
        presentation_proof.signature.r,
        &vectors["presentationNoncePoint"],
    );
    assert_eq!(
        field_hex(credential_proof.signature.s),
        vectors["issuanceScalar"]
    );
    assert_eq!(
        field_hex(presentation_proof.signature.s),
        vectors["presentationScalar"]
    );

    assert_case(
        "issuance_context_signed",
        pure::assertValidIssuanceContextProof(credential_root, credential_proof.clone()),
        rows,
    );
    assert_case(
        "credential_signed",
        pure::assertValidDigitalPassportCredential(credential.clone(), credential_proof.clone()),
        rows,
    );
    let mut changed_credential = credential.clone();
    changed_credential.issuedAt = rt::BoundedUint::<18446744073709551615>::new(101).unwrap();
    assert_case(
        "credential_mutated_after_signing",
        pure::assertValidDigitalPassportCredential(changed_credential, credential_proof.clone()),
        rows,
    );
    assert_case(
        "presentation_context_signed",
        pure::assertValidPresentationContextProof(presentation_root, presentation_proof.clone()),
        rows,
    );
    assert_case(
        "presentation_signed_and_disclosed",
        pure::assertValidDigitalPassportPresentation(
            credential.clone(),
            credential_proof.clone(),
            presentation.clone(),
            presentation_proof.clone(),
        ),
        rows,
    );
    let mut changed_presentation = presentation.clone();
    changed_presentation.disclosed.firstNameValuePadded = b64(30);
    assert_case(
        "presentation_disclosure_mutated",
        pure::assertValidDigitalPassportPresentation(
            credential.clone(),
            credential_proof.clone(),
            changed_presentation,
            presentation_proof.clone(),
        ),
        rows,
    );
    assert_case(
        "request_satisfied",
        pure::assertDigitalPassportPresentationSatisfiesRequest(
            credential.clone(),
            request(),
            presentation.clone(),
            presentation_proof.clone(),
        ),
        rows,
    );
    let mut changed_request = request();
    changed_request.verifierChallengeHash = b32(56);
    assert_case(
        "request_challenge_changed",
        pure::assertDigitalPassportPresentationSatisfiesRequest(
            credential.clone(),
            changed_request,
            presentation.clone(),
            presentation_proof.clone(),
        ),
        rows,
    );
    let mut stricter_request = request();
    stricter_request.requireLastNameDisclosure = true;
    assert_case(
        "request_adds_last_name",
        pure::assertDigitalPassportPresentationSatisfiesRequest(
            credential,
            stricter_request,
            presentation,
            presentation_proof,
        ),
        rows,
    );
}
