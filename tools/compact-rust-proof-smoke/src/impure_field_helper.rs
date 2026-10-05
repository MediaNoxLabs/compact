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

//! Prove a typed internal Field helper whose pure result precedes one Cell read.

use super::*;
use compact_rust_call_arg_declared_type_fixture::ledger_contract as contract;
use compact_rust_call_arg_declared_type_fixture::pure_circuits;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const CIRCUIT: &str = "impureConst";
    let mut rng = StdRng::seed_from_u64(0x494d_5055_5245_4341);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let StateValue::Array(initial_fields) = initial.ledger_state.get_ref() else {
        return Err("initial helper state is not an array".into());
    };
    let arm: Field = read_cell(initial_fields.get(4).ok_or("armCell missing")?)?;
    let expected_value =
        pure_circuits::sumVec(FixedVector::new([Field::from(0_u64), Field::from(1_u64)]))? + arm;
    let deploy = make_deploy(
        root,
        CIRCUIT,
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let recorded = contract::recorded::impureConst(initial.into_circuit_context(deploy.address()))?;
    if !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("Field helper unexpectedly emitted a private output".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, CIRCUIT, recorded, ())?;

    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/impureConst.verifier"),
    )?))?;
    let typed = contract::recorded::Contract.impureConst_call(&observed, ())?;
    let prepared = typed.prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("typed Field helper call differs from direct recording".into());
    }

    check_transaction(root, CIRCUIT, deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven Field helper changed the expected ledger state".into());
        }
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("Field helper state is not an array".into());
        };
        let stored: Field = read_cell(fields.get(3).ok_or("fieldCell missing")?)?;
        if stored != expected_value {
            return Err("proven Field helper stored the wrong pure-plus-read value".into());
        }
        Ok(())
    })?;
    println!("typed Field helper impureConst proved and applied through ledger-8");
    Ok(())
}
