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

use compact_rust_persistent_hash_fixture::pure_circuits::{
    commit_field, degrade_digest, degrade_hash, hash_field, hash_pair, upgrade_field,
};
use midnight_compact_runtime::{Field, FixedBytes, FixedVector};

#[test]
fn persistent_natives_match_generated_typescript_and_ledger_wasm() {
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/persistent-hash.json"
    ))
    .unwrap();
    let digest = hash_field(Field::from(42_u64)).unwrap();
    let opening = FixedBytes::new(std::array::from_fn(|i| (i + 1) as u8));
    let pair = FixedVector::new([Field::from(3_u64), Field::from(5_u64)]);
    let bytes_cases = [
        ("hashField", digest),
        (
            "commitField",
            commit_field(Field::from(42_u64), opening).unwrap(),
        ),
        ("hashPair", hash_pair(pair).unwrap()),
        ("upgrade", upgrade_field(Field::from(42_u64)).unwrap()),
    ];
    for (name, actual) in bytes_cases {
        assert_eq!(
            hex::encode(actual.0),
            expected[name].as_str().unwrap(),
            "{name}"
        );
    }
    let field_cases = [
        ("degrade", degrade_digest(digest).unwrap()),
        ("degradeHash", degrade_hash(Field::from(42_u64)).unwrap()),
    ];
    for (name, actual) in field_cases {
        assert_eq!(
            hex::encode(actual.as_le_bytes()),
            expected[name].as_str().unwrap(),
            "{name}"
        );
    }
}
