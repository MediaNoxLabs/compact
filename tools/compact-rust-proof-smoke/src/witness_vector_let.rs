// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
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

//! Prove an ordered Field witness Let whose formal is a typed Field vector.

use super::*;
use compact_rust_call_arg_declared_type_fixture::ledger_contract as contract;

struct Sum;

impl contract::Witnesses<u64> for Sum {
    fn sumWitness(
        &self,
        context: WitnessContext<'_, u64, contract::LedgerView<'_>>,
        values: FixedVector<Field, 2>,
    ) -> (u64, Field) {
        assert_eq!(values.0, [Field::from(0_u64), Field::from(1_u64)]);
        (*context.private_state + 1, values.0[0] + values.0[1])
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0094_5749_544e_4553);
    let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
    let deploy = make_deploy(
        root,
        "witnessConst",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let generated = contract::Contract::from(Sum);
    let recorded = generated
        .recording()
        .witnessConst(initial.into_circuit_context(deploy.address()))?;
    if recorded.execution.context.private_state != 8
        || recorded.execution.private_transcript_outputs.len() != 1
    {
        return Err("recorded vector witness lost its private effects".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "witnessConst", recorded, ())?;

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
        root.join("keys/witnessConst.verifier"),
    )?))?;
    let typed = generated.recording().witnessConst_call(&observed, 7_u64)?;
    let prepared = typed.prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("observed vector witness differs from direct recorded call".into());
    }

    check_transaction(root, "witnessConst", deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven vector witness changed the expected ledger state".into());
        }
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("vector witness state is not an array".into());
        };
        let stored: Field = read_cell(fields.get(3).ok_or("Field Cell missing")?)?;
        if stored != Field::from(1_u64) {
            return Err("proven vector witness stored a wrong Field value".into());
        }
        Ok(())
    })?;
    println!("typed vector witness Let proved and applied through ledger-8");
    Ok(())
}
