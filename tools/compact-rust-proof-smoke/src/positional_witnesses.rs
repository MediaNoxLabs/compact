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

//! Witness transcript and nested-expression positional scenarios.
//!
//! The caller owns admission, execution order, and the shared seeded RNG.
//! Each scenario borrows that RNG and preserves its own evidence and failures.

use std::error::Error;
use std::fs;
use std::path::Path;

use compact_rust_nested_witness_call_oracle_fixture::ledger_contract as expression_contract;
use compact_rust_witness_cell_write_fixture::ledger_contract as witness_contract;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::{StateValue, read_cell};
use midnight_compact_runtime::transaction::{
    Observation, ObservedContractState, decode_verifier_key,
};
use midnight_onchain_vm::ops::Op;
use midnight_transient_crypto::curve::Fr;
use rand::rngs::StdRng;

use super::{
    NestedSecret, Secret, check_generated_trace, check_observed_call_parity, check_transaction,
    make_deploy,
};

pub(super) fn check_cell_witnesses(
    witness_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let witness_initial = witness_contract::initial_state(ConstructorContext::new(7_u64))?;
    let witness_deploy = make_deploy(
        witness_root,
        "write_twice",
        witness_initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = witness_initial.into_circuit_context(witness_deploy.address());
    let seed = Field::from(2_u64);
    let recorded = witness_contract::Contract::from(Secret)
        .recording()
        .write_twice(context, seed)?;
    if recorded.execution.private_transcript_outputs.len() != 2 {
        return Err("witnessed call did not record two private values".into());
    }
    let witness_call = check_generated_trace(witness_root, "write_twice", recorded, seed)?;
    check_transaction(
        witness_root,
        "write_twice",
        witness_deploy,
        witness_call,
        rng,
        |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("witnessed contract state is not an array".into());
            };
            if read_cell::<Field, _>(fields.get(0).ok_or("witness Cell missing")?)?
                != Field::from(10_u64)
            {
                return Err("proven witness call did not write the final Cell value".into());
            }
            Ok(())
        },
    )?;

    let offset_initial = witness_contract::initial_state(ConstructorContext::new(7_u64))?;
    let offset_deploy = make_deploy(
        witness_root,
        "write_offset",
        offset_initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let offset_context = offset_initial.into_circuit_context(offset_deploy.address());
    let offset = Field::from(5_u64);
    let offset_contract = witness_contract::Contract::from(Secret);
    let offset_recorded = offset_contract
        .recording()
        .write_offset(offset_context, seed, offset)?;
    if offset_recorded.execution.context.private_state != 8
        || offset_recorded.execution.private_transcript_outputs
            != [AlignedValue::from(Field::from(9_u64))]
    {
        return Err("witnessed offset private output differs from TypeScript oracle".into());
    }
    if !matches!(
        offset_recorded.public.verify_ops(),
        [
            Op::Push { storage: false, .. },
            Op::Push { storage: true, .. },
            Op::Ins {
                cached: false,
                n: 1
            },
        ]
    ) {
        return Err("witnessed offset VM shape differs from TypeScript oracle".into());
    }
    let manual_offset = check_generated_trace(
        witness_root,
        "write_offset",
        offset_recorded,
        (seed, offset),
    )?;
    let observed_offset = ObservedContractState::new(
        offset_deploy.address(),
        offset_deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let verifier =
        decode_verifier_key(&fs::read(witness_root.join("keys/write_offset.verifier"))?)?;
    let typed_offset = offset_contract
        .recording()
        .write_offset_call(&observed_offset, 7_u64, seed, offset)?
        .prepare(verifier, Fr::from(0_u64))?;
    check_observed_call_parity(
        witness_root,
        "write_offset",
        &offset_deploy,
        &manual_offset,
        &typed_offset,
    )?;
    check_transaction(
        witness_root,
        "write_offset",
        offset_deploy,
        typed_offset,
        rng,
        |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("witnessed offset state is not an array".into());
            };
            if read_cell::<Field, _>(fields.get(0).ok_or("witnessed offset Cell missing")?)?
                != Field::from(14_u64)
            {
                return Err("proven witnessed offset call wrote the wrong Cell value".into());
            }
            Ok(())
        },
    )?;

    let nested_initial = witness_contract::initial_state(ConstructorContext::new(7_u64))?;
    let nested_deploy = make_deploy(
        witness_root,
        "write_nested_twice",
        nested_initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = nested_initial.into_circuit_context(nested_deploy.address());
    let recorded = witness_contract::Contract::from(Secret)
        .recording()
        .write_nested_twice(context, seed)?;
    if recorded.execution.private_transcript_outputs.len() != 2 {
        return Err("nested witness call lost a private value".into());
    }
    let nested_call = check_generated_trace(witness_root, "write_nested_twice", recorded, seed)?;
    check_transaction(
        witness_root,
        "write_nested_twice",
        nested_deploy,
        nested_call,
        rng,
        |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("nested witnessed state is not an array".into());
            };
            if read_cell::<Field, _>(fields.get(0).ok_or("nested witness Cell missing")?)?
                != Field::from(10_u64)
            {
                return Err("proven nested witness call did not write both values".into());
            }
            Ok(())
        },
    )?;
    Ok(())
}

pub(super) fn check_nested_expressions(
    expression_root: &Path,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    for circuit in ["outer", "outerValue", "outerValue2", "outerValueExpr"] {
        let initial = expression_contract::initial_state(ConstructorContext::new(7_u64))?;
        let deploy = make_deploy(
            expression_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let contract = expression_contract::Contract::from(NestedSecret);
        let recorded = match circuit {
            "outer" => contract.recording().outer(context)?,
            "outerValue" => contract.recording().outerValue(context)?,
            "outerValue2" => contract.recording().outerValue2(context)?,
            "outerValueExpr" => contract.recording().outerValueExpr(context)?,
            _ => unreachable!(),
        };
        let expected_outputs = match circuit {
            "outerValue2" => 3,
            "outerValueExpr" => 2,
            _ => 1,
        };
        if recorded.execution.private_transcript_outputs.len() != expected_outputs {
            return Err(
                format!("{circuit} did not record {expected_outputs} private values").into(),
            );
        }
        let call = check_generated_trace(expression_root, circuit, recorded, ())?;
        check_transaction(expression_root, circuit, deploy, call, rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("nested expression state is not an array".into());
            };
            let expected = match circuit {
                "outerValue2" => 24_u64,
                "outerValueExpr" => 15_u64,
                _ => 7_u64,
            };
            if read_cell::<Field, _>(fields.get(0).ok_or("Field Cell missing")?)?
                != Field::from(expected)
            {
                return Err(
                    format!("proven nested expression did not write Field {expected}").into(),
                );
            }
            Ok(())
        })?;
    }
    Ok(())
}
