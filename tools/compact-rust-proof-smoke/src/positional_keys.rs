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

//! Vector and composite-key positional scenarios.
//!
//! The caller owns admission, execution order, and the shared seeded RNG.
//! Each scenario borrows that RNG and preserves its own evidence and failures.

use std::error::Error;
use std::fs;
use std::path::Path;

use compact_rust_observed_composite_keys_fixture::ledger_contract as composite_key_contract;
use compact_rust_observed_composite_keys_fixture::types::CompositeKey;
use compact_rust_vector_key_adt_fixture::ledger_contract as vector_key_contract;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::{StateValue, set_view_at_path};
use midnight_compact_runtime::transaction::{
    Observation, ObservedContractState, decode_verifier_key,
};
use midnight_compact_runtime::{Field, FixedVector};
use midnight_transient_crypto::curve::Fr;
use rand::rngs::StdRng;

use super::{check_generated_trace, check_observed_call_parity, check_transaction, make_deploy};

pub(super) fn check_vector(vector_root: &Path, rng: &mut StdRng) -> Result<(), Box<dyn Error>> {
    let circuit = "setInsert";
    let initial = vector_key_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        vector_root,
        circuit,
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = vector_key_contract::Contract::default()
        .recording
        .setInsert(context)?;
    let call = check_generated_trace(vector_root, circuit, recorded, ())?;
    check_transaction(vector_root, circuit, deploy, call, rng, |contract| {
        let StateValue::Array(fields) = contract.data.get_ref() else {
            return Err("vector-key contract state is not an array".into());
        };
        let StateValue::Map(set) = fields.get(0).ok_or("vector-key Set missing")? else {
            return Err("vector-key Set has the wrong ledger shape".into());
        };
        let key = FixedVector::new([Field::from(0u64), Field::from(1u64)]);
        let view = set_view_at_path::<FixedVector<Field, 2>, _>(contract.data.get_ref(), &[0])?;
        if set.size() != 1 || !view.member(key) {
            return Err("proven vector-key Set insert lost the declared key or size".into());
        }
        Ok(())
    })?;
    Ok(())
}

pub(super) fn check_composite(
    composite_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let contract = composite_key_contract::Contract::default();
    let vector = FixedVector::new([Field::from(3_u64), Field::from(5_u64)]);
    let pair = (Field::from(42_u64), true);
    let key = CompositeKey {
        vector: vector.clone(),
        pair,
    };
    for (circuit, path) in [
        ("insert_vector", 0_u8),
        ("insert_tuple", 1_u8),
        ("insert_struct", 2_u8),
        ("roundtrip_tuple", 1_u8),
        ("roundtrip_struct", 2_u8),
        ("record_twelve", 3_u8),
    ] {
        let initial = composite_key_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            composite_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier = decode_verifier_key(&fs::read(
            composite_root.join(format!("keys/{circuit}.verifier")),
        )?)?;
        let (manual, typed) = match circuit {
            "insert_vector" => {
                let recorded = contract.recording.insert_vector(context, vector.clone())?;
                let manual =
                    check_generated_trace(composite_root, circuit, recorded, vector.clone())?;
                let typed = contract
                    .recording
                    .insert_vector_call(&observed, (), vector.clone())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "insert_tuple" => {
                let recorded = contract.recording.insert_tuple(context, pair)?;
                let manual = check_generated_trace(composite_root, circuit, recorded, pair)?;
                let typed = contract
                    .recording
                    .insert_tuple_call(&observed, (), pair)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "insert_struct" => {
                let recorded = contract.recording.insert_struct(context, key.clone())?;
                let manual = check_generated_trace(composite_root, circuit, recorded, key.clone())?;
                let typed = contract
                    .recording
                    .insert_struct_call(&observed, (), key.clone())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "roundtrip_tuple" => {
                let recorded = contract.recording.roundtrip_tuple(context, pair)?;
                if recorded.execution.result {
                    return Err("tuple roundtrip unexpectedly found a removed key".into());
                }
                let manual = check_generated_trace(composite_root, circuit, recorded, pair)?;
                let typed = contract
                    .recording
                    .roundtrip_tuple_call(&observed, (), pair)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "roundtrip_struct" => {
                let recorded = contract.recording.roundtrip_struct(context, key.clone())?;
                if recorded.execution.result {
                    return Err("struct roundtrip unexpectedly found a removed key".into());
                }
                let manual = check_generated_trace(composite_root, circuit, recorded, key.clone())?;
                let typed = contract
                    .recording
                    .roundtrip_struct_call(&observed, (), key.clone())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "record_twelve" => {
                let recorded = contract.recording.record_twelve(
                    context,
                    1_u64.into(),
                    2_u64.into(),
                    3_u64.into(),
                    4_u64.into(),
                    5_u64.into(),
                    6_u64.into(),
                    7_u64.into(),
                    8_u64.into(),
                    9_u64.into(),
                    10_u64.into(),
                    11_u64.into(),
                    12_u64.into(),
                )?;
                let input = AlignedValue::concat(
                    &(1_u64..=12)
                        .map(|value| AlignedValue::from(Field::from(value)))
                        .collect::<Vec<_>>(),
                );
                let manual = check_generated_trace(composite_root, circuit, recorded, input)?;
                let typed = contract
                    .recording
                    .record_twelve_call(
                        &observed,
                        (),
                        1_u64.into(),
                        2_u64.into(),
                        3_u64.into(),
                        4_u64.into(),
                        5_u64.into(),
                        6_u64.into(),
                        7_u64.into(),
                        8_u64.into(),
                        9_u64.into(),
                        10_u64.into(),
                        11_u64.into(),
                        12_u64.into(),
                    )?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            _ => unreachable!(),
        };
        check_observed_call_parity(composite_root, circuit, &deploy, &manual, &typed)?;
        check_transaction(composite_root, circuit, deploy, typed, rng, |state| {
            let StateValue::Array(fields) = state.data.get_ref() else {
                return Err("composite-key contract state is not an array".into());
            };
            let StateValue::Map(set) = fields
                .get(path as usize)
                .ok_or("composite-key Set missing")?
            else {
                return Err("composite-key Set has the wrong ledger shape".into());
            };
            let present = match circuit {
                "insert_vector" => {
                    set_view_at_path::<FixedVector<Field, 2>, _>(state.data.get_ref(), &[path])?
                        .member(vector.clone())
                }
                "insert_tuple" | "roundtrip_tuple" => {
                    set_view_at_path::<(Field, bool), _>(state.data.get_ref(), &[path])?
                        .member(pair)
                }
                "insert_struct" | "roundtrip_struct" => {
                    set_view_at_path::<CompositeKey, _>(state.data.get_ref(), &[path])?
                        .member(key.clone())
                }
                "record_twelve" => set_view_at_path::<Field, _>(state.data.get_ref(), &[path])?
                    .member(Field::from(78_u64)),
                _ => unreachable!(),
            };
            let should_exist = !circuit.starts_with("roundtrip_");
            if set.size() != usize::from(should_exist) || present != should_exist {
                return Err("proven composite-key Set differs from expected state".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}
