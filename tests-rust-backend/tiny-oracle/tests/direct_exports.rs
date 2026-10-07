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

use compact_rust_tiny_oracle_fixture::{ledger_slots, pure_circuits};
use midnight_compact_runtime::{
    FixedBytes,
    ledger::{ContractState, DefaultDB},
};

#[test]
fn public_key_matches_authority_in_captured_tiny_constructor() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/tiny-oracle.json"
    ))
    .unwrap();
    let state: ContractState<DefaultDB> = midnight_serialize::tagged_deserialize(
        &mut hex::decode(oracle["afterInit"]["stateHex"].as_str().unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    let expected = ledger_slots::authority
        .inspect(state.data.get_ref())
        .unwrap();
    // The retained TS constructor and existing Tiny witness use [7; 32].
    // Compare the directly exported helper with that independent state value.
    assert_eq!(
        pure_circuits::public_key(FixedBytes::new([7; 32])).unwrap(),
        expected
    );
    // This changed-secret control has no separately captured TS result.
    assert_ne!(
        pure_circuits::public_key(FixedBytes::new([8; 32])).unwrap(),
        expected
    );
}
