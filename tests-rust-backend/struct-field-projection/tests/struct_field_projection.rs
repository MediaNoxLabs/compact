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

use compact_rust_struct_field_projection_fixture::pure_circuits::{hash_projected, make, project};
use compact_rust_struct_field_projection_fixture::types::VecBox;
use midnight_compact_runtime::{Field, FixedVector};

#[test]
fn typed_struct_field_projection_matches_typescript_values_and_hash() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/struct-field-projection.json"
    ))
    .unwrap();
    let input = VecBox {
        values: FixedVector::new([Field::from(1_u64), Field::from(2_u64)]),
    };
    assert_eq!(make().unwrap(), input);
    assert_eq!(oracle["made"], serde_json::json!(["1", "2"]));
    assert_eq!(project(input.clone()).unwrap(), input.values);
    assert_eq!(oracle["projected"], serde_json::json!(["1", "2"]));
    assert_eq!(
        hex::encode(hash_projected(input).unwrap().into_array()),
        oracle["hashHex"]
    );
}
