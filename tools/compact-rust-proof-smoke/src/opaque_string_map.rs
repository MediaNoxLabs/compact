// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0

//! Prove the generated opaque-key, fixed-Field Map calls through ledger-8.

use super::*;
use compact_rust_opaque_string_map_query_oracle_fixture::ledger_contract as contract;
use midnight_compact_runtime::OpaqueString;
use midnight_compact_runtime::ledger::ContractAddress;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0127_4f50_4151_5545);
    let key = OpaqueString::from("asset-1");
    let value = Field::from(42_u64);
    let generated = contract::Contract::default();

    let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
    let deploy = make_deploy(
        root,
        "put",
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
    let recorded =
        generated
            .recording()
            .put(observed.circuit_context(7_u64), key.clone(), value)?;
    if !recorded.execution.private_transcript_outputs.is_empty() {
        return Err("opaque Map insertion unexpectedly produced private output".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "put", recorded, (key.clone(), value))?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/put.verifier"),
    )?))?;
    let typed = generated
        .recording()
        .put_call(&observed, 7_u64, key.clone(), value)?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("opaque Map put observed call differs from direct recording".into());
    }
    check_transaction(root, "put", deploy, typed, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven put changed unexpected ledger state".into());
        }
        if map_view_at_path::<OpaqueString, Field, _>(state.data.get_ref(), &[0])?
            .lookup(key.clone())?
            != value
        {
            return Err("proven put did not store the Field value".into());
        }
        Ok(())
    })?;
    println!("opaque Map put proof verified and applied through ledger-8");

    let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
    let inserted = contract::put(
        initial.into_circuit_context(ContractAddress::default()),
        key.clone(),
        value,
    )?;
    let deploy = make_deploy(
        root,
        "ensure",
        inserted.context.query.state.get_ref().clone(),
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
        .ensure(observed.circuit_context(7_u64), key.clone())?;
    if recorded.execution.result != value
        || !recorded.execution.private_transcript_outputs.is_empty()
    {
        return Err("opaque Map ensure recording changed its result or private effects".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "ensure", recorded, key.clone())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/ensure.verifier"),
    )?))?;
    let typed = generated
        .recording()
        .ensure_call(&observed, 7_u64, key.clone())?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("opaque Map ensure observed call differs from direct recording".into());
    }
    check_transaction(root, "ensure", deploy, typed, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("proven ensure changed unexpected ledger state".into());
        }
        if map_view_at_path::<OpaqueString, Field, _>(state.data.get_ref(), &[0])?
            .lookup(key.clone())?
            != value
        {
            return Err("proven ensure changed the Map Field value".into());
        }
        Ok(())
    })?;
    println!("opaque Map ensure proof verified and applied through ledger-8");
    Ok(())
}
