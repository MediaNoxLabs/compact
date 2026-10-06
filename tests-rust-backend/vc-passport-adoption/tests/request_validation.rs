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

fn padded(value: &str) -> rt::FixedBytes<32> {
    let mut bytes = [0; 32];
    bytes[..value.len()].copy_from_slice(value.as_bytes());
    rt::FixedBytes::new(bytes)
}
fn nonzero(seed: u8) -> rt::FixedBytes<32> {
    rt::FixedBytes::new(std::array::from_fn(|index| seed.wrapping_add(index as u8)))
}
fn uint16(value: u128) -> rt::BoundedUint<65535> {
    rt::BoundedUint::new(value).unwrap()
}
fn schema() -> types::SchemaRef {
    types::SchemaRef {
        packageId: padded("midnight:vc:digital-passport"),
        schemaId: padded("digital-passport:v1"),
        majorVersion: uint16(1),
        minorVersion: uint16(0),
    }
}
fn request() -> types::DigitalPassportPresentationRequest {
    types::DigitalPassportPresentationRequest {
        version: uint16(1),
        schema: schema(),
        issuerVerificationMethodRef: types::VerificationMethodRef {
            controllerAddress: types::ContractAddress { bytes: nonzero(1) },
            methodId: nonzero(2),
        },
        verifierChallengeHash: nonzero(3),
        ..Default::default()
    }
}

fn assert_case(name: &str, result: Result<(), rt::CompactError>, rows: &[serde_json::Value]) {
    let expected = rows.iter().find(|row| row["name"] == name).unwrap();
    match expected["outcome"].as_str().unwrap() {
        "ok" => assert!(result.is_ok(), "{name}: {result:?}"),
        "error" => assert_eq!(
            result.unwrap_err().to_string(),
            expected["message"].as_str().unwrap(),
            "{name}"
        ),
        _ => panic!("invalid capture row {name}"),
    }
}

#[test]
fn family_schema_and_presentation_request_match_dual_typescript_capture() {
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-request-capture.json")).unwrap();
    let rows = capture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 16);
    assert_case(
        "family_schema_valid",
        pure::assertValidDigitalPassportSchemaRef(schema()),
        rows,
    );
    assert_case(
        "family_schema_wrong_package",
        pure::assertValidDigitalPassportSchemaRef(types::SchemaRef {
            packageId: nonzero(4),
            ..schema()
        }),
        rows,
    );
    assert_case(
        "family_schema_wrong_schema",
        pure::assertValidDigitalPassportSchemaRef(types::SchemaRef {
            schemaId: nonzero(5),
            ..schema()
        }),
        rows,
    );
    assert_case(
        "family_schema_wrong_major",
        pure::assertValidDigitalPassportSchemaRef(types::SchemaRef {
            majorVersion: uint16(2),
            ..schema()
        }),
        rows,
    );
    assert_case(
        "family_schema_wrong_minor",
        pure::assertValidDigitalPassportSchemaRef(types::SchemaRef {
            minorVersion: uint16(1),
            ..schema()
        }),
        rows,
    );
    assert_case(
        "matching_schema_valid",
        pure::assertMatchingSchemaRefs(schema(), schema()),
        rows,
    );
    assert_case(
        "matching_schema_minor_mismatch",
        pure::assertMatchingSchemaRefs(
            schema(),
            types::SchemaRef {
                minorVersion: uint16(1),
                ..schema()
            },
        ),
        rows,
    );
    assert_case(
        "matching_schema_invalid_expected",
        pure::assertMatchingSchemaRefs(
            types::SchemaRef {
                packageId: rt::FixedBytes::new([0; 32]),
                ..schema()
            },
            schema(),
        ),
        rows,
    );
    let method = request().issuerVerificationMethodRef;
    assert_case(
        "verification_method_valid",
        pure::assertValidVerificationMethodRef(method.clone()),
        rows,
    );
    assert_case(
        "verification_method_missing_controller",
        pure::assertValidVerificationMethodRef(types::VerificationMethodRef {
            controllerAddress: types::ContractAddress {
                bytes: rt::FixedBytes::new([0; 32]),
            },
            ..method.clone()
        }),
        rows,
    );
    assert_case(
        "verification_method_missing_id",
        pure::assertValidVerificationMethodRef(types::VerificationMethodRef {
            methodId: rt::FixedBytes::new([0; 32]),
            ..method
        }),
        rows,
    );
    assert_case(
        "presentation_request_valid",
        pure::assertValidDigitalPassportPresentationRequest(request()),
        rows,
    );
    assert_case(
        "presentation_request_empty_challenge",
        pure::assertValidDigitalPassportPresentationRequest(
            types::DigitalPassportPresentationRequest {
                verifierChallengeHash: rt::FixedBytes::new([0; 32]),
                ..request()
            },
        ),
        rows,
    );
    assert_case(
        "presentation_request_zero_age_required",
        pure::assertValidDigitalPassportPresentationRequest(
            types::DigitalPassportPresentationRequest {
                requireAgeOverThreshold: true,
                ..request()
            },
        ),
        rows,
    );
    assert_case(
        "presentation_request_nonzero_age_disabled",
        pure::assertValidDigitalPassportPresentationRequest(
            types::DigitalPassportPresentationRequest {
                requestedAgeThresholdYears: rt::BoundedUint::<255>::new(18).unwrap(),
                ..request()
            },
        ),
        rows,
    );
    assert_case(
        "presentation_request_wrong_schema_minor",
        pure::assertValidDigitalPassportPresentationRequest(
            types::DigitalPassportPresentationRequest {
                schema: types::SchemaRef {
                    minorVersion: uint16(2),
                    ..schema()
                },
                ..request()
            },
        ),
        rows,
    );
}
