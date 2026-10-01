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

use compact_rust_asset_registry_oracle_fixture::pure_circuits::registrationGap;
use compact_rust_asset_registry_oracle_fixture::types::AssetRecord;
use midnight_compact_runtime as runtime;

fn record(registered_at: u128) -> AssetRecord {
    let mut record = AssetRecord::default();
    record.provenance.registeredAt = runtime::BoundedUint::new(registered_at).unwrap();
    record
}

#[test]
fn nested_provenance_subtraction_keeps_distinct_compiler_temporaries() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/asset-registry-pure.json"
    ))
    .unwrap();
    let gap = registrationGap(record(120), record(100)).unwrap();
    assert_eq!(gap.value().to_string(), oracle["gap"]);
    let error = registrationGap(record(100), record(120))
        .err()
        .expect("reversed order should fail");
    assert!(matches!(error, runtime::CompactError::AssertionFailed(_)));
    assert_eq!(error.to_string(), oracle["reverseError"]);
}
