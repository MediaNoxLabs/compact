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

use compact_passport_named_record_guide::AgePredicateInputs;
use midnight_compact_runtime::{BoundedUint, FixedBytes};
use passport::types::{Credential, DigitalPassportCivilDate, Presentation};

fn uint32(value: &serde_json::Value) -> BoundedUint<4_294_967_295> {
    BoundedUint::new(value.as_str().unwrap().parse().unwrap()).unwrap()
}

fn bytes32(value: &serde_json::Value) -> FixedBytes<32> {
    FixedBytes::new(
        hex::decode(value.as_str().unwrap())
            .unwrap()
            .try_into()
            .unwrap(),
    )
}

fn civil_date(value: &serde_json::Value) -> DigitalPassportCivilDate {
    DigitalPassportCivilDate {
        year: uint32(&value["year"]),
        month: uint32(&value["month"]),
        day: uint32(&value["day"]),
        yearAdjustedQuotient4: uint32(&value["yearAdjustedQuotient4"]),
        yearAdjustedQuotient100: uint32(&value["yearAdjustedQuotient100"]),
        yearAdjustedQuotient400: uint32(&value["yearAdjustedQuotient400"]),
        marchBasedMonthDayOffset: uint32(&value["marchBasedMonthDayOffset"]),
    }
}

#[test]
fn application_named_inputs_match_twenty_five_captured_age_cases() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/age-inputs.json")).unwrap();
    let scenarios = oracle["scenarios"].as_object().unwrap();
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-age-capture.json")).unwrap();
    let expected = capture["rows"].as_array().unwrap();
    assert_eq!(scenarios.len(), 25);
    let branch_capture: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/branch-age-capture.json")).unwrap();
    assert_eq!(capture["rows"], branch_capture["rows"]);
    assert_eq!(expected.len(), 25);
    let mut passed = 0;
    let mut refused = 0;
    for (name, scenario) in scenarios {
        let inputs = scenario;
        let mut credential = Credential::default();
        credential.claimCommitments.dateOfBirthCommitment =
            bytes32(&inputs["dateOfBirthCommitment"]);
        let mut presentation = Presentation::default();
        presentation.disclosed.proveAgeOverThreshold =
            inputs["proveAgeOverThreshold"].as_bool().unwrap();
        presentation.disclosed.ageThresholdYears = BoundedUint::new(
            inputs["ageThresholdYears"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
        )
        .unwrap();
        let arguments = AgePredicateInputs {
            credential,
            presentation,
            current_day: uint32(&inputs["currentDay"]),
            date_of_birth_days: uint32(&inputs["dateOfBirthDays"]),
            date_of_birth_opening: bytes32(&inputs["dateOfBirthOpening"]),
            current_date: civil_date(&inputs["currentDate"]),
            date_of_birth_date: civil_date(&inputs["dateOfBirthDate"]),
        };
        let result = arguments.evaluate();
        let outcome = expected
            .iter()
            .find(|row| row["name"] == name.as_str())
            .unwrap();
        if outcome["outcome"] == "ok" {
            passed += 1;
            assert!(result.is_ok(), "{name}: {result:?}");
        } else {
            refused += 1;
            assert_eq!(
                result.unwrap_err().to_string(),
                outcome["message"].as_str().unwrap(),
                "{name}"
            );
        }
    }
    assert_eq!(passed, 6);
    assert_eq!(refused, 19);
}
