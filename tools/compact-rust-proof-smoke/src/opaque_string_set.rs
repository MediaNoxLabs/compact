// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0

//! Prove the generated variable-length OpaqueString Set calls through ledger-8.

use super::*;
use compact_rust_opaque_string_set_oracle_fixture::ledger_contract as contract;
use midnight_compact_runtime::OpaqueString;
use midnight_compact_runtime::ledger::ContractAddress;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0125_4f50_4151_5545);
    let key = OpaqueString::from("registry-key");
    let generated = contract::Contract::default();

    let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
    let deploy = make_deploy(
        root,
        "addName",
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
    let recorded = generated
        .recording()
        .addName(observed.circuit_context(7_u64), key.clone())?;
    if !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("opaque Set insertion unexpectedly produced private output".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "addName", recorded, key.clone())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/addName.verifier"),
    )?))?;
    let typed = generated
        .recording()
        .addName_call(&observed, 7_u64, key.clone())?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("opaque Set addName observed call differs from direct recording".into());
    }
    check_transaction(root, "addName", deploy, typed, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven addName changed unexpected ledger state".into());
        }
        if !set_view_at_path::<OpaqueString, _>(state.data.get_ref(), &[0])?.member(key.clone()) {
            return Err("proven addName did not insert the string key".into());
        }
        Ok(())
    })?;
    println!("opaque Set addName proof verified and applied through ledger-8");

    let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
    let added = contract::addName(
        initial.into_circuit_context(ContractAddress::default()),
        key.clone(),
    )?;
    let deploy = make_deploy(
        root,
        "hasName",
        added.context.query.state.get_ref().clone(),
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
    let recorded = generated
        .recording()
        .hasName(observed.circuit_context(7_u64), key.clone())?;
    if !recorded.execution.result || !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("opaque Set membership recording changed its result or private effects".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "hasName", recorded, key.clone())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/hasName.verifier"),
    )?))?;
    let typed = generated
        .recording()
        .hasName_call(&observed, 7_u64, key.clone())?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("opaque Set hasName observed call differs from direct recording".into());
    }
    check_transaction(root, "hasName", deploy, typed, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven hasName changed unexpected ledger state".into());
        }
        if !set_view_at_path::<OpaqueString, _>(state.data.get_ref(), &[0])?.member(key.clone()) {
            return Err("proven hasName lost the string key".into());
        }
        Ok(())
    })?;
    println!("opaque Set hasName proof verified and applied through ledger-8");
    Ok(())
}
