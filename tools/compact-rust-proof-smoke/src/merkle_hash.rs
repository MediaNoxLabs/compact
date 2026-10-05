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

pub(super) fn run_historic(root: &Path) -> Result<(), Box<dyn Error>> {
    let circuit = "append_hash";
    let hash = FixedBytes::new([1; 32]);
    let mut rng = StdRng::seed_from_u64(0x0142_4849_5354);
    let initial = historic_merkle_contract::initial_state(ConstructorContext::new(()))?;
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
    let expected = historic_merkle_contract::append_hash(observed.circuit_context(()), hash)?;
    let recorded =
        historic_merkle_contract::recorded::append_hash(observed.circuit_context(()), hash)?;
    if !recorded.execution.private_transcript_outputs.is_empty()
        || recorded.execution.context.query.state.get_ref()
            != expected.context.query.state.get_ref()
    {
        return Err("recorded historic hash append differs from native execution".into());
    }
    let manual = check_generated_trace(root, circuit, recorded, hash)?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/append_hash.verifier"),
    )?))?;
    let typed = historic_merkle_contract::recorded::Contract
        .append_hash_call(&observed, (), hash)?
        .prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("historic hash observed call differs from recorded prototype".into());
    }
    let expected_state = expected.context.query.state.get_ref().clone();
    let expected_history = historic_merkle_tree_view_at_path(&expected_state, &[0])?.history()?;
    check_transaction(root, circuit, deploy, typed, &mut rng, |contract| {
        let state = contract.data.get_ref();
        if state != &expected_state {
            return Err("proven historic hash append differs from native ledger state".into());
        }
        let tree = historic_merkle_tree_view_at_path(state, &[0])?;
        let generated = historic_merkle_contract::PublicStateView::from(contract).t()?;
        if tree.first_free()?.value() != 1
            || generated.first_free()? != tree.first_free()?
            || generated.root() != tree.root()
            || tree.history()? != expected_history
            || generated.history()? != expected_history
        {
            return Err("proven historic hash append changed root history unexpectedly".into());
        }
        Ok(())
    })?;
    println!("historic Merkle insertHash proved and applied through ledger-8");
    Ok(())
}

pub(super) fn run_historic_indexed(root: &Path) -> Result<(), Box<dyn Error>> {
    let circuit = "place_hash";
    let hash = FixedBytes::new([2; 32]);
    let index = BoundedUint::<{ u64::MAX as u128 }>::new(1)?;
    let mut rng = StdRng::seed_from_u64(0x0144_4849_5354);
    let initial = historic_merkle_contract::initial_state(ConstructorContext::new(()))?;
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
    let expected = historic_merkle_contract::place_hash(observed.circuit_context(()), hash, index)?;
    let recorded =
        historic_merkle_contract::recorded::place_hash(observed.circuit_context(()), hash, index)?;
    if !recorded.execution.private_transcript_outputs.is_empty()
        || recorded.execution.context.query.state.get_ref()
            != expected.context.query.state.get_ref()
    {
        return Err("recorded historic indexed hash differs from native execution".into());
    }
    let manual = check_generated_trace(root, circuit, recorded, (hash, index))?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/place_hash.verifier"),
    )?))?;
    let typed = historic_merkle_contract::recorded::Contract
        .place_hash_call(&observed, (), hash, index)?
        .prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("historic indexed hash observed call differs from recording".into());
    }
    let expected_state = expected.context.query.state.get_ref().clone();
    let expected_history = historic_merkle_tree_view_at_path(&expected_state, &[0])?.history()?;
    check_transaction(root, circuit, deploy, typed, &mut rng, |contract| {
        let state = contract.data.get_ref();
        if state != &expected_state {
            return Err("proven historic indexed hash differs from native ledger state".into());
        }
        let tree = historic_merkle_tree_view_at_path(state, &[0])?;
        let generated = historic_merkle_contract::PublicStateView::from(contract).t()?;
        if tree.first_free()?.value() != 2
            || generated.first_free()? != tree.first_free()?
            || generated.root() != tree.root()
            || tree.history()? != expected_history
            || generated.history()? != expected_history
        {
            return Err("proven historic indexed hash changed root history unexpectedly".into());
        }
        Ok(())
    })?;
    println!("historic Merkle insertHashIndex proved and applied through ledger-8");
    Ok(())
}

pub(super) fn run_historic_reset_history(root: &Path) -> Result<(), Box<dyn Error>> {
    let circuit = "forget_history";
    let mut rng = StdRng::seed_from_u64(0x0148_4849_5354);
    let initial = historic_merkle_contract::initial_state(ConstructorContext::new(()))?;
    let initial_root = historic_merkle_tree_view_at_path(initial.ledger_state.get_ref(), &[0])?
        .root()
        .ok_or("missing initial historic Merkle root")?;
    let context =
        initial.into_circuit_context(midnight_compact_runtime::ledger::ContractAddress::default());
    let context = historic_merkle_contract::append(context, BoundedUint::<255>::new(7)?)?.context;
    let context = historic_merkle_contract::append(context, BoundedUint::<255>::new(8)?)?.context;
    let pre_state = context.query.state.get_ref().clone();
    let previous = historic_merkle_tree_view_at_path(&pre_state, &[0])?;
    let current_root = previous
        .root()
        .ok_or("missing current historic Merkle root")?;
    if previous.history()?.len() < 2 {
        return Err("historic reset proof needs at least two roots".into());
    }
    let deploy = make_deploy(root, circuit, pre_state, &mut rng)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let expected = historic_merkle_contract::forget_history(observed.circuit_context(()))?;
    let recorded =
        historic_merkle_contract::recorded::forget_history(observed.circuit_context(()))?;
    if !recorded.execution.private_transcript_outputs.is_empty()
        || recorded.execution.context.query.state.get_ref()
            != expected.context.query.state.get_ref()
        || recorded.execution.gas_cost != expected.gas_cost
        || recorded.execution.context.query.effects != expected.context.query.effects
    {
        return Err("recorded historic history reset differs from native execution".into());
    }
    let manual = check_generated_trace(root, circuit, recorded, ())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/forget_history.verifier"),
    )?))?;
    let typed = historic_merkle_contract::recorded::Contract
        .forget_history_call(&observed, ())?
        .prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("historic reset observed call differs from recording".into());
    }
    let expected_state = expected.context.query.state.get_ref().clone();
    check_transaction(root, circuit, deploy, typed, &mut rng, |contract| {
        let state = contract.data.get_ref();
        if state != &expected_state {
            return Err("proven historic history reset differs from native ledger state".into());
        }
        let tree = historic_merkle_tree_view_at_path(state, &[0])?;
        let generated = historic_merkle_contract::PublicStateView::from(contract).t()?;
        let expected_history = vec![current_root];
        if tree.history()? != expected_history
            || generated.history()? != expected_history
            || tree.root() != Some(current_root)
            || generated.root() != Some(current_root)
            || tree.contains_root(initial_root)
            || !tree.contains_root(current_root)
        {
            return Err("proven historic history reset did not retain only current root".into());
        }
        Ok(())
    })?;
    println!("historic Merkle resetHistory proved and applied through ledger-8");
    Ok(())
}

pub(super) fn run_historic_reset_tree(root: &Path) -> Result<(), Box<dyn Error>> {
    let circuit = "reset_tree";
    let mut rng = StdRng::seed_from_u64(0x0150_4849_5354);
    let initial = historic_merkle_contract::initial_state(ConstructorContext::new(()))?;
    let blank_state = initial.ledger_state.get_ref().clone();
    let blank_root = historic_merkle_tree_view_at_path(&blank_state, &[0])?
        .root()
        .ok_or("missing blank historic Merkle root")?;
    let context =
        initial.into_circuit_context(midnight_compact_runtime::ledger::ContractAddress::default());
    let context = historic_merkle_contract::append(context, BoundedUint::<255>::new(7)?)?.context;
    let context = historic_merkle_contract::append(context, BoundedUint::<255>::new(8)?)?.context;
    let pre_state = context.query.state.get_ref().clone();
    let previous = historic_merkle_tree_view_at_path(&pre_state, &[0])?;
    let old_root = previous
        .root()
        .ok_or("missing populated historic Merkle root")?;
    if previous.history()?.len() < 2 || old_root == blank_root {
        return Err("historic tree reset proof needs a populated tree".into());
    }
    let deploy = make_deploy(root, circuit, pre_state, &mut rng)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let expected = historic_merkle_contract::reset_tree(observed.circuit_context(()))?;
    let recorded = historic_merkle_contract::recorded::reset_tree(observed.circuit_context(()))?;
    if !recorded.execution.private_transcript_outputs.is_empty()
        || recorded.execution.context.query.state.get_ref()
            != expected.context.query.state.get_ref()
        || recorded.execution.gas_cost != expected.gas_cost
        || recorded.execution.context.query.effects != expected.context.query.effects
        || recorded.execution.context.query.state.get_ref() != &blank_state
    {
        return Err("recorded historic tree reset differs from native blank state".into());
    }
    let manual = check_generated_trace(root, circuit, recorded, ())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/reset_tree.verifier"),
    )?))?;
    let typed = historic_merkle_contract::recorded::Contract
        .reset_tree_call(&observed, ())?
        .prepare(verifier, Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("historic tree reset observed call differs from recording".into());
    }
    check_transaction(root, circuit, deploy, typed, &mut rng, |contract| {
        let state = contract.data.get_ref();
        if state != &blank_state {
            return Err("proven historic reset differs from blank ledger state".into());
        }
        let tree = historic_merkle_tree_view_at_path(state, &[0])?;
        let generated = historic_merkle_contract::PublicStateView::from(contract).t()?;
        let expected_history = vec![blank_root];
        if tree.first_free()?.value() != 0
            || generated.first_free()? != tree.first_free()?
            || tree.history()? != expected_history
            || generated.history()? != expected_history
            || tree.root() != Some(blank_root)
            || generated.root() != Some(blank_root)
            || tree.contains_root(old_root)
            || !tree.contains_root(blank_root)
        {
            return Err("proven historic tree reset did not seed blank-root history".into());
        }
        Ok(())
    })?;
    println!("historic Merkle resetToDefault proved and applied through ledger-8");
    Ok(())
}
