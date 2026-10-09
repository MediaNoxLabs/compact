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

//! List scenarios in the positional proof suite, including constructor state.
//!
//! The caller owns optional-root admission and execution order. These scenarios
//! borrow the existing RNG; they do not seed or schedule their own proof work.

use std::error::Error;
use std::fs;
use std::path::Path;

use compact_rust_chunked_list_fixture::ledger_contract as chunked_list_contract;
use compact_rust_constructor_list_actions_fixture::ledger_contract as constructor_list_contract;
use compact_rust_list_field_fixture::ledger_contract as list_contract;
use compact_rust_witness_list_shapes_fixture::ledger_contract as list_shapes_contract;
use compact_rust_witness_list_shapes_fixture::types::{Choice as ListChoice, Packet};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{StateValue, list_view_at_path, read_cell};
use midnight_compact_runtime::transaction::{
    Observation, ObservedContractState, decode_verifier_key,
};
use midnight_compact_runtime::{BoundedUint, Field, FixedBytes};
use midnight_transient_crypto::curve::Fr;
use rand::rngs::StdRng;

use super::{check_generated_trace, check_observed_call_parity, check_transaction, make_deploy};

pub(super) fn check_operations(list_root: &Path, rng: &mut StdRng) -> Result<(), Box<dyn Error>> {
    for circuit in [
        "item_count",
        "items_empty",
        "first_item",
        "prepend",
        "clear_items",
    ] {
        let initial = list_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            list_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let contract = list_contract::Contract::default();
        let call = match circuit {
            "item_count" => {
                let recorded = contract.recording.item_count(context)?;
                if recorded.execution.result.value() != 0 {
                    return Err("empty List unexpectedly has nonzero length".into());
                }
                check_generated_trace(list_root, circuit, recorded, ())?
            }
            "items_empty" => {
                let recorded = contract.recording.items_empty(context)?;
                if !recorded.execution.result {
                    return Err("new List unexpectedly nonempty".into());
                }
                check_generated_trace(list_root, circuit, recorded, ())?
            }
            "first_item" => {
                let recorded = contract.recording.first_item(context)?;
                if recorded.execution.result.is_some
                    || recorded.execution.result.value != Field::from(0_u64)
                {
                    return Err("empty List head differs from Maybe default".into());
                }
                check_generated_trace(list_root, circuit, recorded, ())?
            }
            "prepend" => {
                let value = Field::from(7_u64);
                check_generated_trace(
                    list_root,
                    circuit,
                    contract.recording.prepend(context, value)?,
                    value,
                )?
            }
            "clear_items" => check_generated_trace(
                list_root,
                circuit,
                contract.recording.clear_items(context)?,
                (),
            )?,
            _ => unreachable!(),
        };
        check_transaction(list_root, circuit, deploy, call, rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("List contract state is not an array".into());
            };
            let StateValue::Array(items) = fields.get(0).ok_or("List field missing")? else {
                return Err("List field is not an array".into());
            };
            let length = read_cell::<u64, _>(items.get(2).ok_or("List length missing")?)?;
            if length != u64::from(circuit == "prepend") {
                return Err("proven List call produced the wrong length".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}

pub(super) fn check_constructor_actions(
    constructor_list_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    for circuit in [
        "item_count",
        "history_count",
        "first_item",
        "drop_first",
        "clear_items",
    ] {
        let initial = constructor_list_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            constructor_list_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let contract = constructor_list_contract::Contract::default();
        let call = match circuit {
            "item_count" => {
                let recorded = contract.recording.item_count(context)?;
                if recorded.execution.result.value() != 1 {
                    return Err("constructor List item count differs".into());
                }
                check_generated_trace(constructor_list_root, circuit, recorded, ())?
            }
            "history_count" => {
                let recorded = contract.recording.history_count(context)?;
                if recorded.execution.result.value() != 1 {
                    return Err("constructor List history count differs".into());
                }
                check_generated_trace(constructor_list_root, circuit, recorded, ())?
            }
            "first_item" => {
                let recorded = contract.recording.first_item(context)?;
                if !recorded.execution.result.is_some
                    || recorded.execution.result.value != Field::from(1_u64)
                {
                    return Err("constructor List head differs".into());
                }
                check_generated_trace(constructor_list_root, circuit, recorded, ())?
            }
            "drop_first" => check_generated_trace(
                constructor_list_root,
                circuit,
                contract.recording.drop_first(context)?,
                (),
            )?,
            "clear_items" => check_generated_trace(
                constructor_list_root,
                circuit,
                contract.recording.clear_items(context)?,
                (),
            )?,
            _ => unreachable!(),
        };
        check_transaction(
            constructor_list_root,
            circuit,
            deploy,
            call,
            rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("constructor List state is not an array".into());
                };
                for index in 0..2 {
                    let StateValue::Array(items) = fields.get(index).ok_or("List field missing")?
                    else {
                        return Err("constructor List field is not an array".into());
                    };
                    let length = read_cell::<u64, _>(items.get(2).ok_or("List length missing")?)?;
                    let expected =
                        u64::from(index != 0 || !matches!(circuit, "drop_first" | "clear_items"));
                    if length != expected {
                        return Err("proven constructor List call produced wrong length".into());
                    }
                }
                Ok(())
            },
        )?;
    }
    Ok(())
}

pub(super) fn check_witness_shapes(
    list_shapes_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let initial = list_shapes_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        list_shapes_root,
        "first_choice",
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = list_shapes_contract::Contract::default()
        .recording
        .first_choice(context)?;
    if recorded.execution.result.is_some || recorded.execution.result.value != ListChoice::yes {
        return Err("empty Choice List head did not return the Compact default".into());
    }
    let call = check_generated_trace(list_shapes_root, "first_choice", recorded, ())?;
    check_transaction(
        list_shapes_root,
        "first_choice",
        deploy,
        call,
        rng,
        |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("List shape contract state is not an array".into());
            };
            let StateValue::Array(items) = fields.get(3).ok_or("Choice List field missing")? else {
                return Err("Choice List field is not an array".into());
            };
            if read_cell::<u64, _>(items.get(2).ok_or("Choice List length missing")?)? != 0 {
                return Err("proven Choice head read changed the List".into());
            }
            Ok(())
        },
    )?;
    let initial = list_shapes_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        list_shapes_root,
        "first_packet",
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = list_shapes_contract::Contract::default()
        .recording
        .first_packet(context)?;
    if recorded.execution.result.is_some || recorded.execution.result.value != Packet::default() {
        return Err("empty packet List head did not return the Compact default".into());
    }
    let call = check_generated_trace(list_shapes_root, "first_packet", recorded, ())?;
    check_transaction(
        list_shapes_root,
        "first_packet",
        deploy,
        call,
        rng,
        |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("List shape contract state is not an array".into());
            };
            let StateValue::Array(items) = fields.get(4).ok_or("packet List field missing")? else {
                return Err("packet List field is not an array".into());
            };
            if read_cell::<u64, _>(items.get(2).ok_or("packet List length missing")?)? != 0 {
                return Err("proven packet head read changed the List".into());
            }
            Ok(())
        },
    )?;
    for (circuit, index) in [
        ("push_flag", 0),
        ("push_count", 1),
        ("push_tag", 2),
        ("push_choice", 3),
        ("push_packet", 4),
    ] {
        let initial = list_shapes_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            list_shapes_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let contract = list_shapes_contract::Contract::default();
        let call = match circuit {
            "push_flag" => check_generated_trace(
                list_shapes_root,
                circuit,
                contract.recording.push_flag(context, true)?,
                true,
            )?,
            "push_count" => {
                let value = BoundedUint::<65535>::new(42)?;
                check_generated_trace(
                    list_shapes_root,
                    circuit,
                    contract.recording.push_count(context, value)?,
                    value,
                )?
            }
            "push_tag" => {
                let value = FixedBytes::new([1, 2, 3]);
                check_generated_trace(
                    list_shapes_root,
                    circuit,
                    contract.recording.push_tag(context, value)?,
                    value,
                )?
            }
            "push_choice" => check_generated_trace(
                list_shapes_root,
                circuit,
                contract.recording.push_choice(context, ListChoice::no)?,
                ListChoice::no,
            )?,
            "push_packet" => {
                let value = Packet {
                    tag: FixedBytes::new([4, 5, 6]),
                    count: BoundedUint::<65535>::new(7)?,
                };
                check_generated_trace(
                    list_shapes_root,
                    circuit,
                    contract.recording.push_packet(context, value.clone())?,
                    value,
                )?
            }
            _ => unreachable!(),
        };
        check_transaction(list_shapes_root, circuit, deploy, call, rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("List shape contract state is not an array".into());
            };
            let StateValue::Array(items) = fields.get(index).ok_or("List field missing")? else {
                return Err("List field is not an array".into());
            };
            if read_cell::<u64, _>(items.get(2).ok_or("List length missing")?)? != 1 {
                return Err("proven typed List call produced wrong length".into());
            }
            let head = items.get(0).ok_or("List head missing")?;
            let correct = match circuit {
                "push_flag" => read_cell::<bool, _>(head)?,
                "push_count" => {
                    read_cell::<BoundedUint<65535>, _>(head)? == BoundedUint::<65535>::new(42)?
                }
                "push_tag" => read_cell::<FixedBytes<3>, _>(head)? == FixedBytes::new([1, 2, 3]),
                "push_choice" => read_cell::<ListChoice, _>(head)? == ListChoice::no,
                "push_packet" => {
                    read_cell::<Packet, _>(head)?
                        == Packet {
                            tag: FixedBytes::new([4, 5, 6]),
                            count: BoundedUint::<65535>::new(7)?,
                        }
                }
                _ => unreachable!(),
            };
            if !correct {
                return Err("proven typed List call wrote wrong head".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}

pub(super) fn check_chunked_layout(
    chunked_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let contract = chunked_list_contract::Contract::default();
    for circuit in [
        "item_count",
        "items_empty",
        "first_item",
        "prepend",
        "drop_first",
        "clear_items",
    ] {
        let initial = chunked_list_contract::initial_state(ConstructorContext::new(()))?;
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
            "item_count" => {
                let recorded = contract.recording.item_count(context)?;
                if recorded.execution.result.value() != 1 {
                    return Err("chunked List constructor did not insert its item".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .item_count_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "items_empty" => {
                let recorded = contract.recording.items_empty(context)?;
                if recorded.execution.result {
                    return Err("constructed chunked List is empty".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .items_empty_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "first_item" => {
                let recorded = contract.recording.first_item(context)?;
                if !recorded.execution.result.is_some
                    || recorded.execution.result.value != Field::from(1_u64)
                {
                    return Err("chunked List head differs from constructor".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .first_item_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "prepend" => {
                let value = Field::from(7_u64);
                let recorded = contract.recording.prepend(context, value)?;
                let manual = check_generated_trace(chunked_root, circuit, recorded, value)?;
                let typed = contract
                    .recording
                    .prepend_call(&observed, (), value)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "drop_first" => {
                let recorded = contract.recording.drop_first(context)?;
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .drop_first_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "clear_items" => {
                let recorded = contract.recording.clear_items(context)?;
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .clear_items_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            _ => unreachable!(),
        };
        check_observed_call_parity(chunked_root, circuit, &deploy, &manual, &typed)?;
        check_transaction(chunked_root, circuit, deploy, typed, rng, |state| {
            let view = list_view_at_path::<Field, _>(state.data.get_ref(), &[1, 14])?;
            let generated = chunked_list_contract::PublicStateView::from(state).items()?;
            if generated.head()? != view.head()?
                || generated.length()? != view.length()?
                || generated.is_empty() != view.is_empty()
            {
                return Err("generated proven chunked List view differs from raw view".into());
            }
            let expected_length = match circuit {
                "prepend" => 2,
                "drop_first" | "clear_items" => 0,
                _ => 1,
            };
            if view.length()?.value() != expected_length {
                return Err("proven chunked List length differs".into());
            }
            let expected_head = match circuit {
                "prepend" => Some(Field::from(7_u64)),
                "drop_first" | "clear_items" => None,
                _ => Some(Field::from(1_u64)),
            };
            if view.head()? != expected_head {
                return Err("proven chunked List head differs".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}
