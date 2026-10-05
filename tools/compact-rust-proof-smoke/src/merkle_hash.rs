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

//! Prove a generated plain-tree `insertHash` call and apply it on ledger-8.

use super::*;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let circuit = "append_hash";
    let hash = FixedBytes::new([1; 32]);
    let mut rng = StdRng::seed_from_u64(0x0137_4841_5348);
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
    let expected = merkle_contract::append_hash(observed.circuit_context(()), hash)?;
    let recorded = merkle_contract::recorded::append_hash(observed.circuit_context(()), hash)?;
    if !recorded.execution.private_transcript_outputs.is_empty()
        || recorded.execution.context.query.state.get_ref()
            != expected.context.query.state.get_ref()
    {
        return Err("recorded hash append differs from native execution".into());
    }
    let manual = check_generated_trace(root, circuit, recorded, hash)?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/append_hash.verifier"),
    )?))?;
    let typed = merkle_contract::recorded::Contract
        .append_hash_call(&observed, (), hash)?
        .prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("hash append observed call differs from recorded prototype".into());
    }
    let expected_state = expected.context.query.state.get_ref().clone();
    check_transaction(root, circuit, deploy, typed, &mut rng, |contract| {
        let state = contract.data.get_ref();
        if state != &expected_state {
            return Err("proven hash append differs from native ledger state".into());
        }
        let tree = merkle_tree_view_at_path(state, &[0])?;
        let generated = merkle_contract::PublicStateView::from(contract).t()?;
        if tree.first_free()?.value() != 1
            || generated.first_free()? != tree.first_free()?
            || generated.root() != tree.root()
        {
            return Err("proven hash append changed Merkle state unexpectedly".into());
        }
        Ok(())
    })?;
    println!("plain Merkle insertHash proved and applied through ledger-8");
    let circuit = "place_hash";
    let hash = FixedBytes::new([2; 32]);
    let index = BoundedUint::<{ u64::MAX as u128 }>::new(1)?;
    let mut rng = StdRng::seed_from_u64(0x0140_4841_5348);
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
    let expected = merkle_contract::place_hash(observed.circuit_context(()), hash, index)?;
    let recorded =
        merkle_contract::recorded::place_hash(observed.circuit_context(()), hash, index)?;
    if !recorded.execution.private_transcript_outputs.is_empty()
        || recorded.execution.context.query.state.get_ref()
            != expected.context.query.state.get_ref()
    {
        return Err("recorded indexed hash placement differs from native execution".into());
    }
    let manual = check_generated_trace(root, circuit, recorded, (hash, index))?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/place_hash.verifier"),
    )?))?;
    let typed = merkle_contract::recorded::Contract
        .place_hash_call(&observed, (), hash, index)?
        .prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("indexed hash observed call differs from recorded prototype".into());
    }
    let expected_state = expected.context.query.state.get_ref().clone();
    check_transaction(root, circuit, deploy, typed, &mut rng, |contract| {
        let state = contract.data.get_ref();
        if state != &expected_state {
            return Err("proven indexed hash differs from native ledger state".into());
        }
        let tree = merkle_tree_view_at_path(state, &[0])?;
        let generated = merkle_contract::PublicStateView::from(contract).t()?;
        if tree.first_free()?.value() != 2
            || generated.first_free()? != tree.first_free()?
            || generated.root() != tree.root()
        {
            return Err("proven indexed hash changed Merkle state unexpectedly".into());
        }
        Ok(())
    })?;
    println!("plain Merkle insertHashIndex proved and applied through ledger-8");
    Ok(())
}
