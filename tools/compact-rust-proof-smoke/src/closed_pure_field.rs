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

//! Source-to-proof-to-ledger check for a closed pure Field call before a Cell write.

use super::*;
use compact_rust_call_arg_declared_type_fixture::ledger_contract as contract;
use compact_rust_call_arg_declared_type_fixture::pure_circuits;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const CIRCUIT: &str = "pureBodyFieldOnly";
    let mut rng = StdRng::seed_from_u64(0x0091_5055_5245_4644);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        CIRCUIT,
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = contract::recorded::pureBodyFieldOnly(context)?;
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
        root.join("keys/pureBodyFieldOnly.verifier"),
    )?))?;
    let typed = contract::recorded::Contract.pureBodyFieldOnly_call(&observed, ())?;
    let prepared = typed.prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("closed pure Field typed call differs from the recorded call".into());
    }

    check_transaction(root, CIRCUIT, deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven closed pure Field call changed the expected ledger state".into());
        }
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("closed pure Field state is not an array".into());
        };
        let stored: Field = read_cell(fields.get(3).ok_or("fieldCell missing")?)?;
        if stored != pure_circuits::fieldOnlyFromPureBody()? {
            return Err("proven closed pure Field call stored a different value".into());
        }
        Ok(())
    })?;
    println!("closed pure Field call proved and applied through ledger-8");
    Ok(())
}
