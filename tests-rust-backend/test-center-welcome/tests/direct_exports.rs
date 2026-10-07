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

use compact_rust_test_center_welcome_fixture::{ledger_slots, pure_circuits};
use midnight_compact_runtime::{
    FixedBytes,
    ledger::{ContractState, DefaultDB},
};

#[test]
fn public_key_matches_the_single_organizer_in_captured_welcome_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/test-center-welcome.json"
    ))
    .unwrap();
    let row = &oracle["cases"][0];
    let state: ContractState<DefaultDB> = midnight_serialize::tagged_deserialize(
        &mut hex::decode(row["stateHex"].as_str().unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    let organizers = ledger_slots::organizer_pks
        .inspect(state.data.get_ref())
        .unwrap();
    assert_eq!(row["organizerSize"], "1");
    assert_eq!(organizers.size().unwrap().value(), 1);
    // capture-test-center-welcome.mjs supplies the zero secret at construction.
    assert!(organizers.member(pure_circuits::public_key(FixedBytes::new([0; 32])).unwrap()));
    // A distinct secret must not resolve to the single captured organizer.
    assert!(!organizers.member(pure_circuits::public_key(FixedBytes::new([1; 32])).unwrap()));
}
