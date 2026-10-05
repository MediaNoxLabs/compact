// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0

use std::cell::RefCell;

use compact_rust_test_center_welcome_fixture::ledger_contract::{
    LedgerView, PublicStateView, Witnesses, initial_state,
};
use compact_rust_test_center_welcome_fixture::types::{Maybe, MaybeCompact1};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{DefaultDB, StateValue};
use runtime::{FixedBytes, FixedVector, OpaqueString};

#[derive(Default)]
struct OrganizerWitness(RefCell<Vec<u64>>);

impl Witnesses<u64> for OrganizerWitness {
    fn local_sk(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, MaybeCompact1) {
        self.0.borrow_mut().push(*context.private_state);
        (
            *context.private_state,
            MaybeCompact1 {
                is_some: true,
                value: FixedBytes::new([0; 32]),
            },
        )
    }

    fn set_local_id(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        _participant: OpaqueString,
    ) -> (u64, ()) {
        (*context.private_state, ())
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["add_participant", "add_organizer", "check_in"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn original_welcome_constructor_matches_typescript_empty_and_one_participant() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/test-center-welcome.json"
    ))
    .unwrap();
    assert_eq!(
        oracle["source"],
        "test-center/test-contracts/welcome.compact"
    );
    for case in oracle["cases"].as_array().unwrap() {
        let present = case["present"].as_bool().unwrap();
        let participants = FixedVector::new(std::array::from_fn(|index| Maybe {
            is_some: present && index == 7,
            value: OpaqueString::from(if present && index == 7 { "alice" } else { "" }),
        }));
        let witness = OrganizerWitness::default();
        let initial =
            initial_state(ConstructorContext::new(7_u64), &witness, participants).unwrap();
        assert_eq!(
            serde_json::to_value(witness.0.borrow().as_slice()).unwrap(),
            case["witnessCalls"]
        );
        assert_eq!(
            initial.private_state,
            case["privateState"].as_u64().unwrap()
        );
        assert_eq!(
            state_hex(initial.ledger_state.get_ref().clone()),
            case["stateHex"]
        );
        let view = PublicStateView::from(&initial);
        assert_eq!(
            view.eligible_participants()
                .unwrap()
                .member(OpaqueString::from("alice")),
            case["eligibleAlice"].as_bool().unwrap()
        );
        assert_eq!(
            view.eligible_participants()
                .unwrap()
                .size()
                .unwrap()
                .value(),
            case["eligibleSize"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
        );
        assert_eq!(
            view.organizer_pks().unwrap().size().unwrap().value(),
            case["organizerSize"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
        );
        // The checked TS fixture also keeps ordered VM tags and all four gas
        // dimensions. ConstructorResult does not expose these to Rust callers,
        // so this test asserts the observable state/private boundary only.
        assert_eq!(
            case["constructorQueries"].as_array().unwrap().len(),
            if present { 5 } else { 4 }
        );
    }
}
