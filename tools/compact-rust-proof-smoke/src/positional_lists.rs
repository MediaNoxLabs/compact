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
use std::path::Path;

use compact_rust_constructor_list_actions_fixture::ledger_contract as constructor_list_contract;
use compact_rust_list_field_fixture::ledger_contract as list_contract;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{StateValue, read_cell};
use rand::rngs::StdRng;

use super::{check_generated_trace, check_transaction, make_deploy};

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
