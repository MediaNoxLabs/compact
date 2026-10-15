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

//! Bounded and wide unsigned positional scenarios.
//!
//! The caller owns admission, execution order, and the shared seeded RNG.
//! Each scenario borrows that RNG and preserves its own evidence and failures.

use std::error::Error;
use std::fs;
use std::path::Path;

use compact_rust_uints_oracle_fixture::ledger_contract as uints_contract;
use compact_rust_wide_uint_oracle_fixture::ledger_contract as wide_contract;
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::read_cell_at_path;
use midnight_compact_runtime::transaction::{
    Observation, ObservedContractState, decode_verifier_key,
};
use midnight_transient_crypto::curve::Fr;
use rand::rngs::StdRng;

use super::{
    Uint248, WideSecret, check_generated_trace, check_observed_call_parity, check_transaction,
    make_deploy,
};

pub(super) fn check_bounded(root: &Path, rng: &mut StdRng) -> Result<(), Box<dyn Error>> {
    let initial = uints_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        "set_byte",
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let amount = BoundedUint::<255>::new(255)?;
    let contract = uints_contract::Contract::default();
    let recorded = contract
        .recording
        .set_byte(initial.into_circuit_context(deploy.address()), amount)?;
    let manual = check_generated_trace(root, "set_byte", recorded, amount)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let verifier = decode_verifier_key(&fs::read(root.join("keys/set_byte.verifier"))?)?;
    let typed = contract
        .recording
        .set_byte_call(&observed, (), amount)?
        .prepare(verifier, Fr::from(0_u64))?;
    check_observed_call_parity(root, "set_byte", &deploy, &manual, &typed)?;
    check_transaction(root, "set_byte", deploy, typed, rng, |state| {
        let byte = read_cell_at_path::<BoundedUint<255>, _>(state.data.get_ref(), &[0])?;
        if byte.value() != 255 {
            return Err("proven Uint<8> Cell write did not persist 255".into());
        }
        Ok(())
    })?;
    Ok(())
}

pub(super) fn check_wide(root: &Path, rng: &mut StdRng) -> Result<(), Box<dyn Error>> {
    let contract = wide_contract::Contract::from(WideSecret);
    let maximum = Uint248::from_le_bytes(&[0xff; 31])?;
    let initial = wide_contract::initial_state(ConstructorContext::new(7_u64))?;
    let deploy = make_deploy(
        root,
        "writeWide",
        initial.ledger_state.get_ref().clone(),
        rng,
    )?;
    let recorded = contract
        .recording()
        .writeWide(initial.into_circuit_context(deploy.address()))?;
    if recorded.execution.context.private_state != 8_u64 {
        return Err("recorded Uint<248> witness did not advance private state".into());
    }
    let manual = check_generated_trace(root, "writeWide", recorded, ())?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let verifier = decode_verifier_key(&fs::read(root.join("keys/writeWide.verifier"))?)?;
    let typed = contract
        .recording()
        .writeWide_call(&observed, 7_u64)?
        .prepare(verifier, Fr::from(0_u64))?;
    check_observed_call_parity(root, "writeWide", &deploy, &manual, &typed)?;
    check_transaction(root, "writeWide", deploy, typed, rng, |state| {
        let wide = read_cell_at_path::<Uint248, _>(state.data.get_ref(), &[0])?;
        if wide != maximum {
            return Err("proven Uint<248> witness write did not persist maximum".into());
        }
        Ok(())
    })?;

    // Prove a read against a seeded deployment whose Cell contains the
    // same witness value. The write and read use separate deployments;
    // this check does not imply a chained ledger transaction history.
    let initial = wide_contract::initial_state(ConstructorContext::new(7_u64))?;
    let seeded = wide_contract::writeWide(
        initial.into_circuit_context(Default::default()),
        &WideSecret,
    )?;
    let seed_state = seeded.context.query.state.get_ref().clone();
    let deploy = make_deploy(root, "readWide", seed_state.clone(), rng)?;
    let context = midnight_compact_runtime::context::CircuitContext::from_contract_state(
        7_u64,
        deploy.address(),
        &deploy.initial_state,
    );
    let recorded = contract.recording().readWide(context)?;
    if recorded.execution.result != maximum {
        return Err("recorded Uint<248> Cell read differed from seeded value".into());
    }
    let manual = check_generated_trace(root, "readWide", recorded, ())?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let verifier = decode_verifier_key(&fs::read(root.join("keys/readWide.verifier"))?)?;
    let typed = contract
        .recording()
        .readWide_call(&observed, 7_u64)?
        .prepare(verifier, Fr::from(0_u64))?;
    check_observed_call_parity(root, "readWide", &deploy, &manual, &typed)?;
    check_transaction(root, "readWide", deploy, typed, rng, |state| {
        if state.data.get_ref() != &seed_state {
            return Err("proven Uint<248> Cell read changed ledger state".into());
        }
        Ok(())
    })?;
    Ok(())
}
