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
    rt::FixedBytes::new(std::array::from_fn(|i| seed.wrapping_add(i as u8)))
}
fn b64(seed: u8) -> rt::FixedBytes<64> {
    rt::FixedBytes::new(std::array::from_fn(|i| seed.wrapping_add(i as u8)))
}
fn parts() -> types::DigitalPassportCredentialPrivateParts {
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
fn commitments(
    p: &types::DigitalPassportCredentialPrivateParts,
) -> types::DigitalPassportClaimCommitments {
    types::DigitalPassportClaimCommitments {
        firstNameCommitment: pure::firstNameCommitment(
            p.claimValues.firstNameValuePadded,
            p.openings.firstNameOpening,
        )
        .unwrap(),
        lastNameCommitment: pure::lastNameCommitment(
            p.claimValues.lastNameValuePadded,
            p.openings.lastNameOpening,
        )
        .unwrap(),
        dateOfBirthCommitment: pure::dateOfBirthCommitment(
            p.claimValues.dateOfBirthDays,
            p.openings.dateOfBirthOpening,
        )
        .unwrap(),
        documentNumberCommitment: pure::documentNumberCommitment(
            p.claimValues.documentNumberValue,
            p.openings.documentNumberOpening,
        )
        .unwrap(),
        issuingStateCommitment: pure::issuingStateCommitment(
            p.claimValues.issuingStateValue,
            p.openings.issuingStateOpening,
        )
        .unwrap(),
    }
}
fn check(
    com: types::DigitalPassportClaimCommitments,
    root: rt::FixedBytes<32>,
    p: types::DigitalPassportCredentialPrivateParts,
) -> Result<(), rt::CompactError> {
    pure::assertValidDigitalPassportCredentialPrivateParts(com, root, p)
}
fn assert_case(name: &str, result: Result<(), rt::CompactError>, rows: &[serde_json::Value]) {
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    match row["outcome"].as_str().unwrap() {
        "ok" => assert!(result.is_ok(), "{name}: {result:?}"),
        "error" => assert_eq!(
            result.unwrap_err().to_string(),
            row["message"].as_str().unwrap(),
            "{name}"
        ),
        _ => panic!("invalid capture row {name}"),
    }
}
#[test]
fn private_parts_validation_matches_dual_typescript_capture() {
    let capture: serde_json::Value = serde_json::from_str(include_str!(
        "../oracle/upstream-private-parts-capture.json"
    ))
    .unwrap();
    let rows = capture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 11);
    let p = parts();
    let com = commitments(&p);
    let root = pure::digitalPassportClaimRoot(com.clone()).unwrap();
    assert_case(
        "private_parts_valid",
        check(com.clone(), root, p.clone()),
        rows,
    );
    let mut t = p.clone();
    t.openings.firstNameOpening = b32(101);
    assert_case(
        "private_parts_wrong_first_opening",
        check(com.clone(), root, t),
        rows,
    );
    let mut t = p.clone();
    t.claimValues.lastNameValuePadded = b64(21);
    assert_case(
        "private_parts_wrong_last_value",
        check(com.clone(), root, t),
        rows,
    );
    let mut t = p.clone();
    t.openings.dateOfBirthOpening = b32(121);
    assert_case(
        "private_parts_wrong_dob_opening",
        check(com.clone(), root, t),
        rows,
    );
    let mut t = p.clone();
    t.claimValues.documentNumberValue = b32(31);
    assert_case(
        "private_parts_wrong_doc_value",
        check(com.clone(), root, t),
        rows,
    );
    let mut t = p.clone();
    t.openings.documentNumberOpening = b32(131);
    assert_case(
        "private_parts_wrong_doc_opening",
        check(com.clone(), root, t),
        rows,
    );
    let mut t = p.clone();
    t.claimValues.issuingStateValue = b32(41);
    assert_case(
        "private_parts_wrong_issuing_state",
        check(com.clone(), root, t),
        rows,
    );
    assert_case(
        "private_parts_wrong_root",
        check(com.clone(), b32(99), p.clone()),
        rows,
    );
    let mut absent = com;
    absent.documentNumberCommitment = pure::documentNumberNullCommitment().unwrap();
    let absent_root = pure::digitalPassportClaimRoot(absent.clone()).unwrap();
    let mut absent_parts = p;
    absent_parts.claimValues.documentNumberValue = rt::FixedBytes::new([0; 32]);
    absent_parts.openings.documentNumberOpening = rt::FixedBytes::new([0; 32]);
    assert_case(
        "private_parts_absent_valid",
        check(absent.clone(), absent_root, absent_parts.clone()),
        rows,
    );
    let mut t = absent_parts.clone();
    t.claimValues.documentNumberValue = b32(30);
    assert_case(
        "private_parts_absent_nonzero_value",
        check(absent.clone(), absent_root, t),
        rows,
    );
    let mut t = absent_parts;
    t.openings.documentNumberOpening = b32(130);
    assert_case(
        "private_parts_absent_nonzero_opening",
        check(absent, absent_root, t),
        rows,
    );
}
