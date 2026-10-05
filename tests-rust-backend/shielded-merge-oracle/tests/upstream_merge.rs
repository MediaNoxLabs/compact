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
//! Upstream allocation/identity evidence only; strict proofs are a separate gate.
use compact_rust_shielded_merge_oracle_fixture::{ledger_contract as c, types};
use midnight_base_crypto::time::Timestamp;
use midnight_compact_runtime as runtime;
use midnight_zswap::{Input, Offer, Output, Transient};
use rand::{SeedableRng, rngs::StdRng};
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, HashOutput};
use runtime::{BoundedUint, FixedBytes};

fn compact(coin: runtime::ledger::CoinInfo) -> types::ShieldedCoinInfo {
    types::ShieldedCoinInfo {
        nonce: FixedBytes::new(coin.nonce.0.0),
        color: FixedBytes::new(coin.type_.0.0),
        value: BoundedUint::new(coin.value).unwrap(),
    }
}
fn qualified(coin: runtime::ledger::QualifiedCoinInfo) -> types::QualifiedShieldedCoinInfo {
    let value = compact((&coin).into());
    types::QualifiedShieldedCoinInfo {
        nonce: value.nonce,
        color: value.color,
        value: value.value,
        mt_index: BoundedUint::new(coin.mt_index.into()).unwrap(),
    }
}
#[test]
fn upstream_merge_covers_historical_and_transient_union_without_index_remapping() {
    for immediate in [false, true] {
        let mut rng = StdRng::seed_from_u64(0x0207_1000 + u64::from(immediate));
        let address = ContractAddress(HashOutput([9; 32]));
        let keys = midnight_zswap::keys::SecretKeys::from_rng_seed(&mut rng);
        let a = runtime::ledger::coin_info_from_compact(
            FixedBytes::new([1; 32]),
            FixedBytes::new([2; 32]),
            17,
        );
        let b = runtime::ledger::coin_info_from_compact(
            FixedBytes::new([3; 32]),
            FixedBytes::new([2; 32]),
            25,
        );
        let output_a = Output::new_contract_owned(&mut rng, &a, None, address).unwrap();
        let a_com = output_a.coin_com;
        let output_b = if immediate {
            Output::new(
                &mut rng,
                &b,
                None,
                &keys.coin_public_key(),
                Some(keys.enc_public_key()),
            )
            .unwrap()
        } else {
            Output::new_contract_owned(&mut rng, &b, None, address).unwrap()
        };
        let b_com = output_b.coin_com;
        let seed: Offer<_, DefaultDB> =
            Offer::new(vec![], vec![output_a, output_b], vec![]).unwrap();
        let (ledger, indices) = midnight_zswap::ledger::State::<DefaultDB>::new()
            .try_apply(&seed, None)
            .unwrap();
        let ledger = ledger.post_block_update(Timestamp::from_secs(0));
        assert_eq!(ledger.first_free, 2);
        let a_qualified = a.qualify(*indices.get(&a_com).unwrap());
        let b_qualified = b.qualify(*indices.get(&b_com).unwrap());
        let input_a =
            Input::new_contract_owned(&mut rng, &a_qualified, None, address, &ledger.coin_coms)
                .unwrap();
        let a_nullifier = input_a.nullifier;
        let input_b = if immediate {
            let wallet = midnight_zswap::local::State::<DefaultDB>::new().apply(&keys, &seed);
            wallet.spend(&mut rng, &keys, &b_qualified, None).unwrap().1
        } else {
            Input::new_contract_owned(&mut rng, &b_qualified, None, address, &ledger.coin_coms)
                .unwrap()
        };
        let b_nullifier = input_b.nullifier;
        let mut context = c::initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(address);
        context.set_zswap_output_start(2).unwrap();
        let result = if immediate {
            c::receive_then_merge(context, qualified(a_qualified), compact(b)).unwrap()
        } else {
            c::merge_qualified(context, qualified(a_qualified), qualified(b_qualified)).unwrap()
        };
        let merged = runtime::ledger::coin_info_from_compact(
            result.result.nonce,
            result.result.color,
            result.result.value.value(),
        );
        assert_eq!(merged.value, 42);
        let output = Output::new_contract_owned(&mut rng, &merged, None, address).unwrap();
        let output_com = output.coin_com;
        let transients = if immediate {
            let received = Output::new_contract_owned(&mut rng, &b, None, address).unwrap();
            vec![
                Transient::new_from_contract_owned_output(&mut rng, &b.qualify(0), None, received)
                    .unwrap(),
            ]
        } else {
            vec![]
        };
        let offer = Offer::new(
            vec![input_a.clone(), input_b],
            vec![output],
            transients.clone(),
        )
        .unwrap();
        let (updated, actual) = ledger.try_apply(&offer, None).unwrap();
        assert_eq!(actual.get(&output_com), Some(&2));
        assert!(updated.nullifiers.contains_key(&a_nullifier));
        assert!(updated.nullifiers.contains_key(&b_nullifier));
        assert_eq!(updated.first_free, if immediate { 4 } else { 3 });
        if immediate {
            assert_eq!(actual.get(&transients[0].coin_com), Some(&3));
            assert!(updated.nullifiers.contains_key(&transients[0].nullifier));
            assert_eq!(
                result.context.circuit_zswap().outputs()[0].provisional_index,
                2
            );
            assert_eq!(
                result.context.circuit_zswap().outputs()[1].provisional_index,
                3
            );
            assert_eq!(result.context.circuit_zswap().inputs()[1].mt_index, 0);
        }
        assert!(matches!(
            updated.try_apply(&offer, None),
            Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(_))
        ));

        // Raw execution accepts the same coin twice. Upstream ordinary input
        // uniqueness is separate, and refuses the duplicate nullifier.
        let duplicate_result = c::merge_qualified(
            c::initial_state(ConstructorContext::new(()))
                .unwrap()
                .into_circuit_context(address),
            qualified(a_qualified),
            qualified(a_qualified),
        )
        .unwrap();
        let duplicate_coin = runtime::ledger::coin_info_from_compact(
            duplicate_result.result.nonce,
            duplicate_result.result.color,
            duplicate_result.result.value.value(),
        );
        let duplicate_output =
            Output::new_contract_owned(&mut rng, &duplicate_coin, None, address).unwrap();
        let duplicate_offer = Offer::new(
            vec![input_a.clone(), input_a],
            vec![duplicate_output],
            vec![],
        )
        .unwrap();
        assert!(
            matches!(ledger.try_apply(&duplicate_offer, None), Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(n)) if n == a_nullifier)
        );
    }
}
