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
use compact_rust_passport_dogfood_fixture::types::{
    Credential, DigitalPassportCivilDate, Presentation,
};
use midnight_compact_runtime::{BoundedUint, FixedBytes};

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
fn digital_passport_age_predicate_matches_all_typescript_scenarios() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/passport-dogfood-oracle.json"
    ))
    .unwrap();
    let scenarios = oracle["civilDateHelpers"].as_object().unwrap();
    assert_eq!(scenarios.len(), 25);
    for (name, scenario) in scenarios {
        let inputs = &scenario["inputs"];
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
        let result = pure_circuits::assertValidDigitalPassportAgePredicate(
            credential,
            presentation,
            uint32(&inputs["currentDay"]),
            uint32(&inputs["dateOfBirthDays"]),
            bytes32(&inputs["dateOfBirthOpening"]),
            civil_date(&inputs["currentDate"]),
            civil_date(&inputs["dateOfBirthDate"]),
        );
        let outcome = &scenario["outcome"];
        if outcome["ok"].as_bool().unwrap() {
            assert!(result.is_ok(), "{name}: {result:?}");
        } else {
            assert_eq!(
                result.unwrap_err().to_string(),
                outcome["error"].as_str().unwrap(),
                "{name}"
            );
        }
    }
}
