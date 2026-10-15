// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_effect_plan_witness_inputs_source_fixture::support::*;
use compact_rust_effect_plan_witness_inputs_source_fixture::{ledger_contract as c, runtime as r};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, EntryPointBuf,
};
use r::ledger::ContractState;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};
fn setup() -> (ObservedContractState, VerifierKey) {
    let key: VerifierKey = StdRng::seed_from_u64(557).r#gen();
    let operations = midnight_storage::storage::HashMap::new().insert(
        EntryPointBuf(b"run".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    let state = ContractState::new(
        context().query.state.get_ref().clone(),
        operations,
        ContractMaintenanceAuthority::default(),
    );
    (
        ObservedContractState::new(
            Default::default(),
            state,
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        key,
    )
}
#[test]
fn canonical_public_inputs_prepare_literal_atoms_and_only_boolean_private_outputs() {
    for (text, bytes) in [
        ("", vec![]),
        ("é文🦀", vec![255, 254]),
        ("hi\0world", vec![0, 1, 0, 2]),
    ] {
        for selected in [false, true] {
            let (state, key) = setup();
            let witness = Witness::default();
            let contract = c::Contract::from(witness.clone());
            let call = contract
                .recording()
                .run_call(&state, 7, text.into(), bytes.clone().into(), selected)
                .unwrap();
            let count = if selected { 2 } else { 1 };
            assert_eq!(
                call.recorded().execution.private_transcript_outputs,
                vec![r::fab::AlignedValue::from(true); count]
            );
            assert!(!call.recorded().public.verify_ops().is_empty());
            assert_eq!(witness.events.borrow().len(), count);
            for event in witness.events.borrow().iter() {
                assert_eq!(event.arguments, arguments(text, &bytes));
                assert_eq!(event.counter, 6);
            }
            let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
            let atoms = prepared
                .input
                .value
                .0
                .iter()
                .map(|atom| atom.0.clone())
                .collect::<Vec<_>>();
            assert_eq!(
                atoms,
                vec![
                    text.as_bytes().to_vec(),
                    bytes.clone(),
                    if selected { vec![1] } else { vec![] }
                ]
            );
            assert_eq!(prepared.output.value.0.len(), 1);
            assert_eq!(prepared.output.value.0[0].0, vec![1]);
        }
    }
}
#[test]
fn observed_public_normal_form_policy_does_not_become_a_witness_input_policy() {
    for (text, bytes) in [("bad\0", vec![1]), ("good", vec![1, 0]), ("\0", vec![0])] {
        let (state, _) = setup();
        let witness = Witness::default();
        let contract = c::Contract::from(witness.clone());
        assert_eq!(
            contract
                .recording()
                .run_call(&state, 7, text.into(), bytes.into(), true)
                .err()
                .unwrap(),
            r::CompactError::InvalidLedgerCell(
                "opaque ledger input has trailing zero bytes".into()
            )
        );
        assert!(witness.events.borrow().is_empty());
    }
}
