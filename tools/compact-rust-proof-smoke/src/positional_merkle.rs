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

//! Plain and historic Merkle positional scenarios.
//!
//! The caller owns admission, execution order, and the shared seeded RNG.
//! Each scenario borrows that RNG and preserves its own evidence and failures.

use std::error::Error;
use std::path::Path;

use compact_rust_hmt_insert_oracle_fixture::ledger_contract as historic_merkle_contract;
use compact_rust_merkle_tree_oracle_fixture::ledger_contract as merkle_contract;
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{
    historic_merkle_tree_view_at_path, merkle_tree_view_at_path,
};
use rand::rngs::StdRng;

use super::{check_generated_trace, check_transaction, make_deploy};

pub(super) fn check_plain(merkle_root: &Path, rng: &mut StdRng) -> Result<(), Box<dyn Error>> {
    let circuit = "append";
    let initial = merkle_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        merkle_root,
        circuit,
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let value = BoundedUint::<255>::new(7)?;
    let recorded = merkle_contract::Contract::default()
        .recording
        .append(context, value)?;
    let call = check_generated_trace(merkle_root, circuit, recorded, value)?;
    check_transaction(merkle_root, circuit, deploy, call, rng, |contract| {
        let tree = merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
        let generated = merkle_contract::PublicStateView::from(contract).t()?;
        if generated.root() != tree.root()
            || generated.first_free()? != tree.first_free()?
            || generated.find_path_for_leaf(value).is_some()
                != tree.find_path_for_leaf(value).is_some()
        {
            return Err("generated proven Merkle view differs from raw view".into());
        }
        if tree.first_free()?.value() != 1 {
            return Err("proven Merkle append did not advance the leaf index".into());
        }
        if tree.find_path_for_leaf(value).is_none() {
            return Err("proven Merkle append did not store the typed leaf".into());
        }
        Ok(())
    })?;

    let circuit = "full";
    let initial = merkle_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        merkle_root,
        circuit,
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = merkle_contract::Contract::default()
        .recording
        .full(context)?;
    if recorded.execution.result {
        return Err("new plain Merkle tree unexpectedly full".into());
    }
    let call = check_generated_trace(merkle_root, circuit, recorded, ())?;
    check_transaction(merkle_root, circuit, deploy, call, rng, |contract| {
        let tree = merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
        if tree.first_free()?.value() != 0 {
            return Err("proven plain Merkle fullness read changed the tree".into());
        }
        Ok(())
    })?;
    Ok(())
}

pub(super) fn check_historic(historic_root: &Path, rng: &mut StdRng) -> Result<(), Box<dyn Error>> {
    let circuit = "append";
    let initial = historic_merkle_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        historic_root,
        circuit,
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let value = BoundedUint::<255>::new(7)?;
    let recorded = historic_merkle_contract::Contract::default()
        .recording
        .append(context, value)?;
    let call = check_generated_trace(historic_root, circuit, recorded, value)?;
    check_transaction(historic_root, circuit, deploy, call, rng, |contract| {
        let tree = historic_merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
        let generated = historic_merkle_contract::PublicStateView::from(contract).t()?;
        if generated.root() != tree.root()
            || generated.first_free()? != tree.first_free()?
            || generated.find_path_for_leaf(value).is_some()
                != tree.find_path_for_leaf(value).is_some()
            || generated.history()? != tree.history()?
        {
            return Err("generated proven historic Merkle view differs from raw view".into());
        }
        if tree.first_free()?.value() != 1 {
            return Err("proven historic append did not advance the leaf index".into());
        }
        if tree.find_path_for_leaf(value).is_none() {
            return Err("proven historic append did not store the typed leaf".into());
        }
        if tree.history()?.len() != 2 {
            return Err("proven historic append did not retain both roots".into());
        }
        Ok(())
    })?;

    let circuit = "full";
    let initial = historic_merkle_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        historic_root,
        circuit,
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = historic_merkle_contract::Contract::default()
        .recording
        .full(context)?;
    if recorded.execution.result {
        return Err("new historic Merkle tree unexpectedly full".into());
    }
    let call = check_generated_trace(historic_root, circuit, recorded, ())?;
    check_transaction(historic_root, circuit, deploy, call, rng, |contract| {
        let tree = historic_merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
        if tree.first_free()?.value() != 0 || tree.history()?.len() != 1 {
            return Err("proven historic Merkle fullness read changed the tree".into());
        }
        Ok(())
    })?;
    Ok(())
}
