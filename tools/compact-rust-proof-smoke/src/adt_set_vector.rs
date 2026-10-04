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

//! Proves the source ADT `Set<Vector<2, Field>>` circuit and applies its recorded program.

use super::*;
use compact_rust_adt_set_vector_fixture::ledger_contract as contract;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0098_5345_545f_5643);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        "test",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let recorded = contract::recorded::test(initial.into_circuit_context(deploy.address()))?;
    if !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("vector Set recording unexpectedly produced private output".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "test", recorded, ())?;

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
        root.join("keys/test.verifier"),
    )?))?;
    let generated = contract::Contract::default();
    let typed = generated.recording.test_call(&observed, ())?;
    let prepared = typed.prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("observed vector Set call differs from direct recorded call".into());
    }

    check_transaction(root, "test", deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven vector Set call changed the expected public state".into());
        }
        let set = set_view_at_path::<FixedVector<Field, 2>, _>(state.data.get_ref(), &[0])?;
        if !set.is_empty() || set.size()?.value() != 0 {
            return Err("proven vector Set did not reset to empty".into());
        }
        Ok(())
    })?;
    println!("ADT vector Set proof verified and applied through ledger-8");
    Ok(())
}
