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

//! Rejection must precede the VM query, including where a normalized key could alias.

use midnight_compact_runtime::context::{CircuitContext, ConstructorContext, ConstructorResult};
use midnight_compact_runtime::ledger::{
    self, CellValue, ChargedState, ContractAddress, DefaultDB, StateValue,
};
use midnight_compact_runtime::recording::RecordingFrame;
use midnight_compact_runtime::{BoundedUint, CompactError, OpaqueBytes, OpaqueString};

fn context(state: StateValue<DefaultDB>) -> CircuitContext<()> {
    ConstructorResult::new(ConstructorContext::new(()), ChargedState::new(state))
        .into_circuit_context(ContractAddress::default())
}

// A query against this absent slot would fail. A raw-input error demonstrates
// rejection before that query; it does not claim rollback of earlier operations.
fn absent() -> CircuitContext<()> {
    context(StateValue::Null)
}
fn frame() -> RecordingFrame<()> {
    RecordingFrame::new(absent())
}
fn invalid<T>(result: Result<T, CompactError>) {
    assert!(
        matches!(result.err(), Some(CompactError::InvalidLedgerCell(message))
        if message == "opaque ledger input has trailing zero bytes")
    );
}

fn check_collection_inputs<T: CellValue + Clone>(value: T) {
    invalid(absent().insert_set(&[0], value.clone()));
    invalid(frame().insert_set(&[0], value.clone()));
    invalid(absent().remove_set(&[0], value.clone()));
    invalid(frame().remove_set(&[0], value.clone()));
    invalid(absent().member_set(&[0], value.clone()));
    invalid(frame().member_set(&[0], value.clone()));
    invalid(absent().insert_map(&[0], value.clone(), true));
    invalid(frame().insert_map(&[0], value.clone(), true));
    invalid(absent().insert_map(&[0], true, value.clone()));
    invalid(frame().insert_map(&[0], true, value.clone()));
    invalid(absent().remove_map(&[0], value.clone()));
    invalid(frame().remove_map(&[0], value.clone()));
    invalid(absent().member_map(&[0], value.clone()));
    invalid(frame().member_map(&[0], value.clone()));
    invalid(absent().lookup_map::<_, bool>(&[0], value.clone()));
    invalid(frame().lookup_map::<_, bool>(&[0], value.clone()));
    invalid(absent().push_front_list(&[0], value.clone()));
    invalid(frame().push_front_list(&[0], value));
}

#[test]
fn raw_and_nested_opaque_collection_inputs_fail_before_query() {
    check_collection_inputs(OpaqueBytes(vec![7, 0]));
    check_collection_inputs(OpaqueString::from("secret\0"));
    check_collection_inputs((true, OpaqueBytes(vec![0])));
}

fn check_tree_inputs<T: CellValue + Clone>(value: T) {
    let position = BoundedUint::new(0).unwrap();
    invalid(absent().merkle_insert(&[0], value.clone()));
    invalid(frame().insert_merkle(&[0], value.clone()));
    invalid(absent().historic_insert(&[0], value.clone()));
    invalid(frame().insert_historic_merkle(&[0], value.clone()));
    invalid(absent().merkle_insert_index(&[0], value.clone(), position));
    invalid(frame().insert_merkle_index(&[0], value.clone(), position));
    invalid(absent().historic_insert_index(&[0], value.clone(), position));
    invalid(frame().insert_historic_merkle_index(&[0], value.clone(), position));
    invalid(absent().merkle_check_root(&[0], value.clone()));
    invalid(frame().merkle_check_root(&[0], value.clone()));
    invalid(absent().historic_check_root(&[0], value.clone()));
    invalid(frame().historic_merkle_check_root(&[0], value));
}

#[test]
fn typed_merkle_leaves_and_roots_fail_before_normalizing_or_querying() {
    check_tree_inputs(OpaqueBytes(vec![7, 0]));
    check_tree_inputs(OpaqueString::from("secret\0"));
    check_tree_inputs((true, OpaqueBytes(vec![0])));
}

#[test]
fn invalid_keys_do_not_alias_an_existing_canonical_map_key() {
    let initial = context(StateValue::Array(vec![ledger::constructor_map()].into()));
    let native = initial
        .insert_map(&[0], OpaqueBytes(vec![7]), true)
        .unwrap();
    let canonical = native
        .context
        .lookup_map::<_, bool>(&[0], OpaqueBytes(vec![7]))
        .unwrap();
    assert!(canonical.result);
    let query = canonical.context.query.clone();
    invalid(
        canonical
            .context
            .lookup_map::<_, bool>(&[0], OpaqueBytes(vec![7, 0])),
    );
    // The retained persistent query snapshot still contains the original entry.
    let (_, value) = ledger::lookup_map::<_, bool, _>(
        &query,
        &[0],
        OpaqueBytes(vec![7]),
        None,
        &midnight_onchain_vm::cost_model::INITIAL_COST_MODEL,
    )
    .unwrap();
    assert!(value);
}
