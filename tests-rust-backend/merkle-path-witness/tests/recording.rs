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
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.enter(*context.private_state, false)?;
        let tree = context.ledger.t()?;
        let root = MerkleTreeDigest {
            field: tree.root().unwrap().0,
        };
        let full = tree.is_full()?;
        let known = tree.check_root(root)?;
        Ok((8, !full && known))
    }
    fn historic_checks(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.enter(*context.private_state, true)?;
        let tree = context.ledger.h()?;
        let root = MerkleTreeDigest {
            field: tree.root().unwrap().0,
        };
        let prior = tree
            .history()?
            .into_iter()
            .find(|entry| entry.0 != root.field)
            .unwrap();
        let full = tree.is_full()?;
        let known = tree.check_root(root)?;
        let prior_known = tree.check_root(MerkleTreeDigest { field: prior.0 })?;
        Ok((8, !full && known && prior_known))
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

#[test]
fn metered_merkle_check_witnesses_match_native_observed_and_ts_query_costs() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-path-witness.json"
    ))
    .unwrap();
    for historic in [false, true] {
        for fail in [false, true] {
            let state = setup();
            let (a, b, w) = (
                Witness::new(historic, fail),
                Witness::new(historic, fail),
                Witness::new(historic, fail),
            );
            let native = if historic {
                c::check_witness_history(state.circuit_context(7), &a)
            } else {
                c::check_witness_merkle(state.circuit_context(7), &a)
            };
            let recorded = if historic {
                c::recorded::check_witness_history(state.circuit_context(7), &b)
            } else {
                c::recorded::check_witness_merkle(state.circuit_context(7), &b)
            };
            let observed_calls = Rc::clone(&w.calls);
            let contract = c::Contract::from(w);
            let handle = contract.recording();
            let call = if historic {
                handle.check_witness_history_call(&state, 7)
            } else {
                handle.check_witness_merkle_call(&state, 7)
            };
            if fail {
                for error in [
                    native.err().unwrap(),
                    recorded.err().unwrap(),
                    call.err().unwrap(),
                ] {
                    assert_eq!(error, failure());
                }
            } else {
                let native = native.unwrap();
                let recorded = recorded.unwrap();
                let call = call.unwrap();
                let row = &oracle[if historic {
                    "historicReads"
                } else {
                    "plainReads"
                }];
                assert_eq!(native.result, row["result"].as_bool().unwrap());
                for actual in [&recorded.execution, &call.recorded().execution] {
                    assert_eq!(actual.result, native.result);
                    assert_eq!(actual.context.query.state, native.context.query.state);
                    assert_eq!(actual.context.query.effects, native.context.query.effects);
                    assert_eq!(actual.context.private_state, 8);
                    assert_eq!(actual.gas_cost, native.gas_cost);
                    assert_eq!(
                        actual.private_transcript_outputs,
                        native.private_transcript_outputs
                    );
                }
                let gas = serde_json::to_value(native.gas_cost).unwrap();
                for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
                    let expected: u64 = row["queries"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|q| {
                            q["gasCost"][dimension]
                                .as_str()
                                .unwrap()
                                .parse::<u64>()
                                .unwrap()
                        })
                        .sum();
                    assert_eq!(gas[dimension], expected);
                }
                let output = &native.private_transcript_outputs[0];
                assert_eq!(
                    serde_json::to_value(output.value.0.iter().map(|a| &a.0).collect::<Vec<_>>())
                        .unwrap(),
                    row["privateTranscriptOutputs"][0]["valueAtoms"]
                );
                assert_eq!(
                    serde_json::to_value(&output.alignment).unwrap(),
                    row["privateTranscriptOutputs"][0]["alignment"]
                );
                assert!(
                    recorded.public.verify_ops().is_empty(),
                    "private witness queries are metered, not public proof queries"
                );
            }
            assert_eq!(a.calls.get(), 1);
            assert_eq!(b.calls.get(), 1);
            assert_eq!(observed_calls.get(), 1);
        }
    }
}
