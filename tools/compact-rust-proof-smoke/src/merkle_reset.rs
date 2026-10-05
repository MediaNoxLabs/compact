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

//! Prove a populated plain-tree reset and apply it through ledger-8.

use super::*;
use midnight_compact_runtime::ledger::ContractAddress;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let circuit = "reset_tree";
    let mut rng = StdRng::seed_from_u64(0x0147_5253_4554);
    let initial = merkle_contract::initial_state(ConstructorContext::new(()))?;
    let blank = initial.ledger_state.get_ref().clone();
    let populated = merkle_contract::append(
        initial.into_circuit_context(ContractAddress::default()),
        BoundedUint::<255>::new(7)?,
    )?;
    let deploy = make_deploy(
        root,
        circuit,
        populated.context.query.state.get_ref().clone(),
        &mut rng,
    )?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let native = merkle_contract::reset_tree(observed.circuit_context(()))?;
    let recorded = merkle_contract::recorded::reset_tree(observed.circuit_context(()))?;
    if recorded.execution.context.query.state.get_ref() != &blank
        || native.context.query.state.get_ref() != &blank
        || !recorded.execution.private_transcript_outputs.is_empty()
    {
        return Err("plain reset recording differs from native blank state".into());
    }
    let manual = check_generated_trace(root, circuit, recorded, ())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/reset_tree.verifier"),
    )?))?;
    let typed = merkle_contract::recorded::Contract
        .reset_tree_call(&observed, ())?
        .prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("plain reset observed call differs from recorded prototype".into());
    }
    check_transaction(root, circuit, deploy, typed, &mut rng, |contract| {
        if contract.data.get_ref() != &blank {
            return Err("plain reset proof did not restore the blank state".into());
        }
        let tree = merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
        if tree.first_free()?.value() != 0 {
            return Err("plain reset proof retained a populated first-free index".into());
        }
        Ok(())
    })?;
    println!("plain Merkle reset proved and applied through ledger-8");
    Ok(())
}
