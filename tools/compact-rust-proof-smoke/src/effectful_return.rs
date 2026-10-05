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

//! Prove both selected branches of the original effectful return source.
use super::*;
use compact_rust_effectful_return_oracle_fixture::ledger_contract as contract;
use midnight_compact_runtime::ledger::ContractAddress;

struct Mark;
impl contract::Witnesses<u64> for Mark {
    fn mark(&self, context: WitnessContext<'_, u64, contract::LedgerView<'_>>) -> (u64, Field) {
        (*context.private_state + 1, Field::from(77_u64))
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (index, primed, next, expected, private) in
        [(0, false, 9_u64, 0_u64, 0_u64), (1, true, 8, 109, 1)]
    {
        let mut rng = StdRng::seed_from_u64(0x0182_0000 + index);
        let initial = contract::initial_state(ConstructorContext::new(0_u64))?;
        let state = if primed {
            contract::choose(
                initial.into_circuit_context(ContractAddress::default()),
                &Mark,
                Fr::from(9_u64),
            )?
            .context
            .query
            .state
            .get_ref()
            .clone()
        } else {
            initial.ledger_state.get_ref().clone()
        };
        let deploy = make_deploy(root, "choose", state, &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let native = contract::choose(observed.circuit_context(0_u64), &Mark, Fr::from(next))?;
        let recorded =
            contract::recorded::choose(observed.circuit_context(0_u64), &Mark, Fr::from(next))?;
        if recorded.execution.result != Fr::from(expected)
            || recorded.execution.result != native.result
            || recorded.execution.context.private_state != private
            || recorded.execution.private_transcript_outputs.len() != private as usize
            || recorded.execution.private_transcript_outputs != native.private_transcript_outputs
            || recorded.execution.gas_cost != native.gas_cost
            || recorded.execution.context.query.state != native.context.query.state
        {
            return Err(
                "effectful branch native/recorded result, state, gas, or witness mismatch".into(),
            );
        }
        let expected_state = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "choose", recorded, Fr::from(next))?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/choose.verifier"),
        )?))?;
        let contract = contract::Contract::from(Mark);
        let typed = contract
            .recording()
            .choose_call(&observed, 0_u64, Fr::from(next))?;
        let prepared = typed.prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("effectful observed call differs from direct recording".into());
        }
        check_transaction(root, "choose", deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected_state {
                return Err("effectful branch ledger application changed state".into());
            }
            Ok(())
        })?;
        println!(
            "effectful return branch {index} proved, verified, and ledger-applied under shared unbalanced smoke policy"
        );
    }
    Ok(())
}
