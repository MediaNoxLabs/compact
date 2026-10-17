// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
#[path = "../build_support/legacy_abi.rs"]
mod legacy_abi;

#[test]
fn whole_frozen_reference_projects_only_two_guards_and_reverses_exactly() {
    let original = include_str!("../legacy_reference.rs");
    let projected = legacy_abi::project(original).unwrap();
    assert_eq!(midnight_compact_runtime::RUST_RUNTIME_ABI, 53);
    assert_eq!(
        projected.replace("RUST_RUNTIME_ABI == 53", "RUST_RUNTIME_ABI == 51"),
        original
    );
    for changed in [
        original.to_owned() + "\n// changed body",
        original.replacen("RUST_RUNTIME_ABI == 51", "RUST_RUNTIME_ABI == 53", 1),
        original.replace("RUST_RUNTIME_ABI == 51", "RUST_RUNTIME_ABI == 50"),
    ] {
        assert!(legacy_abi::project(&changed).is_err());
    }
}
