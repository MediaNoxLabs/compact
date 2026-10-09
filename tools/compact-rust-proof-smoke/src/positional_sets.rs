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
use std::fs;
use std::path::Path;

use compact_rust_chunked_set_observed_fixture::ledger_contract as chunked_set_contract;
use compact_rust_set_boolean_fixture::ledger_contract as set_contract;
use compact_rust_set_oracle_fixture::ledger_contract as set_oracle_contract;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{StateValue, read_cell, set_view_at_path};
use midnight_compact_runtime::transaction::{
    Observation, ObservedContractState, decode_verifier_key,
};
use midnight_transient_crypto::curve::Fr;
use rand::rngs::StdRng;

use super::{check_generated_trace, check_observed_call_parity, check_transaction, make_deploy};

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

pub(super) fn check_chunked_layout(
    chunked_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let contract = chunked_set_contract::Contract::default();
    let key = (Field::from(42_u64), true);
    for circuit in ["insert_key", "roundtrip_key", "key_count", "empty"] {
        let initial = chunked_set_contract::initial_state(ConstructorContext::new(()))?;
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
            "insert_key" => {
                let recorded = contract.recording.insert_key(context, key)?;
                let manual = check_generated_trace(chunked_root, circuit, recorded, key)?;
                let typed = contract
                    .recording
                    .insert_key_call(&observed, (), key)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "roundtrip_key" => {
                let recorded = contract.recording.roundtrip_key(context, key)?;
                if recorded.execution.result {
                    return Err("chunked Set roundtrip found a removed key".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, key)?;
                let typed = contract
                    .recording
                    .roundtrip_key_call(&observed, (), key)?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "key_count" => {
                let recorded = contract.recording.key_count(context)?;
                if recorded.execution.result.value() != 0 {
                    return Err("fresh chunked Set count is not zero".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .key_count_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            "empty" => {
                let recorded = contract.recording.empty(context)?;
                if !recorded.execution.result {
                    return Err("fresh chunked Set is not empty".into());
                }
                let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                let typed = contract
                    .recording
                    .empty_call(&observed, ())?
                    .prepare(verifier, Fr::from(0_u64))?;
                (manual, typed)
            }
            _ => unreachable!(),
        };
        check_observed_call_parity(chunked_root, circuit, &deploy, &manual, &typed)?;
        check_transaction(chunked_root, circuit, deploy, typed, rng, |state| {
            let view = set_view_at_path::<(Field, bool), _>(state.data.get_ref(), &[1, 14])?;
            let generated = chunked_set_contract::PublicStateView::from(state).keySet()?;
            let should_exist = circuit == "insert_key";
            if generated.size()? != view.size()? || generated.member(key) != view.member(key) {
                return Err("generated chunked Set view differs from raw applied state".into());
            }
            if view.size()?.value() != u128::from(should_exist) || view.member(key) != should_exist
            {
                return Err("proven chunked Set differs from expected state".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}
