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

use compact_rust_test_center_coracle_fixture::{pure_circuits, types};
use midnight_compact_runtime::{Field, FixedBytes};

fn bytes32(value: &serde_json::Value) -> FixedBytes<32> {
    FixedBytes::new(serde_json::from_value(value.clone()).unwrap())
}

#[test]
fn color_predicates_match_captured_player_keys_and_reject_other_roles() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/coracle-start.json"
    ))
    .unwrap();
    let rows = oracle["rows"].as_array().unwrap();
    let red = bytes32(&rows.iter().find(|r| r["name"] == "red").unwrap()["ledgerAfter"]["red"]);
    let blue = bytes32(&rows.iter().find(|r| r["name"] == "blue").unwrap()["ledgerAfter"]["blue"]);
    // Both retained TS start cases use [4; 32], but distinct source hash domains.
    let secret = FixedBytes::new([4; 32]);
    assert_ne!(red, blue);
    assert!(pure_circuits::is_red(secret, red).unwrap());
    assert!(pure_circuits::is_blue(secret, blue).unwrap());
    assert!(!pure_circuits::is_red(secret, blue).unwrap());
    assert!(!pure_circuits::is_blue(secret, red).unwrap());
    assert!(!pure_circuits::is_red(FixedBytes::new([5; 32]), red).unwrap());
    assert!(!pure_circuits::is_blue(FixedBytes::new([5; 32]), blue).unwrap());
}

#[test]
fn alive_predicate_checks_position_without_interpreting_the_opening() {
    // Source-semantic controls for coracle.compact:is_player_alive, not TS
    // capture claims. The source compares positions; it does not validate guesses.
    for nonce in [0_u64, 9] {
        for (position, guess, expected) in [
            (1_u64, 1_u64, false),
            (1, 9, true),
            (4, 4, false),
            (4, 0, true),
            (9, 9, false),
            (9, 1, true),
        ] {
            let board = types::Committable {
                nonce: Field::from(nonce),
                contents: types::Board {
                    position: Field::from(position),
                },
            };
            assert_eq!(
                pure_circuits::is_player_alive(board, Field::from(guess)).unwrap(),
                expected,
                "position={position}, guess={guess}, nonce={nonce}"
            );
        }
    }
}
