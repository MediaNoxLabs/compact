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

use compact_rust_zerocash_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, spend, zerocash_mint,
};
use compact_rust_zerocash_oracle_fixture::types::{
    MerkleTreeDigest, MerkleTreePath, MerkleTreePathEntry, Nonce, coin_info, commitment, opening,
    public_key, zk_public_key, zk_secret_key,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

const FIXED_PK: [u8; 32] = [
    0x72, 0xcd, 0x6e, 0x84, 0x22, 0xc4, 0x07, 0xfb, 0x6d, 0x09, 0x86, 0x90, 0xf1, 0x13, 0x0b, 0x7d,
    0xed, 0x7e, 0xc2, 0xf7, 0xf5, 0xe1, 0xd3, 0x0b, 0xd9, 0xd5, 0x21, 0xf0, 0x15, 0x36, 0x37, 0x93,
];

fn fixed_coin() -> coin_info {
    coin_info {
        nonce: Nonce {
            bytes: runtime::FixedBytes::new([3; 32]),
        },
        opening: opening {
            bytes: runtime::FixedBytes::new([4; 32]),
        },
    }
}

struct FixedWitness {
    spend_path: Option<MerkleTreePath>,
}

impl Witnesses<()> for FixedWitness {
    fn private_zk_secret_key(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), zk_secret_key) {
        (
            (),
            zk_secret_key {
                bytes: runtime::FixedBytes::new([1; 32]),
            },
        )
    }

    fn private_remove_coin(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
        _coin: coin_info,
    ) -> ((), ()) {
        ((), ())
    }

    fn private_zk_public_key(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), zk_public_key) {
        (
            (),
            zk_public_key {
                bytes: runtime::FixedBytes::new(FIXED_PK),
            },
        )
    }

    fn private_add_coin(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
        _coin: coin_info,
    ) -> ((), ()) {
        ((), ())
    }

    fn context_path_of(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
        _commitment: commitment,
    ) -> ((), MerkleTreePath) {
        ((), self.spend_path.clone().unwrap_or_default())
    }

    fn context_new_coin_info(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), coin_info) {
        ((), fixed_coin())
    }

    fn context_encrypt(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
        _key: runtime::OpaqueBytes,
        _coin: coin_info,
    ) -> ((), runtime::OpaqueBytes) {
        ((), runtime::OpaqueBytes::from(vec![10, 11, 12]))
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["zerocash_mint", "spend"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn captured_path(value: &serde_json::Value) -> MerkleTreePath {
    let leaf_bytes: [u8; 32] = hex::decode(value["leafHex"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let entries = value["path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let mut sibling = hex::decode(entry["siblingHex"].as_str().unwrap()).unwrap();
            sibling.reverse();
            MerkleTreePathEntry {
                sibling: MerkleTreeDigest {
                    field: runtime::Field::from_le_bytes(&sibling).unwrap(),
                },
                goes_left: entry["goesLeft"].as_bool().unwrap(),
            }
        })
        .collect::<Vec<_>>();
    MerkleTreePath {
        leaf: commitment {
            bytes: runtime::FixedBytes::new(leaf_bytes),
        },
        path: runtime::FixedVector::new(entries.try_into().unwrap()),
    }
}

#[test]
fn zerocash_constructor_and_mint_match_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/zerocash-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]["stateHex"]
    );
    let witness = FixedWitness { spend_path: None };
    let mint = zerocash_mint(
        initial.into_circuit_context(ContractAddress::default()),
        &witness,
    )
    .unwrap();
    assert_eq!(
        state_hex(mint.context.query.state.get_ref().clone()),
        oracle["afterMint"]["stateHex"]
    );
}

#[test]
fn zerocash_spend_matches_typescript_state_with_captured_merkle_path() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/zerocash-oracle.json"
    ))
    .unwrap();
    let witness = FixedWitness {
        spend_path: Some(captured_path(&oracle["spendPath"])),
    };
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let mint = zerocash_mint(
        initial.into_circuit_context(ContractAddress::default()),
        &witness,
    )
    .unwrap();
    let destination = public_key {
        zk: zk_public_key {
            bytes: runtime::FixedBytes::new([5; 32]),
        },
        encryption: runtime::OpaqueBytes::from(vec![6; 32]),
    };
    let spent = spend(mint.context, &witness, destination, fixed_coin()).unwrap();
    assert_eq!(
        state_hex(spent.context.query.state.get_ref().clone()),
        oracle["afterSpend"]["stateHex"]
    );
}
