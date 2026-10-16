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

//! Boolean, enum, optional-value, and chunked Cell positional scenarios.
//!
//! Each scenario borrows the caller's shared RNG. The caller retains admission
//! and ordering, including separate deployments for write/read evidence.

use std::error::Error;
use std::fs;
use std::path::Path;

use compact_rust_cell_boolean_fixture::ledger_contract as cell_contract;
use compact_rust_cell_read_fixture::ledger_contract as cell_read_contract;
use compact_rust_chunked_cell_fixture::ledger_contract as chunked_cell_contract;
use compact_rust_recorded_enum_cell_fixture::ledger_contract as enum_cell_contract;
use compact_rust_recorded_enum_cell_fixture::types::Choice;
use compact_rust_tiny_oracle_fixture::ledger_contract as tiny_contract;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{StateValue, read_cell, read_cell_at_path};
use midnight_compact_runtime::transaction::{
    Observation, ObservedContractState, decode_verifier_key,
};
use midnight_transient_crypto::curve::Fr;
use rand::rngs::StdRng;

use super::{
    TinySecret, check_generated_trace, check_observed_call_parity, check_transaction, make_deploy,
};

pub(super) fn check_write_and_read(
    cell_root: &Path,
    cell_read_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let cell_initial = cell_contract::initial_state(ConstructorContext::new(()))?;
    let cell_deploy = make_deploy(
        cell_root,
        "set_flag",
        cell_initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let cell_context = cell_initial.into_circuit_context(cell_deploy.address());
    let cell_recorded = cell_contract::Contract::default()
        .recording
        .set_flag(cell_context)?;
    let cell_call = check_generated_trace(cell_root, "set_flag", cell_recorded, ())?;
    check_transaction(
        cell_root,
        "set_flag",
        cell_deploy,
        cell_call,
        rng,
        |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("cell contract state is not an array".into());
            };
            if !read_cell::<bool, _>(fields.get(0).ok_or("cell field missing")?)? {
                return Err("proven Cell call did not set the ledger flag".into());
            }
            Ok(())
        },
    )?;

    let cell_read_initial = cell_read_contract::initial_state(ConstructorContext::new(()))?;
    let cell_read_deploy = make_deploy(
        cell_read_root,
        "read_flag",
        cell_read_initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let cell_read_context = cell_read_initial.into_circuit_context(cell_read_deploy.address());
    let cell_read_recorded = cell_read_contract::Contract::default()
        .recording
        .read_flag(cell_read_context)?;
    if cell_read_recorded.execution.result {
        return Err("new Cell contract unexpectedly read true".into());
    }
    let cell_read_call =
        check_generated_trace(cell_read_root, "read_flag", cell_read_recorded, ())?;
    check_transaction(
        cell_read_root,
        "read_flag",
        cell_read_deploy,
        cell_read_call,
        rng,
        |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("Cell read contract state is not an array".into());
            };
            if read_cell::<bool, _>(fields.get(0).ok_or("cell read field missing")?)? {
                return Err("proven Cell read unexpectedly changed ledger state".into());
            }
            Ok(())
        },
    )?;
    Ok(())
}

pub(super) fn check_enum_value(
    enum_cell_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let initial = enum_cell_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        enum_cell_root,
        "choose",
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = enum_cell_contract::Contract::default()
        .recording
        .choose(context, Choice::no)?;
    let call = check_generated_trace(enum_cell_root, "choose", recorded, Choice::no)?;
    check_transaction(enum_cell_root, "choose", deploy, call, rng, |contract| {
        let StateValue::Array(fields) = contract.data.get_ref() else {
            return Err("enum Cell state is not an array".into());
        };
        if read_cell::<Choice, _>(fields.get(0).ok_or("enum Cell missing")?)? != Choice::no {
            return Err("proven enum Cell call did not write Choice::no".into());
        }
        Ok(())
    })?;
    Ok(())
}

pub(super) fn check_optional_value(
    tiny_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let witness = TinySecret;
    let initial =
        tiny_contract::initial_state(ConstructorContext::new(()), &witness, Field::from(42u64))?;
    let deploy = make_deploy(
        tiny_root,
        "clear",
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = tiny_contract::Contract::from(TinySecret)
        .recording()
        .clear(context)?;
    let call = check_generated_trace(tiny_root, "clear", recorded, ())?;
    check_transaction(tiny_root, "clear", deploy, call, rng, |contract| {
        let StateValue::Array(fields) = contract.data.get_ref() else {
            return Err("tiny state is not an array".into());
        };
        if read_cell::<compact_rust_tiny_oracle_fixture::types::STATE, _>(
            fields.get(2).ok_or("tiny state Cell missing")?,
        )? != compact_rust_tiny_oracle_fixture::types::STATE::unset
        {
            return Err("proven tiny clear did not unset the state".into());
        }
        Ok(())
    })?;

    let initial =
        tiny_contract::initial_state(ConstructorContext::new(()), &witness, Field::from(42u64))?;
    let cleared = tiny_contract::clear(
        initial.into_circuit_context(midnight_compact_runtime::ledger::ContractAddress::default()),
        &witness,
    )?;
    let deploy = make_deploy(
        tiny_root,
        "set",
        cleared.context.query.state.get_ref().clone(),
        rng,
    )?;
    let context = cleared
        .context
        .into_constructor_result()
        .into_circuit_context(deploy.address());
    let recorded = tiny_contract::Contract::from(TinySecret)
        .recording()
        .set(context, Field::from(99u64))?;
    let call = check_generated_trace(tiny_root, "set", recorded, Field::from(99u64))?;
    check_transaction(tiny_root, "set", deploy, call, rng, |contract| {
        let StateValue::Array(fields) = contract.data.get_ref() else {
            return Err("tiny state is not an array".into());
        };
        if read_cell::<Field, _>(fields.get(1).ok_or("tiny value Cell missing")?)?
            != Field::from(99u64)
        {
            return Err("proven tiny set did not write 99".into());
        }
        Ok(())
    })?;

    let initial =
        tiny_contract::initial_state(ConstructorContext::new(()), &witness, Field::from(42u64))?;
    let deploy = make_deploy(
        tiny_root,
        "get",
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = tiny_contract::Contract::default().recording.get(context)?;
    let call = check_generated_trace(tiny_root, "get", recorded, ())?;
    check_transaction(tiny_root, "get", deploy, call, rng, |contract| {
        let StateValue::Array(fields) = contract.data.get_ref() else {
            return Err("tiny state is not an array".into());
        };
        if read_cell::<Field, _>(fields.get(1).ok_or("tiny value Cell missing")?)?
            != Field::from(42u64)
        {
            return Err("proven tiny get changed the value".into());
        }
        Ok(())
    })?;

    let initial =
        tiny_contract::initial_state(ConstructorContext::new(()), &witness, Field::from(42u64))?;
    let cleared = tiny_contract::clear(
        initial.into_circuit_context(midnight_compact_runtime::ledger::ContractAddress::default()),
        &witness,
    )?;
    let deploy = make_deploy(
        tiny_root,
        "get",
        cleared.context.query.state.get_ref().clone(),
        rng,
    )?;
    let context = cleared
        .context
        .into_constructor_result()
        .into_circuit_context(deploy.address());
    let recorded = tiny_contract::Contract::default().recording.get(context)?;
    if recorded.execution.result.is_some {
        return Err("tiny get returned Some for an unset value".into());
    }
    let call = check_generated_trace(tiny_root, "get", recorded, ())?;
    check_transaction(tiny_root, "get", deploy, call, rng, |contract| {
        let StateValue::Array(fields) = contract.data.get_ref() else {
            return Err("tiny state is not an array".into());
        };
        if read_cell::<compact_rust_tiny_oracle_fixture::types::STATE, _>(
            fields.get(2).ok_or("tiny state Cell missing")?,
        )? != compact_rust_tiny_oracle_fixture::types::STATE::unset
        {
            return Err("proven tiny absent get changed the state".into());
        }
        Ok(())
    })?;
    Ok(())
}

pub(super) fn check_chunked_layout(
    chunked_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let contract = chunked_cell_contract::Contract::default();
    for circuit in [
        "set_active",
        "get_active",
        "assert_active",
        "set_amount",
        "get_amount",
        "add_amount",
        "active_equals",
        "plus_amount",
        "subtract_amount",
        "multiply_amount",
    ] {
        let initial = chunked_cell_contract::initial_state(ConstructorContext::new(()))?;
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
            "set_active" => {
                let recorded = contract.recording.set_active(context, false)?;
                let manual = check_generated_trace(chunked_root, circuit, recorded, false)?;
                let typed = contract
                    .recording
                    .set_active_call(&observed, (), false)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "get_active" => {
                let recorded = contract.recording.get_active(context)?;
                if !recorded.execution.result {
                    return Err("seeded chunked Cell Boolean differs".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .get_active_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "assert_active" => {
                let recorded = contract.recording.assert_active(context, true)?;
                let manual = check_generated_trace(chunked_root, circuit, recorded, true)?;
                let typed = contract
                    .recording
                    .assert_active_call(&observed, (), true)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "set_amount" => {
                let recorded = contract
                    .recording
                    .set_amount(context, Field::from(11_u64))?;
                let manual =
                    check_generated_trace(chunked_root, circuit, recorded, Field::from(11_u64))?;
                let typed = contract
                    .recording
                    .set_amount_call(&observed, (), Field::from(11_u64))?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "get_amount" => {
                let recorded = contract.recording.get_amount(context)?;
                if recorded.execution.result != Field::from(3_u64) {
                    return Err("seeded chunked Cell Field differs".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .get_amount_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "add_amount" => {
                let recorded = contract.recording.add_amount(context, Field::from(7_u64))?;
                let manual =
                    check_generated_trace(chunked_root, circuit, recorded, Field::from(7_u64))?;
                let typed = contract
                    .recording
                    .add_amount_call(&observed, (), Field::from(7_u64))?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "active_equals" => {
                let recorded = contract.recording.active_equals(context, true)?;
                if !recorded.execution.result {
                    return Err("seeded chunked Cell equality differs".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, true)?;
                let typed = contract
                    .recording
                    .active_equals_call(&observed, (), true)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "plus_amount" => {
                let recorded = contract
                    .recording
                    .plus_amount(context, Field::from(7_u64))?;
                if recorded.execution.result != Field::from(10_u64) {
                    return Err("seeded chunked Cell sum differs".into());
                }
                let manual =
                    check_generated_trace(chunked_root, circuit, recorded, Field::from(7_u64))?;
                let typed = contract
                    .recording
                    .plus_amount_call(&observed, (), Field::from(7_u64))?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "subtract_amount" => {
                let recorded = contract
                    .recording
                    .subtract_amount(context, Field::from(2_u64))?;
                let manual =
                    check_generated_trace(chunked_root, circuit, recorded, Field::from(2_u64))?;
                let typed = contract
                    .recording
                    .subtract_amount_call(&observed, (), Field::from(2_u64))?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "multiply_amount" => {
                let recorded = contract
                    .recording
                    .multiply_amount(context, Field::from(7_u64))?;
                let manual =
                    check_generated_trace(chunked_root, circuit, recorded, Field::from(7_u64))?;
                let typed = contract
                    .recording
                    .multiply_amount_call(&observed, (), Field::from(7_u64))?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            _ => unreachable!(),
        };
        check_observed_call_parity(chunked_root, circuit, &deploy, &manual, &typed)?;
        check_transaction(chunked_root, circuit, deploy, typed, rng, |state| {
            let actual_active = read_cell_at_path::<bool, _>(state.data.get_ref(), &[1, 14])?;
            let actual_amount = read_cell_at_path::<Field, _>(state.data.get_ref(), &[1, 13])?;
            let expected_active = circuit != "set_active";
            let expected_amount = match circuit {
                "set_amount" => Field::from(11_u64),
                "add_amount" => Field::from(10_u64),
                "subtract_amount" => Field::from(1_u64),
                "multiply_amount" => Field::from(21_u64),
                _ => Field::from(3_u64),
            };
            if actual_active != expected_active || actual_amount != expected_amount {
                return Err("proven chunked Cell state differs".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}
