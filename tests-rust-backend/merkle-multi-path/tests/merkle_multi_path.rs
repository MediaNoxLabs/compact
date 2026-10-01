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

use compact_rust_merkle_multi_path_fixture::pure_circuits::{root_hash, root_u8};
use compact_rust_merkle_multi_path_fixture::types::{MerkleTreePath, MerkleTreePathCompact1};
use midnight_compact_runtime as runtime;

#[test]
fn each_merkle_path_instantiation_converts_to_and_from_ledger_paths() {
    let entry = runtime::ledger::MerklePathEntry {
        sibling: runtime::ledger::MerkleTreeDigest(runtime::Field::default()),
        goes_left: false,
    };
    let u8_path = runtime::ledger::MerklePath {
        leaf: runtime::BoundedUint::<255>::new(7).unwrap(),
        path: vec![entry.clone(); 3],
    };
    let compact_u8 = MerkleTreePathCompact1::from_ledger_path(u8_path.clone()).unwrap();
    let roundtrip_u8 = compact_u8.clone().into_ledger_path();
    assert_eq!(roundtrip_u8.leaf, u8_path.leaf);
    assert_eq!(roundtrip_u8.path.len(), u8_path.path.len());
    assert_eq!(roundtrip_u8.root(), u8_path.root());
    assert_eq!(root_u8(compact_u8).unwrap().field, u8_path.root().0);

    let hash_path = runtime::ledger::MerklePath {
        leaf: runtime::FixedBytes::<32>::new([1; 32]),
        path: vec![entry; 3],
    };
    let compact_hash = MerkleTreePath::from_ledger_path(hash_path.clone()).unwrap();
    let roundtrip_hash = compact_hash.clone().into_ledger_path();
    assert_eq!(roundtrip_hash.leaf, hash_path.leaf);
    assert_eq!(roundtrip_hash.path.len(), hash_path.path.len());
    let mut expected = runtime::degrade_to_transient(compact_hash.leaf);
    for entry in compact_hash.path.0.iter() {
        expected = runtime::transient_hash(if entry.goes_left {
            (expected, entry.sibling.field)
        } else {
            (entry.sibling.field, expected)
        });
    }
    let root = root_hash(compact_hash).unwrap();
    assert_eq!(root.field, expected);
}
