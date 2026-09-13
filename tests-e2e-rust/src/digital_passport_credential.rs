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
// TS reference shape for the vendored dogfood contract
// `examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact`.
//
// Unlike the state-byte fixtures, this contract is a pure helper library
// (no ledger, no constructor, no impure circuits), so
// `capture-digital-passport-credential.mjs` records the observable
// *outcome* of each exported pure circuit instead of a serialised
// `ContractState`: either `{ "ok": true }` (returned without asserting,
// with an optional hex `result`) or `{ "ok": false, "error": "<message>" }`
// (an `assert` fired). `tests/digital_passport_credential.rs` replays the
// same inputs and asserts the Rust outcome equals the reference at every
// step.
//
// Two families:
//   * `civilDateHelpers` — one entry per civil-date scenario, each naming
//     the code site it exercises and carrying the normalised inputs.
//   * `roundTrip` — one entry per issuance/presentation/verification step
//     (the derived body roots and the validator circuits).

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Deserialize, Debug)]
pub struct DigitalPassportCredentialTsReference {
    pub contract: String,
    pub target: String,
    pub producer: String,
    #[serde(rename = "civilDateHelpers")]
    pub civil_date_helpers: BTreeMap<String, CivilDateHelperScenario>,
    #[serde(rename = "roundTrip")]
    pub round_trip: BTreeMap<String, StepOutcome>,
}

/// One civil-date scenario: the site it drives, its replayable inputs, and
/// the TS outcome.
#[derive(Deserialize, Debug)]
pub struct CivilDateHelperScenario {
    pub site: String,
    pub inputs: CivilDateHelperInputs,
    pub outcome: StepOutcome,
}

/// The seven predicate arguments, normalised for replay. Every field is a
/// decimal string (BigInt) except the booleans and the two date objects.
#[derive(Deserialize, Debug)]
pub struct CivilDateHelperInputs {
    #[serde(rename = "proveAgeOverThreshold")]
    pub prove_age_over_threshold: bool,
    #[serde(rename = "ageThresholdYears")]
    pub age_threshold_years: String,
    #[serde(rename = "currentDay")]
    pub current_day: String,
    #[serde(rename = "dateOfBirthDays")]
    pub date_of_birth_days: String,
    #[serde(rename = "dateOfBirthOpening")]
    pub date_of_birth_opening: String,
    #[serde(rename = "dateOfBirthCommitment")]
    pub date_of_birth_commitment: String,
    #[serde(rename = "currentDate")]
    pub current_date: CivilDateFields,
    #[serde(rename = "dateOfBirthDate")]
    pub date_of_birth_date: CivilDateFields,
}

/// A captured `DigitalPassportCivilDate` decomposition, fields stringified.
#[derive(Deserialize, Debug)]
pub struct CivilDateFields {
    pub year: String,
    pub month: String,
    pub day: String,
    #[serde(rename = "yearAdjustedQuotient4")]
    pub year_adjusted_quotient4: String,
    #[serde(rename = "yearAdjustedQuotient100")]
    pub year_adjusted_quotient100: String,
    #[serde(rename = "yearAdjustedQuotient400")]
    pub year_adjusted_quotient400: String,
    #[serde(rename = "marchBasedMonthDayOffset")]
    pub march_based_month_day_offset: String,
}

/// A captured step outcome: `ok` plus, for failures, the exact message the
/// TS runtime throws; for the root steps, `result` is the returned 32-byte
/// value as hex (null for the void validator circuits).
#[derive(Deserialize, Debug)]
pub struct StepOutcome {
    pub ok: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
}

impl DigitalPassportCredentialTsReference {
    pub fn load(path: impl AsRef<Path>) -> Self {
        let raw = std::fs::read_to_string(path).expect("read fixture");
        serde_json::from_str(&raw).expect("parse fixture")
    }
}
