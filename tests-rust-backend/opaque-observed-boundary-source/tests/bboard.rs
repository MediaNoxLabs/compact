// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
include!(concat!(env!("OUT_DIR"), "/bboard.rs"));
use ledger_contract as c;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, EntryPointBuf,
};
use r::context::{ConstructorContext, WitnessContext};
use r::ledger::{ContractState, StateValue};
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};
use runtime as r;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default, Clone)]
struct Witness {
    calls: Rc<RefCell<Vec<u64>>>,
}
impl c::Witnesses<u64> for Witness {
    fn local_secret_key(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> (u64, r::FixedBytes<32>) {
        self.calls.borrow_mut().push(*ctx.private_state);
        (*ctx.private_state + 1, r::FixedBytes::new([7; 32]))
    }
}
fn observed(
    state: StateValue<r::ledger::DefaultDB>,
    verifier: &VerifierKey,
) -> ObservedContractState {
    let operations = midnight_storage::storage::HashMap::new()
        .insert(
            EntryPointBuf(b"post".to_vec()),
            ContractOperation::new(Some(verifier.clone())),
        )
        .insert(
            EntryPointBuf(b"take_down".to_vec()),
            ContractOperation::new(Some(verifier.clone())),
        );
    ObservedContractState::new(
        Default::default(),
        ContractState::new(state, operations, ContractMaintenanceAuthority::default()),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    )
}
fn initial(verifier: &VerifierKey) -> ObservedContractState {
    let initialized = c::initial_state(ConstructorContext::new(5_u64))
        .unwrap()
        .into_circuit_context(Default::default());
    observed(initialized.query.state.get_ref().clone(), verifier)
}
fn invalid() -> r::CompactError {
    r::CompactError::InvalidLedgerCell("opaque ledger input has trailing zero bytes".into())
}
#[test]
fn noncanonical_observed_input_is_rejected_before_the_first_witness() {
    let verifier: VerifierKey = StdRng::seed_from_u64(555).r#gen();
    for value in ["\0", "hello\0", "é文\0\0"] {
        let witness = Witness::default();
        let contract = c::Contract::from(witness.clone());
        let state = initial(&verifier);
        assert_eq!(
            contract
                .recording()
                .post_call(&state, 5, value.into())
                .err()
                .unwrap(),
            invalid()
        );
        assert!(witness.calls.borrow().is_empty());
    }
}

#[test]
fn canonical_generated_post_and_take_down_preserve_exact_prepared_atoms() {
    let verifier: VerifierKey = StdRng::seed_from_u64(556).r#gen();
    for text in [
        "",
        "é文🦀",
        "hi\0world",
        "a message longer than thirty-one bytes",
    ] {
        let witness = Witness::default();
        let contract = c::Contract::from(witness.clone());
        let first = initial(&verifier);
        let post = contract
            .recording()
            .post_call(&first, 5, text.into())
            .unwrap();
        assert_eq!(*witness.calls.borrow(), vec![5]);
        let next_state = post
            .recorded()
            .execution
            .context
            .query
            .state
            .get_ref()
            .clone();
        assert_eq!(post.recorded().execution.context.private_state, 6);
        assert!(!post.recorded().public.verify_ops().is_empty());
        let prepared = post
            .prepare(verifier.clone(), r::Field::from(0_u64))
            .unwrap();
        assert_eq!(prepared.input.value.0.len(), 1);
        assert_eq!(prepared.input.value.0[0].0, text.as_bytes());
        let next = observed(next_state, &verifier);
        let take = contract.recording().take_down_call(&next, 6).unwrap();
        assert_eq!(*witness.calls.borrow(), vec![5, 6]);
        assert_eq!(
            take.recorded().execution.result,
            r::OpaqueString::from(text)
        );
        let prepared = take
            .prepare(verifier.clone(), r::Field::from(0_u64))
            .unwrap();
        assert_eq!(prepared.output.value.0.len(), 1);
        assert_eq!(prepared.output.value.0[0].0, text.as_bytes());
    }
}
