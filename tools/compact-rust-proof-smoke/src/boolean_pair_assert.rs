// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Prove a typed pair hash assertion followed by an ordered Counter update.

use super::*;
use compact_rust_call_arg_declared_type_fixture::ledger_contract as contract;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const CIRCUIT: &str = "inlinedAssert";
    let mut rng = StdRng::seed_from_u64(0x0110_4153_5345_5254);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        CIRCUIT,
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = contract::recorded::inlinedAssert(context)?;
    if !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("pure Boolean assertion emitted a private output".into());
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
        root.join("keys/inlinedAssert.verifier"),
    )?))?;
    let typed = contract::recorded::Contract.inlinedAssert_call(&observed, ())?;
    let prepared = typed.prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("Boolean hash assertion typed call differs from recording".into());
    }

    check_transaction(root, CIRCUIT, deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven Boolean hash assertion changed the expected state".into());
        }
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("Boolean hash assertion state is not an array".into());
        };
        if read_counter(fields.get(7).ok_or("asserts Counter missing")?)? != 1 {
            return Err("proven Boolean hash assertion did not increment Counter".into());
        }
        Ok(())
    })?;
    println!("Boolean pair hash assertion proved and applied through ledger-8");
    Ok(())
}
