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

use compact_rust_jubjub_hash_fixture::pure_circuits::{pair_point, point, point_x, point_y};
use midnight_compact_runtime::{Field, FixedVector, JubjubPoint};

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
fn generated_curve_hash_and_coordinates_match_ledger_wasm() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/jubjub-hash.json"
    ))
    .unwrap();
    let field = Field::from(42_u64);
    assert_point(point(field).unwrap(), &oracle["point"]);
    assert_eq!(
        hex::encode(point_x(field).unwrap().as_le_bytes()),
        oracle["x"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(point_y(field).unwrap().as_le_bytes()),
        oracle["y"].as_str().unwrap()
    );
    assert_point(
        pair_point(FixedVector::new([Field::from(3_u64), Field::from(5_u64)])).unwrap(),
        &oracle["pairPoint"],
    );
}
