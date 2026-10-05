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

//! Prove composite witness values and a typed commitment through historic Merkle insertion.
use super::*;
use compact_rust_zerocash_oracle_fixture::ledger_contract as contract;
use compact_rust_zerocash_oracle_fixture::types::{
    MerkleTreePath, Nonce, coin_info, commitment, opening, zk_public_key, zk_secret_key,
};
use contract::{LedgerView, Witnesses};
use midnight_compact_runtime as runtime;
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

#[derive(Default)]
struct FixedWitness {
    calls: std::cell::RefCell<Vec<&'static str>>,
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
        self.calls.borrow_mut().push("public_key");
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
        coin: coin_info,
    ) -> ((), ()) {
        assert_eq!(coin, fixed_coin());
        self.calls.borrow_mut().push("add_coin");
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
        self.calls.borrow_mut().push("new_coin");
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

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let mut rng = StdRng::seed_from_u64(0x159);
    let deploy = make_deploy(
        root,
        "zerocash_mint",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let native = contract::zerocash_mint(observed.circuit_context(()), &FixedWitness::default())?;
    let witness = FixedWitness::default();
    let recorded = contract::recorded::zerocash_mint(observed.circuit_context(()), &witness)?;
    if native.gas_cost != recorded.execution.gas_cost
        || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
        || recorded.execution.private_transcript_outputs.len() != 3
        || *witness.calls.borrow() != ["new_coin", "public_key", "add_coin"]
        || native.context.query.state.get_ref() != recorded.execution.context.query.state.get_ref()
        || native.context.query.effects != recorded.execution.context.query.effects
    {
        return Err("mint recording differs from native execution".into());
    }
    let expected_state = native.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "zerocash_mint", recorded, ())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/zerocash_mint.verifier"),
    )?))?;
    let generated = contract::Contract::from(FixedWitness::default());
    let prepared = generated
        .recording()
        .zerocash_mint_call(&observed, ())?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("typed mint call differs from manual recording".into());
    }
    check_transaction(
        root,
        "zerocash_mint",
        deploy,
        prepared,
        &mut rng,
        |applied| {
            if applied.data.get_ref() != &expected_state {
                return Err("applied mint state differs from native".into());
            }
            Ok(())
        },
    )?;
    println!(
        "Zerocash mint composite witnesses and historic Merkle insertion proved and ledger-applied"
    );
    Ok(())
}
