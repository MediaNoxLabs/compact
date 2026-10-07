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

use compact_rust_test_center_micro_dao_fixture::{ledger_slots, pure_circuits, types};
use midnight_compact_runtime::{
    Field, FixedBytes, FixedVector,
    ledger::{ContractState, DefaultDB},
};

fn captured_reveal_state() -> (ContractState<DefaultDB>, FixedBytes<32>) {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/micro-dao-reveal.json"
    ))
    .unwrap();
    let row = oracle
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "yes7")
        .unwrap();
    assert!(row.get("error").is_none());
    let state = midnight_serialize::tagged_deserialize(
        &mut hex::decode(row["before"].as_str().unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    let leaf = FixedBytes::new(serde_json::from_value(row["pathArgs"][0].clone()).unwrap());
    (state, leaf)
}

#[test]
fn public_key_matches_organizer_in_captured_micro_dao_state() {
    let (state, _) = captured_reveal_state();
    let expected = ledger_slots::organizer
        .inspect(state.data.get_ref())
        .unwrap();
    // capture-micro-dao-reveal.mjs constructs the DAO with organizer secret [4; 32].
    assert_eq!(
        pure_circuits::public_key(FixedBytes::new([4; 32])).unwrap(),
        expected
    );
    assert_ne!(
        pure_circuits::public_key(FixedBytes::new([5; 32])).unwrap(),
        expected
    );
}

#[test]
fn path_root_matches_captured_vote_tree_and_detects_changed_path_data() {
    let (state, leaf) = captured_reveal_state();
    let tree = ledger_slots::committed_votes
        .inspect(state.data.get_ref())
        .unwrap();
    let expected: types::MerkleTreeDigest = tree.root().unwrap().into();
    // The leaf argument and tree are from the same successful TS reveal capture.
    // Obtain its actual ledger path; never construct expected root via path_root.
    let path =
        types::MerkleTreePath::from_ledger_path(tree.find_path_for_leaf(leaf).unwrap()).unwrap();
    assert_eq!(pure_circuits::path_root(path.clone()).unwrap(), expected);

    let mut changed_sibling = path.clone();
    let mut entries = changed_sibling.path.into_array();
    entries[0].sibling.field = entries[0].sibling.field + Field::from(1_u64);
    changed_sibling.path = FixedVector::new(entries);
    assert_ne!(pure_circuits::path_root(changed_sibling).unwrap(), expected);

    let mut changed_leaf = path;
    assert_ne!(changed_leaf.leaf, FixedBytes::new([0; 32]));
    changed_leaf.leaf = FixedBytes::new([0; 32]);
    assert_ne!(pure_circuits::path_root(changed_leaf).unwrap(), expected);
}

#[test]
fn tdust_preserves_the_source_defined_dust_denomination() {
    // micro-dao.compact declares this constant. This is not an independent TS vector.
    assert_eq!(pure_circuits::tdust().unwrap().value(), 1_000_000);
}
