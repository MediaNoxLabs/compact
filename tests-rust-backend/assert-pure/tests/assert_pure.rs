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

use compact_rust_assert_pure_fixture::pure_circuits::checked_value;
use midnight_compact_runtime::Field;

#[test]
fn pure_assertions_match_typescript_success_and_ordered_failures() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/assert-pure.json"
    ))
    .unwrap();
    assert_eq!(
        checked_value(true, true, Field::from(42_u64)).unwrap(),
        Field::from(
            oracle["pass"]["ok"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        )
    );
    for (name, first, second) in [("firstFails", false, false), ("secondFails", true, false)] {
        let error = checked_value(first, second, Field::from(42_u64)).unwrap_err();
        assert_eq!(error.to_string(), oracle[name]["error"], "{name}");
    }
}
