// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Pinned upstream construction/allocation evidence, separate from contract
//! proof/default-strict application. Genesis setup is explicit and privileged.
use compact_rust_transient_receive_send_oracle_fixture::{ledger_contract, types};
use midnight_base_crypto::time::Timestamp;
use midnight_compact_runtime as runtime;
use midnight_zswap::{Offer, Output, Transient};
use rand::{SeedableRng, rngs::StdRng};
use runtime::context::ConstructorContext;
use runtime::ledger::{CoinPublicKey, ContractAddress, DefaultDB, HashOutput};
use runtime::{BoundedUint, FixedBytes};

fn bytes(first: u8) -> [u8; 32] {
    let mut value = [0; 32];
    value[0] = first;
    value
}
#[test]
fn upstream_transient_uses_singleton_input_but_allocates_after_normal_outputs() {
    let mut rng = StdRng::seed_from_u64(0x0204_0000);
    let keys = midnight_zswap::keys::SecretKeys::from_rng_seed(&mut rng);
    let address = ContractAddress(HashOutput(bytes(9)));
    let coin = types::ShieldedCoinInfo {
        nonce: FixedBytes::new(bytes(1)),
        color: FixedBytes::new(bytes(2)),
        value: BoundedUint::new(42).unwrap(),
    };
    let info = runtime::ledger::coin_info_from_compact(coin.nonce, coin.color, 42);
    let seeded = [
        runtime::ledger::coin_info_from_compact(FixedBytes::new(bytes(91)), coin.color, 42),
        runtime::ledger::coin_info_from_compact(FixedBytes::new(bytes(92)), coin.color, 1),
    ];
    let seed_outputs = seeded
        .iter()
        .map(|coin| {
            Output::new(
                &mut rng,
                coin,
                None,
                &keys.coin_public_key(),
                Some(keys.enc_public_key()),
            )
            .unwrap()
        })
        .collect();
    let seed: Offer<_, DefaultDB> = Offer::new(vec![], seed_outputs, vec![]).unwrap();
    let (ledger, _) = midnight_zswap::ledger::State::<DefaultDB>::new()
        .try_apply(&seed, None)
        .unwrap();
    let ledger = ledger.post_block_update(Timestamp::from_secs(0));
    assert_eq!(ledger.first_free, 2);
    let wallet = midnight_zswap::local::State::<DefaultDB>::new().apply(&keys, &seed);
    assert_eq!(wallet.merkle_tree.root(), ledger.coin_coms.root());
    let input_coin = *wallet
        .coins
        .iter()
        .find(|(_, coin)| coin.value == 42)
        .unwrap()
        .1;
    let (_, input) = wallet.spend(&mut rng, &keys, &input_coin, None).unwrap();
    let wallet_nullifier = input.nullifier;
    let received = Output::new_contract_owned(&mut rng, &info, None, address).unwrap();
    let transient = Transient::new_from_contract_owned_output(
        &mut rng,
        &info.qualify(0),
        None,
        received.clone(),
    )
    .unwrap();
    assert_eq!(transient.coin_com, received.coin_com);
    assert_eq!(transient.proof_output, received.proof);
    assert_eq!(transient.as_output(), received);
    assert_eq!(transient.as_input().proof, transient.proof_input);
    assert_ne!(
        Some(transient.as_input().merkle_tree_root),
        ledger.coin_coms.root()
    );
    // Upstream constructors build preimages; they do not prove validity. Even
    // a wrong singleton index constructs successfully, with the same public
    // root. The policy must check intent index0 and final proof verification
    // must reject malformed private proof inputs.
    let wrong_index =
        Transient::new_from_contract_owned_output(&mut rng, &info.qualify(1), None, received)
            .unwrap();
    assert_eq!(
        wrong_index.as_input().merkle_tree_root,
        transient.as_input().merkle_tree_root
    );
    assert_eq!(wrong_index.coin_com, transient.coin_com);
    assert_eq!(wrong_index.nullifier, transient.nullifier);
    assert_ne!(wrong_index.proof_input, transient.proof_input);
    let mut context = ledger_contract::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(address);
    context.set_zswap_output_start(ledger.first_free).unwrap();
    let recipient = types::Either {
        is_left: true,
        left: types::ZswapCoinPublicKey {
            bytes: FixedBytes::new(bytes(7)),
        },
        right: types::ContractAddress::default(),
    };
    let native = ledger_contract::receive_then_send(context, coin, recipient).unwrap();
    let sent = &native.result.sent;
    let sent_info =
        runtime::ledger::coin_info_from_compact(sent.nonce, sent.color, sent.value.value());
    let sent = Output::new(
        &mut rng,
        &sent_info,
        None,
        &CoinPublicKey(HashOutput(bytes(7))),
        None,
    )
    .unwrap();
    let sent_commitment = sent.coin_com;
    let offer = Offer::new(vec![input], vec![sent], vec![transient.clone()]).unwrap();
    let (applied, indices) = ledger.try_apply(&offer, None).unwrap();
    assert_eq!(applied.first_free, 4);
    assert_eq!(indices.get(&sent_commitment), Some(&2));
    assert_eq!(indices.get(&transient.coin_com), Some(&3));
    // The raw TS/native source cursor assigns the reverse physical indices.
    // Neither evidence set is rewritten to hide this policy distinction.
    assert_eq!(
        native.context.circuit_zswap().outputs()[0].provisional_index,
        2
    );
    assert_eq!(
        native.context.circuit_zswap().outputs()[1].provisional_index,
        3
    );
    assert_eq!(native.context.circuit_zswap().next_index(), 4);
    assert_eq!(applied.coin_coms.index(2).unwrap().0, sent_commitment.0);
    assert_eq!(applied.coin_coms.index(3).unwrap().0, transient.coin_com.0);
    assert!(applied.nullifiers.contains_key(&wallet_nullifier));
    assert!(applied.nullifiers.contains_key(&transient.nullifier));
    assert!(matches!(applied.try_apply(&offer, None),
        Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(n)) if n == wallet_nullifier));
}
