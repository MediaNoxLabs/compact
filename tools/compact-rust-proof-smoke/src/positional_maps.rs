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

//! Map scenarios in the positional proof suite, including constructor state.
//!
//! The caller owns optional-root admission and execution order. These scenarios
//! borrow the existing RNG; they do not seed or schedule their own proof work.

use std::error::Error;
use std::fs;
use std::path::Path;

use compact_rust_chunked_map_fixture::ledger_contract as chunked_map_contract;
use compact_rust_constructor_map_actions_fixture::ledger_contract as constructor_map_contract;
use compact_rust_map_boolean_field_fixture::ledger_contract as map_contract;
use compact_rust_nested_map_shape_fixture::ledger_contract as nested_map_shape_contract;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{StateValue, map_view_at_path};
use midnight_compact_runtime::transaction::{
    Observation, ObservedContractState, decode_verifier_key,
};
use midnight_onchain_vm::ops::Op;
use midnight_transient_crypto::curve::Fr;
use rand::rngs::StdRng;

use super::{check_generated_trace, check_observed_call_parity, check_transaction, make_deploy};

mod remaining;
pub(crate) use remaining::run as run_remaining;

pub(super) fn check_operations(map_root: &Path, rng: &mut StdRng) -> Result<(), Box<dyn Error>> {
    for circuit in [
        "put",
        "put_pair",
        "put_default",
        "has",
        "remove_key",
        "table_size",
        "table_is_empty",
        "reset_table",
    ] {
        let initial = map_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            map_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let contract = map_contract::Contract::default();
        let call = match circuit {
            "put" => {
                let recorded = contract.recording.put(context, true, Field::from(42_u64))?;
                // Independently captured in map-boolean-put-input.json.
                if !matches!(
                    recorded.public.verify_ops(),
                    [
                        Op::Idx { cached: false, push_path: true, path },
                        Op::Push { storage: false, .. },
                        Op::Push { storage: true, .. },
                        Op::Ins { cached: false, n: 1 },
                        Op::Ins { cached: true, n: 1 },
                    ] if path.len() == 1
                ) {
                    return Err("Map put VM operation shape differs from TypeScript oracle".into());
                }
                let manual = check_generated_trace(
                    map_root,
                    circuit,
                    recorded,
                    (true, Field::from(42_u64)),
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
                let verifier = decode_verifier_key(&fs::read(map_root.join("keys/put.verifier"))?)?;
                let typed = contract
                    .recording
                    .put_call(&observed, (), true, Field::from(42_u64))?
                    .prepare(verifier, Fr::from(0u64))?;
                check_observed_call_parity(map_root, "put", &deploy, &manual, &typed)?;
                typed
            }
            "put_pair" => {
                let recorded =
                    contract
                        .recording
                        .put_pair(context, true, false, Field::from(42_u64))?;
                // Two ordered Map insertions captured independently in
                // map-three-parameter-input.json.
                if !matches!(
                    recorded.public.verify_ops(),
                    [
                        Op::Idx { cached: false, push_path: true, path: first },
                        Op::Push { storage: false, .. },
                        Op::Push { storage: true, .. },
                        Op::Ins { cached: false, n: 1 },
                        Op::Ins { cached: true, n: 1 },
                        Op::Idx { cached: false, push_path: true, path: second },
                        Op::Push { storage: false, .. },
                        Op::Push { storage: true, .. },
                        Op::Ins { cached: false, n: 1 },
                        Op::Ins { cached: true, n: 1 },
                    ] if first.len() == 1 && second.len() == 1
                ) {
                    return Err(
                        "Map put_pair VM operation shape differs from TypeScript oracle".into(),
                    );
                }
                let manual = check_generated_trace(
                    map_root,
                    circuit,
                    recorded,
                    (true, false, Field::from(42_u64)),
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
                let verifier =
                    decode_verifier_key(&fs::read(map_root.join("keys/put_pair.verifier"))?)?;
                let typed = contract
                    .recording
                    .put_pair_call(&observed, (), true, false, Field::from(42_u64))?
                    .prepare(verifier, Fr::from(0_u64))?;
                check_observed_call_parity(map_root, "put_pair", &deploy, &manual, &typed)?;
                typed
            }
            "put_default" => check_generated_trace(
                map_root,
                circuit,
                contract.recording.put_default(context, true)?,
                true,
            )?,
            "has" => {
                let recorded = contract.recording.has(context, true)?;
                if recorded.execution.result {
                    return Err("empty Map unexpectedly contains true".into());
                }
                check_generated_trace(map_root, circuit, recorded, true)?
            }
            "remove_key" => check_generated_trace(
                map_root,
                circuit,
                contract.recording.remove_key(context, true)?,
                true,
            )?,
            "table_size" => {
                let recorded = contract.recording.table_size(context)?;
                if recorded.execution.result.value() != 0 {
                    return Err("empty Map unexpectedly has nonzero size".into());
                }
                check_generated_trace(map_root, circuit, recorded, ())?
            }
            "table_is_empty" => {
                let recorded = contract.recording.table_is_empty(context)?;
                if !recorded.execution.result {
                    return Err("new Map unexpectedly nonempty".into());
                }
                check_generated_trace(map_root, circuit, recorded, ())?
            }
            "reset_table" => check_generated_trace(
                map_root,
                circuit,
                contract.recording.reset_table(context)?,
                (),
            )?,
            _ => unreachable!(),
        };
        check_transaction(map_root, circuit, deploy, call, rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("Map contract state is not an array".into());
            };
            let StateValue::Map(table) = fields.get(0).ok_or("Map table missing")? else {
                return Err("Map table field is not a map".into());
            };
            let expected = match circuit {
                "put_pair" => 2,
                "put" | "put_default" => 1,
                _ => 0,
            };
            if table.size() != expected {
                return Err("proven Map call produced the wrong size".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}

pub(super) fn check_constructor_actions(
    constructor_map_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    for circuit in [
        "table_size",
        "history_size",
        "get_true",
        "get_false_history",
    ] {
        let initial = constructor_map_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            constructor_map_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let contract = constructor_map_contract::Contract::default();
        let call = match circuit {
            "table_size" => {
                let recorded = contract.recording.table_size(context)?;
                if recorded.execution.result.value() != 1 {
                    return Err("constructor Map table size differs".into());
                }
                check_generated_trace(constructor_map_root, circuit, recorded, ())?
            }
            "history_size" => {
                let recorded = contract.recording.history_size(context)?;
                if recorded.execution.result.value() != 1 {
                    return Err("constructor Map history size differs".into());
                }
                check_generated_trace(constructor_map_root, circuit, recorded, ())?
            }
            "get_true" => {
                let recorded = contract.recording.get_true(context)?;
                if recorded.execution.result != Field::from(1_u64) {
                    return Err("constructor Map lookup of true differs".into());
                }
                check_generated_trace(constructor_map_root, circuit, recorded, ())?
            }
            "get_false_history" => {
                let recorded = contract.recording.get_false_history(context)?;
                if recorded.execution.result != Field::from(0_u64) {
                    return Err("constructor Map default lookup differs".into());
                }
                check_generated_trace(constructor_map_root, circuit, recorded, ())?
            }
            _ => unreachable!(),
        };
        check_transaction(
            constructor_map_root,
            circuit,
            deploy,
            call,
            rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("constructor Map state is not an array".into());
                };
                for index in 0..2 {
                    let StateValue::Map(table) =
                        fields.get(index).ok_or("constructor Map field missing")?
                    else {
                        return Err("constructor Map field is not a map".into());
                    };
                    if table.size() != 1 {
                        return Err("constructor Map read changed state".into());
                    }
                }
                Ok(())
            },
        )?;
    }
    Ok(())
}

pub(super) fn check_nested_shape(
    shape_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let circuit = "check_nested_empty";
    let initial = nested_map_shape_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        shape_root,
        circuit,
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = nested_map_shape_contract::Contract::default()
        .recording
        .check_nested_empty(context)?;
    if !recorded.execution.result {
        return Err("new nested Map unexpectedly nonempty".into());
    }
    let call = check_generated_trace(shape_root, circuit, recorded, ())?;
    check_transaction(shape_root, circuit, deploy, call, rng, |contract| {
        let StateValue::Array(fields) = contract.data.get_ref() else {
            return Err("nested Map state is not an array".into());
        };
        let StateValue::Map(map) = fields.get(0).ok_or("nested Map field missing")? else {
            return Err("nested Map field has wrong ledger shape".into());
        };
        if map.size() != 0 {
            return Err("proven nested Map shape read changed the Map".into());
        }
        Ok(())
    })?;
    Ok(())
}

pub(super) fn check_chunked_layout(
    chunked_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let contract = chunked_map_contract::Contract::default();
    for circuit in [
        "put",
        "put_pair",
        "put_default",
        "has",
        "get",
        "remove_key",
        "table_size",
        "table_is_empty",
        "reset_table",
    ] {
        let initial = chunked_map_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            chunked_root,
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
            chunked_root.join(format!("keys/{circuit}.verifier")),
        )?)?;
        let (manual, typed) = match circuit {
            "put" => {
                let recorded = contract.recording.put(context, false, Field::from(7_u64))?;
                let manual = check_generated_trace(
                    chunked_root,
                    circuit,
                    recorded,
                    (false, Field::from(7_u64)),
                )?;
                let typed = contract
                    .recording
                    .put_call(&observed, (), false, Field::from(7_u64))?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "put_pair" => {
                let recorded =
                    contract
                        .recording
                        .put_pair(context, true, false, Field::from(42_u64))?;
                let manual = check_generated_trace(
                    chunked_root,
                    circuit,
                    recorded,
                    (true, false, Field::from(42_u64)),
                )?;
                let typed = contract
                    .recording
                    .put_pair_call(&observed, (), true, false, Field::from(42_u64))?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "put_default" => {
                let recorded = contract.recording.put_default(context, false)?;
                let manual = check_generated_trace(chunked_root, circuit, recorded, false)?;
                let typed = contract
                    .recording
                    .put_default_call(&observed, (), false)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "has" => {
                let recorded = contract.recording.has(context, true)?;
                if !recorded.execution.result {
                    return Err("seeded chunked Map key is absent".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, true)?;
                let typed = contract
                    .recording
                    .has_call(&observed, (), true)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "get" => {
                let recorded = contract.recording.get(context, true)?;
                if recorded.execution.result != Field::from(1_u64) {
                    return Err("seeded chunked Map value differs".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, true)?;
                let typed = contract
                    .recording
                    .get_call(&observed, (), true)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "remove_key" => {
                let recorded = contract.recording.remove_key(context, true)?;
                let manual = check_generated_trace(chunked_root, circuit, recorded, true)?;
                let typed = contract
                    .recording
                    .remove_key_call(&observed, (), true)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "table_size" => {
                let recorded = contract.recording.table_size(context)?;
                if recorded.execution.result.value() != 1 {
                    return Err("seeded chunked Map size differs".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .table_size_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "table_is_empty" => {
                let recorded = contract.recording.table_is_empty(context)?;
                if recorded.execution.result {
                    return Err("seeded chunked Map is empty".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .table_is_empty_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "reset_table" => {
                let recorded = contract.recording.reset_table(context)?;
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .reset_table_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            _ => unreachable!(),
        };
        check_observed_call_parity(chunked_root, circuit, &deploy, &manual, &typed)?;
        check_transaction(chunked_root, circuit, deploy, typed, rng, |state| {
            let view = map_view_at_path::<bool, Field, _>(state.data.get_ref(), &[1, 14])?;
            let generated = chunked_map_contract::PublicStateView::from(state).table()?;
            if generated.size()? != view.size()?
                || generated.member(true) != view.member(true)
                || generated.is_empty() != view.is_empty()
            {
                return Err("generated chunked Map view differs from raw applied state".into());
            }
            if view.member(true) && generated.lookup(true)? != view.lookup(true)? {
                return Err("generated chunked Map value differs from raw applied state".into());
            }
            let expected_size = match circuit {
                "put" | "put_pair" | "put_default" => 2,
                "remove_key" | "reset_table" => 0,
                _ => 1,
            };
            if view.size()?.value() != expected_size {
                return Err("proven chunked Map size differs".into());
            }
            let expected_true = match circuit {
                "put_pair" => Some(Field::from(42_u64)),
                "remove_key" | "reset_table" => None,
                _ => Some(Field::from(1_u64)),
            };
            if view.member(true) != expected_true.is_some() {
                return Err("proven chunked Map membership differs".into());
            }
            if let Some(value) = expected_true
                && view.lookup(true)? != value
            {
                return Err("proven chunked Map value differs".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}
