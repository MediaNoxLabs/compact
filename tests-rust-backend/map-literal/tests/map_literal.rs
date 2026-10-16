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

use compact_rust_map_literal_fixture::ledger_contract::{initial_state, lookup_true, put_literal};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::ContractAddress;

#[test]
fn compiler_typed_local_flows_into_map_insert() {
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let write = put_literal(context).unwrap();
    let read = lookup_true(write.context).unwrap();
    assert_eq!(read.result, Field::from(42_u64));
}

#[cfg(feature = "ledger-transaction")]
#[test]
fn recorded_literal_insert_and_replacement_match_manual_slots_and_observation() {
    use compact_rust_map_literal_fixture::{ledger_contract as c, ledger_slots as slots};
    use midnight_compact_runtime as r;
    use r::recording::RecordingFrame;
    use r::transaction::{Observation, ObservedContractState};
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let mut public = initial.ledger_state.get_ref().clone();
    for _ in 0..2 {
        let observed = ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(public, Default::default(), Default::default()),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let native = put_literal(observed.circuit_context(())).unwrap();
        let recorded = c::recorded::put_literal(observed.circuit_context(())).unwrap();
        let manual = slots::table
            .record_insert(
                RecordingFrame::new(observed.circuit_context(())),
                true,
                Field::from(42_u64),
            )
            .unwrap()
            .finish(());
        let call = c::recorded::Contract
            .put_literal_call(&observed, ())
            .unwrap();
        // Verification replay groups queries differently from native gathering.
        // Compare its cost against independent manual verification replay, while
        // execution gas above/below remains equal to native execution gas.
        let manual_replay = manual
            .public
            .initial()
            .query(manual.public.verify_ops(), None, &native.context.cost_model)
            .unwrap();
        for actual in [&recorded, &manual, call.recorded()] {
            let _: () = actual.execution.result;
            assert_eq!(
                actual.execution.context.query.state,
                native.context.query.state
            );
            assert_eq!(
                actual.execution.context.query.effects,
                native.context.query.effects
            );
            let _: () = actual.execution.context.private_state;
            assert_eq!(actual.execution.gas_cost, native.gas_cost);
            assert_eq!(
                actual.execution.private_transcript_outputs,
                native.private_transcript_outputs
            );
            assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
            let replay = actual
                .public
                .initial()
                .query(actual.public.verify_ops(), None, &native.context.cost_model)
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(replay.gas_cost, manual_replay.gas_cost);
        }
        let table = slots::table
            .inspect(native.context.query.state.get_ref())
            .unwrap();
        assert_eq!(table.size().unwrap().value(), 1);
        assert_eq!(table.lookup(true).unwrap(), Field::from(42_u64));
        assert!(!table.try_member(false).unwrap());
        public = native.context.query.state.get_ref().clone();
    }
}
