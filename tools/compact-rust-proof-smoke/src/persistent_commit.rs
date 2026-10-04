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

//! Source-to-proof-to-ledger check for a Field commitment to a persistent opening.

use super::*;
use compact_rust_call_arg_declared_type_fixture::ledger_contract as contract;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0090_434f_4d4d_4954);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        "commitSmall",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = contract::recorded::commitSmall(context)?;
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "commitSmall", recorded, ())?;

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
        root.join("keys/commitSmall.verifier"),
    )?))?;
    let typed = contract::recorded::Contract.commitSmall_call(&observed, ())?;
    let prepared = typed.prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("persistent commit typed call differs from the recorded call".into());
    }

    check_transaction(root, "commitSmall", deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven persistent commitment changed the expected ledger state".into());
        }
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("persistent commitment state is not an array".into());
        };
        let opening: FixedBytes<32> = read_cell(fields.get(0).ok_or("opening Cell missing")?)?;
        let committed: FixedBytes<32> = read_cell(fields.get(1).ok_or("commitment Cell missing")?)?;
        let expected = midnight_compact_runtime::persistent_commit(Field::from(5_u64), opening);
        if committed != expected {
            return Err("proven commitment did not bind the Field value and opening".into());
        }
        Ok(())
    })?;
    println!("Field persistent commitment proved and applied through ledger-8");
    Ok(())
}
