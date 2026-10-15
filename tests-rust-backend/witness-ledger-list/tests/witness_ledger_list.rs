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

use compact_rust_witness_ledger_list_fixture::ledger_contract::{
    LedgerView, Witnesses, drop_first, initial_state, prepend, private_first_is_42,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{
    CircuitContext, CircuitResult, ConstructorContext, RunningCost, WitnessContext,
    WitnessReadMeter,
};
use midnight_compact_runtime::ledger::{
    ContractAddress, DefaultDB, StateValue, head_list, metered_list_view,
};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["prepend", "drop_first"] {
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

struct FirstIs42;

impl Witnesses<u64> for FirstIs42 {
    fn first_is_42(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, bool) {
        let list = context.ledger.items().unwrap();
        let head = list.head().unwrap();
        let empty = list.is_empty().unwrap();
        assert_eq!(empty, list.length().unwrap().value() == 0);
        assert_eq!(empty, head.is_none());
        (
            *context.private_state + 1,
            head == Some(Field::from(42_u64)),
        )
    }
}

fn assert_oracle_output(result: &CircuitResult<u64, bool>, oracle: &serde_json::Value) {
    assert_eq!(result.result, oracle["result"].as_bool().unwrap());
    assert_eq!(
        result.context.private_state,
        oracle["privateState"].as_u64().unwrap()
    );
    assert_eq!(result.private_transcript_outputs.len(), 1);
    let output = &result.private_transcript_outputs[0];
    let atoms = output
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        oracle["privateTranscriptOutputs"][0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&output.alignment).unwrap(),
        oracle["privateTranscriptOutputs"][0]["alignment"]
    );
    let actual_gas = serde_json::to_value(result.gas_cost).unwrap();
    let queries = oracle["queryCosts"].as_array().unwrap();
    assert_eq!(queries.len(), 3, "head, isEmpty, and length queries");
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let total: u64 = queries
            .iter()
            .map(|query| query[key].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(actual_gas[key].as_u64().unwrap(), total, "{key} total gas");
    }
}

fn assert_projection_costs(
    context: &CircuitContext<u64>,
    oracle: &serde_json::Value,
    head: Option<Field>,
) {
    let meter = WitnessReadMeter::new(context);
    let list = metered_list_view::<Field, _>(&meter, 0).unwrap();
    let queries = oracle["queryCosts"].as_array().unwrap();
    let assert_prefix = |count: usize| {
        let total = serde_json::to_value(meter.gas_cost()).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected: u64 = queries[..count]
                .iter()
                .map(|query| query[key].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(
                total[key].as_u64().unwrap(),
                expected,
                "{key} query {count}"
            );
        }
    };
    assert_eq!(list.head().unwrap(), head);
    assert_prefix(1);
    assert_eq!(list.is_empty().unwrap(), head.is_none());
    assert_prefix(2);
    assert_eq!(list.length().unwrap().value() == 0, head.is_none());
    assert_prefix(3);
}

#[test]
fn witness_reads_current_typed_list() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-ledger-list-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    assert_eq!(
        state_hex(context.query.state.get_ref().clone()),
        oracle["initialState"]
    );
    assert_projection_costs(&context, &oracle["before"], None);
    let before = private_first_is_42(context, &FirstIs42).unwrap();
    assert_oracle_output(&before, &oracle["before"]);
    let write = prepend(before.context, Field::from(42_u64)).unwrap();
    assert_eq!(
        state_hex(write.context.query.state.get_ref().clone()),
        oracle["afterState"]
    );
    assert!(write.private_transcript_outputs.is_empty());
    assert_projection_costs(&write.context, &oracle["after"], Some(Field::from(42_u64)));
    let after = private_first_is_42(write.context, &FirstIs42).unwrap();
    assert_oracle_output(&after, &oracle["after"]);
    let write = prepend(after.context, Field::from(7_u64)).unwrap();
    assert_eq!(
        state_hex(write.context.query.state.get_ref().clone()),
        oracle["coveredState"]
    );
    assert_projection_costs(&write.context, &oracle["covered"], Some(Field::from(7_u64)));
    let covered = private_first_is_42(write.context, &FirstIs42).unwrap();
    assert_oracle_output(&covered, &oracle["covered"]);
    let write = drop_first(covered.context).unwrap();
    assert_eq!(
        state_hex(write.context.query.state.get_ref().clone()),
        oracle["restoredState"]
    );
    assert_projection_costs(
        &write.context,
        &oracle["restored"],
        Some(Field::from(42_u64)),
    );
    let restored = private_first_is_42(write.context, &FirstIs42).unwrap();
    assert_oracle_output(&restored, &oracle["restored"]);
}

#[test]
fn repeated_list_head_reads_charge_each_vm_query() {
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let (query, head): (_, (bool, Field)) = head_list::<Field, (bool, Field), _>(
        &context.query,
        0,
        context.gas_limit,
        &context.cost_model,
    )
    .unwrap();
    assert!(!head.0);
    let cost = query.gas_cost;
    let meter = WitnessReadMeter::new(&context);
    let list = metered_list_view::<Field, _>(&meter, 0).unwrap();
    assert_eq!(list.head().unwrap(), None);
    assert_eq!(list.head().unwrap(), None);
    assert_eq!(meter.gas_cost(), cost + cost);
}

#[test]
fn invalid_list_index_does_not_charge_meter() {
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let meter = WitnessReadMeter::new(&context);
    assert!(metered_list_view::<Field, _>(&meter, 1).is_err());
    assert!(meter.read_list_head::<Field>(1).is_err());
    assert_eq!(meter.gas_cost(), RunningCost::ZERO);
}

// Qualify newly recorded witness-only exports against the retained TS oracle.
mod recording_parity {
    use super::*;
    use compact_rust_witness_ledger_list_fixture::ledger_contract as contract;
    use midnight_compact_runtime as runtime;
    use std::cell::RefCell;

    struct Logged<'a>(&'a RefCell<Vec<u64>>);
    impl Witnesses<u64> for Logged<'_> {
        fn first_is_42(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, bool) {
            self.0.borrow_mut().push(*context.private_state);
            Witnesses::first_is_42(&FirstIs42, context)
        }
    }
    fn same_execution<T: std::fmt::Debug + PartialEq>(
        native: &CircuitResult<u64, T>,
        actual: &CircuitResult<u64, T>,
    ) {
        assert_eq!(native.result, actual.result);
        assert_eq!(native.context.private_state, actual.context.private_state);
        assert_eq!(
            native.context.query.state.get_ref(),
            actual.context.query.state.get_ref()
        );
        assert_eq!(native.context.query.effects, actual.context.query.effects);
        assert_eq!(native.gas_cost, actual.gas_cost);
        assert_eq!(
            native.private_transcript_outputs,
            actual.private_transcript_outputs
        );
    }

    #[test]
    fn recorded_and_observed_witness_reads_match_native_and_ts() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../runtime-rs/tests/fixtures/witness-ledger-list-ts-output.json"
        ))
        .unwrap();
        for (populated, key) in [(false, "before"), (true, "after")] {
            let private = if populated { 8_u64 } else { 7_u64 };
            let context = initial_state(ConstructorContext::new(private))
                .unwrap()
                .into_circuit_context(ContractAddress::default());
            let context = if populated {
                prepend(context, Field::from(42_u64)).unwrap().context
            } else {
                context
            };
            let state = runtime::ledger::ContractState::new(
                context.query.state.get_ref().clone(),
                Default::default(),
                Default::default(),
            );
            let context = || {
                runtime::context::CircuitContext::from_contract_state(
                    private,
                    ContractAddress::default(),
                    &state,
                )
            };
            let native_calls = RefCell::new(Vec::new());
            let native = private_first_is_42(context(), &Logged(&native_calls)).unwrap();
            assert_eq!(*native_calls.borrow(), [private]);
            assert_oracle_output(&native, &oracle[key]);
            let recorded_calls = RefCell::new(Vec::new());
            let recorded =
                contract::recorded::private_first_is_42(context(), &Logged(&recorded_calls))
                    .unwrap();
            assert_eq!(*recorded_calls.borrow(), [private]);
            same_execution(&native, &recorded.execution);
            assert_oracle_output(&recorded.execution, &oracle[key]);
            // Witness ledger queries are metered privately; they do not invent public VM ops.
            assert!(recorded.public.verify_ops().is_empty());
            #[cfg(feature = "ledger-transaction")]
            {
                use runtime::transaction::{Observation, ObservedContractState};
                let observed = ObservedContractState::new(
                    ContractAddress::default(),
                    state.clone(),
                    Observation {
                        transaction_hash: [0; 32],
                        block_hash: [1; 32],
                        block_height: 1,
                    },
                );
                let observed_calls = RefCell::new(Vec::new());
                let contract = contract::Contract::from(Logged(&observed_calls));
                let call = contract
                    .recording()
                    .private_first_is_42_call(&observed, private)
                    .unwrap();
                assert_eq!(*observed_calls.borrow(), [private]);
                same_execution(&native, &call.recorded().execution);
                assert_oracle_output(&call.recorded().execution, &oracle[key]);
                assert!(call.recorded().public.verify_ops().is_empty());
            }
        }
    }
}
