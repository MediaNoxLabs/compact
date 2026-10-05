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

//! Prove an explicit Field-to-Bytes32 cast of a recorded Counter read.

use super::*;
use compact_rust_field_to_bytes32_oracle_fixture::ledger_contract as contract;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const CIRCUIT: &str = "snapshot";
    let mut rng = StdRng::seed_from_u64(0x4144_5231_3435);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        CIRCUIT,
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let recorded = contract::recorded::snapshot(initial.into_circuit_context(deploy.address()))?;
    if recorded.execution.result.into_array()[..2] != [1, 1]
        || !recorded.execution.private_transcript_outputs.is_empty()
    {
        return Err("Field-to-Bytes32 recorded result or private transcript changed".into());
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
        root.join("keys/snapshot.verifier"),
    )?))?;
    let typed = contract::recorded::Contract.snapshot_call(&observed, ())?;
    let prepared = typed.prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("Field-to-Bytes32 observed call differs from direct recording".into());
    }

    check_transaction(root, CIRCUIT, deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("Field-to-Bytes32 proof changed Counter state".into());
        }
        if read_cell_at_path::<u64, _>(state.data.get_ref(), &[0])? != 257 {
            return Err("Field-to-Bytes32 proof observed the wrong Counter".into());
        }
        Ok(())
    })?;
    println!("Field-to-Bytes32 snapshot proved and applied through ledger-8");
    Ok(())
}
