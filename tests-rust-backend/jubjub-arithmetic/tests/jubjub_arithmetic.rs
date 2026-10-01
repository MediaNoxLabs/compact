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

use compact_rust_jubjub_arithmetic_fixture::pure_circuits::{
    add_points, generator_point, generator_reduced, multiply_point, negate_point, reduce_scalar,
};
use midnight_compact_runtime::{CompactError, Field, JubjubPoint};

fn field(hex_string: &str) -> Field {
    Field::from_le_bytes(&hex::decode(hex_string).unwrap()).unwrap()
}

fn assert_point(actual: JubjubPoint, oracle: &serde_json::Value) {
    assert_eq!(
        hex::encode(actual.x().unwrap().as_le_bytes()),
        oracle["x"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(actual.y().unwrap().as_le_bytes()),
        oracle["y"].as_str().unwrap()
    );
}

#[test]
fn group_arithmetic_matches_ledger_wasm_and_checks_scalar_range() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/jubjub-arithmetic.json"
    ))
    .unwrap();
    assert_point(
        add_points(Field::from(3_u64), Field::from(5_u64)).unwrap(),
        &oracle["add"],
    );
    assert_point(negate_point(Field::from(3_u64)).unwrap(), &oracle["negate"]);
    assert_point(
        multiply_point(Field::from(3_u64), Field::from(7_u64)).unwrap(),
        &oracle["multiply"],
    );
    assert_point(
        generator_point(Field::from(7_u64)).unwrap(),
        &oracle["generator"],
    );

    let high_scalar = field(oracle["highScalar"].as_str().unwrap());
    assert_eq!(
        hex::encode(reduce_scalar(high_scalar).unwrap().as_le_bytes()),
        oracle["reduced"].as_str().unwrap()
    );
    assert_point(
        generator_reduced(high_scalar).unwrap(),
        &oracle["generatorReduced"],
    );
    assert!(oracle["highScalarRejected"].as_bool().unwrap());
    assert!(matches!(
        generator_point(high_scalar),
        Err(CompactError::InvalidJubjubScalar)
    ));
    assert!(oracle["highMultiplyRejected"].as_bool().unwrap());
    assert!(matches!(
        multiply_point(Field::from(3_u64), high_scalar),
        Err(CompactError::InvalidJubjubScalar)
    ));
}
