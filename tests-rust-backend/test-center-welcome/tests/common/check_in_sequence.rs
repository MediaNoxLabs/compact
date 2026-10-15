// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Same generated composition laws for Unit and discarded Field witness results.
use crate::{
    WitnessResult, expected_private_output, ledger_contract as c, ledger_slots as slots,
    runtime as r, witness_result,
};
use compact_rust_test_center_welcome_fixture::{ledger_contract as welcome, types};
use r::context::{ConstructorContext, WitnessContext};
use r::recording::RecordingFrame;
use r::transaction::{Observation, ObservedContractState};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Default)]
struct Witness {
    calls: Rc<RefCell<Vec<(u64, String, bool)>>>,
    refuse: bool,
}
impl Witness {
    fn respond(
        &self,
        private: u64,
        key: r::OpaqueString,
        inserted: bool,
    ) -> Result<(u64, WitnessResult), r::CompactError> {
        self.calls.borrow_mut().push((private, key.0, inserted));
        assert!(
            inserted,
            "the callback must observe the preceding insertion"
        );
        if self.refuse {
            return Err(provider_error());
        }
        Ok((private + 1, witness_result()))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn set_local_id(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
        key: r::OpaqueString,
    ) -> Result<(u64, WitnessResult), r::CompactError> {
        let inserted = context
            .ledger
            .checked_in_participants()?
            .member(key.clone())?;
        self.respond(*context.private_state, key, inserted)
    }
}
struct ConstructorWitness;
impl welcome::Witnesses<u64> for ConstructorWitness {
    fn local_sk(
        &self,
        context: WitnessContext<'_, u64, welcome::LedgerView<'_>>,
    ) -> (u64, types::MaybeCompact1) {
        (
            *context.private_state,
            types::MaybeCompact1 {
                is_some: true,
                value: r::FixedBytes::new([0; 32]),
            },
        )
    }
    fn set_local_id(
        &self,
        _: WitnessContext<'_, u64, welcome::LedgerView<'_>>,
        _: r::OpaqueString,
    ) -> (u64, ()) {
        panic!("constructor must not set participant id")
    }
}
fn state() -> ObservedContractState {
    // Original constructor supplies state. These synthetic variants only own check-in.
    let initial = welcome::initial_state(
        ConstructorContext::new(7_u64),
        &ConstructorWitness,
        r::FixedVector::new(std::array::from_fn(|i| types::Maybe {
            is_some: i == 7,
            value: if i == 7 { "alice" } else { "" }.into(),
        })),
    )
    .unwrap();
    ObservedContractState::new(
        Default::default(),
        r::ledger::ContractState::new(
            initial.ledger_state.get_ref().clone(),
            Default::default(),
            Default::default(),
        ),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    )
}
fn provider_error() -> r::CompactError {
    r::CompactError::InvalidLedgerCell("set_local_id provider refused".into())
}
fn manual(
    state: &ObservedContractState,
    witness: &Witness,
    key: r::OpaqueString,
) -> Result<r::recording::RecordedCircuitResult<u64, ()>, r::CompactError> {
    let (frame, member) = slots::eligible_participants.record_member(
        RecordingFrame::new(state.circuit_context(7_u64)),
        key.clone(),
    )?;
    if !member {
        return Err(r::CompactError::AssertionFailed(
            "Not eligible participant".into(),
        ));
    }
    let frame = slots::checked_in_participants.record_insert(frame, key.clone())?;
    let (frame, _) = frame.try_witness_metered(|context, meter| {
        let inserted = slots::checked_in_participants
            .witness_view(meter)?
            .member(key.clone())?;
        witness.respond(context.private_state, key, inserted)
    })?;
    Ok(frame.finish(()))
}
#[test]
fn callback_observes_insertion_and_preserves_exact_discarded_output() {
    let state = state();
    let witnesses: [Witness; 4] = std::array::from_fn(|_| Witness::default());
    let native = c::check_in(state.circuit_context(7_u64), &witnesses[0], "alice".into()).unwrap();
    let recorded =
        c::recorded::check_in(state.circuit_context(7_u64), &witnesses[1], "alice".into()).unwrap();
    let contract = c::Contract::from(witnesses[2].clone());
    let observed = contract
        .recording()
        .check_in_call(&state, 7_u64, "alice".into())
        .unwrap();
    let manual = manual(&state, &witnesses[3], "alice".into()).unwrap();
    assert_eq!(native.context.private_state, 8);
    assert_eq!(
        native.private_transcript_outputs,
        [expected_private_output()]
    );
    for actual in [&recorded, observed.recorded(), &manual] {
        assert_eq!(
            actual.execution.context.query.state,
            native.context.query.state
        );
        assert_eq!(
            actual.execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(actual.execution.context.private_state, 8);
        assert_eq!(
            actual.execution.private_transcript_outputs,
            native.private_transcript_outputs
        );
        assert_eq!(actual.execution.gas_cost, native.gas_cost);
        assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
        let replay = actual
            .public
            .initial()
            .query(actual.public.verify_ops(), None, &native.context.cost_model)
            .unwrap();
        assert_eq!(replay.context.state, native.context.query.state);
        assert_eq!(replay.context.effects, native.context.query.effects);
    }
    for witness in witnesses {
        assert_eq!(*witness.calls.borrow(), [(7, "alice".into(), true)]);
    }
}
#[test]
fn assertion_precedes_callback_and_provider_failure_keeps_the_observed_prefix() {
    let state = state();
    for key in ["bob", "alice"] {
        let expected = if key == "bob" {
            r::CompactError::AssertionFailed("Not eligible participant".into())
        } else {
            provider_error()
        };
        for mode in 0..4 {
            let witness = Witness {
                refuse: true,
                ..Default::default()
            };
            let error = match mode {
                0 => c::check_in(state.circuit_context(7_u64), &witness, key.into())
                    .err()
                    .unwrap(),
                1 => c::recorded::check_in(state.circuit_context(7_u64), &witness, key.into())
                    .err()
                    .unwrap(),
                2 => c::Contract::from(witness.clone())
                    .recording()
                    .check_in_call(&state, 7_u64, key.into())
                    .err()
                    .unwrap(),
                _ => manual(&state, &witness, key.into()).err().unwrap(),
            };
            assert_eq!(error, expected);
            let expected_calls = if key == "bob" {
                vec![]
            } else {
                vec![(7, "alice".into(), true)]
            };
            assert_eq!(*witness.calls.borrow(), expected_calls);
        }
    }
    // We assert callback-visible ordering, not rollback of an inaccessible moved context.
}
