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

use midnight_compact_runtime::ledger::{
    self, ChargedState, ContractAddress, DefaultDB, QueryContext, StateValue, TranscriptRejected,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;
use midnight_onchain_vm::error::OnchainProgramError;

fn context() -> QueryContext<DefaultDB> {
    QueryContext::new(
        ChargedState::new(StateValue::Null),
        ContractAddress::default(),
    )
}

#[test]
fn empty_plain_reset_is_invalid_argument() {
    assert!(matches!(
        ledger::merkle_reset_to_default(&context(), &[][..], 3, None, &INITIAL_COST_MODEL),
        Err(TranscriptRejected::Execution(
            OnchainProgramError::InvalidArgs(_)
        ))
    ));
}

#[test]
fn empty_historic_reset_is_invalid_argument() {
    assert!(matches!(
        ledger::historic_reset_to_default(&context(), &[][..], 3, None, &INITIAL_COST_MODEL),
        Err(TranscriptRejected::Execution(
            OnchainProgramError::InvalidArgs(_)
        ))
    ));
}

#[test]
fn append_operand_overflow_is_bounds_error() {
    assert!(matches!(
        ledger::merkle_insert(
            &context(),
            &[0_u8; 255][..],
            true,
            None,
            &INITIAL_COST_MODEL
        ),
        Err(TranscriptRejected::Execution(
            OnchainProgramError::BoundsExceeded
        ))
    ));
}

use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::context::{CircuitContext, ConstructorContext, ConstructorResult};
use midnight_compact_runtime::recording::RecordingFrame;
use midnight_serialize::tagged_serialize;

fn circuit(state: StateValue<DefaultDB>) -> CircuitContext<()> {
    ConstructorResult::new(ConstructorContext::new(()), ChargedState::new(state))
        .into_circuit_context(ContractAddress::default())
}

fn bytes(state: &StateValue<DefaultDB>) -> Vec<u8> {
    let mut result = Vec::new();
    tagged_serialize(state, &mut result).unwrap();
    result
}

fn nested(mut state: StateValue<DefaultDB>, depth: usize) -> StateValue<DefaultDB> {
    for _ in 0..depth {
        state = StateValue::Array(vec![state].into());
    }
    state
}

fn assert_bounds<T>(result: Result<T, TranscriptRejected<DefaultDB>>) {
    assert!(matches!(
        result,
        Err(TranscriptRejected::Execution(
            OnchainProgramError::BoundsExceeded
        ))
    ));
}

fn assert_query_error<T>(result: Result<T, CompactError>, expected: &str) {
    match result {
        Err(CompactError::LedgerQueryRejected(message)) => assert_eq!(message, expected),
        Err(other) => panic!("wrong error: {other:?}"),
        Ok(_) => panic!("expected path rejection"),
    }
}

#[test]
fn public_mutations_reject_first_unencodable_path_before_state_lookup() {
    // Null deliberately cannot satisfy any lookup. BoundsExceeded must arise
    // from the path itself rather than a later state-shape failure.
    let query = context();
    let before = bytes(query.state.get_ref());
    assert_bounds(ledger::merkle_insert(
        &query,
        &[0; 15],
        true,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::historic_insert(
        &query,
        &[0; 15],
        true,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::merkle_insert_index(
        &query,
        &[0; 16],
        true,
        0,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::historic_insert_index(
        &query,
        &[0; 15],
        true,
        0,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::historic_reset_history(
        &query,
        &[0; 14],
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::merkle_reset_to_default(
        &query,
        &[0; 17],
        3,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::historic_reset_to_default(
        &query,
        &[0; 17],
        3,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::push_front_list(
        &query,
        &[0; 15],
        true,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::pop_front_list(
        &query,
        &[0; 16],
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::reset_list(
        &query,
        &[0; 17],
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::insert_set(
        &query,
        &[0; 16],
        true,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::remove_set(
        &query,
        &[0; 16],
        true,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::reset_set(
        &query,
        &[0; 17],
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::insert_map(
        &query,
        &[0; 16],
        true,
        false,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::increment_counter(
        &query,
        &[0; 16],
        1,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_bounds(ledger::write_cell_at_path(
        &query,
        &[0; 17],
        true,
        None,
        &INITIAL_COST_MODEL,
    ));
    assert_eq!(bytes(query.state.get_ref()), before);
}

#[test]
fn context_and_recorded_routes_preserve_path_rejection_class() {
    let invalid = "Execution(InvalidArgs(\"ledger path contains no field index\"))";
    let bounds = "Execution(BoundsExceeded)";
    let fresh = || circuit(StateValue::Null);
    assert_query_error(fresh().merkle_reset_to_default(&[][..], 3), invalid);
    assert_query_error(fresh().historic_reset_to_default(&[][..], 3), invalid);
    assert_query_error(fresh().reset_list(&[][..]), invalid);
    assert_query_error(
        RecordingFrame::new(fresh()).reset_merkle_to_default(&[][..], 3),
        invalid,
    );
    assert_query_error(
        RecordingFrame::new(fresh()).reset_historic_merkle_to_default(&[][..], 3),
        invalid,
    );
    assert_query_error(RecordingFrame::new(fresh()).reset_list(&[][..]), invalid);

    assert_query_error(fresh().push_front_list(&[0; 15], true), bounds);
    assert_query_error(fresh().insert_set(&[0; 16], true), bounds);
    assert_query_error(fresh().write_cell_at_path(&[0; 17], true), bounds);
    assert_query_error(fresh().increment_counter(&[0; 16], 1), bounds);
    assert_query_error(
        RecordingFrame::new(fresh()).insert_merkle(&[0; 15], true),
        bounds,
    );
    assert_query_error(
        RecordingFrame::new(fresh()).insert_historic_merkle(&[0; 15], true),
        bounds,
    );
    assert_query_error(
        RecordingFrame::new(fresh()).reset_historic_merkle_history(&[0; 14]),
        bounds,
    );
    assert_query_error(
        RecordingFrame::new(fresh()).push_front_list(&[0; 15], true),
        bounds,
    );
    assert_query_error(
        RecordingFrame::new(fresh()).pop_front_list(&[0; 16]),
        bounds,
    );
    assert_query_error(
        RecordingFrame::new(fresh()).insert_set(&[0; 16], true),
        bounds,
    );
    assert_query_error(
        RecordingFrame::new(fresh()).remove_set(&[0; 16], true),
        bounds,
    );
    assert_query_error(RecordingFrame::new(fresh()).reset_set(&[0; 17]), bounds);
    assert_query_error(
        RecordingFrame::new(fresh()).insert_map(&[0; 16], true, false),
        bounds,
    );
    assert_query_error(
        RecordingFrame::new(fresh()).write_cell(&[0; 17], true),
        bounds,
    );
    assert_query_error(
        RecordingFrame::new(fresh()).increment_counter(&[0; 16], 1),
        bounds,
    );
}

#[test]
fn public_reads_reject_overwidth_indices_consistently() {
    let bounds = "Execution(BoundsExceeded)";
    let query = context();
    let path = &[0; 17];
    assert_query_error(
        ledger::query_cell_at_path::<bool, _>(&query, path, None, &INITIAL_COST_MODEL),
        bounds,
    );
    assert_query_error(
        ledger::length_list(&query, path, None, &INITIAL_COST_MODEL),
        bounds,
    );
    assert_query_error(
        ledger::member_set(&query, path, true, None, &INITIAL_COST_MODEL),
        bounds,
    );
    assert_query_error(
        ledger::lookup_map::<_, bool, _>(&query, path, true, None, &INITIAL_COST_MODEL),
        bounds,
    );
    assert_query_error(
        ledger::historic_is_full(&query, path, 3, None, &INITIAL_COST_MODEL),
        bounds,
    );
    assert_query_error(
        ledger::merkle_check_root(&query, path, 0_u8, None, &INITIAL_COST_MODEL),
        bounds,
    );
    let fresh = || RecordingFrame::new(circuit(StateValue::Null));
    assert_query_error(fresh().read_cell::<bool>(path), bounds);
    assert_query_error(fresh().length_list(path), bounds);
    assert_query_error(fresh().member_set(path, true), bounds);
    assert_query_error(fresh().lookup_map::<_, bool>(path, true), bounds);
    assert_query_error(fresh().merkle_is_full(path, 3), bounds);
    assert_query_error(fresh().historic_merkle_check_root(path, 0_u8), bounds);
}

#[test]
fn valid_merkle_resets_preserve_state_gas_and_recorded_replay() {
    for depth in [1, 2, 3] {
        let path = vec![0; depth];
        for historic in [false, true] {
            let blank = if historic {
                ledger::constructor_historic_merkle_tree(3)
            } else {
                ledger::constructor_merkle_tree(3)
            };
            let initial = nested(blank, depth);
            let expected = bytes(&initial);
            let native = circuit(initial.clone());
            let mut native_gas = Default::default();
            let native = if historic {
                let append = native.historic_insert(path.as_slice(), true).unwrap();
                native_gas += append.gas_cost;
                append
                    .context
                    .historic_reset_to_default(path.as_slice(), 3)
                    .unwrap()
            } else {
                let append = native.merkle_insert(path.as_slice(), true).unwrap();
                native_gas += append.gas_cost;
                append
                    .context
                    .merkle_reset_to_default(path.as_slice(), 3)
                    .unwrap()
            };
            native_gas += native.gas_cost;
            let frame = RecordingFrame::new(circuit(initial));
            let frame = if historic {
                frame
                    .insert_historic_merkle(path.as_slice(), true)
                    .unwrap()
                    .reset_historic_merkle_to_default(path.as_slice(), 3)
                    .unwrap()
            } else {
                frame
                    .insert_merkle(path.as_slice(), true)
                    .unwrap()
                    .reset_merkle_to_default(path.as_slice(), 3)
                    .unwrap()
            };
            let recorded = frame.finish(());
            let replay = recorded
                .public
                .initial()
                .query(recorded.public.verify_ops(), None, &INITIAL_COST_MODEL)
                .unwrap();
            assert_eq!(bytes(native.context.query.state.get_ref()), expected);
            assert_eq!(
                bytes(recorded.execution.context.query.state.get_ref()),
                expected
            );
            assert_eq!(bytes(replay.context.state.get_ref()), expected);
            assert_eq!(
                recorded.execution.context.query.effects,
                native.context.query.effects
            );
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(recorded.execution.gas_cost, native_gas);
        }
    }
}

#[test]
fn root_mutations_remain_usable_without_a_field_index() {
    for historic in [false, true] {
        let root = if historic {
            ledger::constructor_historic_merkle_tree(3)
        } else {
            ledger::constructor_merkle_tree(3)
        };
        let original = bytes(&root);
        let native = circuit(root.clone());
        let native = if historic {
            native.historic_insert(&[][..], true).unwrap()
        } else {
            native.merkle_insert(&[][..], true).unwrap()
        };
        let frame = RecordingFrame::new(circuit(root));
        let recorded = if historic {
            frame.insert_historic_merkle(&[][..], true).unwrap()
        } else {
            frame.insert_merkle(&[][..], true).unwrap()
        }
        .finish(());
        let changed = bytes(native.context.query.state.get_ref());
        assert_ne!(changed, original);
        assert_eq!(
            bytes(recorded.execution.context.query.state.get_ref()),
            changed
        );
        let replay = recorded
            .public
            .initial()
            .query(recorded.public.verify_ops(), None, &INITIAL_COST_MODEL)
            .unwrap();
        assert_eq!(bytes(replay.context.state.get_ref()), changed);
        if historic {
            let native = native.context.historic_reset_history(&[][..]).unwrap();
            let recorded = RecordingFrame::new(recorded.execution.context)
                .reset_historic_merkle_history(&[][..])
                .unwrap()
                .finish(());
            assert_eq!(
                bytes(recorded.execution.context.query.state.get_ref()),
                bytes(native.context.query.state.get_ref())
            );
        }
    }
    let set = circuit(ledger::constructor_set());
    let set = set.insert_set(&[][..], true).unwrap().context;
    assert_eq!(
        ledger::size_set(&set.query, &[][..], None, &INITIAL_COST_MODEL)
            .unwrap()
            .1,
        1
    );
    let list = circuit(ledger::constructor_list());
    let list = list.push_front_list(&[][..], true).unwrap().context;
    assert_eq!(
        ledger::length_list(&list.query, &[][..], None, &INITIAL_COST_MODEL)
            .unwrap()
            .1,
        1
    );
}
