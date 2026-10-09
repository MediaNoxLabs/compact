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

//! Set scenarios in the positional proof suite, sharing its seeded RNG.
//!
//! The caller owns optional-root admission and execution order. These scenarios
//! borrow the existing RNG; they do not seed or schedule their own proof work.

use std::error::Error;
use std::path::Path;

use compact_rust_set_boolean_fixture::ledger_contract as set_contract;
use compact_rust_set_oracle_fixture::ledger_contract as set_oracle_contract;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{StateValue, read_cell};
use rand::rngs::StdRng;

use super::{check_generated_trace, check_transaction, make_deploy};

pub(super) fn check_operations(set_root: &Path, rng: &mut StdRng) -> Result<(), Box<dyn Error>> {
    for circuit in [
        "add",
        "contains",
        "remove",
        "seen_size",
        "seen_is_empty",
        "add_field",
        "contains_field",
        "reset_fields",
    ] {
        let initial = set_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            set_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let contract = set_contract::Contract::default();
        let call = match circuit {
            "add" => check_generated_trace(
                set_root,
                circuit,
                contract.recording.add(context, true)?,
                true,
            )?,
            "contains" => {
                let recorded = contract.recording.contains(context, true)?;
                if recorded.execution.result {
                    return Err("empty Set unexpectedly contains true".into());
                }
                check_generated_trace(set_root, circuit, recorded, true)?
            }
            "remove" => check_generated_trace(
                set_root,
                circuit,
                contract.recording.remove(context, true)?,
                true,
            )?,
            "seen_size" => {
                let recorded = contract.recording.seen_size(context)?;
                if recorded.execution.result.value() != 0 {
                    return Err("empty Set unexpectedly has nonzero size".into());
                }
                check_generated_trace(set_root, circuit, recorded, ())?
            }
            "seen_is_empty" => {
                let recorded = contract.recording.seen_is_empty(context)?;
                if !recorded.execution.result {
                    return Err("new Set unexpectedly nonempty".into());
                }
                check_generated_trace(set_root, circuit, recorded, ())?
            }
            "add_field" => {
                let key = Field::from(7_u64);
                check_generated_trace(
                    set_root,
                    circuit,
                    contract.recording.add_field(context, key)?,
                    key,
                )?
            }
            "contains_field" => {
                let key = Field::from(7_u64);
                let recorded = contract.recording.contains_field(context, key)?;
                if recorded.execution.result {
                    return Err("empty Field Set unexpectedly contains seven".into());
                }
                check_generated_trace(set_root, circuit, recorded, key)?
            }
            "reset_fields" => check_generated_trace(
                set_root,
                circuit,
                contract.recording.reset_fields(context)?,
                (),
            )?,
            _ => unreachable!(),
        };
        check_transaction(set_root, circuit, deploy, call, rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("Set contract state is not an array".into());
            };
            let StateValue::Map(set) = fields.get(0).ok_or("Set field missing")? else {
                return Err("Set field is not a map".into());
            };
            if set.size() != usize::from(circuit == "add") {
                return Err("proven Set call produced the wrong size".into());
            }
            let StateValue::Map(fields_set) = fields.get(1).ok_or("Field Set missing")? else {
                return Err("Field Set is not a map".into());
            };
            if fields_set.size() != usize::from(circuit == "add_field") {
                return Err("proven Field Set call produced the wrong size".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}

pub(super) fn check_membership_oracle(
    set_oracle_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let initial = set_oracle_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        set_oracle_root,
        "check",
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let key = Field::from(7_u64);
    let recorded = set_oracle_contract::Contract::default()
        .recording
        .check(context, key)?;
    let call = check_generated_trace(set_oracle_root, "check", recorded, key)?;
    check_transaction(set_oracle_root, "check", deploy, call, rng, |contract| {
        let StateValue::Array(fields) = contract.data.get_ref() else {
            return Err("Set oracle state is not an array".into());
        };
        if read_cell::<bool, _>(fields.get(0).ok_or("Set oracle flag missing")?)? {
            return Err("proven Set member query unexpectedly found an element".into());
        }
        Ok(())
    })?;
    Ok(())
}
