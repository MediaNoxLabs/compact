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
// Licensed under the Apache License, Version 2.0 (the "License");
// You may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Prove scalar pure Field arguments and conditional casts in ledger-8 calls.

use super::*;
use compact_rust_internal_pure_call_fixture::ledger_contract as internal;
use compact_rust_ternary_cond_oracle_fixture::ledger_contract as conditional;

struct Echo;

impl conditional::Witnesses<u64> for Echo {
    fn echoField(
        &self,
        context: WitnessContext<'_, u64, conditional::LedgerView<'_>>,
        value: Field,
    ) -> (u64, Field) {
        (*context.private_state + 1, value)
    }
}

pub(super) fn observed_state(deploy: &ContractDeploy<DefaultDB>) -> ObservedContractState {
    ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    )
}

pub(super) fn verifier(root: &Path, circuit: &str) -> Result<VerifierKey, Box<dyn Error>> {
    Ok(tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("keys/{circuit}.verifier")),
    )?))?)
}

pub(super) fn check_stored_field(
    state: &ContractState<DefaultDB>,
    expected_state: &StateValue<DefaultDB>,
    index: usize,
    expected: u64,
) -> Result<(), Box<dyn Error>> {
    if state.data.get_ref() != expected_state {
        return Err("proven scalar pure call changed the expected ledger state".into());
    }
    let StateValue::Array(fields) = state.data.get_ref() else {
        return Err("scalar pure call state is not an array".into());
    };
    let stored: Field = read_cell(fields.get(index).ok_or("Field Cell missing")?)?;
    if stored != Field::from(expected) {
        return Err("proven scalar pure call stored a wrong Field".into());
    }
    Ok(())
}

pub(super) fn run(internal_root: &Path, ternary_root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0096_5343_414c_4152);
    let initial = internal::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        internal_root,
        "save",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let value = Field::from(7_u64);
    let recorded = internal::recorded::save(initial.into_circuit_context(deploy.address()), value)?;
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(internal_root, "save", recorded, value)?;
    let observed = observed_state(&deploy);
    let typed = internal::recorded::Contract.save_call(&observed, (), value)?;
    let prepared = typed.prepare(verifier(internal_root, "save")?, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("scalar pure save observed call differs from direct recording".into());
    }
    check_transaction(internal_root, "save", deploy, prepared, &mut rng, |state| {
        check_stored_field(state, &expected_state, 0, 8)
    })?;

    for (circuit, witnessed) in [("streamCallPure", false), ("streamCallWitness", true)] {
        let initial = conditional::initial_state(
            ConstructorContext::new(7_u64),
            true,
            true,
            Field::from(111_u64),
        )?;
        let deploy = make_deploy(
            ternary_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let generated = conditional::Contract::from(Echo);
        let observed = observed_state(&deploy);
        if witnessed {
            let recorded = generated.recording().streamCallWitness(context)?;
            if recorded.execution.context.private_state != 8
                || recorded.execution.private_transcript_outputs.len() != 1
            {
                return Err("conditional witness call lost its private effects".into());
            }
            let expected_state = recorded.execution.context.query.state.get_ref().clone();
            let manual = check_generated_trace(ternary_root, circuit, recorded, ())?;
            let typed = generated
                .recording()
                .streamCallWitness_call(&observed, 7_u64)?;
            let prepared = typed.prepare(verifier(ternary_root, circuit)?, Fr::from(0_u64))?;
            if format!("{manual:?}") != format!("{prepared:?}") {
                return Err("conditional witness observed call differs from recording".into());
            }
            check_transaction(ternary_root, circuit, deploy, prepared, &mut rng, |state| {
                check_stored_field(state, &expected_state, 1, 2)
            })?;
        } else {
            let recorded = conditional::recorded::streamCallPure(context)?;
            if recorded.execution.context.private_state != 7
                || !recorded.execution.private_transcript_outputs.is_empty()
            {
                return Err("conditional pure call changed private state".into());
            }
            let expected_state = recorded.execution.context.query.state.get_ref().clone();
            let manual = check_generated_trace(ternary_root, circuit, recorded, ())?;
            let typed = conditional::recorded::Contract.streamCallPure_call(&observed, 7_u64)?;
            let prepared = typed.prepare(verifier(ternary_root, circuit)?, Fr::from(0_u64))?;
            if format!("{manual:?}") != format!("{prepared:?}") {
                return Err("conditional pure observed call differs from recording".into());
            }
            check_transaction(ternary_root, circuit, deploy, prepared, &mut rng, |state| {
                check_stored_field(state, &expected_state, 1, 2)
            })?;
        }
    }
    println!("scalar pure Field arguments and conditional witness proved on ledger-8");
    Ok(())
}
