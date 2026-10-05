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

//! Prove typed indexed insertion and default-leaf insertion on ledger-8.

use super::*;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0114_4d45_524b_4c45);
    for circuit in ["place", "place_default"] {
        let initial = merkle_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            root,
            circuit,
            initial.ledger_state.get_ref().clone(),
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
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let index =
            BoundedUint::<{ u64::MAX as u128 }>::new(if circuit == "place" { 3 } else { 6 })?;
        let (manual, typed, expected_index, expected_leaf) = if circuit == "place" {
            let leaf = BoundedUint::<255>::new(9)?;
            let recorded = merkle_contract::recorded::place(
                initial.into_circuit_context(deploy.address()),
                leaf,
                index,
            )?;
            if !recorded.execution.private_transcript_outputs.is_empty() {
                return Err("indexed Merkle insertion emitted private outputs".into());
            }
            let manual = check_generated_trace(root, circuit, recorded, (leaf, index))?;
            let typed = merkle_contract::recorded::Contract
                .place_call(&observed, (), leaf, index)?
                .prepare(verifier, Fr::from(0u64))?;
            (manual, typed, 4, leaf)
        } else {
            let recorded = merkle_contract::recorded::place_default(
                initial.into_circuit_context(deploy.address()),
                index,
            )?;
            if !recorded.execution.private_transcript_outputs.is_empty() {
                return Err("default Merkle insertion emitted private outputs".into());
            }
            let manual = check_generated_trace(root, circuit, recorded, index)?;
            let typed = merkle_contract::recorded::Contract
                .place_default_call(&observed, (), index)?
                .prepare(verifier, Fr::from(0u64))?;
            (manual, typed, 7, BoundedUint::<255>::new(0)?)
        };
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(format!("{circuit} typed observed call differs from recording").into());
        }
        check_transaction(root, circuit, deploy, typed, &mut rng, |contract| {
            let tree = merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
            let generated = merkle_contract::PublicStateView::from(contract).t()?;
            if generated.root() != tree.root()
                || generated.first_free()? != tree.first_free()?
                || tree.first_free()?.value() != expected_index
                || tree.find_path_for_leaf(expected_leaf).is_none()
            {
                return Err(format!("proven {circuit} state differs from indexed tree").into());
            }
            Ok(())
        })?;
    }
    println!("typed indexed Merkle insertions proved and applied through ledger-8");
    Ok(())
}

pub(super) fn run_historic(insert_root: &Path, default_root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0114_4849_5354_4f52);
    let circuit = "place";
    let initial = historic_merkle_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        insert_root,
        circuit,
        initial.ledger_state.get_ref().clone(),
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
    let leaf = BoundedUint::<255>::new(9)?;
    let index = BoundedUint::<{ u64::MAX as u128 }>::new(3)?;
    let recorded = historic_merkle_contract::recorded::place(
        initial.into_circuit_context(deploy.address()),
        leaf,
        index,
    )?;
    if !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("historic indexed insertion emitted private outputs".into());
    }
    let manual = check_generated_trace(insert_root, circuit, recorded, (leaf, index))?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        insert_root.join("keys/place.verifier"),
    )?))?;
    let typed = historic_merkle_contract::recorded::Contract
        .place_call(&observed, (), leaf, index)?
        .prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("historic place typed observed call differs from recording".into());
    }
    check_transaction(insert_root, circuit, deploy, typed, &mut rng, |contract| {
        let tree = historic_merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
        let generated = historic_merkle_contract::PublicStateView::from(contract).t()?;
        if generated.root() != tree.root()
            || generated.first_free()? != tree.first_free()?
            || tree.first_free()?.value() != 4
            || tree.find_path_for_leaf(leaf).is_none()
            || tree.history()?.is_empty()
        {
            return Err("proven historic place state differs from indexed tree".into());
        }
        Ok(())
    })?;

    let circuit = "add_default";
    let initial = historic_merkle_default_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        default_root,
        circuit,
        initial.ledger_state.get_ref().clone(),
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
    let index = BoundedUint::<{ u64::MAX as u128 }>::new(2)?;
    let recorded = historic_merkle_default_contract::recorded::add_default(
        initial.into_circuit_context(deploy.address()),
        index,
    )?;
    if !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("historic default insertion emitted private outputs".into());
    }
    let manual = check_generated_trace(default_root, circuit, recorded, index)?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        default_root.join("keys/add_default.verifier"),
    )?))?;
    let typed = historic_merkle_default_contract::recorded::Contract
        .add_default_call(&observed, (), index)?
        .prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("historic add_default typed observed call differs from recording".into());
    }
    check_transaction(default_root, circuit, deploy, typed, &mut rng, |contract| {
        let tree = historic_merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
        let generated = historic_merkle_default_contract::PublicStateView::from(contract).t()?;
        if generated.root() != tree.root()
            || generated.first_free()? != tree.first_free()?
            || tree.first_free()?.value() != 3
            || tree
                .find_path_for_leaf(BoundedUint::<255>::new(0)?)
                .is_none()
            || tree.history()?.is_empty()
        {
            return Err("proven historic default state differs from indexed tree".into());
        }
        Ok(())
    })?;
    println!("typed historic indexed Merkle insertions proved and applied through ledger-8");
    Ok(())
}
