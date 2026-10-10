// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]

use compact_rust_merkle_path_witness_fixture::{
    ledger_contract as c, ledger_slots, runtime as r,
    types::{MerkleTreeDigest, MerkleTreePath},
};
use r::context::{ConstructorContext, WitnessContext};
use r::transaction::{Observation, ObservedContractState};
use std::{cell::Cell, rc::Rc};

struct Witness {
    calls: Rc<Cell<usize>>,
    historic: bool,
    fail: bool,
}
impl Witness {
    fn new(historic: bool, fail: bool) -> Self {
        Self {
            calls: Rc::new(Cell::new(0)),
            historic,
            fail,
        }
    }
    fn enter(&self, private: u64, historic: bool) -> Result<(), r::CompactError> {
        assert_eq!(private, 7);
        assert_eq!(historic, self.historic);
        self.calls.set(self.calls.get() + 1);
        if self.fail { Err(failure()) } else { Ok(()) }
    }
}
fn failure() -> r::CompactError {
    r::CompactError::InvalidLedgerCell("path witness refused".into())
}
impl c::TryWitnesses<u64> for Witness {
    fn leaf_path(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, MerkleTreePath), r::CompactError> {
        self.enter(*context.private_state, false)?;
        let path = context
            .ledger
            .t()?
            .path_for_leaf(0, r::BoundedUint::<255>::new(7)?)?;
        Ok((8, MerkleTreePath::from_ledger_path(path)?))
    }
    fn historic_path(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, MerkleTreePath), r::CompactError> {
        self.enter(*context.private_state, true)?;
        let path = context
            .ledger
            .h()?
            .path_for_leaf(0, r::BoundedUint::<255>::new(7)?)?;
        Ok((8, MerkleTreePath::from_ledger_path(path)?))
    }
    fn merkle_checks(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        panic!("unexpected witness")
    }
    fn historic_checks(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        panic!("unexpected witness")
    }
}
fn setup() -> ObservedContractState {
    let context = c::initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(Default::default());
    let context = c::append(context, r::BoundedUint::new(7).unwrap())
        .unwrap()
        .context;
    let context = c::append_h(context, r::BoundedUint::new(7).unwrap())
        .unwrap()
        .context;
    ObservedContractState::new(
        Default::default(),
        r::ledger::ContractState::new(
            context.query.state.get_ref().clone(),
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

#[test]
fn new_path_recordings_match_native_manual_and_observed_execution() {
    for historic in [false, true] {
        let state = setup();
        let (a, b, w) = (
            Witness::new(historic, false),
            Witness::new(historic, false),
            Witness::new(historic, false),
        );
        let native = if historic {
            c::get_historic_path(state.circuit_context(7), &a)
        } else {
            c::get_path(state.circuit_context(7), &a)
        }
        .unwrap();
        let recorded = if historic {
            c::recorded::get_historic_path(state.circuit_context(7), &b)
        } else {
            c::recorded::get_path(state.circuit_context(7), &b)
        }
        .unwrap();
        // Independent operation recipe: one metered witness, no public VM operation.
        let frame = r::recording::RecordingFrame::new(state.circuit_context(7));
        let (frame, expected) = frame
            .try_witness_metered(|context, meter| {
                assert_eq!(context.private_state, 7);
                let leaf = r::BoundedUint::<255>::new(7)?;
                let path = if historic {
                    ledger_slots::h
                        .witness_view::<MerkleTreeDigest, _>(meter)?
                        .path_for_leaf(0, leaf)?
                } else {
                    ledger_slots::t
                        .witness_view::<MerkleTreeDigest, _>(meter)?
                        .path_for_leaf(0, leaf)?
                };
                Ok((8, MerkleTreePath::from_ledger_path(path)?))
            })
            .unwrap();
        let manual = frame.finish(expected.clone());
        let observed_calls = Rc::clone(&w.calls);
        let contract = c::Contract::from(w);
        let handle = contract.recording();
        let call = if historic {
            handle.get_historic_path_call(&state, 7)
        } else {
            handle.get_path_call(&state, 7)
        }
        .unwrap();
        assert_eq!(native.result, expected);
        for actual in [
            &recorded.execution,
            &manual.execution,
            &call.recorded().execution,
        ] {
            assert_eq!(actual.result, expected);
            assert_eq!(actual.context.query.state, native.context.query.state);
            assert_eq!(actual.context.query.effects, native.context.query.effects);
            assert_eq!(actual.context.private_state, 8);
            assert_eq!(actual.gas_cost, native.gas_cost);
            assert_eq!(
                actual.private_transcript_outputs,
                native.private_transcript_outputs
            );
            assert_eq!(
                actual.private_transcript_outputs,
                vec![r::fab::AlignedValue::from(expected.clone())]
            );
        }
        for actual in [&recorded, &manual, call.recorded()] {
            assert!(actual.public.verify_ops().is_empty());
            let replay = actual
                .public
                .initial()
                .query(
                    actual.public.verify_ops(),
                    None,
                    &actual.execution.context.cost_model,
                )
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
        }
        assert_eq!(a.calls.get(), 1);
        assert_eq!(b.calls.get(), 1);
        // The handle borrows the original witness; no extra callback is needed for observation.
        assert_eq!(observed_calls.get(), 1);
    }
}

#[test]
fn new_path_apis_propagate_the_first_witness_failure_once() {
    for historic in [false, true] {
        let state = setup();
        let (a, b, w) = (
            Witness::new(historic, true),
            Witness::new(historic, true),
            Witness::new(historic, true),
        );
        let native = if historic {
            c::get_historic_path(state.circuit_context(7), &a)
        } else {
            c::get_path(state.circuit_context(7), &a)
        };
        let recorded = if historic {
            c::recorded::get_historic_path(state.circuit_context(7), &b)
        } else {
            c::recorded::get_path(state.circuit_context(7), &b)
        };
        assert!(matches!(native, Err(e) if e==failure()));
        assert!(matches!(recorded, Err(e) if e==failure()));
        let observed_calls = Rc::clone(&w.calls);
        let contract = c::Contract::from(w);
        let handle = contract.recording();
        let call = if historic {
            handle.get_historic_path_call(&state, 7)
        } else {
            handle.get_path_call(&state, 7)
        };
        assert!(matches!(call, Err(e) if e==failure()));
        assert_eq!(a.calls.get(), 1);
        assert_eq!(b.calls.get(), 1);
        assert_eq!(observed_calls.get(), 1);
    }
}
