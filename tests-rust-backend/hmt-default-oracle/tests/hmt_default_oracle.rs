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

use compact_rust_hmt_default_oracle_fixture::ledger_contract::{
    add_default, initial_state, recorded,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let operations = operations.insert(
        EntryPointBuf(b"add_default".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn historic_merkle_tree_matches_typescript_after_each_insert() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-default-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let state = initial.ledger_state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterInit"]);
    let tree = runtime::ledger::historic_merkle_tree_view_at_path(state, &[0]).unwrap();
    assert_eq!(tree.first_free().unwrap().value(), 0);
    assert!(tree.root().is_some());

    let context = initial.into_circuit_context(ContractAddress::default());
    let after_zero = add_default(context, runtime::BoundedUint::new(0).unwrap()).unwrap();
    let state = after_zero.context.query.state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterAddDefault0"]);
    assert_eq!(
        runtime::ledger::historic_merkle_tree_view_at_path(state, &[0])
            .unwrap()
            .first_free()
            .unwrap()
            .value(),
        1
    );

    let after_two = add_default(after_zero.context, runtime::BoundedUint::new(2).unwrap()).unwrap();
    let state = after_two.context.query.state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterAddDefault2"]);
    assert_eq!(
        runtime::ledger::historic_merkle_tree_view_at_path(state, &[0])
            .unwrap()
            .first_free()
            .unwrap()
            .value(),
        3
    );

    // A lower index follows the VM's swap/pop path and preserves first_free.
    let repeated = add_default(after_two.context, runtime::BoundedUint::new(0).unwrap()).unwrap();
    let state = repeated.context.query.state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterRepeat0"]);
    assert_eq!(
        runtime::ledger::historic_merkle_tree_view_at_path(state, &[0])
            .unwrap()
            .first_free()
            .unwrap()
            .value(),
        3
    );
}

#[test]
fn recorded_historic_default_indexed_insertion_matches_typescript_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-default-oracle.json"
    ))
    .unwrap();
    for (prior, index, state, query, first_free) in [
        (&[][..], 0, "afterAddDefault0", "addDefault0", 1),
        (&[0][..], 2, "afterAddDefault2", "addDefault2", 3),
        (&[0, 2][..], 0, "afterRepeat0", "repeat0", 3),
    ] {
        let start = || {
            let mut context = initial_state(ConstructorContext::new(()))
                .unwrap()
                .into_circuit_context(ContractAddress::default());
            for preceding in prior {
                context = add_default(context, runtime::BoundedUint::new(*preceding).unwrap())
                    .unwrap()
                    .context;
            }
            context
        };
        let index = runtime::BoundedUint::new(index).unwrap();
        let native = add_default(start(), index).unwrap();
        let recorded = recorded::add_default(start(), index).unwrap();
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert!(recorded.execution.private_transcript_outputs.is_empty());
        let value = recorded.execution.context.query.state.get_ref();
        assert_eq!(state_hex(value.clone()), oracle[state]);
        assert_eq!(
            state_hex(native.context.query.state.get_ref().clone()),
            state_hex(value.clone())
        );
        assert_eq!(
            runtime::ledger::historic_merkle_tree_view_at_path(value, &[0])
                .unwrap()
                .first_free()
                .unwrap()
                .value(),
            first_free
        );
        let captured = &oracle["nativeQueries"][query];
        assert_eq!(captured["result"], serde_json::json!([]));
        assert_eq!(captured["privateOutputs"], 0);
        let queries = captured["queries"].as_array().unwrap();
        assert_eq!(queries.len(), 1);
        let actual_gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                actual_gas[key].as_u64().unwrap(),
                queries[0]["gasCost"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap(),
                "{query}: {key}"
            );
            assert_eq!(captured["reportedGas"][key], queries[0]["gasCost"][key]);
        }
        assert_eq!(
            serde_json::to_value(recorded.public.verify_ops()).unwrap(),
            queries[0]["program"]
        );
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(replay.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            replay.context.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            state_hex(replay.context.state.get_ref().clone()),
            oracle[state]
        );
    }
}
