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

use compact_rust_vc_passport_adoption_fixture::{pure_circuits as pure, runtime as rt, types};

fn b32(seed: u8) -> rt::FixedBytes<32> {
    rt::FixedBytes::new(std::array::from_fn(|i| seed.wrapping_add(i as u8)))
}
fn b64(seed: u8) -> rt::FixedBytes<64> {
    rt::FixedBytes::new(std::array::from_fn(|i| seed.wrapping_add(i as u8)))
}
fn hex<const N: usize>(bytes: rt::FixedBytes<N>) -> String {
    bytes
        .into_array()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn schema() -> types::SchemaRef {
    types::SchemaRef {
        packageId: b32(1),
        schemaId: b32(2),
        majorVersion: rt::BoundedUint::<65535>::new(1).unwrap(),
        minorVersion: rt::BoundedUint::<65535>::new(0).unwrap(),
    }
}
fn envelope() -> types::ProtocolMessageEnvelope {
    types::ProtocolMessageEnvelope {
        version: rt::BoundedUint::<65535>::new(1).unwrap(),
        messageId: b32(1),
        threadId: b32(2),
        initialMessage: true,
        respondsToMessageId: pure::noProtocolResponseReference().unwrap(),
        createdAt: rt::BoundedUint::<18446744073709551615>::new(100).unwrap(),
        hasExpiresAt: true,
        expiresAt: rt::BoundedUint::<18446744073709551615>::new(200).unwrap(),
    }
}

#[test]
fn constructors_and_hashes_match_independent_ts_capture() {
    let no_ref = pure::noProtocolResponseReference().unwrap();
    assert_eq!(
        hex(no_ref),
        "6d69646e696768743a76633a70726f746f636f6c3a6e6f6e6500000000000000"
    );
    let null_document = pure::documentNumberNullCommitment().unwrap();
    assert_eq!(
        hex(null_document),
        "c1a62a311c7f9a87675762a656cc7fc023a01b1143b59c60124e3d8ee7ac2f29"
    );
    let opening_a = pure::firstNameCommitment(b64(0), b32(160)).unwrap();
    let opening_b = pure::firstNameCommitment(b64(0), b32(161)).unwrap();
    assert_eq!(
        hex(opening_a),
        "a11ec2726b6ccdfe8e1cc0e3066923a1cd71366cb276fc2d26ec82b27c7d9e14"
    );
    assert_eq!(
        hex(opening_b),
        "316e280303745fedcc3aba12f79658198dd6539ba4e3ca5383313c172605d684"
    );
    assert_ne!(opening_a, opening_b);
    let dob_zero =
        pure::dateOfBirthCommitment(rt::BoundedUint::<4294967295>::new(0).unwrap(), b32(32))
            .unwrap();
    let dob_max = pure::dateOfBirthCommitment(
        rt::BoundedUint::<4294967295>::new(4294967295).unwrap(),
        b32(32),
    )
    .unwrap();
    assert_eq!(
        hex(dob_zero),
        "efaef20e6b8940753e828d4bf7d093ce4017867f3f95ed88eaa25d73f8dcc811"
    );
    assert_eq!(
        hex(dob_max),
        "056bfd667478157190d0996298d40b5f16b064484a62ba13287d38f5783866c7"
    );
    let commitments = types::DigitalPassportClaimCommitments {
        firstNameCommitment: b32(1),
        lastNameCommitment: b32(2),
        dateOfBirthCommitment: b32(3),
        documentNumberCommitment: b32(4),
        issuingStateCommitment: b32(5),
    };
    let base_root = pure::digitalPassportClaimRoot(commitments.clone()).unwrap();
    let changed_root = pure::digitalPassportClaimRoot(types::DigitalPassportClaimCommitments {
        documentNumberCommitment: b32(99),
        ..commitments
    })
    .unwrap();
    assert_eq!(
        hex(base_root),
        "1719d1840150ddc72ed9bd11126c673158594527eb03c0e3935868201a64b28d"
    );
    assert_eq!(
        hex(changed_root),
        "32414247efdc42abbb2c15d54195fa5b74b5d75eb8cb60aff7e27380cea338c1"
    );
    assert_ne!(base_root, changed_root);
}

#[test]
fn schema_and_envelope_errors_match_independent_ts_capture() {
    assert!(pure::assertValidSchemaRef(schema()).is_ok());
    assert_eq!(
        pure::assertValidSchemaRef(types::SchemaRef {
            packageId: rt::FixedBytes::new([0; 32]),
            ..schema()
        })
        .unwrap_err()
        .to_string(),
        "failed assert: Schema package id must be set"
    );
    assert_eq!(
        pure::assertValidSchemaRef(types::SchemaRef {
            schemaId: rt::FixedBytes::new([0; 32]),
            ..schema()
        })
        .unwrap_err()
        .to_string(),
        "failed assert: Schema id must be set"
    );
    assert_eq!(
        pure::assertValidSchemaRef(types::SchemaRef {
            majorVersion: rt::BoundedUint::<65535>::new(0).unwrap(),
            ..schema()
        })
        .unwrap_err()
        .to_string(),
        "failed assert: Schema major version must be positive"
    );
    assert!(pure::assertValidProtocolMessageEnvelope(envelope()).is_ok());
    assert!(
        pure::assertValidProtocolMessageEnvelope(types::ProtocolMessageEnvelope {
            initialMessage: false,
            respondsToMessageId: b32(3),
            ..envelope()
        })
        .is_ok()
    );
    assert_eq!(
        pure::assertValidProtocolMessageEnvelope(types::ProtocolMessageEnvelope {
            version: rt::BoundedUint::<65535>::new(2).unwrap(),
            ..envelope()
        })
        .unwrap_err()
        .to_string(),
        "failed assert: Protocol message version mismatch"
    );
    assert_eq!(
        pure::assertValidProtocolMessageEnvelope(types::ProtocolMessageEnvelope {
            respondsToMessageId: b32(3),
            ..envelope()
        })
        .unwrap_err()
        .to_string(),
        "failed assert: Initial protocol message must not reference a previous message"
    );
    assert_eq!(
        pure::assertValidProtocolMessageEnvelope(types::ProtocolMessageEnvelope {
            expiresAt: rt::BoundedUint::<18446744073709551615>::new(99).unwrap(),
            ..envelope()
        })
        .unwrap_err()
        .to_string(),
        "failed assert: Protocol message expiration must not precede creation"
    );
}

#[test]
fn context_tags_and_remaining_claim_commitments_match_dual_ts_capture() {
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-ts-capture.json")).unwrap();
    let rows = capture["rows"].as_array().unwrap();
    let expected = |name: &str| {
        let row = rows.iter().find(|row| row["name"] == name).unwrap();
        assert_eq!(row["outcome"], "ok");
        row["value"].as_str().unwrap().to_owned()
    };
    assert_eq!(
        hex(pure::issuanceContextTag().unwrap()),
        expected("issuance_context_tag")
    );
    assert_eq!(
        hex(pure::presentationContextTag().unwrap()),
        expected("presentation_context_tag")
    );
    assert_eq!(
        hex(pure::signerAuthorizationContextTag().unwrap()),
        expected("signer_authorization_context_tag")
    );
    assert_eq!(
        hex(pure::verifierRequestContextTag().unwrap()),
        expected("verifier_request_context_tag")
    );
    assert_eq!(
        hex(pure::lastNameCommitment(b64(7), b32(71)).unwrap()),
        expected("last_name_commitment")
    );
    assert_eq!(
        hex(pure::documentNumberCommitment(b32(13), b32(72)).unwrap()),
        expected("document_number_commitment")
    );
    assert_eq!(
        hex(pure::issuingStateCommitment(b32(14), b32(73)).unwrap()),
        expected("issuing_state_commitment")
    );
}
