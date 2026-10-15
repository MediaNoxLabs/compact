// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
include!(concat!(env!("OUT_DIR"), "/nested.rs"));
use ledger_contract as c;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, EntryPointBuf,
};
use r::context::{ConstructorContext, WitnessContext};
use r::ledger::ContractState;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};
use runtime as r;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default, Clone)]
struct Witness {
    calls: Rc<RefCell<Vec<(u64, u128)>>>,
}
impl c::TryWitnesses<u64> for Witness {
    fn permit(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.calls
            .borrow_mut()
            .push((*ctx.private_state, ctx.ledger.calls()?.value()));
        Ok((*ctx.private_state + 1, true))
    }
}
fn initial(verifier: &VerifierKey) -> ObservedContractState {
    let state = c::initial_state(ConstructorContext::new(5_u64))
        .unwrap()
        .into_circuit_context(Default::default())
        .query
        .state
        .get_ref()
        .clone();
    let operations = midnight_storage::storage::HashMap::new().insert(
        EntryPointBuf(b"accept".to_vec()),
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
fn input(text: &str) -> types::Outer {
    types::Outer {
        inner: types::Inner { text: text.into() },
    }
}
#[test]
fn recursive_observed_input_guard_precedes_callback_and_ledger_effects() {
    let verifier: VerifierKey = StdRng::seed_from_u64(557).r#gen();
    for text in ["\0", "hello\0", "é文\0\0"] {
        let witness = Witness::default();
        let contract = c::Contract::from(witness.clone());
        let state = initial(&verifier);
        let expected = r::CompactError::InvalidLedgerCell(
            "opaque ledger input has trailing zero bytes".into(),
        );
        assert_eq!(
            contract
                .recording()
                .accept_call(&state, 5, input(text))
                .err()
                .unwrap(),
            expected
        );
        assert!(witness.calls.borrow().is_empty());
        assert_eq!(
            ledger_slots::calls
                .inspect(state.contract().data.get_ref())
                .unwrap(),
            0
        );
        // Raw execution remains useful for local owned values and does not
        // encode this intentionally unused argument into the public call ABI.
        let native = c::accept(state.circuit_context(5), &witness, input(text)).unwrap();
        let recorded =
            c::recorded::accept(state.circuit_context(5), &witness, input(text)).unwrap();
        assert_eq!(*witness.calls.borrow(), vec![(5, 1), (5, 1)]);
        assert_eq!(
            native.context.query.state,
            recorded.execution.context.query.state
        );
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.private_transcript_outputs,
            recorded.execution.private_transcript_outputs
        );
    }
}
#[test]
fn canonical_nested_argument_is_exact_in_prepared_public_input() {
    let verifier: VerifierKey = StdRng::seed_from_u64(558).r#gen();
    for text in [
        "",
        "é文🦀",
        "hello\0world",
        "a message longer than thirty-one bytes",
    ] {
        let witness = Witness::default();
        let contract = c::Contract::from(witness.clone());
        let state = initial(&verifier);
        let call = contract
            .recording()
            .accept_call(&state, 5, input(text))
            .unwrap();
        assert_eq!(*witness.calls.borrow(), vec![(5, 1)]);
        assert_eq!(call.recorded().execution.context.private_state, 6);
        assert_eq!(
            call.recorded().execution.private_transcript_outputs,
            vec![r::fab::AlignedValue::from(true)]
        );
        assert!(!call.recorded().public.verify_ops().is_empty());
        let prepared = call
            .prepare(verifier.clone(), r::Field::from(0_u64))
            .unwrap();
        assert_eq!(prepared.input.value.0.len(), 1);
        assert_eq!(prepared.input.value.0[0].0, text.as_bytes());
        assert_eq!(prepared.output, r::fab::AlignedValue::from(()));
    }
}
